use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CapabilityDescriptor, RuntimeExecutionProfile, RuntimeSessionSummary};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub provider_id: String,
    pub credential_ref: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub default_surface: Option<String>,
    pub protocol_family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRequestPolicyInput {
    pub auth_strategy: String,
    pub base_url_policy: String,
    pub default_base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResolvedRequestAuthMode {
    None,
    BearerToken,
    Header,
    QueryParam,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRequestAuth {
    pub mode: ResolvedRequestAuthMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRequestPolicy {
    pub base_url: String,
    pub headers: BTreeMap<String, String>,
    pub auth: ResolvedRequestAuth,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedExecutionTarget {
    pub configured_model_id: String,
    pub configured_model_name: String,
    pub provider_id: String,
    pub registry_model_id: String,
    pub model_id: String,
    pub surface: String,
    pub protocol_family: String,
    #[serde(default)]
    pub execution_profile: RuntimeExecutionProfile,
    pub credential_ref: Option<String>,
    pub credential_source: String,
    #[serde(default)]
    pub request_policy: ResolvedRequestPolicyInput,
    pub base_url: Option<String>,
    pub max_output_tokens: Option<u32>,
    pub capabilities: Vec<CapabilityDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBootstrap {
    pub provider: ProviderConfig,
    pub sessions: Vec<RuntimeSessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuntimeSessionInput {
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub title: String,
    pub session_kind: Option<String>,
    pub selected_actor_ref: String,
    pub selected_configured_model_id: Option<String>,
    pub execution_permission_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRuntimeTurnInput {
    pub content: String,
    pub permission_mode: Option<String>,
    #[serde(default)]
    pub recall_mode: Option<String>,
    #[serde(default)]
    pub ignored_memory_ids: Vec<String>,
    #[serde(default)]
    pub memory_intent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRuntimeApprovalInput {
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRuntimeAuthChallengeInput {
    pub resolution: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelRuntimeSubrunInput {
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRuntimeMemoryProposalInput {
    pub decision: String,
    #[serde(default)]
    pub note: Option<String>,
}
