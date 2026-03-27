use chrono::Utc;
use octopus_execution::ExecutionAction;
use octopus_governance::ApprovalRequestRecord;
use octopus_knowledge::{KnowledgeAssetRecord, KnowledgeCandidateRecord, KnowledgeSpaceRecord};
use octopus_observe_artifact::{
    ArtifactRecord, AuditRecord, InboxItemRecord, KnowledgeLineageRecord, NotificationRecord,
    PolicyDecisionLogRecord, TraceRecord,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateTaskInput {
    pub workspace_id: String,
    pub project_id: String,
    pub source_kind: String,
    pub automation_id: Option<String>,
    pub title: String,
    pub instruction: String,
    pub action: ExecutionAction,
    pub capability_id: String,
    pub estimated_cost: i64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub source_kind: String,
    pub automation_id: Option<String>,
    pub title: String,
    pub instruction: String,
    pub action: ExecutionAction,
    pub capability_id: String,
    pub estimated_cost: i64,
    pub idempotency_key: String,
    pub created_at: String,
    pub updated_at: String,
}

impl TaskRecord {
    pub fn new(input: CreateTaskInput) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: input.workspace_id,
            project_id: input.project_id,
            source_kind: input.source_kind,
            automation_id: input.automation_id,
            title: input.title,
            instruction: input.instruction,
            action: input.action,
            capability_id: input.capability_id,
            estimated_cost: input.estimated_cost,
            idempotency_key: input.idempotency_key,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunRecord {
    pub id: String,
    pub task_id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub automation_id: Option<String>,
    pub trigger_delivery_id: Option<String>,
    pub run_type: String,
    pub status: String,
    pub approval_request_id: Option<String>,
    pub idempotency_key: String,
    pub attempt_count: i64,
    pub max_attempts: i64,
    pub checkpoint_seq: i64,
    pub resume_token: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub terminated_at: Option<String>,
}

impl RunRecord {
    pub fn new(task: &TaskRecord, trigger_delivery_id: Option<String>) -> Self {
        let now = current_timestamp();
        let run_type = if task.source_kind == "automation" {
            "automation".to_string()
        } else {
            "task".to_string()
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            workspace_id: task.workspace_id.clone(),
            project_id: task.project_id.clone(),
            automation_id: task.automation_id.clone(),
            trigger_delivery_id,
            run_type,
            status: "created".to_string(),
            approval_request_id: None,
            idempotency_key: format!("run:task:{}", task.id),
            attempt_count: 0,
            max_attempts: 2,
            checkpoint_seq: 0,
            resume_token: None,
            last_error: None,
            created_at: now.clone(),
            updated_at: now,
            started_at: None,
            completed_at: None,
            terminated_at: None,
        }
    }

    pub fn can_retry(&self) -> bool {
        self.status == "failed"
            && self.resume_token.is_some()
            && self.attempt_count < self.max_attempts
    }

    pub fn can_terminate(&self) -> bool {
        matches!(
            self.status.as_str(),
            "created" | "running" | "failed" | "resuming" | "blocked" | "waiting_approval"
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunSummaryRecord {
    pub id: String,
    pub task_id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub title: String,
    pub run_type: String,
    pub status: String,
    pub approval_request_id: Option<String>,
    pub attempt_count: i64,
    pub max_attempts: i64,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub terminated_at: Option<String>,
}

impl RunSummaryRecord {
    pub fn new(run: &RunRecord, task: &TaskRecord) -> Self {
        Self {
            id: run.id.clone(),
            task_id: run.task_id.clone(),
            workspace_id: run.workspace_id.clone(),
            project_id: run.project_id.clone(),
            title: task.title.clone(),
            run_type: run.run_type.clone(),
            status: run.status.clone(),
            approval_request_id: run.approval_request_id.clone(),
            attempt_count: run.attempt_count,
            max_attempts: run.max_attempts,
            last_error: run.last_error.clone(),
            created_at: run.created_at.clone(),
            updated_at: run.updated_at.clone(),
            started_at: run.started_at.clone(),
            completed_at: run.completed_at.clone(),
            terminated_at: run.terminated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunExecutionReport {
    pub run: RunRecord,
    pub artifacts: Vec<ArtifactRecord>,
    pub audits: Vec<AuditRecord>,
    pub traces: Vec<TraceRecord>,
    pub approvals: Vec<ApprovalRequestRecord>,
    pub inbox_items: Vec<InboxItemRecord>,
    pub notifications: Vec<NotificationRecord>,
    pub policy_decisions: Vec<PolicyDecisionLogRecord>,
    pub knowledge_candidates: Vec<KnowledgeCandidateRecord>,
    pub recalled_knowledge_assets: Vec<KnowledgeAssetRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgePromotionReport {
    pub knowledge_space: KnowledgeSpaceRecord,
    pub candidate: KnowledgeCandidateRecord,
    pub asset: KnowledgeAssetRecord,
    pub lineage: KnowledgeLineageRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateAutomationInput {
    pub workspace_id: String,
    pub project_id: String,
    pub title: String,
    pub instruction: String,
    pub action: ExecutionAction,
    pub capability_id: String,
    pub estimated_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutomationRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub trigger_id: String,
    pub status: String,
    pub title: String,
    pub instruction: String,
    pub action: ExecutionAction,
    pub capability_id: String,
    pub estimated_cost: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl AutomationRecord {
    pub fn new(input: CreateAutomationInput, trigger_id: String) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: input.workspace_id,
            project_id: input.project_id,
            trigger_id,
            status: "active".to_string(),
            title: input.title,
            instruction: input.instruction,
            action: input.action,
            capability_id: input.capability_id,
            estimated_cost: input.estimated_cost,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn can_transition_to(&self, next_status: &str) -> bool {
        matches!(
            (self.status.as_str(), next_status),
            ("active", "paused")
                | ("paused", "active")
                | ("active", "archived")
                | ("paused", "archived")
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutomationSummaryRecord {
    pub automation: AutomationRecord,
    pub trigger: TriggerRecord,
    pub recent_deliveries: Vec<TriggerDeliveryRecord>,
    pub last_run_summary: Option<RunSummaryRecord>,
}

pub type AutomationDetailRecord = AutomationSummaryRecord;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualEventTriggerConfig {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CronTriggerConfig {
    pub schedule: String,
    pub timezone: String,
    pub next_fire_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebhookTriggerConfig {
    pub ingress_mode: String,
    pub secret_header_name: String,
    pub secret_hint: Option<String>,
    pub secret_present: bool,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub secret_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpEventTriggerConfig {
    pub server_id: String,
    pub event_name: Option<String>,
    pub event_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "trigger_type", rename_all = "snake_case")]
pub enum TriggerSpec {
    ManualEvent { config: ManualEventTriggerConfig },
    Cron { config: CronTriggerConfig },
    Webhook { config: WebhookTriggerConfig },
    McpEvent { config: McpEventTriggerConfig },
}

impl TriggerSpec {
    pub fn manual_event() -> Self {
        Self::ManualEvent {
            config: ManualEventTriggerConfig {},
        }
    }

    pub fn trigger_type(&self) -> &'static str {
        match self {
            Self::ManualEvent { .. } => "manual_event",
            Self::Cron { .. } => "cron",
            Self::Webhook { .. } => "webhook",
            Self::McpEvent { .. } => "mcp_event",
        }
    }

    pub fn cron_config(&self) -> Option<&CronTriggerConfig> {
        match self {
            Self::Cron { config } => Some(config),
            _ => None,
        }
    }

    pub fn webhook_config(&self) -> Option<&WebhookTriggerConfig> {
        match self {
            Self::Webhook { config } => Some(config),
            _ => None,
        }
    }

    pub fn mcp_event_config(&self) -> Option<&McpEventTriggerConfig> {
        match self {
            Self::McpEvent { config } => Some(config),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerRecord {
    pub id: String,
    pub automation_id: String,
    #[serde(flatten)]
    pub spec: TriggerSpec,
    pub created_at: String,
    pub updated_at: String,
}

impl TriggerRecord {
    pub fn new(automation_id: impl Into<String>, spec: TriggerSpec) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            automation_id: automation_id.into(),
            spec,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn manual_event(automation_id: impl Into<String>) -> Self {
        Self::new(automation_id, TriggerSpec::manual_event())
    }

    pub fn trigger_type(&self) -> &'static str {
        self.spec.trigger_type()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchManualEventInput {
    pub trigger_id: String,
    pub dedupe_key: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CreateTriggerInput {
    ManualEvent,
    Cron {
        schedule: String,
        timezone: String,
        next_fire_at: String,
    },
    Webhook {
        ingress_mode: String,
        secret_header_name: String,
        secret_hint: Option<String>,
        secret_plaintext: Option<String>,
    },
    McpEvent {
        server_id: String,
        event_name: Option<String>,
        event_pattern: Option<String>,
    },
}

impl CreateTriggerInput {
    pub fn manual_event() -> Self {
        Self::ManualEvent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchWebhookEventInput {
    pub trigger_id: String,
    pub idempotency_key: String,
    pub secret: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchMcpEventInput {
    pub trigger_id: String,
    pub server_id: String,
    pub event_name: String,
    pub dedupe_key: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAutomationReport {
    pub automation: AutomationRecord,
    pub trigger: TriggerRecord,
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerDeliveryRecord {
    pub id: String,
    pub trigger_id: String,
    pub run_id: Option<String>,
    pub status: String,
    pub dedupe_key: String,
    pub payload: Value,
    pub attempt_count: i64,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl TriggerDeliveryRecord {
    pub fn new(
        trigger_id: impl Into<String>,
        dedupe_key: impl Into<String>,
        payload: Value,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            trigger_id: trigger_id.into(),
            run_id: None,
            status: "pending".to_string(),
            dedupe_key: dedupe_key.into(),
            payload,
            attempt_count: 0,
            last_error: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TriggerDeliveryReport {
    pub automation: AutomationRecord,
    pub trigger: TriggerRecord,
    pub delivery: TriggerDeliveryRecord,
    pub task: TaskRecord,
    pub run_report: RunExecutionReport,
}

pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
