use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use harness_contracts::{PluginId, TrustLevel};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::PluginError;

pub const SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    #[serde(default = "default_manifest_schema_version")]
    pub manifest_schema_version: u32,
    pub name: PluginName,
    pub version: String,
    pub trust_level: TrustLevel,
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    pub repository: Option<String>,
    pub signature: Option<ManifestSignature>,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    #[serde(default = "default_version_req")]
    pub min_harness_version: String,
}

impl PluginManifest {
    pub fn plugin_id(&self) -> PluginId {
        PluginId(format!("{}@{}", self.name, self.version))
    }

    pub fn validate_basic(&self) -> Result<(), PluginError> {
        if self.manifest_schema_version > SUPPORTED_MANIFEST_SCHEMA_VERSION {
            return Err(PluginError::InvalidManifest(format!(
                "unsupported manifest_schema_version {}",
                self.manifest_schema_version
            )));
        }
        if self.version.trim().is_empty() {
            return Err(PluginError::InvalidManifest(
                "version must not be empty".to_owned(),
            ));
        }
        if self.min_harness_version.trim().is_empty() {
            return Err(PluginError::InvalidManifest(
                "min_harness_version must not be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub tools: Vec<ToolManifestEntry>,
    #[serde(default)]
    pub skills: Vec<SkillManifestEntry>,
    #[serde(default)]
    pub hooks: Vec<HookManifestEntry>,
    #[serde(default)]
    pub mcp_servers: Vec<McpManifestEntry>,
    pub memory_provider: Option<MemoryProviderManifestEntry>,
    pub coordinator_strategy: Option<CoordinatorStrategyManifestEntry>,
    pub configuration_schema: Option<Value>,
}

impl PluginCapabilities {
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
            && self.skills.is_empty()
            && self.hooks.is_empty()
            && self.mcp_servers.is_empty()
            && self.memory_provider.is_none()
            && self.coordinator_strategy.is_none()
            && self.configuration_schema.is_none()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolManifestEntry {
    pub name: String,
    #[serde(default)]
    pub destructive: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillManifestEntry {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookManifestEntry {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpManifestEntry {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryProviderManifestEntry {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CoordinatorStrategyManifestEntry {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginDependency {
    pub name: PluginName,
    #[serde(default = "default_version_req")]
    pub version_req: String,
    #[serde(default)]
    pub kind: PluginDependencyKind,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginDependencyKind {
    #[default]
    Required,
    Optional,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ManifestSignature {
    pub algorithm: SignatureAlgorithm,
    pub signer: String,
    pub signature: Vec<u8>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAlgorithm {
    Ed25519,
    RsaPkcs1Sha256,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PluginName(String);

impl PluginName {
    pub fn new(value: impl Into<String>) -> Result<Self, PluginError> {
        let value = value.into();
        validate_plugin_name(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for PluginName {
    type Error = PluginError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PluginName> for String {
    fn from(value: PluginName) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestRecord {
    pub manifest: PluginManifest,
    pub origin: ManifestOrigin,
    pub manifest_hash: [u8; 32],
}

impl ManifestRecord {
    pub fn new(
        manifest: PluginManifest,
        origin: ManifestOrigin,
        manifest_hash: [u8; 32],
    ) -> Result<Self, PluginError> {
        manifest.validate_basic()?;
        Ok(Self {
            manifest,
            origin,
            manifest_hash,
        })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestOrigin {
    File {
        path: PathBuf,
    },
    CargoExtension {
        binary: PathBuf,
        package_metadata: BTreeMap<String, Value>,
    },
    RemoteRegistry {
        endpoint: String,
        etag: Option<String>,
    },
}

impl fmt::Display for ManifestOrigin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File { path } => write!(formatter, "file:{}", path.display()),
            Self::CargoExtension { binary, .. } => {
                write!(formatter, "cargo_extension:{}", binary.display())
            }
            Self::RemoteRegistry { endpoint, .. } => write!(formatter, "remote:{endpoint}"),
        }
    }
}

fn validate_plugin_name(value: &str) -> Result<(), PluginError> {
    let len = value.len();
    if !(1..=64).contains(&len) {
        return Err(PluginError::InvalidManifest(
            "plugin name length must be 1..=64".to_owned(),
        ));
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(PluginError::InvalidManifest(
            "plugin name must not be empty".to_owned(),
        ));
    };
    if !first.is_ascii_lowercase() {
        return Err(PluginError::InvalidManifest(
            "plugin name must start with a lowercase ASCII letter".to_owned(),
        ));
    }
    if value.ends_with('-') {
        return Err(PluginError::InvalidManifest(
            "plugin name must not end with '-'".to_owned(),
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(PluginError::InvalidManifest(
            "plugin name may only contain lowercase ASCII letters, digits, and '-'".to_owned(),
        ));
    }
    Ok(())
}

fn default_manifest_schema_version() -> u32 {
    SUPPORTED_MANIFEST_SCHEMA_VERSION
}

fn default_version_req() -> String {
    ">=0.0.0".to_owned()
}
