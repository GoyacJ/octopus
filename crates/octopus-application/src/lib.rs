//! Application-layer models and persistence contracts for the Phase 3 MVP slice.

use anyhow::Result;
use async_trait::async_trait;
pub use octopus_domain::{InboxItemStatus, InteractionKind, InteractionResponseType, RunStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub id: String,
    pub workspace_id: String,
    pub agent_id: String,
    pub interaction_type: InteractionKind,
    pub status: RunStatus,
    pub summary: String,
    pub input: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxItemRecord {
    pub id: String,
    pub run_id: String,
    pub kind: InteractionKind,
    pub status: InboxItemStatus,
    pub title: String,
    pub prompt: String,
    pub response_type: InteractionResponseType,
    pub options: Vec<String>,
    pub resume_token: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEventRecord {
    pub id: String,
    pub run_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub summary: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventRecord {
    pub id: String,
    pub actor_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub action: String,
    pub summary: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventEnvelope {
    pub id: String,
    pub run_id: String,
    pub object_type: String,
    pub event_type: String,
    pub actor_id: String,
    pub surface: String,
    pub resume_token: Option<String>,
    pub idempotency_key: Option<String>,
    pub risk_level: String,
    pub budget_context: String,
    pub summary: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunInput {
    pub workspace_id: String,
    pub agent_id: String,
    pub input: String,
    pub interaction_type: InteractionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionResponsePayload {
    #[serde(rename = "type")]
    pub response_type: InteractionResponseType,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<bool>,
    #[serde(default)]
    pub goal_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeRunInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox_item_id: Option<String>,
    pub resume_token: String,
    pub idempotency_key: String,
    pub response: InteractionResponsePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeResult {
    pub accepted: bool,
    pub deduplicated: bool,
    pub run_id: String,
    pub status: RunStatus,
    pub run: RunRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunContext {
    pub run: RunRecord,
    pub pending_inbox_item: Option<InboxItemRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCreationBundle {
    pub run: RunRecord,
    pub inbox_item: InboxItemRecord,
    pub event_envelopes: Vec<EventEnvelope>,
    pub timeline_events: Vec<TimelineEventRecord>,
    pub audit_events: Vec<AuditEventRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeReceipt {
    pub run_id: String,
    pub idempotency_key: String,
    pub final_status: RunStatus,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResumeBundle {
    pub run: RunRecord,
    pub inbox_item: InboxItemRecord,
    pub event_envelopes: Vec<EventEnvelope>,
    pub timeline_events: Vec<TimelineEventRecord>,
    pub audit_events: Vec<AuditEventRecord>,
    pub receipt: ResumeReceipt,
}

#[async_trait]
pub trait Phase3Store: Send + Sync {
    async fn create_run(&self, bundle: RunCreationBundle) -> Result<RunContext>;
    async fn get_run_context(&self, run_id: &str) -> Result<Option<RunContext>>;
    async fn list_runs(&self) -> Result<Vec<RunRecord>>;
    async fn get_run(&self, run_id: &str) -> Result<Option<RunRecord>>;
    async fn list_run_timeline(&self, run_id: &str) -> Result<Vec<TimelineEventRecord>>;
    async fn list_inbox_items(&self) -> Result<Vec<InboxItemRecord>>;
    async fn list_audit_events(&self) -> Result<Vec<AuditEventRecord>>;
    async fn append_audit_events(&self, events: &[AuditEventRecord]) -> Result<()>;
    async fn find_resume_receipt(
        &self,
        run_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<ResumeReceipt>>;
    async fn apply_resume(&self, bundle: RunResumeBundle) -> Result<RunContext>;
}
