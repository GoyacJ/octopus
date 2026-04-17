use super::*;
use octopus_core::{
    default_task_view_status, timestamp_now, CreateTaskInterventionRequest, CreateTaskRequest,
    ProjectTaskInterventionRecord, ProjectTaskRecord, ProjectTaskRunRecord,
    ProjectTaskSchedulerClaimRecord, TaskAnalyticsSummary, TaskContextBundle,
    TaskStateTransitionSummary, UpdateTaskRequest,
};
use octopus_platform::ProjectTaskService;
use serde::de::DeserializeOwned;
use serde_json::json;

fn parse_json_with_default<T, F>(raw: Option<String>, default: F) -> T
where
    T: DeserializeOwned,
    F: Fn() -> T,
{
    raw.as_deref()
        .filter(|value| !value.trim().is_empty())
        .and_then(|value| serde_json::from_str::<T>(value).ok())
        .unwrap_or_else(default)
}

pub(super) fn load_project_tasks(
    connection: &Connection,
) -> Result<Vec<ProjectTaskRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, title, goal, brief, default_actor_ref, status,
                    schedule_spec, next_run_at, last_run_at, active_task_run_id,
                    latest_result_summary, latest_failure_category, latest_transition_json,
                    view_status, attention_reasons_json, attention_updated_at,
                    analytics_summary_json, context_bundle_json,
                    latest_deliverable_refs_json, latest_artifact_refs_json,
                    created_by, updated_by, created_at, updated_at
             FROM project_tasks
             ORDER BY updated_at DESC, id DESC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectTaskRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                goal: row.get(4)?,
                brief: row.get(5)?,
                default_actor_ref: row.get(6)?,
                status: row.get(7)?,
                schedule_spec: row.get(8)?,
                next_run_at: row.get::<_, Option<i64>>(9)?.map(|value| value as u64),
                last_run_at: row.get::<_, Option<i64>>(10)?.map(|value| value as u64),
                active_task_run_id: row.get(11)?,
                latest_result_summary: row.get(12)?,
                latest_failure_category: row.get(13)?,
                latest_transition: parse_json_with_default::<Option<TaskStateTransitionSummary>, _>(
                    row.get(14)?,
                    || None,
                ),
                view_status: row.get(15)?,
                attention_reasons: parse_json_with_default(row.get(16)?, Vec::new),
                attention_updated_at: row.get::<_, Option<i64>>(17)?.map(|value| value as u64),
                analytics_summary: parse_json_with_default(
                    row.get(18)?,
                    TaskAnalyticsSummary::default,
                ),
                context_bundle: parse_json_with_default(row.get(19)?, TaskContextBundle::default),
                latest_deliverable_refs: parse_json_with_default(row.get(20)?, Vec::new),
                latest_artifact_refs: parse_json_with_default(row.get(21)?, Vec::new),
                created_by: row.get(22)?,
                updated_by: row.get(23)?,
                created_at: row.get::<_, i64>(24)? as u64,
                updated_at: row.get::<_, i64>(25)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_project_task_runs(
    connection: &Connection,
) -> Result<Vec<ProjectTaskRunRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, task_id, trigger_type, status,
                    session_id, conversation_id, runtime_run_id, actor_ref,
                    started_at, completed_at, result_summary, pending_approval_id,
                    failure_category, failure_summary, view_status,
                    attention_reasons_json, attention_updated_at, deliverable_refs_json,
                    artifact_refs_json, latest_transition_json
             FROM project_task_runs
             ORDER BY started_at DESC, id DESC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectTaskRunRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                task_id: row.get(3)?,
                trigger_type: row.get(4)?,
                status: row.get(5)?,
                session_id: row.get(6)?,
                conversation_id: row.get(7)?,
                runtime_run_id: row.get(8)?,
                actor_ref: row.get(9)?,
                started_at: row.get::<_, i64>(10)? as u64,
                completed_at: row.get::<_, Option<i64>>(11)?.map(|value| value as u64),
                result_summary: row.get(12)?,
                pending_approval_id: row.get(13)?,
                failure_category: row.get(14)?,
                failure_summary: row.get(15)?,
                view_status: row.get(16)?,
                attention_reasons: parse_json_with_default(row.get(17)?, Vec::new),
                attention_updated_at: row.get::<_, Option<i64>>(18)?.map(|value| value as u64),
                deliverable_refs: parse_json_with_default(row.get(19)?, Vec::new),
                artifact_refs: parse_json_with_default(row.get(20)?, Vec::new),
                latest_transition: parse_json_with_default::<Option<TaskStateTransitionSummary>, _>(
                    row.get(21)?,
                    || None,
                ),
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_project_task_interventions(
    connection: &Connection,
) -> Result<Vec<ProjectTaskInterventionRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, task_id, task_run_id, type,
                    payload_json, created_by, created_at, applied_to_session_id, status
             FROM project_task_interventions
             ORDER BY created_at DESC, id DESC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectTaskInterventionRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                task_id: row.get(3)?,
                task_run_id: row.get(4)?,
                r#type: row.get(5)?,
                payload: parse_json_with_default(row.get(6)?, || json!({})),
                created_by: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
                applied_to_session_id: row.get(9)?,
                status: row.get(10)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(super) fn load_project_task_scheduler_claims(
    connection: &Connection,
) -> Result<Vec<ProjectTaskSchedulerClaimRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT workspace_id, project_id, task_id, claim_token, claimed_by,
                    claim_until, last_dispatched_at, last_evaluated_at, updated_at
             FROM project_task_scheduler_claims
             ORDER BY updated_at DESC, task_id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectTaskSchedulerClaimRecord {
                workspace_id: row.get(0)?,
                project_id: row.get(1)?,
                task_id: row.get(2)?,
                claim_token: row.get(3)?,
                claimed_by: row.get(4)?,
                claim_until: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
                last_dispatched_at: row.get::<_, Option<i64>>(6)?.map(|value| value as u64),
                last_evaluated_at: row.get::<_, Option<i64>>(7)?.map(|value| value as u64),
                updated_at: row.get::<_, i64>(8)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

#[allow(dead_code)]
pub(super) fn persist_project_task_record(
    connection: &Connection,
    record: &ProjectTaskRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    let sql = format!(
        "{verb} INTO project_tasks (
            id, workspace_id, project_id, title, goal, brief, default_actor_ref, status,
            schedule_spec, next_run_at, last_run_at, active_task_run_id,
            latest_result_summary, latest_failure_category, latest_transition_json,
            view_status, attention_reasons_json, attention_updated_at,
            analytics_summary_json, context_bundle_json,
            latest_deliverable_refs_json, latest_artifact_refs_json,
            created_by, updated_by, created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
            ?9, ?10, ?11, ?12,
            ?13, ?14, ?15,
            ?16, ?17, ?18,
            ?19, ?20,
            ?21, ?22,
            ?23, ?24, ?25, ?26
        )"
    );
    let latest_transition_json = record
        .latest_transition
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    connection
        .execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.title,
                record.goal,
                record.brief,
                record.default_actor_ref,
                record.status,
                record.schedule_spec,
                record.next_run_at.map(|value| value as i64),
                record.last_run_at.map(|value| value as i64),
                record.active_task_run_id,
                record.latest_result_summary,
                record.latest_failure_category,
                latest_transition_json,
                record.view_status,
                serde_json::to_string(&record.attention_reasons)?,
                record.attention_updated_at.map(|value| value as i64),
                serde_json::to_string(&record.analytics_summary)?,
                serde_json::to_string(&record.context_bundle)?,
                serde_json::to_string(&record.latest_deliverable_refs)?,
                serde_json::to_string(&record.latest_artifact_refs)?,
                record.created_by,
                record.updated_by,
                record.created_at as i64,
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

#[allow(dead_code)]
pub(super) fn persist_project_task_run_record(
    connection: &Connection,
    record: &ProjectTaskRunRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    let sql = format!(
        "{verb} INTO project_task_runs (
            id, workspace_id, project_id, task_id, trigger_type, status,
            session_id, conversation_id, runtime_run_id, actor_ref,
            started_at, completed_at, result_summary, pending_approval_id,
            failure_category, failure_summary, view_status, attention_reasons_json,
            attention_updated_at, deliverable_refs_json, artifact_refs_json,
            latest_transition_json
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19,
            ?20, ?21, ?22
        )"
    );
    let latest_transition_json = record
        .latest_transition
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    connection
        .execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.task_id,
                record.trigger_type,
                record.status,
                record.session_id,
                record.conversation_id,
                record.runtime_run_id,
                record.actor_ref,
                record.started_at as i64,
                record.completed_at.map(|value| value as i64),
                record.result_summary,
                record.pending_approval_id,
                record.failure_category,
                record.failure_summary,
                record.view_status,
                serde_json::to_string(&record.attention_reasons)?,
                record.attention_updated_at.map(|value| value as i64),
                serde_json::to_string(&record.deliverable_refs)?,
                serde_json::to_string(&record.artifact_refs)?,
                latest_transition_json,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

#[allow(dead_code)]
pub(super) fn persist_project_task_intervention_record(
    connection: &Connection,
    record: &ProjectTaskInterventionRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    let sql = format!(
        "{verb} INTO project_task_interventions (
            id, workspace_id, project_id, task_id, task_run_id, type,
            payload_json, created_by, created_at, applied_to_session_id, status
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11
        )"
    );
    connection
        .execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.task_id,
                record.task_run_id,
                record.r#type,
                serde_json::to_string(&record.payload)?,
                record.created_by,
                record.created_at as i64,
                record.applied_to_session_id,
                record.status,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

#[allow(dead_code)]
pub(super) fn persist_project_task_scheduler_claim_record(
    connection: &Connection,
    record: &ProjectTaskSchedulerClaimRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    let sql = format!(
        "{verb} INTO project_task_scheduler_claims (
            workspace_id, project_id, task_id, claim_token, claimed_by,
            claim_until, last_dispatched_at, last_evaluated_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5,
            ?6, ?7, ?8, ?9
        )"
    );
    connection
        .execute(
            &sql,
            params![
                record.workspace_id,
                record.project_id,
                record.task_id,
                record.claim_token,
                record.claimed_by,
                record.claim_until.map(|value| value as i64),
                record.last_dispatched_at.map(|value| value as i64),
                record.last_evaluated_at.map(|value| value as i64),
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn normalize_required_task_text(value: &str, field: &str) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::invalid_input(format!("{field} is required")));
    }
    Ok(normalized.to_string())
}

fn normalize_optional_task_text(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_task_context_bundle(mut bundle: TaskContextBundle) -> TaskContextBundle {
    bundle.refs = bundle
        .refs
        .into_iter()
        .map(|mut reference| {
            reference.kind = reference.kind.trim().to_string();
            reference.ref_id = reference.ref_id.trim().to_string();
            reference.title = reference.title.trim().to_string();
            reference.subtitle = reference.subtitle.trim().to_string();
            reference.version_ref = normalize_optional_task_text(reference.version_ref);
            reference.pin_mode = reference.pin_mode.trim().to_string();
            reference
        })
        .filter(|reference| {
            !reference.kind.is_empty()
                && !reference.ref_id.is_empty()
                && !reference.title.is_empty()
        })
        .collect();
    bundle.pinned_instructions = bundle.pinned_instructions.trim().to_string();
    let resolution_mode = bundle.resolution_mode.trim();
    bundle.resolution_mode = if resolution_mode.is_empty() {
        "explicit_only".into()
    } else {
        resolution_mode.to_string()
    };
    bundle
}

fn sort_task_records(records: &mut [ProjectTaskRecord]) {
    records.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.id.cmp(&left.id))
    });
}

fn sort_task_run_records(records: &mut [ProjectTaskRunRecord]) {
    records.sort_by(|left, right| {
        right
            .started_at
            .cmp(&left.started_at)
            .then_with(|| right.id.cmp(&left.id))
    });
}

fn sort_task_intervention_records(records: &mut [ProjectTaskInterventionRecord]) {
    records.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
}

fn task_intervention_applies_run_state(intervention_type: &str) -> bool {
    matches!(intervention_type, "approve" | "reject" | "resume")
}

fn task_intervention_messages(intervention_type: &str) -> Option<(&'static str, &'static str)> {
    match intervention_type {
        "approve" => Some((
            "Approval received. Task run resumed.",
            "Approval received. Continuing the active run.",
        )),
        "reject" => Some((
            "Approval rejected. Task run is waiting for updated guidance.",
            "Approval rejected. Waiting for updated guidance.",
        )),
        "resume" => Some((
            "Task run resumed after updated guidance.",
            "Updated guidance received. Continuing the active run.",
        )),
        _ => None,
    }
}

fn task_intervention_transition_summary(intervention_type: &str) -> String {
    format!("Task intervention recorded: {intervention_type}.")
}

fn task_intervention_payload_text(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn apply_task_intervention_to_run(
    run: &ProjectTaskRunRecord,
    intervention_type: &str,
    created_at: u64,
) -> Option<ProjectTaskRunRecord> {
    let (transition_summary, result_summary) = task_intervention_messages(intervention_type)?;
    let (status, view_status, attention_reasons, attention_updated_at) = match intervention_type {
        "approve" | "resume" => ("running", "healthy", Vec::new(), None),
        "reject" => (
            "waiting_input",
            "attention",
            vec!["waiting_input".into()],
            Some(created_at),
        ),
        _ => return None,
    };

    Some(ProjectTaskRunRecord {
        status: status.into(),
        completed_at: None,
        result_summary: Some(result_summary.into()),
        pending_approval_id: None,
        failure_category: None,
        failure_summary: None,
        view_status: view_status.into(),
        attention_reasons,
        attention_updated_at,
        latest_transition: Some(TaskStateTransitionSummary {
            kind: "intervened".into(),
            summary: transition_summary.into(),
            at: created_at,
            run_id: run.runtime_run_id.clone().or_else(|| Some(run.id.clone())),
        }),
        ..run.clone()
    })
}

fn apply_accepted_task_intervention_to_run(
    run: &ProjectTaskRunRecord,
    intervention_type: &str,
    payload: &serde_json::Value,
) -> Option<ProjectTaskRunRecord> {
    match intervention_type {
        "change_actor" => Some(ProjectTaskRunRecord {
            actor_ref: task_intervention_payload_text(payload, "actorRef")
                .unwrap_or_else(|| run.actor_ref.clone()),
            ..run.clone()
        }),
        _ => None,
    }
}

fn apply_task_intervention_to_task(
    task: &ProjectTaskRecord,
    run: &ProjectTaskRunRecord,
    intervention_type: &str,
    user_id: &str,
    created_at: u64,
) -> Option<ProjectTaskRecord> {
    let (transition_summary, result_summary) = task_intervention_messages(intervention_type)?;
    let (status, view_status, attention_reasons, attention_updated_at) = match intervention_type {
        "approve" | "resume" => ("running", "healthy", Vec::new(), None),
        "reject" => (
            "attention",
            "attention",
            vec!["waiting_input".into()],
            Some(created_at),
        ),
        _ => return None,
    };

    Some(ProjectTaskRecord {
        status: status.into(),
        active_task_run_id: Some(run.id.clone()),
        latest_result_summary: Some(result_summary.into()),
        latest_failure_category: None,
        latest_transition: Some(TaskStateTransitionSummary {
            kind: "intervened".into(),
            summary: transition_summary.into(),
            at: created_at,
            run_id: run.runtime_run_id.clone().or_else(|| Some(run.id.clone())),
        }),
        view_status: view_status.into(),
        attention_reasons,
        attention_updated_at,
        updated_by: Some(user_id.to_string()),
        updated_at: created_at,
        ..task.clone()
    })
}

fn apply_accepted_task_intervention_to_task(
    task: &ProjectTaskRecord,
    task_run_id: Option<String>,
    intervention_type: &str,
    payload: &serde_json::Value,
    user_id: &str,
    created_at: u64,
) -> Option<ProjectTaskRecord> {
    match intervention_type {
        "comment" | "takeover" | "edit_brief" | "change_actor" => {
            let (view_status, attention_reasons, attention_updated_at) =
                if intervention_type == "takeover" {
                    (
                        "attention".into(),
                        vec!["takeover_recommended".into()],
                        Some(created_at),
                    )
                } else {
                    (
                        task.view_status.clone(),
                        task.attention_reasons.clone(),
                        task.attention_updated_at,
                    )
                };

            Some(ProjectTaskRecord {
                brief: if intervention_type == "edit_brief" {
                    task_intervention_payload_text(payload, "brief")
                        .unwrap_or_else(|| task.brief.clone())
                } else {
                    task.brief.clone()
                },
                default_actor_ref: if intervention_type == "change_actor" {
                    task_intervention_payload_text(payload, "actorRef")
                        .unwrap_or_else(|| task.default_actor_ref.clone())
                } else {
                    task.default_actor_ref.clone()
                },
                latest_transition: Some(TaskStateTransitionSummary {
                    kind: "intervened".into(),
                    summary: task_intervention_transition_summary(intervention_type),
                    at: created_at,
                    run_id: task_run_id,
                }),
                view_status,
                attention_reasons,
                attention_updated_at,
                updated_by: Some(user_id.to_string()),
                updated_at: created_at,
                ..task.clone()
            })
        }
        _ => None,
    }
}

#[async_trait]
impl ProjectTaskService for InfraWorkspaceService {
    async fn list_tasks(&self, project_id: &str) -> Result<Vec<ProjectTaskRecord>, AppError> {
        self.ensure_project_exists(project_id)?;
        let mut records = self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect::<Vec<_>>();
        sort_task_records(&mut records);
        Ok(records)
    }

    async fn create_task(
        &self,
        project_id: &str,
        user_id: &str,
        request: CreateTaskRequest,
    ) -> Result<ProjectTaskRecord, AppError> {
        self.ensure_project_exists(project_id)?;
        let workspace_id = self.state.workspace_id()?;
        let now = timestamp_now();
        let record = ProjectTaskRecord {
            id: format!("task-{}", Uuid::new_v4()),
            workspace_id,
            project_id: project_id.to_string(),
            title: normalize_required_task_text(&request.title, "task title")?,
            goal: normalize_required_task_text(&request.goal, "task goal")?,
            brief: normalize_required_task_text(&request.brief, "task brief")?,
            default_actor_ref: normalize_required_task_text(
                &request.default_actor_ref,
                "default actor",
            )?,
            status: "ready".into(),
            schedule_spec: normalize_optional_task_text(request.schedule_spec),
            next_run_at: None,
            last_run_at: None,
            active_task_run_id: None,
            latest_result_summary: None,
            latest_failure_category: None,
            latest_transition: None,
            view_status: default_task_view_status(),
            attention_reasons: Vec::new(),
            attention_updated_at: None,
            analytics_summary: TaskAnalyticsSummary::default(),
            context_bundle: normalize_task_context_bundle(request.context_bundle),
            latest_deliverable_refs: Vec::new(),
            latest_artifact_refs: Vec::new(),
            created_by: user_id.to_string(),
            updated_by: None,
            created_at: now,
            updated_at: now,
        };

        persist_project_task_record(&self.state.open_db()?, &record, false)?;
        let mut tasks = self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?;
        tasks.push(record.clone());
        Ok(record)
    }

    async fn get_task(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<ProjectTaskRecord, AppError> {
        self.state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?
            .iter()
            .find(|record| record.project_id == project_id && record.id == task_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("task not found"))
    }

    async fn update_task(
        &self,
        project_id: &str,
        task_id: &str,
        user_id: &str,
        request: UpdateTaskRequest,
    ) -> Result<ProjectTaskRecord, AppError> {
        let existing = self.get_task(project_id, task_id).await?;
        let updated = ProjectTaskRecord {
            title: match request.title {
                Some(value) => normalize_required_task_text(&value, "task title")?,
                None => existing.title.clone(),
            },
            goal: match request.goal {
                Some(value) => normalize_required_task_text(&value, "task goal")?,
                None => existing.goal.clone(),
            },
            brief: match request.brief {
                Some(value) => normalize_required_task_text(&value, "task brief")?,
                None => existing.brief.clone(),
            },
            default_actor_ref: match request.default_actor_ref {
                Some(value) => normalize_required_task_text(&value, "default actor")?,
                None => existing.default_actor_ref.clone(),
            },
            schedule_spec: request
                .schedule_spec
                .map(Some)
                .map(normalize_optional_task_text)
                .unwrap_or(existing.schedule_spec.clone()),
            context_bundle: request
                .context_bundle
                .map(normalize_task_context_bundle)
                .unwrap_or_else(|| existing.context_bundle.clone()),
            updated_by: Some(user_id.to_string()),
            updated_at: timestamp_now(),
            ..existing
        };

        persist_project_task_record(&self.state.open_db()?, &updated, true)?;
        let mut tasks = self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?;
        Self::replace_or_push(&mut tasks, updated.clone(), |record| record.id == task_id);
        Ok(updated)
    }

    async fn save_task(&self, record: ProjectTaskRecord) -> Result<ProjectTaskRecord, AppError> {
        self.ensure_project_exists(&record.project_id)?;
        persist_project_task_record(&self.state.open_db()?, &record, true)?;
        let mut tasks = self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?;
        Self::replace_or_push(&mut tasks, record.clone(), |item| item.id == record.id);
        Ok(record)
    }

    async fn list_task_runs(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<Vec<ProjectTaskRunRecord>, AppError> {
        let mut records = self
            .state
            .project_task_runs
            .lock()
            .map_err(|_| AppError::runtime("project task runs mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id && record.task_id == task_id)
            .cloned()
            .collect::<Vec<_>>();
        sort_task_run_records(&mut records);
        Ok(records)
    }

    async fn save_task_run(
        &self,
        record: ProjectTaskRunRecord,
    ) -> Result<ProjectTaskRunRecord, AppError> {
        self.ensure_project_exists(&record.project_id)?;
        persist_project_task_run_record(&self.state.open_db()?, &record, true)?;
        let mut runs = self
            .state
            .project_task_runs
            .lock()
            .map_err(|_| AppError::runtime("project task runs mutex poisoned"))?;
        Self::replace_or_push(&mut runs, record.clone(), |item| item.id == record.id);
        Ok(record)
    }

    async fn list_task_interventions(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<Vec<ProjectTaskInterventionRecord>, AppError> {
        let mut records = self
            .state
            .project_task_interventions
            .lock()
            .map_err(|_| AppError::runtime("project task interventions mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id && record.task_id == task_id)
            .cloned()
            .collect::<Vec<_>>();
        sort_task_intervention_records(&mut records);
        Ok(records)
    }

    async fn create_task_intervention(
        &self,
        project_id: &str,
        task_id: &str,
        user_id: &str,
        request: CreateTaskInterventionRequest,
    ) -> Result<ProjectTaskInterventionRecord, AppError> {
        let task = self.get_task(project_id, task_id).await?;
        let task_run_id = normalize_optional_task_text(request.task_run_id);
        let intervention_type = normalize_required_task_text(&request.r#type, "intervention type")?;
        let created_at = timestamp_now();
        let payload = request.payload;
        let target_run_id = task_run_id
            .clone()
            .or_else(|| task.active_task_run_id.clone());
        let (updated_run, updated_task, applied_to_session_id, intervention_status) =
            if task_intervention_applies_run_state(&intervention_type) {
                let target_run_id = target_run_id.clone().ok_or_else(|| {
                    AppError::invalid_input(
                        "task intervention requires a target task run or active task run",
                    )
                })?;
                let target_run = self
                    .list_task_runs(project_id, task_id)
                    .await?
                    .into_iter()
                    .find(|record| record.id == target_run_id)
                    .ok_or_else(|| AppError::not_found("task run not found"))?;
                let next_run =
                    apply_task_intervention_to_run(&target_run, &intervention_type, created_at)
                        .ok_or_else(|| {
                            AppError::invalid_input("unsupported task intervention type")
                        })?;
                let next_task = apply_task_intervention_to_task(
                    &task,
                    &next_run,
                    &intervention_type,
                    user_id,
                    created_at,
                );
                (
                    Some(next_run.clone()),
                    next_task,
                    next_run.session_id.clone(),
                    "applied".to_string(),
                )
            } else {
                let task_runs = self.list_task_runs(project_id, task_id).await?;
                let accepted_run = target_run_id.as_ref().and_then(|run_id| {
                    task_runs
                        .iter()
                        .find(|record| &record.id == run_id)
                        .and_then(|record| {
                            apply_accepted_task_intervention_to_run(
                                record,
                                &intervention_type,
                                &payload,
                            )
                        })
                });
                let accepted_task = apply_accepted_task_intervention_to_task(
                    &task,
                    target_run_id.clone(),
                    &intervention_type,
                    &payload,
                    user_id,
                    created_at,
                );
                (accepted_run, accepted_task, None, "accepted".to_string())
            };
        let record = ProjectTaskInterventionRecord {
            id: format!("task-intervention-{}", Uuid::new_v4()),
            workspace_id: task.workspace_id,
            project_id: project_id.to_string(),
            task_id: task_id.to_string(),
            task_run_id,
            r#type: intervention_type,
            payload,
            created_by: user_id.to_string(),
            created_at,
            applied_to_session_id,
            status: intervention_status,
        };
        let mut connection = self.state.open_db()?;
        let transaction = connection
            .transaction()
            .map_err(|error| AppError::database(error.to_string()))?;
        persist_project_task_intervention_record(&transaction, &record, false)?;
        if let Some(run) = updated_run.as_ref() {
            persist_project_task_run_record(&transaction, run, true)?;
        }
        if let Some(task) = updated_task.as_ref() {
            persist_project_task_record(&transaction, task, true)?;
        }
        transaction
            .commit()
            .map_err(|error| AppError::database(error.to_string()))?;
        let mut interventions = self
            .state
            .project_task_interventions
            .lock()
            .map_err(|_| AppError::runtime("project task interventions mutex poisoned"))?;
        interventions.push(record.clone());
        drop(interventions);
        if let Some(run) = updated_run {
            let run_id = run.id.clone();
            let mut runs = self
                .state
                .project_task_runs
                .lock()
                .map_err(|_| AppError::runtime("project task runs mutex poisoned"))?;
            Self::replace_or_push(&mut runs, run, |item| item.id == run_id);
        }
        if let Some(task) = updated_task {
            let mut tasks = self
                .state
                .project_tasks
                .lock()
                .map_err(|_| AppError::runtime("project tasks mutex poisoned"))?;
            Self::replace_or_push(&mut tasks, task, |item| item.id == task_id);
        }
        Ok(record)
    }
}
