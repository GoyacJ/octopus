use std::sync::Arc;
use std::time::Duration;

use harness_contracts::MemoryError;
#[cfg(feature = "threat-scanner")]
use harness_contracts::ThreatAction;
use tokio::sync::{watch, Mutex, RwLock};
use tokio::time::timeout;

#[cfg(feature = "threat-scanner")]
use crate::MemoryThreatScanner;
use crate::{
    visibility_matches, MemoryKindFilter, MemoryProvider, MemoryQuery, MemoryRecord,
    MemoryVisibilityFilter,
};

pub struct MemoryManager {
    external: RwLock<Option<Arc<dyn MemoryProvider>>>,
    recall_policy: RecallPolicy,
    recall_gate: Mutex<Option<TurnRecallGate>>,
    #[cfg(feature = "threat-scanner")]
    threat_scanner: Option<Arc<MemoryThreatScanner>>,
}

type RecallResult = Result<Vec<MemoryRecord>, MemoryError>;

struct TurnRecallGate {
    turn: u64,
    phase: TurnRecallPhase,
}

enum TurnRecallPhase {
    InFlight(watch::Receiver<Option<RecallResult>>),
    Completed,
}

enum RecallGateAction {
    Lead(watch::Sender<Option<RecallResult>>),
    Wait(watch::Receiver<Option<RecallResult>>),
    Skip,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecallPolicy {
    pub max_records_per_turn: u32,
    pub max_chars_per_turn: u32,
    pub default_deadline: Duration,
    pub min_similarity: f32,
    pub fail_open: FailMode,
    pub trigger: RecallTriggerStrategy,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FailMode {
    Skip,
    Surface,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RecallTriggerStrategy {
    AlwaysOnUserMessage,
    OnQuestionMark,
    Custom(String),
}

impl Default for RecallPolicy {
    fn default() -> Self {
        Self {
            max_records_per_turn: 8,
            max_chars_per_turn: 4_000,
            default_deadline: Duration::from_millis(300),
            min_similarity: 0.65,
            fail_open: FailMode::Skip,
            trigger: RecallTriggerStrategy::AlwaysOnUserMessage,
        }
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            external: RwLock::new(None),
            recall_policy: RecallPolicy::default(),
            recall_gate: Mutex::new(None),
            #[cfg(feature = "threat-scanner")]
            threat_scanner: None,
        }
    }
}

impl MemoryManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_external(&self, provider: Arc<dyn MemoryProvider>) -> Result<(), MemoryError> {
        let mut slot = self.external.try_write().map_err(|_| {
            MemoryError::Message("external memory provider slot lock busy".to_owned())
        })?;
        if slot.is_some() {
            return Err(MemoryError::Message(
                "external memory provider slot occupied".to_owned(),
            ));
        }

        *slot = Some(provider);
        Ok(())
    }

    pub fn external(&self) -> Option<Arc<dyn MemoryProvider>> {
        self.external.try_read().ok().and_then(|slot| slot.clone())
    }

    #[must_use]
    pub fn has_external(&self) -> bool {
        self.external().is_some()
    }

    #[must_use]
    pub fn with_recall_policy(mut self, policy: RecallPolicy) -> Self {
        self.recall_policy = policy;
        self
    }

    #[cfg(feature = "threat-scanner")]
    #[must_use]
    pub fn with_threat_scanner(mut self, scanner: Arc<MemoryThreatScanner>) -> Self {
        self.threat_scanner = Some(scanner);
        self
    }

    pub async fn recall(&self, query: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        let deadline = query
            .deadline
            .unwrap_or(self.recall_policy.default_deadline);
        if deadline.is_zero() {
            return Ok(Vec::new());
        }

        let Some(provider) = self.external.read().await.clone() else {
            return Ok(Vec::new());
        };

        let mut provider_query = query.clone();
        provider_query.max_records = provider_query
            .max_records
            .min(self.recall_policy.max_records_per_turn);
        provider_query.min_similarity = provider_query
            .min_similarity
            .max(self.recall_policy.min_similarity);

        let recalled = match timeout(deadline, provider.recall(provider_query)).await {
            Ok(Ok(records)) => records,
            Ok(Err(error)) => return self.handle_recall_failure(error),
            Err(_) => {
                return self.handle_recall_failure(MemoryError::Message(
                    "memory recall deadline exceeded".to_owned(),
                ));
            }
        };

        let records = recalled
            .into_iter()
            .filter(|record| record_matches_query(record, &query))
            .take(self.recall_policy.max_records_per_turn as usize)
            .collect::<Vec<_>>();
        let records = self.scan_records(records);

        Ok(apply_char_budget(
            records,
            self.recall_policy.max_chars_per_turn,
        ))
    }

    pub async fn recall_once_per_turn(
        &self,
        turn: u64,
        query: MemoryQuery,
    ) -> Result<Vec<MemoryRecord>, MemoryError> {
        let action = {
            let mut gate = self.recall_gate.lock().await;
            match gate.as_ref() {
                Some(TurnRecallGate {
                    turn: gate_turn,
                    phase: TurnRecallPhase::InFlight(receiver),
                }) if *gate_turn == turn => RecallGateAction::Wait(receiver.clone()),
                Some(TurnRecallGate {
                    turn: gate_turn,
                    phase: TurnRecallPhase::Completed,
                }) if *gate_turn == turn => RecallGateAction::Skip,
                _ => {
                    let (sender, receiver) = watch::channel(None);
                    *gate = Some(TurnRecallGate {
                        turn,
                        phase: TurnRecallPhase::InFlight(receiver),
                    });
                    RecallGateAction::Lead(sender)
                }
            }
        };

        match action {
            RecallGateAction::Lead(sender) => {
                let result = self.recall(query).await;
                sender.send_replace(Some(result.clone()));

                let mut gate = self.recall_gate.lock().await;
                if gate.as_ref().is_some_and(|gate| gate.turn == turn) {
                    *gate = Some(TurnRecallGate {
                        turn,
                        phase: TurnRecallPhase::Completed,
                    });
                }

                result
            }
            RecallGateAction::Wait(receiver) => wait_for_recall_result(receiver).await,
            RecallGateAction::Skip => Ok(Vec::new()),
        }
    }

    fn handle_recall_failure(&self, error: MemoryError) -> Result<Vec<MemoryRecord>, MemoryError> {
        match self.recall_policy.fail_open {
            FailMode::Skip => Ok(Vec::new()),
            FailMode::Surface => Err(error),
        }
    }

    #[cfg(feature = "threat-scanner")]
    fn scan_records(&self, records: Vec<MemoryRecord>) -> Vec<MemoryRecord> {
        let Some(scanner) = &self.threat_scanner else {
            return records;
        };

        records
            .into_iter()
            .filter_map(|mut record| {
                let report = scanner.scan(&record.content);
                if report.action == ThreatAction::Block {
                    return None;
                }

                if report.action == ThreatAction::Redact {
                    if let Some(redacted_content) = report.redacted_content {
                        record.content = redacted_content;
                        record.metadata.redacted_segments += report
                            .hits
                            .iter()
                            .filter(|hit| hit.action == ThreatAction::Redact)
                            .count()
                            as u32;
                    }
                }

                Some(record)
            })
            .collect()
    }

    #[cfg(not(feature = "threat-scanner"))]
    fn scan_records(&self, records: Vec<MemoryRecord>) -> Vec<MemoryRecord> {
        records
    }
}

async fn wait_for_recall_result(
    mut receiver: watch::Receiver<Option<RecallResult>>,
) -> RecallResult {
    loop {
        if let Some(result) = receiver.borrow().clone() {
            return result;
        }

        if receiver.changed().await.is_err() {
            return Ok(Vec::new());
        }
    }
}

fn record_matches_query(record: &MemoryRecord, query: &MemoryQuery) -> bool {
    record.tenant_id == query.tenant_id
        && kind_matches(record, query.kind_filter.as_ref())
        && visibility_filter_matches(record, &query.visibility_filter)
}

fn kind_matches(record: &MemoryRecord, filter: Option<&MemoryKindFilter>) -> bool {
    match filter {
        None | Some(MemoryKindFilter::Any) => true,
        Some(MemoryKindFilter::OnlyKinds(kinds)) => kinds.contains(&record.kind),
    }
}

fn visibility_filter_matches(record: &MemoryRecord, filter: &MemoryVisibilityFilter) -> bool {
    match filter {
        MemoryVisibilityFilter::EffectiveFor(actor) => {
            record.tenant_id == actor.tenant_id && visibility_matches(&record.visibility, actor)
        }
        MemoryVisibilityFilter::Exact(visibility) => &record.visibility == visibility,
    }
}

fn apply_char_budget(records: Vec<MemoryRecord>, max_chars: u32) -> Vec<MemoryRecord> {
    let mut used = 0usize;
    let max_chars = max_chars as usize;

    records
        .into_iter()
        .filter(|record| {
            let record_chars = record.content.chars().count();
            if record_chars > max_chars || used + record_chars > max_chars {
                return false;
            }

            used += record_chars;
            true
        })
        .collect()
}
