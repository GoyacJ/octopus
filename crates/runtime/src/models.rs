use chrono::Utc;
use octopus_execution::ExecutionAction;
use octopus_governance::ApprovalRequestRecord;
use octopus_observe_artifact::{
    ArtifactRecord, AuditRecord, InboxItemRecord, NotificationRecord, PolicyDecisionLogRecord,
    TraceRecord,
};
use serde_json::Value;
use serde::{Deserialize, Serialize};

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
            title: input.title,
            instruction: input.instruction,
            action: input.action,
            capability_id: input.capability_id,
            estimated_cost: input.estimated_cost,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerRecord {
    pub id: String,
    pub automation_id: String,
    pub trigger_type: String,
    pub created_at: String,
    pub updated_at: String,
}

impl TriggerRecord {
    pub fn manual_event(automation_id: impl Into<String>) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            automation_id: automation_id.into(),
            trigger_type: "manual_event".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchManualEventInput {
    pub trigger_id: String,
    pub dedupe_key: String,
    pub payload: Value,
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
