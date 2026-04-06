use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
};

use api as _;
use async_trait::async_trait;
use octopus_core::{
    normalize_runtime_permission_mode_label, timestamp_now, AppError, ApprovalRequestRecord,
    AuditRecord, CostLedgerEntry, CreateRuntimeSessionInput, ProviderConfig,
    ResolveRuntimeApprovalInput, RuntimeBootstrap, RuntimeEventEnvelope, RuntimeMessage,
    RuntimeRunSnapshot, RuntimeSessionDetail, RuntimeSessionSummary, RuntimeTraceItem,
    SubmitRuntimeTurnInput, TraceEventRecord, RUNTIME_PERMISSION_WORKSPACE_WRITE,
};
use octopus_infra::WorkspacePaths;
use octopus_platform::{
    ObservationService, RuntimeExecutionService, RuntimeSessionService,
};
use plugins as _;
use runtime as _;
use serde_json::json;
use tokio::sync::broadcast;
use tools as _;
use uuid::Uuid;

#[derive(Clone)]
pub struct RuntimeAdapter {
    state: Arc<RuntimeState>,
}

struct RuntimeState {
    workspace_id: String,
    paths: WorkspacePaths,
    observation: Arc<dyn ObservationService>,
    sessions: Mutex<HashMap<String, RuntimeAggregate>>,
    order: Mutex<Vec<String>>,
    broadcasters: Mutex<HashMap<String, broadcast::Sender<RuntimeEventEnvelope>>>,
}

#[derive(Clone)]
struct RuntimeAggregate {
    detail: RuntimeSessionDetail,
    events: Vec<RuntimeEventEnvelope>,
}

fn optional_project_id(project_id: &str) -> Option<String> {
    if project_id.is_empty() {
        None
    } else {
        Some(project_id.to_string())
    }
}

impl RuntimeAdapter {
    pub fn new(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
    ) -> Self {
        Self {
            state: Arc::new(RuntimeState {
                workspace_id: workspace_id.into(),
                paths,
                observation,
                sessions: Mutex::new(HashMap::new()),
                order: Mutex::new(Vec::new()),
                broadcasters: Mutex::new(HashMap::new()),
            }),
        }
    }

    fn session_sender(
        &self,
        session_id: &str,
    ) -> Result<broadcast::Sender<RuntimeEventEnvelope>, AppError> {
        let mut broadcasters = self
            .state
            .broadcasters
            .lock()
            .map_err(|_| AppError::runtime("broadcast mutex poisoned"))?;
        Ok(broadcasters
            .entry(session_id.to_string())
            .or_insert_with(|| broadcast::channel(128).0)
            .clone())
    }

    fn persist_session(&self, session_id: &str, aggregate: &RuntimeAggregate) -> Result<(), AppError> {
        let session_path = self
            .state
            .paths
            .runtime_sessions_dir
            .join(format!("{session_id}.json"));
        fs::write(&session_path, serde_json::to_vec_pretty(&aggregate.detail)?)?;

        let events_path = self
            .state
            .paths
            .runtime_sessions_dir
            .join(format!("{session_id}-events.json"));
        fs::write(&events_path, serde_json::to_vec_pretty(&aggregate.events)?)?;
        Ok(())
    }

    async fn emit_event(
        &self,
        session_id: &str,
        mut event: RuntimeEventEnvelope,
    ) -> Result<(), AppError> {
        let mut sessions = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        event.sequence = aggregate
            .events
            .last()
            .map(|existing| existing.sequence + 1)
            .unwrap_or(1);
        if event.kind.is_none() {
            event.kind = Some(event.event_type.clone());
        }
        aggregate.events.push(event.clone());
        self.persist_session(session_id, aggregate)?;
        drop(sessions);

        let sender = self.session_sender(session_id)?;
        let _ = sender.send(event);
        Ok(())
    }
}

#[async_trait]
impl RuntimeSessionService for RuntimeAdapter {
    async fn bootstrap(&self) -> Result<RuntimeBootstrap, AppError> {
        Ok(RuntimeBootstrap {
            provider: ProviderConfig {
                provider: "anthropic".into(),
                api_key: None,
                base_url: None,
                default_model: Some("claude-sonnet-4-5".into()),
            },
            sessions: self.list_sessions().await?,
        })
    }

    async fn list_sessions(&self) -> Result<Vec<RuntimeSessionSummary>, AppError> {
        let sessions = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let order = self
            .state
            .order
            .lock()
            .map_err(|_| AppError::runtime("runtime order mutex poisoned"))?;

        Ok(order
            .iter()
            .filter_map(|session_id| sessions.get(session_id).map(|aggregate| aggregate.detail.summary.clone()))
            .collect())
    }

    async fn create_session(
        &self,
        input: CreateRuntimeSessionInput,
    ) -> Result<RuntimeSessionDetail, AppError> {
        let session_id = format!("rt-{}", Uuid::new_v4());
        let conversation_id = if input.conversation_id.is_empty() {
            format!("conv-{}", Uuid::new_v4())
        } else {
            input.conversation_id
        };
        let run_id = format!("run-{}", Uuid::new_v4());
        let now = timestamp_now();

        let detail = RuntimeSessionDetail {
            summary: RuntimeSessionSummary {
                id: session_id.clone(),
                conversation_id: conversation_id.clone(),
                project_id: input.project_id,
                title: input.title,
                status: "draft".into(),
                updated_at: now,
                last_message_preview: None,
            },
            run: RuntimeRunSnapshot {
                id: run_id,
                session_id: session_id.clone(),
                conversation_id: conversation_id.clone(),
                status: "draft".into(),
                current_step: "ready".into(),
                started_at: now,
                updated_at: now,
                model_id: None,
                next_action: Some("submit_turn".into()),
            },
            messages: Vec::new(),
            trace: Vec::new(),
            pending_approval: None,
        };
        let aggregate = RuntimeAggregate {
            detail: detail.clone(),
            events: Vec::new(),
        };

        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?
            .insert(session_id.clone(), aggregate.clone());
        self.state
            .order
            .lock()
            .map_err(|_| AppError::runtime("runtime order mutex poisoned"))?
            .push(session_id.clone());
        self.persist_session(&session_id, &aggregate)?;

        let event = RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.session.updated".into(),
            kind: Some("runtime.session.updated".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&detail.summary.project_id),
            session_id: session_id.clone(),
            conversation_id,
            run_id: Some(detail.run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "summary": detail.summary.clone(),
                "run": detail.run.clone(),
            })),
            run: Some(detail.run.clone()),
            message: None,
            trace: None,
            approval: None,
            decision: None,
            summary: Some(detail.summary.clone()),
            error: None,
        };
        self.emit_event(&session_id, event).await?;

        Ok(detail)
    }

    async fn get_session(&self, session_id: &str) -> Result<RuntimeSessionDetail, AppError> {
        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?
            .get(session_id)
            .map(|aggregate| aggregate.detail.clone())
            .ok_or_else(|| AppError::not_found("runtime session"))
    }

    async fn list_events(
        &self,
        session_id: &str,
        after: Option<&str>,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError> {
        let sessions = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        if let Some(after_id) = after {
            let position = aggregate
                .events
                .iter()
                .position(|event| event.id == after_id)
                .map(|index| index + 1)
                .unwrap_or(0);
            return Ok(aggregate.events[position..].to_vec());
        }

        Ok(aggregate.events.clone())
    }
}

#[async_trait]
impl RuntimeExecutionService for RuntimeAdapter {
    async fn submit_turn(
        &self,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let normalized_permission_mode = normalize_runtime_permission_mode_label(&input.permission_mode)
            .ok_or_else(|| AppError::invalid_input(format!(
                "unsupported permission mode: {}",
                input.permission_mode
            )))?;
        let requires_approval = normalized_permission_mode == RUNTIME_PERMISSION_WORKSPACE_WRITE;

        let (message, trace, approval, run, conversation_id, project_id) = {
            let mut sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;

            let message = RuntimeMessage {
                id: format!("msg-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                sender_type: "user".into(),
                sender_label: "User".into(),
                content: input.content.clone(),
                timestamp: now,
                model_id: Some(input.model_id.clone()),
                status: if requires_approval {
                    "waiting_approval".into()
                } else {
                    "completed".into()
                },
            };
            aggregate.detail.messages.push(message.clone());

            let trace = RuntimeTraceItem {
                id: format!("trace-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                run_id: aggregate.detail.run.id.clone(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                kind: "step".into(),
                title: "Turn submitted".into(),
                detail: if requires_approval {
                    format!(
                        "Permission mode {} requires explicit approval before execution.",
                        normalized_permission_mode
                    )
                } else {
                    format!(
                        "Turn submitted and completed with permission mode {}.",
                        normalized_permission_mode
                    )
                },
                tone: if requires_approval { "warning".into() } else { "success".into() },
                timestamp: now,
                actor: "user".into(),
                related_message_id: Some(message.id.clone()),
                related_tool_name: None,
            };
            aggregate.detail.trace.push(trace.clone());

            let approval = requires_approval.then(|| ApprovalRequestRecord {
                id: format!("approval-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                run_id: aggregate.detail.run.id.clone(),
                tool_name: "runtime.turn".into(),
                summary: "Turn requires approval".into(),
                detail: format!(
                    "Permission mode {} requires explicit approval.",
                    normalized_permission_mode
                ),
                risk_level: "medium".into(),
                created_at: now,
                status: "pending".into(),
            });
            aggregate.detail.pending_approval = approval.clone();

            aggregate.detail.summary.status = if requires_approval {
                "waiting_approval".into()
            } else {
                "completed".into()
            };
            aggregate.detail.summary.updated_at = now;
            aggregate.detail.summary.last_message_preview = Some(input.content.clone());
            aggregate.detail.run.status = if requires_approval {
                "waiting_approval".into()
            } else {
                "completed".into()
            };
            aggregate.detail.run.current_step = if requires_approval {
                "awaiting_approval".into()
            } else {
                "completed".into()
            };
            aggregate.detail.run.updated_at = now;
            aggregate.detail.run.model_id = Some(input.model_id);
            aggregate.detail.run.next_action = Some(if requires_approval {
                "approval".into()
            } else {
                "idle".into()
            });

            let run = aggregate.detail.run.clone();
            let conversation_id = aggregate.detail.summary.conversation_id.clone();
            let project_id = aggregate.detail.summary.project_id.clone();
            self.persist_session(session_id, aggregate)?;
            (message, trace, approval, run, conversation_id, project_id)
        };

        self.state
            .observation
            .append_trace(TraceEventRecord {
                id: trace.id.clone(),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                run_id: Some(run.id.clone()),
                session_id: Some(session_id.into()),
                event_kind: "turn_submitted".into(),
                title: trace.title.clone(),
                detail: trace.detail.clone(),
                created_at: now,
            })
            .await?;
        self.state
            .observation
            .append_audit(AuditRecord {
                id: format!("audit-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                actor_type: "session".into(),
                actor_id: session_id.into(),
                action: "runtime.submit_turn".into(),
                resource: run.id.clone(),
                outcome: run.status.clone(),
                created_at: now,
            })
            .await?;
        self.state
            .observation
            .append_cost(CostLedgerEntry {
                id: format!("cost-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                run_id: Some(run.id.clone()),
                metric: "turns".into(),
                amount: 1,
                unit: "count".into(),
                created_at: now,
            })
            .await?;

        let mut events = vec![
            RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.message.created".into(),
                kind: Some("runtime.message.created".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "message": message.clone(),
                })),
                run: None,
                message: Some(message),
                trace: None,
                approval: None,
                decision: None,
                summary: None,
                error: None,
            },
            RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.trace.emitted".into(),
                kind: Some("runtime.trace.emitted".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "trace": trace.clone(),
                })),
                run: None,
                message: None,
                trace: Some(trace),
                approval: None,
                decision: None,
                summary: None,
                error: None,
            },
        ];

        if let Some(approval) = approval.clone() {
            events.push(RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.approval.requested".into(),
                kind: Some("runtime.approval.requested".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "approval": approval.clone(),
                    "run": run.clone(),
                })),
                run: Some(run.clone()),
                message: None,
                trace: None,
                approval: Some(approval),
                decision: None,
                summary: None,
                error: None,
            });
        }

        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.run.updated".into(),
            kind: Some("runtime.run.updated".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id,
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
        });

        for event in events {
            self.emit_event(session_id, event).await?;
        }

        Ok(run)
    }

    async fn resolve_approval(
        &self,
        session_id: &str,
        approval_id: &str,
        input: ResolveRuntimeApprovalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let (approval, run, conversation_id, project_id) = {
            let mut sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;
            let pending = aggregate
                .detail
                .pending_approval
                .as_mut()
                .ok_or_else(|| AppError::not_found("runtime approval"))?;
            if pending.id != approval_id {
                return Err(AppError::not_found("runtime approval"));
            }
            pending.status = match input.decision.as_str() {
                "approve" => "approved".into(),
                "reject" => "rejected".into(),
                _ => return Err(AppError::invalid_input("approval decision must be approve or reject")),
            };

            aggregate.detail.run.status = if input.decision == "approve" {
                "completed".into()
            } else {
                "blocked".into()
            };
            aggregate.detail.run.current_step = if input.decision == "approve" {
                "completed".into()
            } else {
                "approval_rejected".into()
            };
            aggregate.detail.run.updated_at = now;
            aggregate.detail.run.next_action = None;
            aggregate.detail.summary.status = aggregate.detail.run.status.clone();
            aggregate.detail.summary.updated_at = now;

            let approval = pending.clone();
            aggregate.detail.pending_approval = None;
            let run = aggregate.detail.run.clone();
            let conversation_id = aggregate.detail.summary.conversation_id.clone();
            let project_id = aggregate.detail.summary.project_id.clone();
            self.persist_session(session_id, aggregate)?;
            (approval, run, conversation_id, project_id)
        };

        self.state
            .observation
            .append_trace(TraceEventRecord {
                id: format!("trace-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                run_id: Some(run.id.clone()),
                session_id: Some(session_id.into()),
                event_kind: "approval_resolved".into(),
                title: "Approval resolved".into(),
                detail: input.decision.clone(),
                created_at: now,
            })
            .await?;
        self.state
            .observation
            .append_audit(AuditRecord {
                id: format!("audit-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                actor_type: "session".into(),
                actor_id: session_id.into(),
                action: "runtime.resolve_approval".into(),
                resource: approval.id.clone(),
                outcome: input.decision.clone(),
                created_at: now,
            })
            .await?;

        for event in [
            RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.approval.resolved".into(),
                kind: Some("runtime.approval.resolved".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "approval": approval.clone(),
                    "decision": input.decision.clone(),
                    "run": run.clone(),
                })),
                run: Some(run.clone()),
                message: None,
                trace: None,
                approval: Some(approval.clone()),
                decision: Some(input.decision.clone()),
                summary: None,
                error: None,
            },
            RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.run.updated".into(),
                kind: Some("runtime.run.updated".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id,
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "approval": approval.clone(),
                    "decision": input.decision.clone(),
                    "run": run.clone(),
                })),
                run: Some(run.clone()),
                message: None,
                trace: None,
                approval: Some(approval),
                decision: Some(input.decision),
                summary: None,
                error: None,
            },
        ] {
            self.emit_event(session_id, event).await?;
        }

        Ok(run)
    }

    async fn subscribe_events(
        &self,
        session_id: &str,
    ) -> Result<broadcast::Receiver<RuntimeEventEnvelope>, AppError> {
        Ok(self.session_sender(session_id)?.subscribe())
    }
}
