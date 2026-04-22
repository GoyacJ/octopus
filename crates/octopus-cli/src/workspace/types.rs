use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefinitionSource {
    ProjectClaw,
    ProjectCodex,
    ProjectClaude,
    ProjectManaged,
    UserClawConfigHome,
    UserCodexHome,
    UserClaw,
    UserCodex,
    UserClaude,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefinitionScope {
    Project,
    UserConfigHome,
    UserHome,
}

impl DefinitionScope {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Project => "Project (.claw)",
            Self::UserConfigHome => "User ($CLAW_CONFIG_HOME)",
            Self::UserHome => "User (~/.claw)",
        }
    }
}

impl DefinitionSource {
    pub(crate) fn report_scope(self) -> DefinitionScope {
        match self {
            Self::ProjectClaw | Self::ProjectCodex | Self::ProjectClaude | Self::ProjectManaged => {
                DefinitionScope::Project
            }
            Self::UserClawConfigHome | Self::UserCodexHome => DefinitionScope::UserConfigHome,
            Self::UserClaw | Self::UserCodex | Self::UserClaude => DefinitionScope::UserHome,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::ProjectManaged => "Project (data/skills)",
            _ => self.report_scope().label(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentSummary {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<String>,
    pub(crate) source: DefinitionSource,
    pub(crate) shadowed_by: Option<DefinitionSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillSummary {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) source: DefinitionSource,
    pub(crate) shadowed_by: Option<DefinitionSource>,
    pub(crate) origin: SkillOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillOrigin {
    SkillsDir,
    LegacyCommandsDir,
}

impl SkillOrigin {
    pub(crate) fn detail_label(self) -> Option<&'static str> {
        match self {
            Self::SkillsDir => None,
            Self::LegacyCommandsDir => Some("legacy /commands"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillRoot {
    pub(crate) source: DefinitionSource,
    pub(crate) path: PathBuf,
    pub(crate) origin: SkillOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledSkill {
    pub invocation_name: String,
    pub display_name: Option<String>,
    pub source: PathBuf,
    pub registry_root: PathBuf,
    pub installed_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillInstallSource {
    Directory { root: PathBuf, prompt_path: PathBuf },
    MarkdownFile { path: PathBuf },
}

impl SkillInstallSource {
    pub(crate) fn prompt_path(&self) -> &Path {
        match self {
            Self::Directory { prompt_path, .. } => prompt_path,
            Self::MarkdownFile { path } => path,
        }
    }

    pub(crate) fn fallback_name(&self) -> Option<String> {
        match self {
            Self::Directory { root, .. } => root
                .file_name()
                .map(|name| name.to_string_lossy().to_string()),
            Self::MarkdownFile { path } => path
                .file_stem()
                .map(|name| name.to_string_lossy().to_string()),
        }
    }

    pub(crate) fn report_path(&self) -> &Path {
        match self {
            Self::Directory { root, .. } => root,
            Self::MarkdownFile { path } => path,
        }
    }
}
