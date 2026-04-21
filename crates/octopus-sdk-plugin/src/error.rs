use std::path::PathBuf;

use octopus_sdk_contracts::PluginErrorKind;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PluginError {
    #[error("plugin path not found: {path}")]
    PathNotFound { path: PathBuf },
    #[error("plugin manifest parse error: {cause}")]
    ManifestParseError { cause: String },
    #[error("plugin manifest validation error: {cause}")]
    ManifestValidationError { cause: String },
    #[error("plugin API version {actual} is incompatible with {required}")]
    IncompatibleApi { actual: String, required: String },
    #[error("plugin not found: {plugin_id}")]
    PluginNotFound { plugin_id: String },
    #[error("plugin dependency unsatisfied: {dependency}")]
    DependencyUnsatisfied { dependency: String },
    #[error("plugin duplicate id: {id}")]
    DuplicateId { id: String },
    #[error("plugin path escapes root: {path}")]
    PathEscape { path: PathBuf },
    #[error("plugin file is world-writable: {path}")]
    WorldWritable { path: PathBuf },
    #[error("plugin source unsupported: {source_kind}")]
    UnsupportedSource { source_kind: String },
}

impl PluginError {
    #[must_use]
    pub const fn kind(&self) -> PluginErrorKind {
        match self {
            Self::PathNotFound { .. } => PluginErrorKind::PathNotFound,
            Self::ManifestParseError { .. } => PluginErrorKind::ManifestParseError,
            Self::ManifestValidationError { .. } => PluginErrorKind::ManifestValidationError,
            Self::IncompatibleApi { .. } => PluginErrorKind::IncompatibleApi,
            Self::PluginNotFound { .. } => PluginErrorKind::PluginNotFound,
            Self::DependencyUnsatisfied { .. } => PluginErrorKind::DependencyUnsatisfied,
            Self::DuplicateId { .. } => PluginErrorKind::DuplicateId,
            Self::PathEscape { .. } => PluginErrorKind::PathEscape,
            Self::WorldWritable { .. } => PluginErrorKind::WorldWritable,
            Self::UnsupportedSource { .. } => PluginErrorKind::UnsupportedSource,
        }
    }
}
