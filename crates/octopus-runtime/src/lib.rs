//! Runtime orchestration for the Phase 3 MVP slice.

use std::sync::Arc;

use anyhow::{anyhow, ensure, Result};
use octopus_application::{
    AuditEventRecord, CreateRunInput, EventEnvelope, InboxItemRecord, InteractionResponsePayload,
    Phase3Store, ResumeResult, ResumeRunInput, RunContext, RunCreationBundle, RunRecord,
    RunResumeBundle, TimelineEventRecord,
};
use octopus_domain::{InboxItemStatus, InteractionKind, InteractionResponseType, RunStatus};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

pub struct Phase3Service<S> {
    store: Arc<S>,
}

impl<S> Clone for Phase3Service<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
        }
    }
}

impl<S> Phase3Service<S>
where
    S: Phase3Store + 'static,
{
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    pub async fn create_run(&self, input: CreateRunInput) -> Result<RunRecord> {
        let now = timestamp()?;
        let run_id = generate_id("run");
        let inbox_id = generate_id("inbox");
        let resume_token = generate_id("resume");
        let waiting_status = waiting_status(input.interaction_type);
        let prompt = build_prompt(input.interaction_type, &input.input);

        let run = RunRecord {
            id: run_id.clone(),
            workspace_id: input.workspace_id.clone(),
            agent_id: input.agent_id.clone(),
            interaction_type: input.interaction_type,
            status: waiting_status,
            summary: format!(
                "Waiting for {} before completion",
                input.interaction_type.as_str()
            ),
            input: input.input.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let inbox_item = InboxItemRecord {
            id: inbox_id,
            run_id: run_id.clone(),
            kind: input.interaction_type,
            status: InboxItemStatus::Pending,
            title: prompt.0,
            prompt: prompt.1,
            response_type: response_type(input.interaction_type),
            options: prompt.2,
            resume_token: resume_token.clone(),
            created_at: now.clone(),
            resolved_at: None,
        };

        let event_envelopes = vec![
            build_event(
                &run_id,
                "run",
                "run.created",
                "system",
                None,
                None,
                &format!("Run {run_id} created"),
                &now,
            ),
            build_event(
                &run_id,
                "run",
                "run.started",
                "system",
                None,
                None,
                &format!("Run {run_id} entered execution"),
                &now,
            ),
            build_event(
                &run_id,
                "interaction",
                if input.interaction_type == InteractionKind::Approval {
                    "approval.requested"
                } else {
                    "interaction.requested"
                },
                "system",
                Some(resume_token.clone()),
                None,
                &inbox_item.prompt,
                &now,
            ),
            build_event(
                &run_id,
                "run",
                if waiting_status == RunStatus::WaitingApproval {
                    "run.waiting_approval"
                } else {
                    "run.waiting_input"
                },
                "system",
                Some(resume_token.clone()),
                None,
                &run.summary,
                &now,
            ),
        ];

        let timeline_events = vec![
            build_timeline(&run_id, "run.created", "Run accepted into the queue", &now),
            build_timeline(&run_id, "run.running", "Run entered execution", &now),
            build_timeline(
                &run_id,
                if waiting_status == RunStatus::WaitingApproval {
                    "run.waiting_approval"
                } else {
                    "run.waiting_input"
                },
                &run.summary,
                &now,
            ),
        ];

        let audit_events = vec![
            build_audit(
                "system",
                "run",
                &run_id,
                "run.created",
                "Created a new governed run",
                &now,
            ),
            build_audit(
                "system",
                "inbox_item",
                &inbox_item.id,
                if inbox_item.kind == InteractionKind::Approval {
                    "approval.requested"
                } else {
                    "interaction.requested"
                },
                &inbox_item.title,
                &now,
            ),
        ];

        let context = self
            .store
            .create_run(RunCreationBundle {
                run,
                inbox_item,
                event_envelopes,
                timeline_events,
                audit_events,
            })
            .await?;

        Ok(context.run)
    }

    pub async fn list_runs(&self) -> Result<Vec<RunRecord>> {
        self.store.list_runs().await
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Option<RunRecord>> {
        self.store.get_run(run_id).await
    }

    pub async fn list_run_timeline(&self, run_id: &str) -> Result<Vec<TimelineEventRecord>> {
        self.store.list_run_timeline(run_id).await
    }

    pub async fn list_inbox_items(&self) -> Result<Vec<InboxItemRecord>> {
        self.store.list_inbox_items().await
    }

    pub async fn list_audit_events(&self) -> Result<Vec<AuditEventRecord>> {
        self.store.list_audit_events().await
    }

    pub async fn resume_run(&self, run_id: &str, input: ResumeRunInput) -> Result<ResumeResult> {
        if self
            .store
            .find_resume_receipt(run_id, &input.idempotency_key)
            .await?
            .is_some()
        {
            let now = timestamp()?;
            self.store
                .append_audit_events(&[build_audit(
                    "system",
                    "run",
                    run_id,
                    "run.resume.deduplicated",
                    "Ignored a duplicate resume request with the same idempotency key",
                    &now,
                )])
                .await?;
            let run = self
                .store
                .get_run(run_id)
                .await?
                .ok_or_else(|| anyhow!("run {run_id} not found for deduplicated resume"))?;

            return Ok(ResumeResult {
                accepted: true,
                deduplicated: true,
                run_id: run.id.clone(),
                status: run.status,
                run,
            });
        }

        let context = self
            .store
            .get_run_context(run_id)
            .await?
            .ok_or_else(|| anyhow!("run {run_id} not found"))?;
        let pending = context
            .pending_inbox_item
            .clone()
            .ok_or_else(|| anyhow!("run {run_id} has no pending inbox item"))?;

        validate_resume(&context, &pending, &input)?;

        let now = timestamp()?;
        let final_status = final_status(&pending, &input.response);
        let summary = final_summary(&pending, &input.response, final_status);
        let run = RunRecord {
            status: final_status,
            summary: summary.clone(),
            updated_at: now.clone(),
            ..context.run.clone()
        };
        let inbox_item = InboxItemRecord {
            status: InboxItemStatus::Resolved,
            resolved_at: Some(now.clone()),
            ..pending.clone()
        };

        let event_envelopes = vec![
            build_event(
                run_id,
                "run",
                "run.resuming",
                "user",
                Some(input.resume_token.clone()),
                Some(input.idempotency_key.clone()),
                "Run entered resume validation",
                &now,
            ),
            build_event(
                run_id,
                "run",
                "run.freshness_checked",
                "system",
                Some(input.resume_token.clone()),
                Some(input.idempotency_key.clone()),
                freshness_summary(input.response.goal_changed),
                &now,
            ),
            build_event(
                run_id,
                if pending.kind == InteractionKind::Approval {
                    "approval"
                } else {
                    "interaction"
                },
                response_event_type(&pending, &input.response),
                "user",
                Some(input.resume_token.clone()),
                Some(input.idempotency_key.clone()),
                &summary,
                &now,
            ),
            build_event(
                run_id,
                "run",
                if final_status == RunStatus::Completed {
                    "run.completed"
                } else {
                    "run.failed"
                },
                "system",
                Some(input.resume_token.clone()),
                Some(input.idempotency_key.clone()),
                &summary,
                &now,
            ),
        ];

        let timeline_events = vec![
            build_timeline(run_id, "run.resuming", "Run entered resume flow", &now),
            build_timeline(
                run_id,
                "run.freshness_checked",
                freshness_summary(input.response.goal_changed),
                &now,
            ),
            build_timeline(
                run_id,
                response_event_type(&pending, &input.response),
                &summary,
                &now,
            ),
            build_timeline(
                run_id,
                if final_status == RunStatus::Completed {
                    "run.completed"
                } else {
                    "run.failed"
                },
                &summary,
                &now,
            ),
        ];

        let audit_events = vec![
            build_audit(
                "user",
                "run",
                run_id,
                "run.resume.accepted",
                "Accepted a governed resume request",
                &now,
            ),
            build_audit(
                "system",
                "run",
                run_id,
                "run.freshness.checked",
                freshness_summary(input.response.goal_changed),
                &now,
            ),
            build_audit(
                "user",
                if pending.kind == InteractionKind::Approval {
                    "approval"
                } else {
                    "interaction"
                },
                &pending.id,
                response_event_type(&pending, &input.response),
                &summary,
                &now,
            ),
        ];

        let context = self
            .store
            .apply_resume(RunResumeBundle {
                run,
                inbox_item,
                event_envelopes,
                timeline_events,
                audit_events,
                receipt: octopus_application::ResumeReceipt {
                    run_id: run_id.to_owned(),
                    idempotency_key: input.idempotency_key,
                    final_status,
                    recorded_at: now,
                },
            })
            .await?;

        Ok(ResumeResult {
            accepted: true,
            deduplicated: false,
            run_id: context.run.id.clone(),
            status: context.run.status,
            run: context.run,
        })
    }
}

fn validate_resume(
    context: &RunContext,
    pending: &InboxItemRecord,
    input: &ResumeRunInput,
) -> Result<()> {
    ensure!(
        context.run.status == RunStatus::WaitingInput
            || context.run.status == RunStatus::WaitingApproval,
        "run {} is not waiting for input or approval",
        context.run.id
    );
    if let Some(inbox_item_id) = &input.inbox_item_id {
        ensure!(
            inbox_item_id == &pending.id,
            "resume request targets inbox item {inbox_item_id}, expected {}",
            pending.id
        );
    }
    ensure!(
        input.resume_token == pending.resume_token,
        "resume token does not match the pending inbox item"
    );
    ensure!(
        !input.idempotency_key.trim().is_empty(),
        "idempotency key must not be empty"
    );
    match pending.kind {
        InteractionKind::AskUser => {
            ensure!(
                matches!(
                    input.response.response_type,
                    InteractionResponseType::Text
                        | InteractionResponseType::SingleSelect
                        | InteractionResponseType::MultiSelect
                ),
                "ask-user inbox items only accept text or selection responses"
            );
            ensure!(
                input
                    .response
                    .text
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                    || !input.response.values.is_empty(),
                "ask-user resume requests must provide text or selected values"
            );
        }
        InteractionKind::Approval => {
            ensure!(
                input.response.response_type == InteractionResponseType::Approval,
                "approval inbox items only accept approval responses"
            );
            ensure!(
                input.response.approved.is_some(),
                "approval responses must specify approved=true|false"
            );
        }
    }
    Ok(())
}

fn waiting_status(kind: InteractionKind) -> RunStatus {
    match kind {
        InteractionKind::AskUser => RunStatus::WaitingInput,
        InteractionKind::Approval => RunStatus::WaitingApproval,
    }
}

fn response_type(kind: InteractionKind) -> InteractionResponseType {
    match kind {
        InteractionKind::AskUser => InteractionResponseType::Text,
        InteractionKind::Approval => InteractionResponseType::Approval,
    }
}

fn build_prompt(kind: InteractionKind, input: &str) -> (String, String, Vec<String>) {
    match kind {
        InteractionKind::AskUser => (
            "User input required".to_owned(),
            format!("Provide the missing context needed to continue: {input}"),
            Vec::new(),
        ),
        InteractionKind::Approval => (
            "Approval required".to_owned(),
            format!("Approve or reject the requested action: {input}"),
            Vec::new(),
        ),
    }
}

fn final_status(pending: &InboxItemRecord, response: &InteractionResponsePayload) -> RunStatus {
    if pending.kind == InteractionKind::Approval && response.approved == Some(false) {
        RunStatus::Failed
    } else {
        RunStatus::Completed
    }
}

fn final_summary(
    pending: &InboxItemRecord,
    response: &InteractionResponsePayload,
    status: RunStatus,
) -> String {
    match (pending.kind, status, response.goal_changed) {
        (InteractionKind::Approval, RunStatus::Failed, true) => {
            "Run stopped after approval rejection and freshness revalidation".to_owned()
        }
        (InteractionKind::Approval, RunStatus::Failed, false) => {
            "Run stopped after approval rejection".to_owned()
        }
        (InteractionKind::Approval, _, true) => {
            "Run resumed after approval with refreshed context".to_owned()
        }
        (InteractionKind::Approval, _, false) => "Run resumed after approval".to_owned(),
        (InteractionKind::AskUser, _, true) => {
            "Run resumed after the user changed the goal".to_owned()
        }
        (InteractionKind::AskUser, _, false) => "Run resumed after user input".to_owned(),
    }
}

fn freshness_summary(goal_changed: bool) -> &'static str {
    if goal_changed {
        "Freshness check detected goal drift and revalidated the resume plan"
    } else {
        "Freshness check confirmed the waiting context is still valid"
    }
}

fn response_event_type(
    pending: &InboxItemRecord,
    response: &InteractionResponsePayload,
) -> &'static str {
    match pending.kind {
        InteractionKind::AskUser => "interaction.responded",
        InteractionKind::Approval => {
            if response.approved == Some(true) {
                "approval.approved"
            } else {
                "approval.rejected"
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_event(
    run_id: &str,
    object_type: &str,
    event_type: &str,
    actor_id: &str,
    resume_token: Option<String>,
    idempotency_key: Option<String>,
    summary: &str,
    occurred_at: &str,
) -> EventEnvelope {
    EventEnvelope {
        id: generate_id("evt"),
        run_id: run_id.to_owned(),
        object_type: object_type.to_owned(),
        event_type: event_type.to_owned(),
        actor_id: actor_id.to_owned(),
        surface: "web".to_owned(),
        resume_token,
        idempotency_key,
        risk_level: "medium".to_owned(),
        budget_context: "phase3-mvp".to_owned(),
        summary: summary.to_owned(),
        occurred_at: occurred_at.to_owned(),
    }
}

fn build_timeline(
    run_id: &str,
    event_type: &str,
    summary: &str,
    occurred_at: &str,
) -> TimelineEventRecord {
    TimelineEventRecord {
        id: generate_id("timeline"),
        run_id: run_id.to_owned(),
        event_type: event_type.to_owned(),
        summary: summary.to_owned(),
        occurred_at: occurred_at.to_owned(),
    }
}

fn build_audit(
    actor_id: &str,
    subject_type: &str,
    subject_id: &str,
    action: &str,
    summary: &str,
    occurred_at: &str,
) -> AuditEventRecord {
    AuditEventRecord {
        id: generate_id("audit"),
        actor_id: actor_id.to_owned(),
        subject_type: subject_type.to_owned(),
        subject_id: subject_id.to_owned(),
        action: action.to_owned(),
        summary: summary.to_owned(),
        occurred_at: occurred_at.to_owned(),
    }
}

fn generate_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4())
}

fn timestamp() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

#[cfg(test)]
mod tests {
    use octopus_application::InteractionResponsePayload;
    use octopus_domain::{InboxItemStatus, InteractionKind, InteractionResponseType};

    use super::{final_status, response_event_type, InboxItemRecord, RunStatus};

    #[test]
    fn rejects_approval_runs_when_approval_is_denied() {
        let pending = InboxItemRecord {
            id: "inbox-1".to_owned(),
            run_id: "run-1".to_owned(),
            kind: InteractionKind::Approval,
            status: InboxItemStatus::Pending,
            title: "Approval required".to_owned(),
            prompt: "Approve the deploy".to_owned(),
            response_type: InteractionResponseType::Approval,
            options: Vec::new(),
            resume_token: "resume-1".to_owned(),
            created_at: "2026-03-24T00:00:00Z".to_owned(),
            resolved_at: None,
        };

        let response = InteractionResponsePayload {
            response_type: InteractionResponseType::Approval,
            values: Vec::new(),
            text: None,
            approved: Some(false),
            goal_changed: false,
        };

        assert_eq!(final_status(&pending, &response), RunStatus::Failed);
        assert_eq!(
            response_event_type(&pending, &response),
            "approval.rejected"
        );
    }
}
