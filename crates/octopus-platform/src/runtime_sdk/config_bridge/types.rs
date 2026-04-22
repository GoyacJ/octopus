use std::path::PathBuf;

use octopus_core::{
    RuntimeConfigValidationResult, RuntimeEffectiveConfig, RuntimeSecretReferenceStatus,
};
use serde_json::{Map, Value};

pub(crate) const KNOWN_RUNTIME_CONFIG_TOP_LEVEL_KEYS: &[&str] = &[
    "$schema",
    "aliases",
    "configuredModels",
    "credentialRefs",
    "defaultSelections",
    "enabledPlugins",
    "env",
    "hooks",
    "mcpServers",
    "model",
    "modelRegistry",
    "oauth",
    "permissionMode",
    "permissions",
    "plugins",
    "projectSettings",
    "provider",
    "providerFallbacks",
    "providerOverrides",
    "sandbox",
    "trustedRoots",
];

pub(crate) const DEPRECATED_RUNTIME_CONFIG_TOP_LEVEL_KEYS: &[(&str, &str)] = &[
    ("allowedTools", "permissions.allow"),
    ("ignorePatterns", "permissions.deny"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeConfigScopeKind {
    Workspace,
    Project,
    User,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeConfigDocumentRecord {
    pub(crate) scope: RuntimeConfigScopeKind,
    pub(crate) owner_id: Option<String>,
    pub(crate) source_key: String,
    pub(crate) display_path: String,
    pub(crate) storage_path: PathBuf,
    pub(crate) exists: bool,
    pub(crate) loaded: bool,
    pub(crate) document: Option<Map<String, Value>>,
    pub(crate) secret_reference_statuses: Vec<RuntimeSecretReferenceStatus>,
}

#[derive(Debug, Clone)]
pub(crate) struct ManagedConfiguredModelCredentialWrite {
    pub(crate) credential_ref: String,
    pub(crate) api_key: String,
    pub(crate) previous_value: Option<String>,
}

pub(crate) fn apply_validation(
    mut effective: RuntimeEffectiveConfig,
    validation: RuntimeConfigValidationResult,
) -> RuntimeEffectiveConfig {
    effective.validation = validation;
    effective
}
