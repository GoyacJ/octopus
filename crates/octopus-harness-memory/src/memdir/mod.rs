use std::ops::RangeInclusive;
use std::path::PathBuf;
#[cfg(feature = "threat-scanner")]
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
#[cfg(feature = "threat-scanner")]
use harness_contracts::ThreatAction;
use harness_contracts::{ContentHash, MemoryError, TakesEffect, TenantId};

#[cfg(feature = "threat-scanner")]
use crate::MemoryThreatScanner;

mod fence;
mod file;
mod lock;

pub use fence::{escape_for_fence, sanitize_context, wrap_memory_context};
pub use harness_contracts::MemdirFileTag as MemdirFile;

#[derive(Debug, Clone)]
pub struct BuiltinMemory {
    root: PathBuf,
    tenant_id: TenantId,
    max_chars_memory: usize,
    max_chars_user: usize,
    section_separator: String,
    snapshot_strategy: SnapshotStrategy,
    concurrency: MemdirConcurrencyPolicy,
    #[cfg(feature = "threat-scanner")]
    threat_scanner: Option<Arc<MemoryThreatScanner>>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SnapshotStrategy {
    None,
    DailyOnFirstWrite,
    BeforeEachReplace,
}

#[derive(Debug, Clone)]
pub struct MemdirSnapshot {
    pub memory: String,
    pub user: String,
    pub memory_chars: usize,
    pub user_chars: usize,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MemdirWriteOutcome {
    pub bytes_written: u64,
    pub previous_hash: ContentHash,
    pub new_hash: ContentHash,
    pub snapshot_path: Option<PathBuf>,
    pub takes_effect: TakesEffect,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MemdirConcurrencyPolicy {
    pub lock_timeout: Duration,
    pub retry_max: u32,
    pub retry_jitter_ms: RangeInclusive<u64>,
}

impl Default for MemdirConcurrencyPolicy {
    fn default() -> Self {
        Self {
            lock_timeout: Duration::from_secs(2),
            retry_max: 5,
            retry_jitter_ms: 20..=150,
        }
    }
}

impl BuiltinMemory {
    pub fn at(root: impl Into<PathBuf>, tenant_id: TenantId) -> Self {
        Self {
            root: root.into(),
            tenant_id,
            max_chars_memory: 16_000,
            max_chars_user: 8_000,
            section_separator: "§".to_owned(),
            snapshot_strategy: SnapshotStrategy::None,
            concurrency: MemdirConcurrencyPolicy::default(),
            #[cfg(feature = "threat-scanner")]
            threat_scanner: None,
        }
    }

    #[must_use]
    pub const fn with_limits(mut self, memory_chars: usize, user_chars: usize) -> Self {
        self.max_chars_memory = memory_chars;
        self.max_chars_user = user_chars;
        self
    }

    #[must_use]
    pub const fn with_snapshot_strategy(mut self, strategy: SnapshotStrategy) -> Self {
        self.snapshot_strategy = strategy;
        self
    }

    #[must_use]
    pub fn with_concurrency_policy(mut self, policy: MemdirConcurrencyPolicy) -> Self {
        self.concurrency = policy;
        self
    }

    #[cfg(feature = "threat-scanner")]
    #[must_use]
    pub fn with_threat_scanner(mut self, scanner: Arc<MemoryThreatScanner>) -> Self {
        self.threat_scanner = Some(scanner);
        self
    }

    pub async fn read_all(&self) -> Result<MemdirSnapshot, MemoryError> {
        let this = self.clone();
        spawn_memdir(move || file::read_all(&this)).await
    }

    pub async fn append_section(
        &self,
        file: MemdirFile,
        section: &str,
        content: &str,
    ) -> Result<MemdirWriteOutcome, MemoryError> {
        let this = self.clone();
        let section = section.to_owned();
        let content = content.to_owned();
        spawn_memdir(move || {
            file::write_section(&this, file, &section, &content, file::Edit::Append)
        })
        .await
    }

    pub async fn replace_section(
        &self,
        file: MemdirFile,
        section: &str,
        content: &str,
    ) -> Result<MemdirWriteOutcome, MemoryError> {
        let this = self.clone();
        let section = section.to_owned();
        let content = content.to_owned();
        spawn_memdir(move || {
            file::write_section(&this, file, &section, &content, file::Edit::Replace)
        })
        .await
    }

    pub async fn delete_section(
        &self,
        file: MemdirFile,
        section: &str,
    ) -> Result<MemdirWriteOutcome, MemoryError> {
        let this = self.clone();
        let section = section.to_owned();
        spawn_memdir(move || file::write_section(&this, file, &section, "", file::Edit::Delete))
            .await
    }

    pub(crate) fn tenant_dir(&self) -> PathBuf {
        self.root.join(self.tenant_id.to_string())
    }

    pub(crate) const fn snapshot_strategy(&self) -> SnapshotStrategy {
        self.snapshot_strategy
    }

    pub(crate) const fn concurrency(&self) -> &MemdirConcurrencyPolicy {
        &self.concurrency
    }

    pub(crate) fn separator(&self) -> &str {
        &self.section_separator
    }

    pub(crate) const fn limit_for(&self, file: MemdirFile) -> usize {
        match file {
            MemdirFile::User => self.max_chars_user,
            _ => self.max_chars_memory,
        }
    }

    #[cfg(feature = "threat-scanner")]
    pub(crate) fn scan_content_before_write(
        &self,
        content: &str,
    ) -> Result<Option<String>, MemoryError> {
        let Some(scanner) = &self.threat_scanner else {
            return Ok(None);
        };

        let report = scanner.scan(content);
        if report.action == ThreatAction::Block {
            let detail = report
                .hits
                .first()
                .map(|hit| format!("{} {:?}", hit.pattern_id, hit.category))
                .unwrap_or_else(|| "unknown pattern".to_owned());
            return Err(MemoryError::Message(format!(
                "memory threat detected: {detail}"
            )));
        }

        if report.action == ThreatAction::Redact {
            return Ok(report.redacted_content);
        }

        Ok(None)
    }
}

async fn spawn_memdir<T>(
    op: impl FnOnce() -> Result<T, MemoryError> + Send + 'static,
) -> Result<T, MemoryError>
where
    T: Send + 'static,
{
    tokio::task::spawn_blocking(op)
        .await
        .map_err(|error| MemoryError::Message(format!("memdir task failed: {error}")))?
}
