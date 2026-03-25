use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::contracts::{ApprovalType, RunStatus, RunType, TriggerSource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSubmissionRequest {
    pub workspace_id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub requested_by: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationCreateRequest {
    pub workspace_id: String,
    pub project_id: String,
    pub name: String,
    pub trigger_source: TriggerSource,
    pub requested_by: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationStateUpdateRequest {
    pub state: AutomationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDeliveryRequest {
    pub trigger_id: String,
    pub dedupe_key: String,
    pub requested_by: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResolutionRequest {
    pub decision: String,
    pub reviewed_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxState {
    Open,
    Acknowledged,
    Resolved,
    Dismissed,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationState {
    Draft,
    Active,
    Paused,
    Suspended,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerDeliveryState {
    Pending,
    Claimed,
    Dispatched,
    Succeeded,
    Failed,
    Retried,
    DeadLetter,
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
pub struct AutomationRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub name: String,
    pub trigger_ids: Vec<String>,
    pub state: AutomationState,
    pub requires_approval: bool,
    pub last_run_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRecord {
    pub id: String,
    pub automation_id: String,
    pub source_type: TriggerSource,
    pub dedupe_key: String,
    pub owner_ref: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDeliveryRecord {
    pub id: String,
    pub trigger_id: String,
    pub source_type: TriggerSource,
    pub dedupe_key: String,
    pub state: TriggerDeliveryState,
    pub run_id: Option<String>,
    pub failure_reason: Option<String>,
    pub occurred_at: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationDetailResponse {
    pub automation: AutomationRecord,
    pub trigger: TriggerRecord,
    pub latest_delivery: Option<TriggerDeliveryRecord>,
    pub latest_run: Option<RunDetailResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationListResponse {
    pub items: Vec<AutomationDetailResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDeliveryResponse {
    pub delivery: TriggerDeliveryRecord,
    pub run: Option<RunDetailResponse>,
}

#[derive(Default)]
struct RuntimeState {
    runs: HashMap<String, RunRecord>,
    artifacts: HashMap<String, ArtifactRecord>,
    approvals: HashMap<String, ApprovalRequestRecord>,
    inbox_items: HashMap<String, InboxItemRecord>,
    traces: HashMap<String, Vec<TraceEvent>>,
    audits: HashMap<String, Vec<AuditEntry>>,
    automations: HashMap<String, AutomationRecord>,
    triggers: HashMap<String, TriggerRecord>,
    trigger_deliveries: HashMap<String, TriggerDeliveryRecord>,
    delivery_dedupe_index: HashMap<String, String>,
    latest_delivery_by_trigger: HashMap<String, String>,
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
    #[error("invalid approval decision: {decision}")]
    InvalidDecision { decision: String },
}

struct RunStartRequest {
    workspace_id: String,
    project_id: String,
    run_type: RunType,
    idempotency_key: String,
    requested_by: String,
    title: String,
    description: Option<String>,
    requires_approval: bool,
    initial_audit_action: &'static str,
    initial_audit_target: AuditTarget,
}

enum AuditTarget {
    Run,
    Explicit(String),
}

impl InMemoryRuntimeService {
    pub fn submit_task(&self, request: TaskSubmissionRequest) -> RunDetailResponse {
        let mut state = self.state.lock().expect("runtime state should lock");

        start_run(
            &mut state,
            RunStartRequest {
                workspace_id: request.workspace_id,
                project_id: request.project_id.clone(),
                run_type: RunType::Task,
                idempotency_key: format!("task:{}", Uuid::new_v4()),
                requested_by: request.requested_by,
                title: request.title,
                description: request.description,
                requires_approval: request.requires_approval,
                initial_audit_action: "task.submitted",
                initial_audit_target: AuditTarget::Run,
            },
        )
    }

    pub fn create_automation(&self, request: AutomationCreateRequest) -> AutomationDetailResponse {
        let mut state = self.state.lock().expect("runtime state should lock");
        let automation_id = Uuid::new_v4().to_string();
        let trigger_id = Uuid::new_v4().to_string();
        let now = now_iso();

        let automation = AutomationRecord {
            id: automation_id.clone(),
            workspace_id: request.workspace_id,
            project_id: request.project_id,
            name: request.name,
            trigger_ids: vec![trigger_id.clone()],
            state: AutomationState::Active,
            requires_approval: request.requires_approval,
            last_run_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let trigger = TriggerRecord {
            id: trigger_id.clone(),
            automation_id: automation_id.clone(),
            source_type: request.trigger_source,
            dedupe_key: format!("automation:{automation_id}"),
            owner_ref: format!("automation:{automation_id}"),
            state: "active".into(),
            created_at: now,
        };

        state.automations.insert(automation_id.clone(), automation);
        state.triggers.insert(trigger_id, trigger);

        hydrate_automation_detail(&state, &automation_id).expect("new automation should hydrate")
    }

    pub fn list_automations(&self) -> Vec<AutomationDetailResponse> {
        let state = self.state.lock().expect("runtime state should lock");
        let mut automation_ids = state.automations.keys().cloned().collect::<Vec<_>>();
        automation_ids.sort();

        automation_ids
            .into_iter()
            .filter_map(|automation_id| hydrate_automation_detail(&state, &automation_id))
            .collect()
    }

    pub fn update_automation_state(
        &self,
        automation_id: &str,
        request: AutomationStateUpdateRequest,
    ) -> Result<AutomationDetailResponse, RuntimeError> {
        let mut state = self.state.lock().expect("runtime state should lock");
        let automation = state
            .automations
            .get_mut(automation_id)
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "automation",
                id: automation_id.to_string(),
            })?;

        automation.state = request.state;
        automation.updated_at = now_iso();

        sync_trigger_states(&mut state, automation_id, request.state);

        hydrate_automation_detail(&state, automation_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "automation",
            id: automation_id.to_string(),
        })
    }

    pub fn deliver_trigger(
        &self,
        request: TriggerDeliveryRequest,
    ) -> Result<TriggerDeliveryResponse, RuntimeError> {
        let mut state = self.state.lock().expect("runtime state should lock");
        let dedupe_key = format!("{}:{}", request.trigger_id, request.dedupe_key);

        if let Some(delivery_id) = state.delivery_dedupe_index.get(&dedupe_key).cloned() {
            let delivery = state
                .trigger_deliveries
                .get(&delivery_id)
                .cloned()
                .ok_or_else(|| RuntimeError::NotFound {
                    kind: "trigger_delivery",
                    id: delivery_id,
                })?;
            let run = delivery
                .run_id
                .as_deref()
                .and_then(|run_id| hydrate_response(&state, run_id));

            return Ok(TriggerDeliveryResponse { delivery, run });
        }

        let trigger = state
            .triggers
            .get(&request.trigger_id)
            .cloned()
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "trigger",
                id: request.trigger_id.clone(),
            })?;
        let automation = state
            .automations
            .get(&trigger.automation_id)
            .cloned()
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "automation",
                id: trigger.automation_id.clone(),
            })?;

        if automation.state != AutomationState::Active {
            let delivery = record_failed_delivery(
                &mut state,
                &trigger,
                &request.dedupe_key,
                format!(
                    "automation {} is paused and cannot accept deliveries",
                    automation.id
                ),
            );

            return Err(RuntimeError::InvalidState {
                run_id: automation.id,
                reason: delivery
                    .failure_reason
                    .unwrap_or_else(|| "delivery failed".into()),
            });
        }

        let delivery_id = Uuid::new_v4().to_string();
        let run_title = request.title.clone().unwrap_or_else(|| automation.name.clone());
        let run_type = trigger_run_type(trigger.source_type);
        let run_response = start_run(
            &mut state,
            RunStartRequest {
                workspace_id: automation.workspace_id.clone(),
                project_id: automation.project_id.clone(),
                run_type,
                idempotency_key: format!("trigger:{}:{}", trigger.id, request.dedupe_key),
                requested_by: request.requested_by.clone(),
                title: run_title,
                description: request.description.clone(),
                requires_approval: automation.requires_approval,
                initial_audit_action: "trigger.delivered",
                initial_audit_target: AuditTarget::Explicit(delivery_id.clone()),
            },
        );

        push_trace(
            &mut state,
            &run_response.run.id,
            trace_event(
                "TriggerDelivered",
                format!(
                    "Trigger {} delivered {}",
                    trigger.id, request.dedupe_key
                ),
            ),
        );

        let delivery = TriggerDeliveryRecord {
            id: delivery_id.clone(),
            trigger_id: trigger.id.clone(),
            source_type: trigger.source_type,
            dedupe_key: request.dedupe_key.clone(),
            state: TriggerDeliveryState::Succeeded,
            run_id: Some(run_response.run.id.clone()),
            failure_reason: None,
            occurred_at: now_iso(),
        };

        state
            .trigger_deliveries
            .insert(delivery_id.clone(), delivery.clone());
        state
            .delivery_dedupe_index
            .insert(dedupe_key, delivery_id.clone());
        state
            .latest_delivery_by_trigger
            .insert(trigger.id.clone(), delivery_id);

        if let Some(entry) = state.automations.get_mut(&automation.id) {
            entry.last_run_id = Some(run_response.run.id.clone());
            entry.updated_at = now_iso();
        }

        let run = hydrate_response(&state, &run_response.run.id);

        Ok(TriggerDeliveryResponse { delivery, run })
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
        let decision = ApprovalDecision::try_from(request.decision.as_str())?;
        let mut state = self.state.lock().expect("runtime state should lock");
        let (run_id, resolved_approval_id) = {
            let approval = state
                .approvals
                .get(approval_id)
                .ok_or_else(|| RuntimeError::NotFound {
                    kind: "approval",
                    id: approval_id.to_string(),
                })?;

            (approval.run_id.clone(), approval.id.clone())
        };

        {
            let run = state.runs.get(&run_id).ok_or_else(|| RuntimeError::NotFound {
                kind: "run",
                id: run_id.clone(),
            })?;

            if run.status != RunStatus::WaitingApproval {
                return Err(RuntimeError::InvalidState {
                    run_id: run.id.clone(),
                    reason: "approval can only be resolved while waiting_approval".into(),
                });
            }
        }

        {
            let approval = state
                .approvals
                .get_mut(approval_id)
                .ok_or_else(|| RuntimeError::NotFound {
                    kind: "approval",
                    id: approval_id.to_string(),
                })?;

            approval.reviewed_by = Some(request.reviewed_by.clone());
            approval.state = decision.into();
        }

        {
            let run = state.runs.get_mut(&run_id).ok_or_else(|| RuntimeError::NotFound {
                kind: "run",
                id: run_id.clone(),
            })?;

            run.status = decision.next_status();
            if decision == ApprovalDecision::Rejected {
                run.checkpoint_token = None;
            }
            run.updated_at = now_iso();
        }

        if let Some(inbox_item) = state.inbox_items.get_mut(&run_id) {
            inbox_item.state = InboxState::Resolved;
        }

        if decision == ApprovalDecision::Approved {
            push_trace(
                &mut state,
                &run_id,
                trace_event(
                    "ApprovalResolved",
                    format!(
                        "Approval {} approved by {}",
                        resolved_approval_id, request.reviewed_by
                    ),
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
                audit_entry("approval.approved", &request.reviewed_by, &resolved_approval_id),
            );
        } else {
            push_trace(
                &mut state,
                &run_id,
                trace_event(
                    "ApprovalResolved",
                    format!(
                        "Approval {} rejected by {}",
                        resolved_approval_id, request.reviewed_by
                    ),
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
                audit_entry("approval.rejected", &request.reviewed_by, &resolved_approval_id),
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
            run.checkpoint_token = None;
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

        let artifact = build_artifact(
            run_id,
            &project_id,
            &title,
            Some("Generated after explicit resume"),
        );
        store_artifact_and_audit(&mut state, artifact, &requested_by, run_id);

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

fn start_run(state: &mut RuntimeState, request: RunStartRequest) -> RunDetailResponse {
    let run_id = Uuid::new_v4().to_string();
    let now = now_iso();
    let initial_audit_target = match request.initial_audit_target {
        AuditTarget::Run => run_id.clone(),
        AuditTarget::Explicit(target_ref) => target_ref,
    };

    let mut run = RunRecord {
        id: run_id.clone(),
        project_id: request.project_id.clone(),
        run_type: request.run_type,
        status: RunStatus::Planning,
        idempotency_key: request.idempotency_key,
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
    let mut audit = vec![audit_entry(
        request.initial_audit_action,
        &request.requested_by,
        &initial_audit_target,
    )];

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
            workspace_id: request.workspace_id,
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
        let built_artifact = build_artifact(
            &run.id,
            &run.project_id,
            &run.title,
            request.description.as_deref(),
        );
        audit.push(audit_entry(
            "artifact.created",
            &request.requested_by,
            &built_artifact.id,
        ));
        artifact = Some(built_artifact);
        trace.push(trace_event(
            "RunStateChanged",
            format!("Run {} completed without approval", run.id),
        ));
    }

    let response = RunDetailResponse {
        run: run.clone(),
        artifact: artifact.clone(),
        approval: approval.clone(),
        inbox_item: inbox_item.clone(),
        trace: trace.clone(),
        audit: audit.clone(),
    };

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

fn hydrate_automation_detail(
    state: &RuntimeState,
    automation_id: &str,
) -> Option<AutomationDetailResponse> {
    let automation = state.automations.get(automation_id)?.clone();
    let trigger_id = automation.trigger_ids.first()?.clone();
    let trigger = state.triggers.get(&trigger_id)?.clone();
    let latest_delivery = state
        .latest_delivery_by_trigger
        .get(&trigger_id)
        .and_then(|delivery_id| state.trigger_deliveries.get(delivery_id))
        .cloned();
    let latest_run = automation
        .last_run_id
        .as_deref()
        .and_then(|run_id| hydrate_response(state, run_id));

    Some(AutomationDetailResponse {
        automation,
        trigger,
        latest_delivery,
        latest_run,
    })
}

fn record_failed_delivery(
    state: &mut RuntimeState,
    trigger: &TriggerRecord,
    dedupe_key: &str,
    failure_reason: String,
) -> TriggerDeliveryRecord {
    let delivery_id = Uuid::new_v4().to_string();
    let composite_dedupe_key = format!("{}:{dedupe_key}", trigger.id);
    let delivery = TriggerDeliveryRecord {
        id: delivery_id.clone(),
        trigger_id: trigger.id.clone(),
        source_type: trigger.source_type,
        dedupe_key: dedupe_key.to_string(),
        state: TriggerDeliveryState::Failed,
        run_id: None,
        failure_reason: Some(failure_reason),
        occurred_at: now_iso(),
    };

    state
        .trigger_deliveries
        .insert(delivery_id.clone(), delivery.clone());
    state
        .delivery_dedupe_index
        .insert(composite_dedupe_key, delivery_id.clone());
    state
        .latest_delivery_by_trigger
        .insert(trigger.id.clone(), delivery_id);

    delivery
}

fn sync_trigger_states(state: &mut RuntimeState, automation_id: &str, automation_state: AutomationState) {
    let trigger_state = match automation_state {
        AutomationState::Active => "active",
        AutomationState::Draft => "draft",
        AutomationState::Paused => "paused",
        AutomationState::Suspended => "suspended",
        AutomationState::Archived => "archived",
    };

    for trigger in state
        .triggers
        .values_mut()
        .filter(|entry| entry.automation_id == automation_id)
    {
        trigger.state = trigger_state.to_string();
    }
}

fn trigger_run_type(source_type: TriggerSource) -> RunType {
    match source_type {
        TriggerSource::Cron => RunType::Automation,
        TriggerSource::Webhook | TriggerSource::ManualEvent | TriggerSource::McpEvent => {
            RunType::Watch
        }
    }
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

fn store_artifact_and_audit(
    state: &mut RuntimeState,
    artifact: ArtifactRecord,
    actor: &str,
    run_id: &str,
) {
    let artifact_id = artifact.id.clone();
    state.artifacts.insert(run_id.to_string(), artifact);
    push_audit(state, run_id, audit_entry("artifact.created", actor, &artifact_id));
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

impl ApprovalDecision {
    fn next_status(self) -> RunStatus {
        match self {
            Self::Approved => RunStatus::Paused,
            Self::Rejected => RunStatus::Terminated,
        }
    }
}

impl TryFrom<&str> for ApprovalDecision {
    type Error = RuntimeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            _ => Err(RuntimeError::InvalidDecision {
                decision: value.to_string(),
            }),
        }
    }
}

impl From<ApprovalDecision> for ApprovalState {
    fn from(value: ApprovalDecision) -> Self {
        match value {
            ApprovalDecision::Approved => Self::Approved,
            ApprovalDecision::Rejected => Self::Rejected,
        }
    }
}
