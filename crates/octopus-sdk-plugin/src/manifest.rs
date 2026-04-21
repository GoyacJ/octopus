use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use octopus_sdk_contracts::{
    DeclSource, HookDecl, ModelProviderDecl, PluginSourceTag, SkillDecl, ToolDecl,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::{security::validate_security_gates, PluginError};

pub const SDK_PLUGIN_API_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCompat {
    #[serde(rename = "pluginApi")]
    pub plugin_api: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub version: String,
    pub git_sha: Option<String>,
    pub compat: PluginCompat,
    pub components: Vec<PluginComponent>,
    pub source: PluginSourceTag,
}

impl PluginManifest {
    pub fn load_from_path(path: &Path) -> Result<Self, PluginError> {
        let content = fs::read_to_string(path)
            .map_err(|_| PluginError::PathNotFound { path: path.into() })?;
        let manifest: PluginManifest = serde_json::from_str::<PluginManifestRaw>(&content)
            .map_err(|error| PluginError::ManifestParseError {
                cause: error.to_string(),
            })?
            .try_into()?;
        manifest.validate(path)?;
        Ok(manifest)
    }

    pub fn validate(&self, manifest_path: &Path) -> Result<(), PluginError> {
        if self.source != PluginSourceTag::Local {
            return Err(PluginError::UnsupportedSource {
                source_kind: plugin_source_name(self.source).into(),
            });
        }
        validate_compat(&self.compat)?;
        validate_unique_component_ids(&self.components)?;
        validate_security_gates(manifest_path, self)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct PluginManifestRaw {
    id: String,
    version: String,
    git_sha: Option<String>,
    compat: PluginCompat,
    components: Vec<PluginComponent>,
    #[serde(default = "default_manifest_source")]
    source: String,
}

impl TryFrom<PluginManifestRaw> for PluginManifest {
    type Error = PluginError;

    fn try_from(value: PluginManifestRaw) -> Result<Self, Self::Error> {
        let source = match value.source.as_str() {
            "local" => PluginSourceTag::Local,
            other => {
                return Err(PluginError::UnsupportedSource {
                    source_kind: other.into(),
                });
            }
        };

        Ok(Self {
            id: value.id,
            version: value.version,
            git_sha: value.git_sha,
            compat: value.compat,
            components: value.components,
            source,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginComponent {
    Tool(ToolDecl),
    Skill(SkillDecl),
    Command(CommandDecl),
    Agent(AgentDecl),
    OutputStyle(OutputStyleDecl),
    Hook(HookDecl),
    McpServer(McpServerDecl),
    LspServer(LspServerDecl),
    ModelProvider(ModelProviderDecl),
    Channel(ChannelDecl),
    ContextEngine(ContextEngineDecl),
    MemoryBackend(MemoryBackendDecl),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDecl {
    pub id: String,
    pub path: PathBuf,
    pub source: DeclSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDecl {
    pub id: String,
    pub manifest_path: PathBuf,
    pub source: DeclSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputStyleDecl {
    pub id: String,
    pub template_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerDecl {
    pub id: String,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspServerDecl {
    pub id: String,
    pub command: String,
    pub source: DeclSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelDecl {
    pub id: String,
    pub transport: String,
    pub source: DeclSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextEngineDecl {
    pub id: String,
    pub entrypoint: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBackendDecl {
    pub id: String,
    pub entrypoint: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginDiscoveryConfig {
    pub roots: Vec<PathBuf>,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

#[must_use]
pub fn default_roots() -> Vec<PathBuf> {
    Vec::new()
}

fn default_manifest_source() -> String {
    "local".into()
}

fn plugin_source_name(source: PluginSourceTag) -> &'static str {
    match source {
        PluginSourceTag::Local => "local",
        PluginSourceTag::Bundled => "bundled",
    }
}

fn validate_compat(compat: &PluginCompat) -> Result<(), PluginError> {
    let actual = Version::parse(SDK_PLUGIN_API_VERSION).map_err(|error| {
        PluginError::ManifestValidationError {
            cause: error.to_string(),
        }
    })?;
    let required = VersionReq::parse(&compat.plugin_api).map_err(|error| {
        PluginError::ManifestValidationError {
            cause: error.to_string(),
        }
    })?;

    if required.matches(&actual) {
        Ok(())
    } else {
        Err(PluginError::IncompatibleApi {
            actual: SDK_PLUGIN_API_VERSION.into(),
            required: compat.plugin_api.clone(),
        })
    }
}

fn validate_unique_component_ids(components: &[PluginComponent]) -> Result<(), PluginError> {
    let mut seen = HashSet::new();

    for id in components.iter().map(component_id) {
        if !seen.insert(id.to_string()) {
            return Err(PluginError::DuplicateId { id: id.into() });
        }
    }

    Ok(())
}

fn component_id(component: &PluginComponent) -> &str {
    match component {
        PluginComponent::Tool(decl) => &decl.id,
        PluginComponent::Skill(decl) => &decl.id,
        PluginComponent::Command(decl) => &decl.id,
        PluginComponent::Agent(decl) => &decl.id,
        PluginComponent::OutputStyle(decl) => &decl.id,
        PluginComponent::Hook(decl) => &decl.id,
        PluginComponent::McpServer(decl) => &decl.id,
        PluginComponent::LspServer(decl) => &decl.id,
        PluginComponent::ModelProvider(decl) => &decl.id,
        PluginComponent::Channel(decl) => &decl.id,
        PluginComponent::ContextEngine(decl) => &decl.id,
        PluginComponent::MemoryBackend(decl) => &decl.id,
    }
}
