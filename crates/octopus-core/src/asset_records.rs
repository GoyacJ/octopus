use serde::{Deserialize, Serialize};

use crate::{
    ApprovalPreference, ArtifactHandoffPolicy, AssetDependencyResolution, AssetImportMetadata,
    AssetTrustMetadata, AvatarUploadPayload, CapabilityPolicy, DefaultModelStrategy,
    DelegationPolicy, MailboxPolicy, MemoryPolicy, OutputContract, PermissionEnvelope,
    SharedCapabilityPolicy, SharedMemoryPolicy, TeamTopology, WorkflowAffordance,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub avatar_path: Option<String>,
    pub avatar: Option<String>,
    pub personality: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub task_domains: Vec<String>,
    pub manifest_revision: String,
    pub default_model_strategy: DefaultModelStrategy,
    pub capability_policy: CapabilityPolicy,
    pub permission_envelope: PermissionEnvelope,
    pub memory_policy: MemoryPolicy,
    pub delegation_policy: DelegationPolicy,
    pub approval_preference: ApprovalPreference,
    pub output_contract: OutputContract,
    pub shared_capability_policy: SharedCapabilityPolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_source: Option<WorkspaceLinkIntegrationSource>,
    pub trust_metadata: AssetTrustMetadata,
    pub dependency_resolution: Vec<AssetDependencyResolution>,
    pub import_metadata: AssetImportMetadata,
    pub description: String,
    pub status: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub avatar_path: Option<String>,
    pub avatar: Option<String>,
    pub personality: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub task_domains: Vec<String>,
    pub manifest_revision: String,
    pub default_model_strategy: DefaultModelStrategy,
    pub capability_policy: CapabilityPolicy,
    pub permission_envelope: PermissionEnvelope,
    pub memory_policy: MemoryPolicy,
    pub delegation_policy: DelegationPolicy,
    pub approval_preference: ApprovalPreference,
    pub output_contract: OutputContract,
    pub shared_capability_policy: SharedCapabilityPolicy,
    pub leader_agent_id: Option<String>,
    pub member_agent_ids: Vec<String>,
    pub leader_ref: String,
    pub member_refs: Vec<String>,
    pub team_topology: TeamTopology,
    pub shared_memory_policy: SharedMemoryPolicy,
    pub mailbox_policy: MailboxPolicy,
    pub artifact_handoff_policy: ArtifactHandoffPolicy,
    pub workflow_affordance: WorkflowAffordance,
    pub worker_concurrency_limit: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_source: Option<WorkspaceLinkIntegrationSource>,
    pub trust_metadata: AssetTrustMetadata,
    pub dependency_resolution: Vec<AssetDependencyResolution>,
    pub import_metadata: AssetImportMetadata,
    pub description: String,
    pub status: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLinkIntegrationSource {
    pub kind: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BundleAssetDescriptorRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub asset_kind: String,
    pub source_id: String,
    pub display_name: String,
    pub source_path: String,
    pub storage_path: String,
    pub content_hash: String,
    pub byte_size: u64,
    pub manifest_revision: String,
    pub task_domains: Vec<String>,
    pub translation_mode: String,
    pub trust_metadata: AssetTrustMetadata,
    pub dependency_resolution: Vec<AssetDependencyResolution>,
    pub import_metadata: AssetImportMetadata,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertAgentInput {
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub avatar: Option<AvatarUploadPayload>,
    pub remove_avatar: Option<bool>,
    pub personality: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub task_domains: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model_strategy: Option<DefaultModelStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_policy: Option<CapabilityPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_envelope: Option<PermissionEnvelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_policy: Option<MemoryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation_policy: Option<DelegationPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_preference: Option<ApprovalPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<OutputContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_capability_policy: Option<SharedCapabilityPolicy>,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpsertTeamInput {
    pub workspace_id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub name: String,
    pub avatar: Option<AvatarUploadPayload>,
    pub remove_avatar: Option<bool>,
    pub personality: String,
    pub tags: Vec<String>,
    pub prompt: String,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub task_domains: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model_strategy: Option<DefaultModelStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_policy: Option<CapabilityPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_envelope: Option<PermissionEnvelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_policy: Option<MemoryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation_policy: Option<DelegationPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_preference: Option<ApprovalPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<OutputContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_capability_policy: Option<SharedCapabilityPolicy>,
    pub leader_agent_id: Option<String>,
    pub member_agent_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub member_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_topology: Option<TeamTopology>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_memory_policy: Option<SharedMemoryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mailbox_policy: Option<MailboxPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_handoff_policy: Option<ArtifactHandoffPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_affordance: Option<WorkflowAffordance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_concurrency_limit: Option<u64>,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgentLinkRecord {
    pub workspace_id: String,
    pub project_id: String,
    pub agent_id: String,
    pub linked_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTeamLinkRecord {
    pub workspace_id: String,
    pub project_id: String,
    pub team_id: String,
    pub linked_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgentLinkInput {
    pub project_id: String,
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTeamLinkInput {
    pub project_id: String,
    pub team_id: String,
}
