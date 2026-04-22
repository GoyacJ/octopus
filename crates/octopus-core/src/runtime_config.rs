use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub session_id: String,
    pub status: String,
    pub current_step: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub model_id: Option<String>,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigSource {
    pub scope: String,
    pub owner_id: Option<String>,
    pub display_path: String,
    pub source_key: String,
    pub exists: bool,
    pub loaded: bool,
    pub content_hash: Option<String>,
    pub document: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSecretReferenceStatus {
    pub scope: String,
    pub path: String,
    pub reference: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfiguredModelProbeInput {
    pub scope: String,
    pub configured_model_id: String,
    pub patch: serde_json::Value,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfiguredModelCredentialInput {
    pub configured_model_id: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfiguredModelProbeResult {
    pub valid: bool,
    pub reachable: bool,
    pub configured_model_id: String,
    pub configured_model_name: Option<String>,
    pub request_id: Option<String>,
    pub consumed_tokens: Option<u32>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunRuntimeGenerationInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub configured_model_id: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeGenerationResult {
    pub configured_model_id: String,
    pub configured_model_name: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumed_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEffectiveConfig {
    pub effective_config: serde_json::Value,
    pub effective_config_hash: String,
    pub sources: Vec<RuntimeConfigSource>,
    pub validation: RuntimeConfigValidationResult,
    pub secret_references: Vec<RuntimeSecretReferenceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigPatch {
    pub scope: String,
    pub patch: serde_json::Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configured_model_credentials: Vec<RuntimeConfiguredModelCredentialInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigSnapshotSummary {
    pub id: String,
    pub effective_config_hash: String,
    pub started_from_scope_set: Vec<String>,
    pub source_refs: Vec<String>,
    pub created_at: u64,
    pub effective_config: Option<serde_json::Value>,
}
