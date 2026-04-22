use super::*;

pub(crate) fn ensure_runtime_config_snapshot_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    let columns = table_columns(connection, "runtime_config_snapshots")?;

    if columns
        .iter()
        .any(|column| column == "effective_config_json")
    {
        return Ok(());
    }

    connection
        .execute(
            "ALTER TABLE runtime_config_snapshots ADD COLUMN effective_config_json TEXT",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(crate) fn ensure_runtime_session_projection_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "runtime_session_projections",
        &[
            ("session_kind", "TEXT NOT NULL DEFAULT 'project'"),
            ("last_message_preview", "TEXT"),
            ("config_snapshot_id", "TEXT NOT NULL DEFAULT ''"),
            ("effective_config_hash", "TEXT NOT NULL DEFAULT ''"),
            ("started_from_scope_set", "TEXT NOT NULL DEFAULT '[]'"),
            ("selected_actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("manifest_revision", "TEXT NOT NULL DEFAULT ''"),
            ("active_run_id", "TEXT NOT NULL DEFAULT ''"),
            ("subrun_count", "INTEGER NOT NULL DEFAULT 0"),
            ("workflow_run_id", "TEXT"),
            ("workflow_status", "TEXT"),
            ("workflow_total_steps", "INTEGER NOT NULL DEFAULT 0"),
            ("workflow_completed_steps", "INTEGER NOT NULL DEFAULT 0"),
            ("workflow_current_step_id", "TEXT"),
            ("workflow_current_step_label", "TEXT"),
            ("workflow_background_capable", "INTEGER NOT NULL DEFAULT 0"),
            ("pending_mailbox_ref", "TEXT"),
            ("pending_mailbox_count", "INTEGER NOT NULL DEFAULT 0"),
            ("handoff_count", "INTEGER NOT NULL DEFAULT 0"),
            ("background_run_id", "TEXT"),
            ("background_workflow_run_id", "TEXT"),
            ("background_status", "TEXT"),
            ("manifest_snapshot_ref", "TEXT NOT NULL DEFAULT ''"),
            ("session_policy_snapshot_ref", "TEXT NOT NULL DEFAULT ''"),
            ("capability_plan_summary_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("provider_state_summary_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("pending_mediation_json", "TEXT"),
            ("pending_mediation_kind", "TEXT"),
            ("pending_target_kind", "TEXT"),
            ("pending_target_ref", "TEXT"),
            ("pending_approval_layer", "TEXT"),
            ("pending_provider_key", "TEXT"),
            ("pending_checkpoint_ref", "TEXT"),
            ("last_execution_outcome_json", "TEXT"),
            ("last_mediation_outcome_json", "TEXT"),
            ("last_mediation_outcome", "TEXT"),
            ("last_mediation_target_kind", "TEXT"),
            ("last_mediation_target_ref", "TEXT"),
            ("last_mediation_at", "INTEGER"),
            ("auth_challenge_state", "TEXT"),
            ("approval_lineage_json", "TEXT"),
            ("denied_exposure_count", "INTEGER NOT NULL DEFAULT 0"),
            ("granted_tool_count", "INTEGER NOT NULL DEFAULT 0"),
            ("injected_skill_message_count", "INTEGER NOT NULL DEFAULT 0"),
            ("deferred_capability_count", "INTEGER NOT NULL DEFAULT 0"),
            ("hidden_capability_count", "INTEGER NOT NULL DEFAULT 0"),
            ("degraded_provider_count", "INTEGER NOT NULL DEFAULT 0"),
            (
                "detail_json",
                r#"TEXT NOT NULL DEFAULT '{"summary":{"id":"","conversationId":"","projectId":"","title":"","sessionKind":"project","status":"draft","updatedAt":0,"lastMessagePreview":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[],"selectedActorRef":"","manifestRevision":"","sessionPolicy":{"selectedActorRef":"","selectedConfiguredModelId":"","executionPermissionMode":"","configSnapshotId":"","manifestRevision":"","capabilityPolicy":{},"memoryPolicy":{},"delegationPolicy":{},"approvalPreference":{}},"activeRunId":"","subrunCount":0,"memorySummary":{"summary":"","durableMemoryCount":0,"selectedMemoryIds":[]},"capabilitySummary":{"visibleTools":[],"discoverableSkills":[]}},"selectedActorRef":"","manifestRevision":"","sessionPolicy":{"selectedActorRef":"","selectedConfiguredModelId":"","executionPermissionMode":"","configSnapshotId":"","manifestRevision":"","capabilityPolicy":{},"memoryPolicy":{},"delegationPolicy":{},"approvalPreference":{}},"activeRunId":"","subrunCount":0,"memorySummary":{"summary":"","durableMemoryCount":0,"selectedMemoryIds":[]},"capabilitySummary":{"visibleTools":[],"discoverableSkills":[]},"run":{"id":"","sessionId":"","conversationId":"","status":"draft","currentStep":"ready","startedAt":0,"updatedAt":0,"configuredModelId":null,"configuredModelName":null,"modelId":null,"consumedTokens":null,"nextAction":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[],"runKind":"primary","parentRunId":null,"actorRef":"","delegatedByToolCallId":null,"approvalState":"not-required","usageSummary":{"inputTokens":0,"outputTokens":0,"totalTokens":0},"artifactRefs":[],"traceContext":{"sessionId":"","traceId":"","turnId":"","parentRunId":null},"checkpoint":{"currentIterationIndex":0,"usageSummary":{"inputTokens":0,"outputTokens":0,"totalTokens":0},"pendingApproval":null},"resolvedTarget":null,"requestedActorKind":null,"requestedActorId":null,"resolvedActorKind":null,"resolvedActorId":null,"resolvedActorLabel":null},"messages":[],"trace":[],"pendingApproval":null}'"#,
            ),
        ],
    )
}

pub(crate) fn ensure_runtime_run_projection_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "runtime_run_projections",
        &[
            ("run_kind", "TEXT NOT NULL DEFAULT 'primary'"),
            ("parent_run_id", "TEXT"),
            ("actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("delegated_by_tool_call_id", "TEXT"),
            ("workflow_run_id", "TEXT"),
            ("workflow_step_id", "TEXT"),
            ("workflow_status", "TEXT"),
            ("mailbox_ref", "TEXT"),
            ("handoff_ref", "TEXT"),
            ("background_state", "TEXT"),
            ("worker_total_subruns", "INTEGER NOT NULL DEFAULT 0"),
            ("worker_active_subruns", "INTEGER NOT NULL DEFAULT 0"),
            ("worker_completed_subruns", "INTEGER NOT NULL DEFAULT 0"),
            ("worker_failed_subruns", "INTEGER NOT NULL DEFAULT 0"),
            ("worker_dispatch_json", "TEXT"),
            ("workflow_run_detail_json", "TEXT"),
            ("approval_state", "TEXT NOT NULL DEFAULT 'not-required'"),
            ("trace_id", "TEXT NOT NULL DEFAULT ''"),
            ("turn_id", "TEXT NOT NULL DEFAULT ''"),
            ("capability_plan_summary_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("provider_state_summary_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("pending_mediation_json", "TEXT"),
            ("pending_mediation_kind", "TEXT"),
            ("pending_target_kind", "TEXT"),
            ("pending_target_ref", "TEXT"),
            ("pending_approval_layer", "TEXT"),
            ("pending_provider_key", "TEXT"),
            ("pending_checkpoint_ref", "TEXT"),
            ("last_execution_outcome_json", "TEXT"),
            ("last_mediation_outcome_json", "TEXT"),
            ("last_mediation_outcome", "TEXT"),
            ("last_mediation_target_kind", "TEXT"),
            ("last_mediation_target_ref", "TEXT"),
            ("last_mediation_at", "INTEGER"),
            ("auth_challenge_state", "TEXT"),
            ("approval_lineage_json", "TEXT"),
            ("denied_exposure_count", "INTEGER NOT NULL DEFAULT 0"),
            ("granted_tool_count", "INTEGER NOT NULL DEFAULT 0"),
            ("injected_skill_message_count", "INTEGER NOT NULL DEFAULT 0"),
            ("deferred_capability_count", "INTEGER NOT NULL DEFAULT 0"),
            ("hidden_capability_count", "INTEGER NOT NULL DEFAULT 0"),
            ("degraded_provider_count", "INTEGER NOT NULL DEFAULT 0"),
            (
                "run_json",
                r#"TEXT NOT NULL DEFAULT '{"id":"","sessionId":"","conversationId":"","status":"draft","currentStep":"ready","startedAt":0,"updatedAt":0,"configuredModelId":null,"configuredModelName":null,"modelId":null,"consumedTokens":null,"nextAction":null,"configSnapshotId":"","effectiveConfigHash":"","startedFromScopeSet":[],"runKind":"primary","parentRunId":null,"actorRef":"","delegatedByToolCallId":null,"approvalState":"not-required","usageSummary":{"inputTokens":0,"outputTokens":0,"totalTokens":0},"artifactRefs":[],"traceContext":{"sessionId":"","traceId":"","turnId":"","parentRunId":null},"checkpoint":{"currentIterationIndex":0,"usageSummary":{"inputTokens":0,"outputTokens":0,"totalTokens":0},"pendingApproval":null},"resolvedTarget":null,"requestedActorKind":null,"requestedActorId":null,"resolvedActorKind":null,"resolvedActorId":null,"resolvedActorLabel":null}'"#,
            ),
        ],
    )
}

pub(crate) fn ensure_runtime_phase_four_projection_tables(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_subrun_projections (
                run_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                parent_run_id TEXT,
                actor_ref TEXT NOT NULL DEFAULT '',
                label TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                run_kind TEXT NOT NULL DEFAULT 'subrun',
                delegated_by_tool_call_id TEXT,
                workflow_run_id TEXT,
                mailbox_ref TEXT,
                handoff_ref TEXT,
                started_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_subrun_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("parent_run_id", "TEXT"),
            ("actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("label", "TEXT NOT NULL DEFAULT ''"),
            ("status", "TEXT NOT NULL DEFAULT 'draft'"),
            ("run_kind", "TEXT NOT NULL DEFAULT 'subrun'"),
            ("delegated_by_tool_call_id", "TEXT"),
            ("workflow_run_id", "TEXT"),
            ("mailbox_ref", "TEXT"),
            ("handoff_ref", "TEXT"),
            ("started_at", "INTEGER NOT NULL DEFAULT 0"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_mailbox_projections (
                mailbox_ref TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                channel TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                pending_count INTEGER NOT NULL DEFAULT 0,
                total_messages INTEGER NOT NULL DEFAULT 0,
                latest_handoff_ref TEXT,
                body_storage_path TEXT,
                body_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_mailbox_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("channel", "TEXT NOT NULL DEFAULT ''"),
            ("status", "TEXT NOT NULL DEFAULT 'pending'"),
            ("pending_count", "INTEGER NOT NULL DEFAULT 0"),
            ("total_messages", "INTEGER NOT NULL DEFAULT 0"),
            ("latest_handoff_ref", "TEXT"),
            ("body_storage_path", "TEXT"),
            ("body_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_handoff_projections (
                handoff_ref TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                parent_run_id TEXT,
                delegated_by_tool_call_id TEXT,
                sender_actor_ref TEXT NOT NULL DEFAULT '',
                receiver_actor_ref TEXT NOT NULL DEFAULT '',
                mailbox_ref TEXT NOT NULL DEFAULT '',
                state TEXT NOT NULL,
                artifact_refs_json TEXT NOT NULL DEFAULT '[]',
                envelope_storage_path TEXT,
                envelope_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_handoff_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("parent_run_id", "TEXT"),
            ("delegated_by_tool_call_id", "TEXT"),
            ("sender_actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("receiver_actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("mailbox_ref", "TEXT NOT NULL DEFAULT ''"),
            ("state", "TEXT NOT NULL DEFAULT 'pending'"),
            ("artifact_refs_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("envelope_storage_path", "TEXT"),
            ("envelope_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_artifact_projections (
                artifact_ref TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                parent_run_id TEXT,
                delegated_by_tool_call_id TEXT,
                actor_ref TEXT NOT NULL DEFAULT '',
                workflow_run_id TEXT,
                storage_path TEXT NOT NULL DEFAULT '',
                content_hash TEXT NOT NULL DEFAULT '',
                byte_size INTEGER NOT NULL DEFAULT 0,
                content_type TEXT NOT NULL DEFAULT 'application/json',
                updated_at INTEGER NOT NULL DEFAULT 0,
                summary_json TEXT NOT NULL DEFAULT '{}'
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_artifact_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("parent_run_id", "TEXT"),
            ("delegated_by_tool_call_id", "TEXT"),
            ("actor_ref", "TEXT NOT NULL DEFAULT ''"),
            ("workflow_run_id", "TEXT"),
            ("storage_path", "TEXT NOT NULL DEFAULT ''"),
            ("content_hash", "TEXT NOT NULL DEFAULT ''"),
            ("byte_size", "INTEGER NOT NULL DEFAULT 0"),
            ("content_type", "TEXT NOT NULL DEFAULT 'application/json'"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_workflow_projections (
                workflow_run_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                label TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                total_steps INTEGER NOT NULL DEFAULT 0,
                completed_steps INTEGER NOT NULL DEFAULT 0,
                current_step_id TEXT,
                current_step_label TEXT,
                background_capable INTEGER NOT NULL DEFAULT 0,
                detail_storage_path TEXT,
                detail_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL,
                detail_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_workflow_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("label", "TEXT NOT NULL DEFAULT ''"),
            ("status", "TEXT NOT NULL DEFAULT 'draft'"),
            ("total_steps", "INTEGER NOT NULL DEFAULT 0"),
            ("completed_steps", "INTEGER NOT NULL DEFAULT 0"),
            ("current_step_id", "TEXT"),
            ("current_step_label", "TEXT"),
            ("background_capable", "INTEGER NOT NULL DEFAULT 0"),
            ("detail_storage_path", "TEXT"),
            ("detail_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("detail_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_background_projections (
                run_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                workflow_run_id TEXT,
                status TEXT NOT NULL,
                background_capable INTEGER NOT NULL DEFAULT 0,
                state_storage_path TEXT,
                state_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                summary_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_background_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("workflow_run_id", "TEXT"),
            ("status", "TEXT NOT NULL DEFAULT 'draft'"),
            ("background_capable", "INTEGER NOT NULL DEFAULT 0"),
            ("state_storage_path", "TEXT"),
            ("state_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("summary_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_approval_projections (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                summary TEXT NOT NULL,
                detail TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT NOT NULL,
                approval_layer TEXT,
                capability_id TEXT,
                checkpoint_ref TEXT,
                provider_key TEXT,
                required_permission TEXT,
                requires_approval INTEGER NOT NULL DEFAULT 0,
                requires_auth INTEGER NOT NULL DEFAULT 0,
                target_kind TEXT,
                target_ref TEXT,
                escalation_reason TEXT,
                approval_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_approval_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("tool_name", "TEXT NOT NULL DEFAULT ''"),
            ("summary", "TEXT NOT NULL DEFAULT ''"),
            ("detail", "TEXT NOT NULL DEFAULT ''"),
            ("risk_level", "TEXT NOT NULL DEFAULT 'medium'"),
            ("created_at", "INTEGER NOT NULL DEFAULT 0"),
            ("status", "TEXT NOT NULL DEFAULT 'pending'"),
            ("approval_layer", "TEXT"),
            ("capability_id", "TEXT"),
            ("checkpoint_ref", "TEXT"),
            ("provider_key", "TEXT"),
            ("required_permission", "TEXT"),
            ("requires_approval", "INTEGER NOT NULL DEFAULT 0"),
            ("requires_auth", "INTEGER NOT NULL DEFAULT 0"),
            ("target_kind", "TEXT"),
            ("target_ref", "TEXT"),
            ("escalation_reason", "TEXT"),
            ("approval_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_auth_challenge_projections (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                summary TEXT NOT NULL,
                detail TEXT NOT NULL,
                status TEXT NOT NULL,
                resolution TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                approval_layer TEXT,
                capability_id TEXT,
                checkpoint_ref TEXT,
                provider_key TEXT,
                required_permission TEXT,
                requires_approval INTEGER NOT NULL DEFAULT 0,
                requires_auth INTEGER NOT NULL DEFAULT 0,
                target_kind TEXT,
                target_ref TEXT,
                escalation_reason TEXT,
                challenge_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_auth_challenge_projections",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("conversation_id", "TEXT NOT NULL DEFAULT ''"),
            ("summary", "TEXT NOT NULL DEFAULT ''"),
            ("detail", "TEXT NOT NULL DEFAULT ''"),
            ("status", "TEXT NOT NULL DEFAULT 'pending'"),
            ("resolution", "TEXT"),
            ("created_at", "INTEGER NOT NULL DEFAULT 0"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("approval_layer", "TEXT"),
            ("capability_id", "TEXT"),
            ("checkpoint_ref", "TEXT"),
            ("provider_key", "TEXT"),
            ("required_permission", "TEXT"),
            ("requires_approval", "INTEGER NOT NULL DEFAULT 0"),
            ("requires_auth", "INTEGER NOT NULL DEFAULT 0"),
            ("target_kind", "TEXT"),
            ("target_ref", "TEXT"),
            ("escalation_reason", "TEXT"),
            ("challenge_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    Ok(())
}

pub(crate) fn ensure_runtime_memory_projection_tables(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_memory_records (
                memory_id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                project_id TEXT,
                owner_ref TEXT,
                source_run_id TEXT,
                kind TEXT NOT NULL,
                scope TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                freshness_state TEXT NOT NULL,
                last_validated_at INTEGER,
                proposal_state TEXT NOT NULL,
                storage_path TEXT,
                content_hash TEXT,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_memory_records",
        &[
            ("workspace_id", "TEXT NOT NULL DEFAULT ''"),
            ("project_id", "TEXT"),
            ("owner_ref", "TEXT"),
            ("source_run_id", "TEXT"),
            ("kind", "TEXT NOT NULL DEFAULT 'reference'"),
            ("scope", "TEXT NOT NULL DEFAULT 'user'"),
            ("title", "TEXT NOT NULL DEFAULT ''"),
            ("summary", "TEXT NOT NULL DEFAULT ''"),
            ("freshness_state", "TEXT NOT NULL DEFAULT 'fresh'"),
            ("last_validated_at", "INTEGER"),
            ("proposal_state", "TEXT NOT NULL DEFAULT 'pending'"),
            ("storage_path", "TEXT"),
            ("content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
        ],
    )?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS runtime_memory_proposals (
                proposal_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                run_id TEXT NOT NULL,
                memory_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                scope TEXT NOT NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                proposal_state TEXT NOT NULL,
                proposal_reason TEXT NOT NULL,
                review_json TEXT,
                artifact_storage_path TEXT,
                artifact_content_hash TEXT,
                updated_at INTEGER NOT NULL,
                proposal_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "runtime_memory_proposals",
        &[
            ("session_id", "TEXT NOT NULL DEFAULT ''"),
            ("run_id", "TEXT NOT NULL DEFAULT ''"),
            ("memory_id", "TEXT NOT NULL DEFAULT ''"),
            ("kind", "TEXT NOT NULL DEFAULT 'reference'"),
            ("scope", "TEXT NOT NULL DEFAULT 'user'"),
            ("title", "TEXT NOT NULL DEFAULT ''"),
            ("summary", "TEXT NOT NULL DEFAULT ''"),
            ("proposal_state", "TEXT NOT NULL DEFAULT 'pending'"),
            ("proposal_reason", "TEXT NOT NULL DEFAULT ''"),
            ("review_json", "TEXT"),
            ("artifact_storage_path", "TEXT"),
            ("artifact_content_hash", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
            ("proposal_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )?;

    Ok(())
}

pub(crate) fn ensure_cost_entry_columns(connection: &Connection) -> Result<(), AppError> {
    let columns = table_columns(connection, "cost_entries")?;

    if !columns.iter().any(|column| column == "configured_model_id") {
        connection
            .execute(
                "ALTER TABLE cost_entries ADD COLUMN configured_model_id TEXT",
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS configured_model_usage_projections (
              configured_model_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_token_usage_projections (
              project_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS configured_model_budget_reservations (
              id TEXT PRIMARY KEY,
              configured_model_id TEXT NOT NULL,
              traffic_class TEXT NOT NULL DEFAULT 'interactive_turn',
              reserved_tokens INTEGER NOT NULL,
              status TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              released_at INTEGER,
              settled_at INTEGER
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS configured_model_budget_settlements (
              reservation_id TEXT PRIMARY KEY,
              configured_model_id TEXT NOT NULL,
              traffic_class TEXT NOT NULL DEFAULT 'interactive_turn',
              settled_tokens INTEGER NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "configured_model_budget_reservations",
        &[("traffic_class", "TEXT NOT NULL DEFAULT 'interactive_turn'")],
    )?;
    ensure_columns(
        connection,
        "configured_model_budget_settlements",
        &[("traffic_class", "TEXT NOT NULL DEFAULT 'interactive_turn'")],
    )?;
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS configured_model_budget_projections (
              configured_model_id TEXT PRIMARY KEY,
              settled_tokens INTEGER NOT NULL,
              active_reserved_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    let project_projection_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_token_usage_projections",
            [],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    if project_projection_count == 0 {
        rebuild_project_token_usage_projections(connection)?;
    }
    let configured_model_budget_projection_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM configured_model_budget_projections",
            [],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    if configured_model_budget_projection_count == 0 {
        backfill_configured_model_budget_projections(connection)?;
    }

    Ok(())
}

fn rebuild_project_token_usage_projections(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute("DELETE FROM project_token_usage_projections", [])
        .map_err(|error| AppError::database(error.to_string()))?;
    connection
        .execute(
            "INSERT INTO project_token_usage_projections (project_id, used_tokens, updated_at)
             SELECT project_id,
                    SUM(CASE WHEN amount > 0 THEN amount ELSE 0 END) AS used_tokens,
                    MAX(created_at) AS updated_at
             FROM cost_entries
             WHERE project_id IS NOT NULL
               AND metric = 'tokens'
             GROUP BY project_id
             HAVING SUM(CASE WHEN amount > 0 THEN amount ELSE 0 END) > 0",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn backfill_configured_model_budget_projections(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO configured_model_budget_projections (
                configured_model_id,
                settled_tokens,
                active_reserved_tokens,
                updated_at
            )
            SELECT configured_model_id, used_tokens, 0, updated_at
            FROM configured_model_usage_projections",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}
