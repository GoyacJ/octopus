use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogRecord {
    pub id: String,
    pub workspace_id: String,
    pub label: String,
    pub provider: String,
    pub description: String,
    pub recommended_for: String,
    pub availability: String,
    pub default_permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialRecord {
    pub id: String,
    pub workspace_id: String,
    pub provider: String,
    pub name: String,
    pub base_url: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDescriptor {
    pub capability_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeExecutionClass {
    #[default]
    Unsupported,
    SingleShotGeneration,
    AgentConversation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BudgetAccountingMode {
    #[default]
    ProviderReported,
    Estimated,
    NonBillable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BudgetReservationStrategy {
    #[default]
    None,
    Fixed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeExecutionProfile {
    #[serde(default)]
    pub execution_class: RuntimeExecutionClass,
    pub tool_loop: bool,
    pub upstream_streaming: bool,
}

impl RuntimeExecutionProfile {
    pub fn executable(self) -> bool {
        self.execution_class != RuntimeExecutionClass::Unsupported
    }

    pub fn supports(self, execution_class: RuntimeExecutionClass) -> bool {
        self.execution_class == execution_class
    }

    pub fn supports_agent_conversation(self) -> bool {
        self.execution_class == RuntimeExecutionClass::AgentConversation
            && self.tool_loop
            && self.upstream_streaming
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceDescriptor {
    pub surface: String,
    pub protocol_family: String,
    pub transport: Vec<String>,
    pub auth_strategy: String,
    pub base_url: String,
    pub base_url_policy: String,
    pub enabled: bool,
    pub capabilities: Vec<CapabilityDescriptor>,
    #[serde(default)]
    pub execution_profile: RuntimeExecutionProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRegistryRecord {
    pub provider_id: String,
    pub label: String,
    pub enabled: bool,
    pub surfaces: Vec<SurfaceDescriptor>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelSurfaceBinding {
    pub surface: String,
    pub protocol_family: String,
    pub enabled: bool,
    #[serde(default)]
    pub execution_profile: RuntimeExecutionProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRegistryRecord {
    pub model_id: String,
    pub provider_id: String,
    pub label: String,
    pub description: String,
    pub family: String,
    pub track: String,
    pub enabled: bool,
    pub recommended_for: String,
    pub availability: String,
    pub default_permission: String,
    pub surface_bindings: Vec<ModelSurfaceBinding>,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialBinding {
    pub credential_ref: String,
    pub provider_id: String,
    pub label: String,
    pub base_url: Option<String>,
    pub status: String,
    pub configured: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefaultSelection {
    pub configured_model_id: Option<String>,
    pub provider_id: String,
    pub model_id: String,
    pub surface: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredModelBudgetPolicy {
    #[serde(default)]
    pub accounting_mode: BudgetAccountingMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traffic_classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_budget_tokens: Option<u64>,
    #[serde(default)]
    pub reservation_strategy: BudgetReservationStrategy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning_threshold_percentages: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredModelTokenUsage {
    pub used_tokens: u64,
    pub remaining_tokens: Option<u64>,
    pub exhausted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredModelRecord {
    pub configured_model_id: String,
    pub name: String,
    pub provider_id: String,
    pub model_id: String,
    pub credential_ref: Option<String>,
    pub base_url: Option<String>,
    pub budget_policy: Option<ConfiguredModelBudgetPolicy>,
    pub token_usage: ConfiguredModelTokenUsage,
    pub enabled: bool,
    pub source: String,
    pub status: String,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRegistryDiagnostics {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogSnapshot {
    pub providers: Vec<ProviderRegistryRecord>,
    pub models: Vec<ModelRegistryRecord>,
    pub configured_models: Vec<ConfiguredModelRecord>,
    pub credential_bindings: Vec<CredentialBinding>,
    pub default_selections: BTreeMap<String, DefaultSelection>,
    pub diagnostics: ModelRegistryDiagnostics,
}
