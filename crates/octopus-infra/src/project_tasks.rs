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

#[path = "project_tasks/service.rs"]
mod service;
