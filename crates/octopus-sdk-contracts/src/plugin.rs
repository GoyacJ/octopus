use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginsSnapshot {
    pub api_version: String,
    pub plugins: Vec<PluginSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginSummary {
    pub id: String,
    pub version: String,
    pub git_sha: Option<String>,
    pub source: PluginSourceTag,
    pub enabled: bool,
    pub components_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginSourceTag {
    Local,
    Bundled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDecl {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schema: Value,
    pub source: DeclSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookDecl {
    pub id: String,
    pub point: HookPoint,
    pub source: DeclSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPoint {
    PreToolUse,
    PostToolUse,
    Stop,
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreCompact,
    PostCompact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDecl {
    pub id: String,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProviderDecl {
    pub id: String,
    pub provider_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeclSource {
    Bundled,
    Plugin { plugin_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginErrorKind {
    PathNotFound,
    ManifestParseError,
    ManifestValidationError,
    IncompatibleApi,
    PluginNotFound,
    DependencyUnsatisfied,
    DuplicateId,
    PathEscape,
    WorldWritable,
    UnsupportedSource,
}
