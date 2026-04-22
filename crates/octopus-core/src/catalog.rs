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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityAssetManifest {
    pub asset_id: String,
    pub workspace_id: String,
    pub source_key: String,
    pub kind: String,
    pub source_kinds: Vec<String>,
    pub execution_kinds: Vec<String>,
    pub name: String,
    pub description: String,
    pub display_path: String,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub required_permission: Option<String>,
    pub management: WorkspaceToolManagementCapabilities,
    pub installed: bool,
    pub enabled: bool,
    pub health: String,
    pub state: String,
    pub import_status: String,
    pub export_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillPackageManifest {
    pub asset_id: String,
    pub workspace_id: String,
    pub source_key: String,
    pub kind: String,
    pub source_kinds: Vec<String>,
    pub execution_kinds: Vec<String>,
    pub name: String,
    pub description: String,
    pub display_path: String,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub required_permission: Option<String>,
    pub management: WorkspaceToolManagementCapabilities,
    pub installed: bool,
    pub enabled: bool,
    pub health: String,
    pub state: String,
    pub import_status: String,
    pub export_status: String,
    pub package_kind: String,
    pub active: bool,
    pub shadowed_by: Option<String>,
    pub source_origin: String,
    pub workspace_owned: bool,
    pub relative_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerPackageManifest {
    pub asset_id: String,
    pub workspace_id: String,
    pub source_key: String,
    pub kind: String,
    pub source_kinds: Vec<String>,
    pub execution_kinds: Vec<String>,
    pub name: String,
    pub description: String,
    pub display_path: String,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub required_permission: Option<String>,
    pub management: WorkspaceToolManagementCapabilities,
    pub installed: bool,
    pub enabled: bool,
    pub health: String,
    pub state: String,
    pub import_status: String,
    pub export_status: String,
    pub package_kind: String,
    pub server_name: String,
    pub endpoint: String,
    pub tool_names: Vec<String>,
    pub prompt_names: Vec<String>,
    pub resource_uris: Vec<String>,
    pub scope: String,
    pub status_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityManagementEntry {
    pub id: String,
    pub asset_id: String,
    pub capability_id: String,
    pub workspace_id: String,
    pub name: String,
    pub kind: String,
    pub source_kind: String,
    pub execution_kind: String,
    pub description: String,
    pub required_permission: Option<String>,
    pub availability: String,
    pub source_key: String,
    pub display_path: String,
    pub disabled: bool,
    pub management: WorkspaceToolManagementCapabilities,
    pub builtin_key: Option<String>,
    pub active: Option<bool>,
    pub shadowed_by: Option<String>,
    pub source_origin: Option<String>,
    pub workspace_owned: Option<bool>,
    pub relative_path: Option<String>,
    pub server_name: Option<String>,
    pub endpoint: Option<String>,
    pub tool_names: Option<Vec<String>>,
    pub resource_uri: Option<String>,
    pub status_detail: Option<String>,
    pub scope: Option<String>,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub consumers: Option<Vec<WorkspaceToolConsumerSummary>>,
    pub installed: bool,
    pub enabled: bool,
    pub health: String,
    pub state: String,
    pub import_status: String,
    pub export_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityManagementProjection {
    pub entries: Vec<CapabilityManagementEntry>,
    pub assets: Vec<CapabilityAssetManifest>,
    pub skill_packages: Vec<SkillPackageManifest>,
    pub mcp_server_packages: Vec<McpServerPackageManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolManagementCapabilities {
    pub can_disable: bool,
    pub can_edit: bool,
    pub can_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolConsumerSummary {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub scope: String,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolCatalogEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    pub workspace_id: String,
    pub name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_kind: Option<String>,
    pub description: String,
    pub required_permission: Option<String>,
    pub availability: String,
    pub source_key: String,
    pub display_path: String,
    pub disabled: bool,
    pub management: WorkspaceToolManagementCapabilities,
    pub builtin_key: Option<String>,
    pub active: Option<bool>,
    pub shadowed_by: Option<String>,
    pub source_origin: Option<String>,
    pub workspace_owned: Option<bool>,
    pub relative_path: Option<String>,
    pub server_name: Option<String>,
    pub endpoint: Option<String>,
    pub tool_names: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_uri: Option<String>,
    pub status_detail: Option<String>,
    pub scope: Option<String>,
    pub owner_scope: Option<String>,
    pub owner_id: Option<String>,
    pub owner_label: Option<String>,
    pub consumers: Option<Vec<WorkspaceToolConsumerSummary>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityAssetDisablePatch {
    pub source_key: String,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceSkillInput {
    pub slug: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceSkillInput {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillTreeNode {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub children: Option<Vec<WorkspaceSkillTreeNode>>,
    pub byte_size: Option<u64>,
    pub is_text: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillDocument {
    pub id: String,
    pub source_key: String,
    pub name: String,
    pub description: String,
    pub content: String,
    pub display_path: String,
    pub root_path: String,
    pub tree: Vec<WorkspaceSkillTreeNode>,
    pub source_origin: String,
    pub workspace_owned: bool,
    pub relative_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillTreeDocument {
    pub skill_id: String,
    pub source_key: String,
    pub display_path: String,
    pub root_path: String,
    pub tree: Vec<WorkspaceSkillTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSkillFileDocument {
    pub skill_id: String,
    pub source_key: String,
    pub path: String,
    pub display_path: String,
    pub byte_size: u64,
    pub is_text: bool,
    pub content: Option<String>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceSkillFileInput {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFileUploadPayload {
    pub file_name: String,
    pub content_type: String,
    pub data_base64: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDirectoryUploadEntry {
    pub relative_path: String,
    pub file_name: String,
    pub content_type: String,
    pub data_base64: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceSkillArchiveInput {
    pub slug: String,
    pub archive: WorkspaceFileUploadPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceSkillFolderInput {
    pub slug: String,
    pub files: Vec<WorkspaceDirectoryUploadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CopyWorkspaceSkillToManagedInput {
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertWorkspaceMcpServerInput {
    pub server_name: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMcpServerDocument {
    pub server_name: String,
    pub source_key: String,
    pub display_path: String,
    pub scope: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolRecord {
    pub id: String,
    pub workspace_id: String,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub permission_mode: String,
    pub updated_at: u64,
}
