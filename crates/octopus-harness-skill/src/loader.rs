use std::path::PathBuf;
#[cfg(feature = "threat-scanner")]
use std::sync::Arc;

use harness_contracts::{McpServerId, PluginId};
#[cfg(feature = "threat-scanner")]
use harness_memory::MemoryThreatScanner;

use crate::{
    parse_skill_markdown, McpSkillRecord, McpSource, Skill, SkillError, SkillPlatform,
    SkillRejectReason, SkillRejection, SkillSource,
};

#[derive(Debug, Clone)]
pub struct SkillLoader {
    sources: Vec<SkillSourceConfig>,
    runtime_platform: SkillPlatform,
    #[cfg(feature = "threat-scanner")]
    threat_scanner: Option<Arc<MemoryThreatScanner>>,
}

#[derive(Debug, Clone)]
pub enum SkillSourceConfig {
    BundledRecords {
        records: Vec<BundledSkillRecord>,
    },
    Directory {
        path: PathBuf,
        source_kind: DirectorySourceKind,
    },
    McpRecords {
        server_id: McpServerId,
        records: Vec<McpSkillRecord>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BundledSkillRecord {
    pub name: String,
    pub description: String,
    pub body: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DirectorySourceKind {
    Workspace,
    User,
    Plugin(PluginId),
}

#[derive(Debug, Clone)]
pub struct LoadReport {
    pub loaded: Vec<Skill>,
    pub rejected: Vec<SkillRejection>,
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            runtime_platform: current_platform(),
            #[cfg(feature = "threat-scanner")]
            threat_scanner: Some(Arc::new(MemoryThreatScanner::default())),
        }
    }
}

impl SkillLoader {
    #[must_use]
    pub fn with_source(mut self, source: SkillSourceConfig) -> Self {
        self.sources.push(source);
        self
    }

    #[must_use]
    pub fn with_runtime_platform(mut self, platform: SkillPlatform) -> Self {
        self.runtime_platform = platform;
        self
    }

    #[cfg(feature = "threat-scanner")]
    #[must_use]
    pub fn with_threat_scanner(mut self, scanner: Arc<MemoryThreatScanner>) -> Self {
        self.threat_scanner = Some(scanner);
        self
    }

    pub async fn load_all(&self) -> Result<LoadReport, SkillError> {
        let mut loaded = Vec::new();
        let mut rejected = Vec::new();

        for source in &self.sources {
            match source {
                SkillSourceConfig::BundledRecords { records } => {
                    for record in records {
                        let source = SkillSource::Bundled;
                        match parse_skill_markdown(
                            &record.to_markdown(),
                            source.clone(),
                            None,
                            self.runtime_platform,
                        ) {
                            Ok(skill) => loaded.push(skill),
                            Err(error) => rejected.push(SkillRejection {
                                source,
                                raw_path: None,
                                reason: SkillRejectReason::from_error(&error),
                            }),
                        }
                    }
                }
                SkillSourceConfig::McpRecords { server_id, records } => {
                    let report = McpSource::new(server_id.clone(), records.clone())
                        .load(self.runtime_platform)
                        .await?;
                    for skill in report.loaded {
                        let source = skill.source.clone();
                        let skill = match self.apply_threat_scan(skill, &source, None) {
                            Ok(skill) => skill,
                            Err(rejection) => {
                                rejected.push(rejection);
                                continue;
                            }
                        };
                        loaded.push(skill);
                    }
                    rejected.extend(report.rejected);
                }
                SkillSourceConfig::Directory { path, source_kind } => {
                    if !path.exists() {
                        continue;
                    }
                    for entry in std::fs::read_dir(path)? {
                        let entry = entry?;
                        let raw_path = entry.path();
                        if raw_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                            continue;
                        }
                        let source = source_from_directory(path.clone(), source_kind);
                        let markdown = std::fs::read_to_string(&raw_path)?;
                        match parse_skill_markdown(
                            &markdown,
                            source.clone(),
                            Some(raw_path.clone()),
                            self.runtime_platform,
                        ) {
                            Ok(skill) => {
                                let skill =
                                    match self.apply_threat_scan(skill, &source, Some(&raw_path)) {
                                        Ok(skill) => skill,
                                        Err(rejection) => {
                                            rejected.push(rejection);
                                            continue;
                                        }
                                    };
                                loaded.push(skill);
                            }
                            Err(error) => rejected.push(SkillRejection {
                                source,
                                raw_path: Some(raw_path),
                                reason: SkillRejectReason::from_error(&error),
                            }),
                        }
                    }
                }
            }
        }

        Ok(LoadReport { loaded, rejected })
    }

    pub async fn load_by_name(&self, name: &str) -> Result<Skill, SkillError> {
        let report = self.load_all().await?;
        report
            .loaded
            .into_iter()
            .find(|skill| skill.name == name)
            .ok_or_else(|| SkillError::ParseFrontmatter(format!("skill not found: {name}")))
    }

    #[cfg(feature = "threat-scanner")]
    fn apply_threat_scan(
        &self,
        mut skill: Skill,
        source: &SkillSource,
        raw_path: Option<&std::path::Path>,
    ) -> Result<Skill, SkillRejection> {
        if let Some(scanner) = &self.threat_scanner {
            if let Err(error) = crate::scanner::apply_threat_scan(&mut skill, scanner) {
                return Err(SkillRejection {
                    source: source.clone(),
                    raw_path: raw_path.map(std::path::Path::to_path_buf),
                    reason: SkillRejectReason::from_error(&error),
                });
            }
        }
        Ok(skill)
    }

    #[cfg(not(feature = "threat-scanner"))]
    fn apply_threat_scan(
        &self,
        skill: Skill,
        _source: &SkillSource,
        _raw_path: Option<&std::path::Path>,
    ) -> Result<Skill, SkillRejection> {
        Ok(skill)
    }
}

impl BundledSkillRecord {
    fn to_markdown(&self) -> String {
        format!(
            "---\nname: {}\ndescription: {}\n---\n{}",
            self.name, self.description, self.body
        )
    }
}

fn source_from_directory(path: PathBuf, source_kind: &DirectorySourceKind) -> SkillSource {
    match source_kind {
        DirectorySourceKind::Workspace => SkillSource::Workspace(path),
        DirectorySourceKind::User => SkillSource::User(path),
        DirectorySourceKind::Plugin(plugin_id) => SkillSource::Plugin(plugin_id.clone()),
    }
}

fn current_platform() -> SkillPlatform {
    if cfg!(target_os = "macos") {
        SkillPlatform::Macos
    } else if cfg!(target_os = "windows") {
        SkillPlatform::Windows
    } else {
        SkillPlatform::Linux
    }
}
