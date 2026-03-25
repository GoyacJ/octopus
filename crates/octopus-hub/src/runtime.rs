use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::contracts::{ApprovalType, RunStatus, RunType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSubmissionRequest {
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub requested_by: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResolutionRequest {
    pub decision: String,
    pub reviewed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxState {
    Open,
    Acknowledged,
    Resolved,
    Dismissed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub project_id: String,
    pub run_type: RunType,
    pub status: RunStatus,
    pub idempotency_key: String,
    pub requested_by: String,
    pub title: String,
    pub checkpoint_token: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub project_id: String,
    pub run_id: String,
    pub version: u32,
    pub title: String,
    pub content_ref: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequestRecord {
    pub id: String,
    pub run_id: String,
    pub approval_type: ApprovalType,
    pub state: ApprovalState,
    pub target_ref: String,
    pub requested_at: String,
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItemRecord {
    pub id: String,
    pub workspace_id: String,
    pub owner_ref: String,
    pub state: InboxState,
    pub priority: String,
    pub target_ref: String,
    pub dedupe_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub name: String,
    pub message: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub action: String,
    pub actor: String,
    pub target_ref: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDetailResponse {
    pub run: RunRecord,
    pub artifact: Option<ArtifactRecord>,
    pub approval: Option<ApprovalRequestRecord>,
    pub inbox_item: Option<InboxItemRecord>,
    pub trace: Vec<TraceEvent>,
    pub audit: Vec<AuditEntry>,
}

#[derive(Default)]
struct RuntimeState {
    runs: HashMap<String, RunRecord>,
    artifacts: HashMap<String, ArtifactRecord>,
    approvals: HashMap<String, ApprovalRequestRecord>,
    inbox_items: HashMap<String, InboxItemRecord>,
    traces: HashMap<String, Vec<TraceEvent>>,
    audits: HashMap<String, Vec<AuditEntry>>,
}

#[derive(Clone, Default)]
pub struct InMemoryRuntimeService {
    state: Arc<Mutex<RuntimeState>>,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: String },
    #[error("run {run_id} is in invalid state: {reason}")]
    InvalidState { run_id: String, reason: String },
}

impl InMemoryRuntimeService {
    pub fn submit_task(&self, request: TaskSubmissionRequest) -> RunDetailResponse {
        let run_id = Uuid::new_v4().to_string();
        let now = now_iso();
        let idempotency_key = format!("task:{}:{}", request.project_id, run_id);

        let mut run = RunRecord {
            id: run_id.clone(),
            project_id: request.project_id.clone(),
            run_type: RunType::Task,
            status: RunStatus::Planning,
            idempotency_key,
            requested_by: request.requested_by.clone(),
            title: request.title.clone(),
            checkpoint_token: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let mut artifact = None;
        let mut approval = None;
        let mut inbox_item = None;
        let mut trace = vec![trace_event(
            "RunStateChanged",
            format!("Run {} entered planning", run.id),
        )];
        let mut audit = vec![audit_entry("task.submitted", &request.requested_by, &run.id)];

        if request.requires_approval {
            run.status = RunStatus::WaitingApproval;
            run.checkpoint_token = Some(format!("resume:{}", run.id));
            approval = Some(ApprovalRequestRecord {
                id: Uuid::new_v4().to_string(),
                run_id: run.id.clone(),
                approval_type: ApprovalType::Execution,
                state: ApprovalState::Pending,
                target_ref: run.id.clone(),
                requested_at: now_iso(),
                reviewed_by: None,
            });
            inbox_item = Some(InboxItemRecord {
                id: Uuid::new_v4().to_string(),
                workspace_id: request.project_id.clone(),
                owner_ref: "reviewer.execution".into(),
                state: InboxState::Open,
                priority: "high".into(),
                target_ref: run.id.clone(),
                dedupe_key: format!("approval:{}", run.id),
            });
            trace.push(trace_event(
                "ApprovalRequested",
                format!("Run {} requires execution approval", run.id),
            ));
            trace.push(trace_event(
                "RunStateChanged",
                format!("Run {} is waiting for approval", run.id),
            ));
        } else {
            run.status = RunStatus::Completed;
            artifact = Some(build_artifact(
                &run.id,
                &run.project_id,
                &run.title,
                request.description.as_deref(),
            ));
            trace.push(trace_event(
                "RunStateChanged",
                format!("Run {} completed without approval", run.id),
            ));
            audit.push(audit_entry("artifact.created", &request.requested_by, &run.id));
        }

        let response = RunDetailResponse {
            run: run.clone(),
            artifact: artifact.clone(),
            approval: approval.clone(),
            inbox_item: inbox_item.clone(),
            trace: trace.clone(),
            audit: audit.clone(),
        };

        let mut state = self.state.lock().expect("runtime state should lock");
        state.runs.insert(run.id.clone(), run);
        if let Some(entry) = artifact {
            state.artifacts.insert(entry.run_id.clone(), entry);
        }
        if let Some(entry) = approval {
            state.approvals.insert(entry.id.clone(), entry);
        }
        if let Some(entry) = inbox_item {
            state.inbox_items.insert(entry.target_ref.clone(), entry);
        }
        state.traces.insert(response.run.id.clone(), trace);
        state.audits.insert(response.run.id.clone(), audit);

        response
    }

    pub fn get_run(&self, run_id: &str) -> Option<RunDetailResponse> {
        let state = self.state.lock().expect("runtime state should lock");
        hydrate_response(&state, run_id)
    }

    pub fn resolve_approval(
        &self,
        approval_id: &str,
        request: ApprovalResolutionRequest,
    ) -> Result<RunDetailResponse, RuntimeError> {
        let mut state = self.state.lock().expect("runtime state should lock");
        let decision = request.decision.to_lowercase();
        let (run_id, approval_id, next_status) = {
            let approval = state
                .approvals
                .get_mut(approval_id)
                .ok_or_else(|| RuntimeError::NotFound {
                    kind: "approval",
                    id: approval_id.to_string(),
                })?;

            approval.reviewed_by = Some(request.reviewed_by.clone());
            approval.state = if decision == "approved" {
                ApprovalState::Approved
            } else {
                ApprovalState::Rejected
            };

            (
                approval.run_id.clone(),
                approval.id.clone(),
                if decision == "approved" {
                    RunStatus::Paused
                } else {
                    RunStatus::Terminated
                },
            )
        };

        {
            let run = state.runs.get_mut(&run_id).ok_or_else(|| RuntimeError::NotFound {
                kind: "run",
                id: run_id.clone(),
            })?;

            if run.status != RunStatus::WaitingApproval {
                return Err(RuntimeError::InvalidState {
                    run_id: run.id.clone(),
                    reason: "approval can only be resolved while waiting_approval".into(),
                });
            }

            run.status = next_status;
            run.updated_at = now_iso();
        }

        if let Some(inbox_item) = state.inbox_items.get_mut(&run_id) {
            inbox_item.state = InboxState::Resolved;
        }

        if next_status == RunStatus::Paused {
            push_trace(
                &mut state,
                &run_id,
                trace_event(
                    "ApprovalResolved",
                    format!("Approval {} approved by {}", approval_id, request.reviewed_by),
                ),
            );
            push_trace(
                &mut state,
                &run_id,
                trace_event(
                    "RunStateChanged",
                    format!("Run {} paused and ready to resume", run_id),
                ),
            );
            push_audit(
                &mut state,
                &run_id,
                audit_entry("approval.approved", &request.reviewed_by, &approval_id),
            );
        } else {
            push_trace(
                &mut state,
                &run_id,
                trace_event(
                    "ApprovalResolved",
                    format!("Approval {} rejected by {}", approval_id, request.reviewed_by),
                ),
            );
            push_trace(
                &mut state,
                &run_id,
                trace_event(
                    "RunStateChanged",
                    format!("Run {} terminated after rejection", run_id),
                ),
            );
            push_audit(
                &mut state,
                &run_id,
                audit_entry("approval.rejected", &request.reviewed_by, &approval_id),
            );
        }

        hydrate_response(&state, &run_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "run",
            id: run_id,
        })
    }

    pub fn resume_run(&self, run_id: &str) -> Result<RunDetailResponse, RuntimeError> {
        let mut state = self.state.lock().expect("runtime state should lock");
        let (project_id, title, requested_by) = {
            let run = state.runs.get_mut(run_id).ok_or_else(|| RuntimeError::NotFound {
                kind: "run",
                id: run_id.to_string(),
            })?;

            if run.status != RunStatus::Paused {
                return Err(RuntimeError::InvalidState {
                    run_id: run.id.clone(),
                    reason: "resume is only allowed after approval grants a checkpoint".into(),
                });
            }

            run.status = RunStatus::Running;
            run.updated_at = now_iso();

            (
                run.project_id.clone(),
                run.title.clone(),
                run.requested_by.clone(),
            )
        };

        push_trace(
            &mut state,
            run_id,
            trace_event("RunStateChanged", format!("Run {} resumed execution", run_id)),
        );

        let artifact = build_artifact(run_id, &project_id, &title, Some("Generated after explicit resume"));
        state.artifacts.insert(run_id.to_string(), artifact);

        {
            let run = state.runs.get_mut(run_id).ok_or_else(|| RuntimeError::NotFound {
                kind: "run",
                id: run_id.to_string(),
            })?;

            run.status = RunStatus::Completed;
            run.updated_at = now_iso();
        }

        push_trace(
            &mut state,
            run_id,
            trace_event("RunStateChanged", format!("Run {} completed after resume", run_id)),
        );
        push_audit(
            &mut state,
            run_id,
            audit_entry("run.resumed", &requested_by, run_id),
        );

        hydrate_response(&state, run_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "run",
            id: run_id.to_string(),
        })
    }
}

fn hydrate_response(state: &RuntimeState, run_id: &str) -> Option<RunDetailResponse> {
    let run = state.runs.get(run_id)?.clone();
    let artifact = state.artifacts.get(run_id).cloned();
    let approval = state
        .approvals
        .values()
        .find(|entry| entry.run_id == run_id)
        .cloned();
    let inbox_item = state.inbox_items.get(run_id).cloned();
    let trace = state.traces.get(run_id).cloned().unwrap_or_default();
    let audit = state.audits.get(run_id).cloned().unwrap_or_default();

    Some(RunDetailResponse {
        run,
        artifact,
        approval,
        inbox_item,
        trace,
        audit,
    })
}

fn build_artifact(run_id: &str, project_id: &str, title: &str, description: Option<&str>) -> ArtifactRecord {
    ArtifactRecord {
        id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        run_id: run_id.to_string(),
        version: 1,
        title: format!("Artifact for {}", title),
        content_ref: description
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "Generated by the minimal runtime skeleton".into()),
        state: "current".into(),
        created_at: now_iso(),
    }
}

fn trace_event(name: &str, message: String) -> TraceEvent {
    TraceEvent {
        name: name.to_string(),
        message,
        occurred_at: now_iso(),
    }
}

fn audit_entry(action: &str, actor: &str, target_ref: &str) -> AuditEntry {
    AuditEntry {
        action: action.to_string(),
        actor: actor.to_string(),
        target_ref: target_ref.to_string(),
        occurred_at: now_iso(),
    }
}

fn push_trace(state: &mut RuntimeState, run_id: &str, entry: TraceEvent) {
    state.traces.entry(run_id.to_string()).or_default().push(entry);
}

fn push_audit(state: &mut RuntimeState, run_id: &str, entry: AuditEntry) {
    state.audits.entry(run_id.to_string()).or_default().push(entry);
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
