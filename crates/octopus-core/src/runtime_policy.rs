use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    RUNTIME_PERMISSION_DANGER_FULL_ACCESS, RUNTIME_PERMISSION_READ_ONLY,
    RUNTIME_PERMISSION_WORKSPACE_WRITE,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefaultModelStrategy {
    pub selection_mode: String,
    pub preferred_model_ref: Option<String>,
    pub fallback_model_refs: Vec<String>,
    pub allow_turn_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityPolicy {
    pub mode: String,
    pub deny_by_default: bool,
    pub builtin_tool_keys: Vec<String>,
    pub skill_ids: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub plugin_capability_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionEnvelope {
    pub default_mode: String,
    pub max_mode: String,
    pub escalation_allowed: bool,
    pub allowed_resource_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPolicy {
    pub durable_scopes: Vec<String>,
    pub write_requires_approval: bool,
    pub allow_workspace_shared_write: bool,
    pub max_selections: u64,
    pub freshness_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DelegationPolicy {
    pub mode: String,
    pub allow_background_runs: bool,
    pub allow_parallel_workers: bool,
    pub max_worker_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalPreference {
    pub tool_execution: String,
    pub memory_write: String,
    pub mcp_auth: String,
    pub team_spawn: String,
    pub workflow_escalation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTargetPolicyDecision {
    pub target_kind: String,
    pub action: String,
    pub hidden: bool,
    pub visible: bool,
    pub deferred: bool,
    pub requires_approval: bool,
    pub requires_auth: bool,
    pub reason: Option<String>,
    pub capability_id: Option<String>,
    pub provider_key: Option<String>,
    pub required_permission: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilityPolicyDecisions {
    pub builtin: RuntimeTargetPolicyDecision,
    pub skill: RuntimeTargetPolicyDecision,
    pub mcp: RuntimeTargetPolicyDecision,
}

pub type RuntimeTargetPolicyDecisions = BTreeMap<String, RuntimeTargetPolicyDecision>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutputContract {
    pub primary_format: String,
    pub artifact_kinds: Vec<String>,
    pub require_structured_summary: bool,
    pub preserve_lineage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SharedCapabilityPolicy {
    pub allow_team_inherited_capabilities: bool,
    pub deny_direct_member_escalation: bool,
    pub shared_capability_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SharedMemoryPolicy {
    pub share_mode: String,
    pub writable_by_workers: bool,
    pub require_review_before_persist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MailboxPolicy {
    pub mode: String,
    pub allow_worker_to_worker: bool,
    pub retain_messages: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactHandoffPolicy {
    pub mode: String,
    pub require_lineage: bool,
    pub retain_artifacts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAffordance {
    pub supported_task_kinds: Vec<String>,
    pub background_capable: bool,
    pub automation_capable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TeamTopology {
    pub mode: String,
    pub leader_ref: String,
    pub member_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetDependency {
    pub kind: String,
    pub r#ref: String,
    pub version_range: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetDependencyResolution {
    pub kind: String,
    pub r#ref: String,
    pub required: bool,
    pub resolution_state: String,
    pub resolved_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetTrustMetadata {
    pub publisher: String,
    pub origin: String,
    pub signature_state: String,
    pub trust_level: String,
    pub trust_warnings: Vec<String>,
    pub verified_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportMetadata {
    pub origin_kind: String,
    pub source_id: Option<String>,
    pub manifest_version: u64,
    pub translation_status: String,
    pub imported_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetTranslationDiagnostic {
    pub severity: String,
    pub code: String,
    pub stage: String,
    pub asset_kind: Option<String>,
    pub asset_id: Option<String>,
    pub source_path: Option<String>,
    pub dependency_ref: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
    pub details: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetTranslationReport {
    pub status: String,
    pub translated_count: u64,
    pub downgraded_count: u64,
    pub rejected_count: u64,
    pub unsupported_features: Vec<String>,
    pub trust_warnings: Vec<String>,
    pub dependency_resolution: Vec<AssetDependencyResolution>,
    pub diagnostics: Vec<AssetTranslationDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetBundleAssetEntry {
    pub asset_kind: String,
    pub source_id: String,
    pub display_name: String,
    pub source_path: String,
    pub manifest_revision: String,
    pub task_domains: Vec<String>,
    pub translation_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetBundleCompatibilityMapping {
    pub supported_targets: Vec<String>,
    pub downgraded_features: Vec<String>,
    pub rejected_features: Vec<String>,
    pub translator_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetBundlePolicyDefaults {
    pub default_model_strategy: DefaultModelStrategy,
    pub permission_envelope: PermissionEnvelope,
    pub memory_policy: MemoryPolicy,
    pub delegation_policy: DelegationPolicy,
    pub approval_preference: ApprovalPreference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetBundleRegistryMetadata {
    pub publisher: String,
    pub revision: String,
    pub release_channel: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetBundleManifestV2 {
    pub version: u64,
    pub bundle_kind: String,
    pub bundle_root: String,
    pub assets: Vec<AssetBundleAssetEntry>,
    pub dependencies: Vec<AssetDependency>,
    pub trust_metadata: AssetTrustMetadata,
    pub compatibility_mapping: AssetBundleCompatibilityMapping,
    pub policy_defaults: AssetBundlePolicyDefaults,
    pub registry_metadata: Option<AssetBundleRegistryMetadata>,
}

pub const ASSET_MANIFEST_REVISION_V2: &str = "asset-manifest/v2";
pub const ASSET_IMPORT_MANIFEST_VERSION: u64 = 2;

#[must_use]
pub fn normalize_task_domains(task_domains: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let normalized = task_domains
        .into_iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        vec!["general".into()]
    } else {
        normalized
    }
}

#[must_use]
pub fn default_model_strategy() -> DefaultModelStrategy {
    DefaultModelStrategy {
        selection_mode: "session-selected".into(),
        preferred_model_ref: None,
        fallback_model_refs: Vec::new(),
        allow_turn_override: true,
    }
}

#[must_use]
pub fn capability_policy_from_sources(
    builtin_tool_keys: &[String],
    skill_ids: &[String],
    mcp_server_names: &[String],
) -> CapabilityPolicy {
    CapabilityPolicy {
        mode: "allow-list".into(),
        deny_by_default: true,
        builtin_tool_keys: builtin_tool_keys.to_vec(),
        skill_ids: skill_ids.to_vec(),
        mcp_server_names: mcp_server_names.to_vec(),
        plugin_capability_refs: Vec::new(),
    }
}

#[must_use]
pub fn default_permission_envelope() -> PermissionEnvelope {
    PermissionEnvelope {
        default_mode: RUNTIME_PERMISSION_WORKSPACE_WRITE.into(),
        max_mode: RUNTIME_PERMISSION_WORKSPACE_WRITE.into(),
        escalation_allowed: false,
        allowed_resource_scopes: vec!["agent-private".into(), "project-shared".into()],
    }
}

#[must_use]
pub fn runtime_permission_mode_rank(value: &str) -> Option<u8> {
    match value {
        RUNTIME_PERMISSION_READ_ONLY => Some(0),
        RUNTIME_PERMISSION_WORKSPACE_WRITE => Some(1),
        RUNTIME_PERMISSION_DANGER_FULL_ACCESS => Some(2),
        _ => None,
    }
}

#[must_use]
pub fn clamp_runtime_permission_mode(requested: &str, ceiling: &str) -> String {
    match (
        runtime_permission_mode_rank(requested),
        runtime_permission_mode_rank(ceiling),
    ) {
        (Some(requested_rank), Some(ceiling_rank)) if requested_rank > ceiling_rank => {
            ceiling.to_string()
        }
        _ => requested.to_string(),
    }
}

#[must_use]
pub fn default_agent_memory_policy() -> MemoryPolicy {
    MemoryPolicy {
        durable_scopes: vec!["user-private".into(), "agent-private".into()],
        write_requires_approval: true,
        allow_workspace_shared_write: false,
        max_selections: 6,
        freshness_required: true,
    }
}

#[must_use]
pub fn default_team_memory_policy() -> MemoryPolicy {
    MemoryPolicy {
        durable_scopes: vec!["team-shared".into(), "project-shared".into()],
        write_requires_approval: true,
        allow_workspace_shared_write: false,
        max_selections: 8,
        freshness_required: true,
    }
}

#[must_use]
pub fn default_agent_delegation_policy() -> DelegationPolicy {
    DelegationPolicy {
        mode: "disabled".into(),
        allow_background_runs: false,
        allow_parallel_workers: false,
        max_worker_count: 0,
    }
}

#[must_use]
pub fn default_team_delegation_policy() -> DelegationPolicy {
    DelegationPolicy {
        mode: "leader-orchestrated".into(),
        allow_background_runs: true,
        allow_parallel_workers: true,
        max_worker_count: 4,
    }
}

#[must_use]
pub fn default_approval_preference() -> ApprovalPreference {
    ApprovalPreference {
        tool_execution: "auto".into(),
        memory_write: "require-approval".into(),
        mcp_auth: "require-approval".into(),
        team_spawn: "require-approval".into(),
        workflow_escalation: "require-approval".into(),
    }
}

#[must_use]
pub fn default_output_contract() -> OutputContract {
    OutputContract {
        primary_format: "markdown".into(),
        artifact_kinds: Vec::new(),
        require_structured_summary: true,
        preserve_lineage: true,
    }
}

#[must_use]
pub fn default_agent_shared_capability_policy() -> SharedCapabilityPolicy {
    SharedCapabilityPolicy {
        allow_team_inherited_capabilities: false,
        deny_direct_member_escalation: true,
        shared_capability_refs: Vec::new(),
    }
}

#[must_use]
pub fn default_team_shared_capability_policy() -> SharedCapabilityPolicy {
    SharedCapabilityPolicy {
        allow_team_inherited_capabilities: true,
        deny_direct_member_escalation: true,
        shared_capability_refs: Vec::new(),
    }
}

#[must_use]
pub fn team_topology_from_refs(
    leader_ref: Option<String>,
    member_refs: Vec<String>,
) -> TeamTopology {
    TeamTopology {
        mode: "leader-orchestrated".into(),
        leader_ref: leader_ref.unwrap_or_default(),
        member_refs,
    }
}

#[must_use]
pub fn default_shared_memory_policy() -> SharedMemoryPolicy {
    SharedMemoryPolicy {
        share_mode: "team-shared".into(),
        writable_by_workers: false,
        require_review_before_persist: true,
    }
}

#[must_use]
pub fn default_mailbox_policy() -> MailboxPolicy {
    MailboxPolicy {
        mode: "leader-hub".into(),
        allow_worker_to_worker: false,
        retain_messages: true,
    }
}

#[must_use]
pub fn default_artifact_handoff_policy() -> ArtifactHandoffPolicy {
    ArtifactHandoffPolicy {
        mode: "leader-reviewed".into(),
        require_lineage: true,
        retain_artifacts: true,
    }
}

#[must_use]
pub fn workflow_affordance_from_task_domains(
    task_domains: &[String],
    background_capable: bool,
    automation_capable: bool,
) -> WorkflowAffordance {
    WorkflowAffordance {
        supported_task_kinds: task_domains.to_vec(),
        background_capable,
        automation_capable,
    }
}

#[must_use]
pub fn default_asset_trust_metadata() -> AssetTrustMetadata {
    AssetTrustMetadata {
        publisher: "workspace-local".into(),
        origin: "native".into(),
        signature_state: "unsigned".into(),
        trust_level: "trusted".into(),
        trust_warnings: Vec::new(),
        verified_at: None,
    }
}

#[must_use]
pub fn default_asset_import_metadata() -> AssetImportMetadata {
    AssetImportMetadata {
        origin_kind: "native".into(),
        source_id: None,
        manifest_version: ASSET_IMPORT_MANIFEST_VERSION,
        translation_status: "native".into(),
        imported_at: None,
    }
}

#[must_use]
pub fn default_readonly_permission_envelope() -> PermissionEnvelope {
    PermissionEnvelope {
        default_mode: RUNTIME_PERMISSION_READ_ONLY.into(),
        max_mode: RUNTIME_PERMISSION_WORKSPACE_WRITE.into(),
        escalation_allowed: true,
        allowed_resource_scopes: vec!["agent-private".into(), "project-shared".into()],
    }
}
