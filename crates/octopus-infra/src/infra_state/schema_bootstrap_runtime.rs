use super::*;

pub(crate) fn apply_runtime_schema_batch(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS cost_entries (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              run_id TEXT,
              configured_model_id TEXT,
              metric TEXT NOT NULL,
              amount INTEGER NOT NULL,
              unit TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS configured_model_usage_projections (
              configured_model_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS configured_model_budget_reservations (
              id TEXT PRIMARY KEY,
              configured_model_id TEXT NOT NULL,
              traffic_class TEXT NOT NULL DEFAULT 'interactive_turn',
              reserved_tokens INTEGER NOT NULL,
              status TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              released_at INTEGER,
              settled_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS configured_model_budget_settlements (
              reservation_id TEXT PRIMARY KEY,
              configured_model_id TEXT NOT NULL,
              traffic_class TEXT NOT NULL DEFAULT 'interactive_turn',
              settled_tokens INTEGER NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS configured_model_budget_projections (
              configured_model_id TEXT PRIMARY KEY,
              settled_tokens INTEGER NOT NULL,
              active_reserved_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_token_usage_projections (
              project_id TEXT PRIMARY KEY,
              used_tokens INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_config_snapshots (
              id TEXT PRIMARY KEY,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              source_refs TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              effective_config_json TEXT
            );
            CREATE TABLE IF NOT EXISTS runtime_secret_records (
              reference TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              ciphertext BLOB NOT NULL,
              nonce BLOB NOT NULL,
              key_version INTEGER NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_session_projections (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              title TEXT NOT NULL,
              session_kind TEXT NOT NULL DEFAULT 'project',
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              last_message_preview TEXT,
              config_snapshot_id TEXT NOT NULL,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              selected_actor_ref TEXT NOT NULL DEFAULT '',
              manifest_revision TEXT NOT NULL DEFAULT '',
              active_run_id TEXT NOT NULL DEFAULT '',
              subrun_count INTEGER NOT NULL DEFAULT 0,
              workflow_run_id TEXT,
              workflow_status TEXT,
              workflow_total_steps INTEGER NOT NULL DEFAULT 0,
              workflow_completed_steps INTEGER NOT NULL DEFAULT 0,
              workflow_current_step_id TEXT,
              workflow_current_step_label TEXT,
              workflow_background_capable INTEGER NOT NULL DEFAULT 0,
              pending_mailbox_ref TEXT,
              pending_mailbox_count INTEGER NOT NULL DEFAULT 0,
              handoff_count INTEGER NOT NULL DEFAULT 0,
              background_run_id TEXT,
              background_workflow_run_id TEXT,
              background_status TEXT,
              manifest_snapshot_ref TEXT NOT NULL DEFAULT '',
              session_policy_snapshot_ref TEXT NOT NULL DEFAULT '',
              capability_plan_summary_json TEXT NOT NULL DEFAULT '{}',
              provider_state_summary_json TEXT NOT NULL DEFAULT '[]',
              pending_mediation_json TEXT,
              last_execution_outcome_json TEXT,
              granted_tool_count INTEGER NOT NULL DEFAULT 0,
              injected_skill_message_count INTEGER NOT NULL DEFAULT 0,
              deferred_capability_count INTEGER NOT NULL DEFAULT 0,
              hidden_capability_count INTEGER NOT NULL DEFAULT 0,
              degraded_provider_count INTEGER NOT NULL DEFAULT 0,
              detail_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_presence (
              scope_key TEXT PRIMARY KEY,
              owner_user_id TEXT,
              context_scope TEXT NOT NULL DEFAULT 'home',
              project_id TEXT,
              pet_id TEXT NOT NULL,
              is_visible INTEGER NOT NULL,
              chat_open INTEGER NOT NULL,
              motion_state TEXT NOT NULL,
              unread_count INTEGER NOT NULL,
              last_interaction_at INTEGER NOT NULL,
              position_x INTEGER NOT NULL,
              position_y INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_conversation_bindings (
              scope_key TEXT PRIMARY KEY,
              owner_user_id TEXT,
              context_scope TEXT NOT NULL DEFAULT 'home',
              project_id TEXT,
              pet_id TEXT NOT NULL,
              workspace_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              session_id TEXT,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pet_agent_extensions (
              pet_id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              owner_user_id TEXT NOT NULL,
              species TEXT NOT NULL,
              display_name TEXT NOT NULL,
              avatar_label TEXT NOT NULL,
              summary TEXT NOT NULL,
              greeting TEXT NOT NULL,
              mood TEXT NOT NULL,
              favorite_snack TEXT NOT NULL,
              prompt_hints_json TEXT NOT NULL DEFAULT '[]',
              fallback_asset TEXT NOT NULL,
              rive_asset TEXT,
              state_machine TEXT,
              updated_at INTEGER NOT NULL,
              UNIQUE(workspace_id, owner_user_id)
            );
            CREATE TABLE IF NOT EXISTS runtime_run_projections (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              status TEXT NOT NULL,
              current_step TEXT NOT NULL,
              started_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              model_id TEXT,
              next_action TEXT,
              config_snapshot_id TEXT NOT NULL,
              effective_config_hash TEXT NOT NULL,
              started_from_scope_set TEXT NOT NULL,
              run_kind TEXT NOT NULL DEFAULT 'primary',
              parent_run_id TEXT,
              actor_ref TEXT NOT NULL DEFAULT '',
              delegated_by_tool_call_id TEXT,
              workflow_run_id TEXT,
              workflow_step_id TEXT,
              workflow_status TEXT,
              mailbox_ref TEXT,
              handoff_ref TEXT,
              background_state TEXT,
              worker_total_subruns INTEGER NOT NULL DEFAULT 0,
              worker_active_subruns INTEGER NOT NULL DEFAULT 0,
              worker_completed_subruns INTEGER NOT NULL DEFAULT 0,
              worker_failed_subruns INTEGER NOT NULL DEFAULT 0,
              worker_dispatch_json TEXT,
              workflow_run_detail_json TEXT,
              approval_state TEXT NOT NULL DEFAULT 'not-required',
              trace_id TEXT NOT NULL DEFAULT '',
              turn_id TEXT NOT NULL DEFAULT '',
              capability_plan_summary_json TEXT NOT NULL DEFAULT '{}',
              provider_state_summary_json TEXT NOT NULL DEFAULT '[]',
              pending_mediation_json TEXT,
              last_execution_outcome_json TEXT,
              granted_tool_count INTEGER NOT NULL DEFAULT 0,
              injected_skill_message_count INTEGER NOT NULL DEFAULT 0,
              deferred_capability_count INTEGER NOT NULL DEFAULT 0,
              hidden_capability_count INTEGER NOT NULL DEFAULT 0,
              degraded_provider_count INTEGER NOT NULL DEFAULT 0,
              run_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_subrun_projections (
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
            );
            CREATE TABLE IF NOT EXISTS runtime_mailbox_projections (
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
            );
            CREATE TABLE IF NOT EXISTS runtime_handoff_projections (
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
            );
            CREATE TABLE IF NOT EXISTS runtime_workflow_projections (
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
            );
            CREATE TABLE IF NOT EXISTS runtime_background_projections (
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
            );
            CREATE TABLE IF NOT EXISTS runtime_approval_projections (
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
              approval_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_memory_records (
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
            );
            CREATE TABLE IF NOT EXISTS runtime_memory_proposals (
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
            );
            ",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}
