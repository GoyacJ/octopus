use chrono::Utc;
use octopus_execution::ExecutionAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateTaskInput {
    pub workspace_id: String,
    pub project_id: String,
    pub title: String,
    pub instruction: String,
    pub action: ExecutionAction,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub title: String,
    pub instruction: String,
    pub action: ExecutionAction,
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
            title: input.title,
            instruction: input.instruction,
            action: input.action,
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
    pub run_type: String,
    pub status: String,
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
    pub fn new(task: &TaskRecord) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            workspace_id: task.workspace_id.clone(),
            project_id: task.project_id.clone(),
            run_type: "task".to_string(),
            status: "created".to_string(),
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
            "created" | "running" | "failed" | "resuming" | "blocked"
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunExecutionReport {
    pub run: RunRecord,
    pub artifacts: Vec<octopus_observe_artifact::ArtifactRecord>,
    pub audits: Vec<octopus_observe_artifact::AuditRecord>,
    pub traces: Vec<octopus_observe_artifact::TraceRecord>,
}

pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
