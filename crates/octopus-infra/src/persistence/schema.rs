use std::fs;

use octopus_core::{
    AppError, ProjectLinkedWorkspaceAssets, ProjectPermissionOverrides,
    ProjectWorkspaceAssignments, DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
};
use octopus_persistence::Database;
use rusqlite::{params, Connection, OptionalExtension};

use super::map_db_error;
use crate::{
    agent_seed, default_client_apps, default_knowledge_records, default_model_catalog,
    default_project_assignments, default_project_model_assignments,
    default_project_permission_overrides, default_provider_credentials, default_tool_records,
    default_workspace_resources, load_data_policies, normalized_project_member_user_ids,
    WorkspacePaths, BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID,
};

pub(crate) fn initialize_database(database: &Database) -> Result<(), AppError> {
    let connection = database.acquire().map_err(map_db_error)?;
    drop_legacy_access_control_tables(&connection)?;
    reset_legacy_sessions_table(&connection)?;

    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS users (
              id TEXT PRIMARY KEY,
              username TEXT NOT NULL UNIQUE,
              display_name TEXT NOT NULL,
              avatar_path TEXT,
              avatar_content_type TEXT,
              avatar_byte_size INTEGER,
              avatar_content_hash TEXT,
              status TEXT NOT NULL,
              password_hash TEXT NOT NULL,
              password_state TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS client_apps (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              platform TEXT NOT NULL,
              status TEXT NOT NULL,
              first_party INTEGER NOT NULL,
              allowed_origins TEXT NOT NULL,
              allowed_hosts TEXT NOT NULL,
              session_policy TEXT NOT NULL,
              default_scopes TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              user_id TEXT NOT NULL,
              client_app_id TEXT NOT NULL,
              token TEXT NOT NULL UNIQUE,
              status TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              expires_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS projects (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              name TEXT NOT NULL,
              status TEXT NOT NULL,
              description TEXT NOT NULL,
              resource_directory TEXT NOT NULL,
              leader_agent_id TEXT,
              manager_user_id TEXT,
              preset_code TEXT,
              assignments_json TEXT,
              owner_user_id TEXT,
              member_user_ids_json TEXT,
              permission_overrides_json TEXT,
              linked_workspace_assets_json TEXT
            );
            CREATE TABLE IF NOT EXISTS project_promotion_requests (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              asset_type TEXT NOT NULL,
              asset_id TEXT NOT NULL,
              requested_by_user_id TEXT NOT NULL,
              submitted_by_owner_user_id TEXT NOT NULL,
              required_workspace_capability TEXT NOT NULL,
              status TEXT NOT NULL,
              reviewed_by_user_id TEXT,
              review_comment TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              reviewed_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS project_deletion_requests (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              requested_by_user_id TEXT NOT NULL,
              status TEXT NOT NULL,
              reason TEXT,
              reviewed_by_user_id TEXT,
              review_comment TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              reviewed_at INTEGER
            );
            CREATE TABLE IF NOT EXISTS project_tasks (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              title TEXT NOT NULL,
              goal TEXT NOT NULL,
              brief TEXT NOT NULL,
              default_actor_ref TEXT NOT NULL,
              status TEXT NOT NULL,
              schedule_spec TEXT,
              next_run_at INTEGER,
              last_run_at INTEGER,
              active_task_run_id TEXT,
              latest_result_summary TEXT,
              latest_failure_category TEXT,
              latest_transition_json TEXT,
              view_status TEXT NOT NULL,
              attention_reasons_json TEXT NOT NULL DEFAULT '[]',
              attention_updated_at INTEGER,
              analytics_summary_json TEXT NOT NULL DEFAULT '{}',
              context_bundle_json TEXT NOT NULL DEFAULT '{}',
              latest_deliverable_refs_json TEXT NOT NULL DEFAULT '[]',
              latest_artifact_refs_json TEXT NOT NULL DEFAULT '[]',
              created_by TEXT NOT NULL,
              updated_by TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS project_tasks_project_updated_idx
              ON project_tasks (project_id, updated_at DESC, id ASC);
            CREATE TABLE IF NOT EXISTS project_task_runs (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              task_id TEXT NOT NULL,
              trigger_type TEXT NOT NULL,
              status TEXT NOT NULL,
              session_id TEXT,
              conversation_id TEXT,
              runtime_run_id TEXT,
              actor_ref TEXT NOT NULL,
              started_at INTEGER NOT NULL,
              completed_at INTEGER,
              result_summary TEXT,
              pending_approval_id TEXT,
              failure_category TEXT,
              failure_summary TEXT,
              view_status TEXT NOT NULL,
              attention_reasons_json TEXT NOT NULL DEFAULT '[]',
              attention_updated_at INTEGER,
              deliverable_refs_json TEXT NOT NULL DEFAULT '[]',
              artifact_refs_json TEXT NOT NULL DEFAULT '[]',
              latest_transition_json TEXT
            );
            CREATE INDEX IF NOT EXISTS project_task_runs_task_started_idx
              ON project_task_runs (task_id, started_at DESC, id ASC);
            CREATE TABLE IF NOT EXISTS project_task_interventions (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              task_id TEXT NOT NULL,
              task_run_id TEXT,
              type TEXT NOT NULL,
              payload_json TEXT NOT NULL DEFAULT '{}',
              created_by TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              applied_to_session_id TEXT,
              status TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS project_task_interventions_task_created_idx
              ON project_task_interventions (task_id, created_at DESC, id ASC);
            CREATE TABLE IF NOT EXISTS project_task_scheduler_claims (
              task_id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              claim_token TEXT,
              claimed_by TEXT,
              claim_until INTEGER,
              last_dispatched_at INTEGER,
              last_evaluated_at INTEGER,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS resources (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              kind TEXT NOT NULL,
              name TEXT NOT NULL,
              location TEXT,
              origin TEXT NOT NULL,
              scope TEXT NOT NULL,
              visibility TEXT NOT NULL,
              owner_user_id TEXT NOT NULL,
              storage_path TEXT,
              content_type TEXT,
              byte_size INTEGER,
              preview_kind TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              tags TEXT NOT NULL,
              source_artifact_id TEXT
            );
            CREATE TABLE IF NOT EXISTS knowledge_records (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              title TEXT NOT NULL,
              summary TEXT NOT NULL,
              kind TEXT NOT NULL,
              scope TEXT,
              status TEXT NOT NULL,
              visibility TEXT,
              owner_user_id TEXT,
              source_type TEXT NOT NULL,
              source_ref TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS artifact_records (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              session_id TEXT NOT NULL,
              run_id TEXT NOT NULL,
              source_message_id TEXT,
              parent_artifact_id TEXT,
              title TEXT NOT NULL,
              status TEXT NOT NULL,
              preview_kind TEXT NOT NULL,
              latest_version INTEGER NOT NULL,
              promotion_state TEXT NOT NULL DEFAULT 'not-promoted',
              promotion_knowledge_id TEXT,
              updated_at INTEGER NOT NULL,
              storage_path TEXT,
              content_hash TEXT,
              byte_size INTEGER,
              content_type TEXT
            );
            CREATE TABLE IF NOT EXISTS artifact_versions (
              artifact_id TEXT NOT NULL,
              version INTEGER NOT NULL,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              conversation_id TEXT NOT NULL,
              session_id TEXT,
              run_id TEXT,
              source_message_id TEXT,
              parent_version INTEGER,
              title TEXT NOT NULL,
              preview_kind TEXT NOT NULL,
              updated_at INTEGER NOT NULL,
              storage_path TEXT NOT NULL,
              content_hash TEXT NOT NULL,
              byte_size INTEGER NOT NULL DEFAULT 0,
              content_type TEXT,
              PRIMARY KEY (artifact_id, version)
            );
            CREATE INDEX IF NOT EXISTS artifact_records_project_updated_idx
              ON artifact_records (project_id, updated_at DESC, id ASC);
            CREATE INDEX IF NOT EXISTS artifact_versions_artifact_updated_idx
              ON artifact_versions (artifact_id, version DESC);
            CREATE TABLE IF NOT EXISTS agents (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              scope TEXT NOT NULL,
              name TEXT NOT NULL,
              avatar_path TEXT,
              personality TEXT NOT NULL,
              tags TEXT NOT NULL,
              prompt TEXT NOT NULL,
              builtin_tool_keys TEXT NOT NULL,
              skill_ids TEXT NOT NULL,
              mcp_server_names TEXT NOT NULL,
              task_domains TEXT NOT NULL DEFAULT '[]',
              manifest_revision TEXT NOT NULL DEFAULT 'asset-manifest/v2',
              default_model_strategy_json TEXT NOT NULL DEFAULT '{}',
              capability_policy_json TEXT NOT NULL DEFAULT '{}',
              permission_envelope_json TEXT NOT NULL DEFAULT '{}',
              memory_policy_json TEXT NOT NULL DEFAULT '{}',
              delegation_policy_json TEXT NOT NULL DEFAULT '{}',
              approval_preference_json TEXT NOT NULL DEFAULT '{}',
              output_contract_json TEXT NOT NULL DEFAULT '{}',
              shared_capability_policy_json TEXT NOT NULL DEFAULT '{}',
              integration_source_json TEXT,
              trust_metadata_json TEXT NOT NULL DEFAULT '{}',
              dependency_resolution_json TEXT NOT NULL DEFAULT '[]',
              import_metadata_json TEXT NOT NULL DEFAULT '{}',
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_agent_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              agent_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, agent_id)
            );
            CREATE TABLE IF NOT EXISTS teams (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              scope TEXT NOT NULL,
              name TEXT NOT NULL,
              avatar_path TEXT,
              personality TEXT NOT NULL,
              tags TEXT NOT NULL,
              prompt TEXT NOT NULL,
              builtin_tool_keys TEXT NOT NULL,
              skill_ids TEXT NOT NULL,
              mcp_server_names TEXT NOT NULL,
              task_domains TEXT NOT NULL DEFAULT '[]',
              manifest_revision TEXT NOT NULL DEFAULT 'asset-manifest/v2',
              default_model_strategy_json TEXT NOT NULL DEFAULT '{}',
              capability_policy_json TEXT NOT NULL DEFAULT '{}',
              permission_envelope_json TEXT NOT NULL DEFAULT '{}',
              memory_policy_json TEXT NOT NULL DEFAULT '{}',
              delegation_policy_json TEXT NOT NULL DEFAULT '{}',
              approval_preference_json TEXT NOT NULL DEFAULT '{}',
              output_contract_json TEXT NOT NULL DEFAULT '{}',
              shared_capability_policy_json TEXT NOT NULL DEFAULT '{}',
              leader_ref TEXT NOT NULL DEFAULT '',
              member_refs TEXT NOT NULL DEFAULT '[]',
              team_topology_json TEXT NOT NULL DEFAULT '{}',
              shared_memory_policy_json TEXT NOT NULL DEFAULT '{}',
              mailbox_policy_json TEXT NOT NULL DEFAULT '{}',
              artifact_handoff_policy_json TEXT NOT NULL DEFAULT '{}',
              workflow_affordance_json TEXT NOT NULL DEFAULT '{}',
              worker_concurrency_limit INTEGER NOT NULL DEFAULT 1,
              integration_source_json TEXT,
              trust_metadata_json TEXT NOT NULL DEFAULT '{}',
              dependency_resolution_json TEXT NOT NULL DEFAULT '[]',
              import_metadata_json TEXT NOT NULL DEFAULT '{}',
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS bundle_asset_descriptors (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              scope TEXT NOT NULL,
              asset_kind TEXT NOT NULL,
              source_id TEXT NOT NULL,
              display_name TEXT NOT NULL,
              source_path TEXT NOT NULL,
              storage_path TEXT NOT NULL,
              content_hash TEXT NOT NULL,
              byte_size INTEGER NOT NULL,
              manifest_revision TEXT NOT NULL DEFAULT 'asset-manifest/v2',
              task_domains_json TEXT NOT NULL DEFAULT '[]',
              translation_mode TEXT NOT NULL DEFAULT 'native',
              trust_metadata_json TEXT NOT NULL DEFAULT '{}',
              dependency_resolution_json TEXT NOT NULL DEFAULT '[]',
              import_metadata_json TEXT NOT NULL DEFAULT '{}',
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project_team_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              team_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, team_id)
            );
            CREATE TABLE IF NOT EXISTS model_catalog (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              label TEXT NOT NULL,
              provider TEXT NOT NULL,
              description TEXT NOT NULL,
              recommended_for TEXT NOT NULL,
              availability TEXT NOT NULL,
              default_permission TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS provider_credentials (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              provider TEXT NOT NULL,
              name TEXT NOT NULL,
              base_url TEXT,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tools (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              kind TEXT NOT NULL,
              name TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              permission_mode TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS org_units (
              id TEXT PRIMARY KEY,
              parent_id TEXT,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS positions (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_groups (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_org_assignments (
              user_id TEXT NOT NULL,
              org_unit_id TEXT NOT NULL,
              is_primary INTEGER NOT NULL,
              position_ids TEXT NOT NULL,
              user_group_ids TEXT NOT NULL,
              PRIMARY KEY (user_id, org_unit_id)
            );
            CREATE TABLE IF NOT EXISTS access_roles (
              id TEXT PRIMARY KEY,
              code TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              description TEXT NOT NULL,
              status TEXT NOT NULL,
              permission_codes TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS role_bindings (
              id TEXT PRIMARY KEY,
              role_id TEXT NOT NULL,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS data_policies (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              resource_type TEXT NOT NULL,
              scope_type TEXT NOT NULL,
              project_ids TEXT NOT NULL,
              tags TEXT NOT NULL,
              classifications TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS resource_policies (
              id TEXT PRIMARY KEY,
              subject_type TEXT NOT NULL,
              subject_id TEXT NOT NULL,
              resource_type TEXT NOT NULL,
              resource_id TEXT NOT NULL,
              action_name TEXT NOT NULL,
              effect TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS menu_policies (
              menu_id TEXT PRIMARY KEY,
              enabled INTEGER NOT NULL,
              order_value INTEGER NOT NULL,
              group_key TEXT,
              visibility TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS protected_resources (
              resource_type TEXT NOT NULL,
              resource_id TEXT NOT NULL,
              resource_subtype TEXT,
              project_id TEXT,
              tags TEXT NOT NULL,
              classification TEXT NOT NULL,
              owner_subject_type TEXT,
              owner_subject_id TEXT,
              PRIMARY KEY (resource_type, resource_id)
            );
            CREATE TABLE IF NOT EXISTS audit_records (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              actor_type TEXT NOT NULL,
              actor_id TEXT NOT NULL,
              action TEXT NOT NULL,
              resource TEXT NOT NULL,
              outcome TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS trace_events (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT,
              run_id TEXT,
              session_id TEXT,
              event_kind TEXT NOT NULL,
              title TEXT NOT NULL,
              detail TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
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

    ensure_user_avatar_columns(&connection)?;
    ensure_agent_record_columns(&connection)?;
    ensure_pet_agent_extension_columns(&connection)?;
    ensure_pet_projection_columns(&connection)?;
    ensure_team_record_columns(&connection)?;
    ensure_bundle_asset_descriptor_columns(&connection)?;
    ensure_project_assignment_columns(&connection)?;
    ensure_project_promotion_request_table(&connection)?;
    ensure_project_deletion_request_table(&connection)?;
    ensure_project_agent_link_table(&connection)?;
    ensure_project_team_link_table(&connection)?;
    ensure_project_task_run_columns(&connection)?;
    ensure_runtime_config_snapshot_columns(&connection)?;
    ensure_runtime_session_projection_columns(&connection)?;
    ensure_runtime_run_projection_columns(&connection)?;
    ensure_runtime_phase_four_projection_tables(&connection)?;
    ensure_runtime_memory_projection_tables(&connection)?;
    ensure_cost_entry_columns(&connection)?;
    ensure_resource_columns(&connection)?;
    ensure_knowledge_columns(&connection)?;
    agent_seed::ensure_import_source_tables(&connection)?;

    Ok(())
}

pub(crate) fn seed_defaults(database: &Database, paths: &WorkspacePaths) -> Result<(), AppError> {
    let connection = database.acquire().map_err(map_db_error)?;

    let project_exists: Option<String> = connection
        .query_row(
            "SELECT id FROM projects WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if project_exists.is_none() {
        let default_project_resource_directory =
            paths.default_project_resource_directory(DEFAULT_PROJECT_ID);
        let default_project_assignments = serde_json::to_string(&default_project_assignments())?;
        let default_permission_overrides = serde_json::to_string(&ProjectPermissionOverrides {
            agents: "inherit".into(),
            resources: "inherit".into(),
            tools: "inherit".into(),
            knowledge: "inherit".into(),
            tasks: "inherit".into(),
        })?;
        let default_linked_assets = serde_json::to_string(&ProjectLinkedWorkspaceAssets {
            agent_ids: Vec::new(),
            resource_ids: Vec::new(),
            tool_source_keys: Vec::new(),
            knowledge_ids: Vec::new(),
        })?;
        let default_member_user_ids = serde_json::to_string(&vec!["user-owner".to_string()])?;
        connection
            .execute(
                "INSERT INTO projects
                 (id, workspace_id, name, status, description, resource_directory, leader_agent_id, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    DEFAULT_PROJECT_ID,
                    DEFAULT_WORKSPACE_ID,
                    "Default Project",
                    "active",
                    "Bootstrap project for the local workspace.",
                    default_project_resource_directory,
                    Option::<String>::None,
                    Some(default_project_assignments),
                    "user-owner",
                    default_member_user_ids,
                    default_permission_overrides,
                    default_linked_assets,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    let resources_exist: Option<String> = connection
        .query_row("SELECT id FROM resources LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if resources_exist.is_none() {
        for record in default_workspace_resources() {
            connection
                .execute(
                    "INSERT INTO resources (id, workspace_id, project_id, kind, name, location, origin, scope, visibility, owner_user_id, storage_path, content_type, byte_size, preview_kind, status, updated_at, tags, source_artifact_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.kind,
                        record.name,
                        record.location,
                        record.origin,
                        record.scope,
                        record.visibility,
                        record.owner_user_id,
                        record.storage_path,
                        record.content_type,
                        record.byte_size.map(|value| value as i64),
                        record.preview_kind,
                        record.status,
                        record.updated_at as i64,
                        serde_json::to_string(&record.tags)?,
                        record.source_artifact_id,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        fs::create_dir_all(&paths.workspace_resources_dir)?;
        fs::write(
            paths.workspace_resources_dir.join("workspace-handbook.md"),
            "# Workspace Handbook\n\nShared operating rules for this workspace.\n",
        )?;
        fs::create_dir_all(
            paths
                .project_resources_dir(DEFAULT_PROJECT_ID)
                .join("delivery-board"),
        )?;
    }

    let knowledge_exists: Option<String> = connection
        .query_row("SELECT id FROM knowledge_records LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if knowledge_exists.is_none() {
        for record in default_knowledge_records() {
            connection
                .execute(
                    "INSERT INTO knowledge_records (id, workspace_id, project_id, title, summary, kind, scope, status, visibility, owner_user_id, source_type, source_ref, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.project_id,
                        record.title,
                        record.summary,
                        record.kind,
                        record.scope,
                        record.status,
                        record.visibility,
                        record.owner_user_id,
                        record.source_type,
                        record.source_ref,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    connection
        .execute(
            "INSERT OR IGNORE INTO org_units (id, parent_id, code, name, status)
             VALUES ('org-root', NULL, 'root', 'Root Organization', 'active')",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    let models_exist: Option<String> = connection
        .query_row("SELECT id FROM model_catalog LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if models_exist.is_none() {
        for record in default_model_catalog() {
            connection
                .execute(
                    "INSERT INTO model_catalog (id, workspace_id, label, provider, description, recommended_for, availability, default_permission)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.label,
                        record.provider,
                        record.description,
                        record.recommended_for,
                        record.availability,
                        record.default_permission,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let provider_credentials_exist: Option<String> = connection
        .query_row("SELECT id FROM provider_credentials LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if provider_credentials_exist.is_none() {
        for record in default_provider_credentials() {
            connection
                .execute(
                    "INSERT INTO provider_credentials (id, workspace_id, provider, name, base_url, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.provider,
                        record.name,
                        record.base_url,
                        record.status,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    let tools_exist: Option<String> = connection
        .query_row("SELECT id FROM tools LIMIT 1", [], |row| row.get(0))
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?;
    if tools_exist.is_none() {
        for record in default_tool_records() {
            connection
                .execute(
                    "INSERT INTO tools (id, workspace_id, kind, name, description, status, permission_mode, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        record.id,
                        record.workspace_id,
                        record.kind,
                        record.name,
                        record.description,
                        record.status,
                        record.permission_mode,
                        record.updated_at as i64,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
    }

    for app in default_client_apps() {
        connection
            .execute(
                "INSERT OR REPLACE INTO client_apps
                 (id, name, platform, status, first_party, allowed_origins, allowed_hosts, session_policy, default_scopes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    app.id,
                    app.name,
                    app.platform,
                    app.status,
                    if app.first_party { 1 } else { 0 },
                    serde_json::to_string(&app.allowed_origins)?,
                    serde_json::to_string(&app.allowed_hosts)?,
                    app.session_policy,
                    serde_json::to_string(&app.default_scopes)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(crate) fn table_columns(
    connection: &Connection,
    table_name: &str,
) -> Result<Vec<String>, AppError> {
    let mut stmt = connection
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(columns)
}

pub(crate) fn ensure_columns(
    connection: &Connection,
    table_name: &str,
    definitions: &[(&str, &str)],
) -> Result<(), AppError> {
    let columns = table_columns(connection, table_name)?;

    for (name, definition) in definitions {
        if columns.iter().any(|column| column == name) {
            continue;
        }

        connection
            .execute(
                &format!("ALTER TABLE {table_name} ADD COLUMN {name} {definition}"),
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(crate) fn ensure_user_avatar_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "users",
        &[
            ("avatar_path", "TEXT"),
            ("avatar_content_type", "TEXT"),
            ("avatar_byte_size", "INTEGER"),
            ("avatar_content_hash", "TEXT"),
        ],
    )
}

pub(crate) fn ensure_agent_record_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "agents",
        &[
            ("owner_user_id", "TEXT"),
            ("asset_role", "TEXT NOT NULL DEFAULT 'default'"),
            ("avatar_path", "TEXT"),
            ("personality", "TEXT NOT NULL DEFAULT ''"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("prompt", "TEXT NOT NULL DEFAULT ''"),
            ("builtin_tool_keys", "TEXT NOT NULL DEFAULT '[]'"),
            ("skill_ids", "TEXT NOT NULL DEFAULT '[]'"),
            ("mcp_server_names", "TEXT NOT NULL DEFAULT '[]'"),
            ("task_domains", "TEXT NOT NULL DEFAULT '[]'"),
            (
                "manifest_revision",
                "TEXT NOT NULL DEFAULT 'asset-manifest/v2'",
            ),
            ("default_model_strategy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("capability_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("permission_envelope_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("memory_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("delegation_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("approval_preference_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("output_contract_json", "TEXT NOT NULL DEFAULT '{}'"),
            (
                "shared_capability_policy_json",
                "TEXT NOT NULL DEFAULT '{}'",
            ),
            ("integration_source_json", "TEXT"),
            ("trust_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("dependency_resolution_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("import_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )
}

pub(crate) fn ensure_pet_agent_extension_columns(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS pet_agent_extensions (
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
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "pet_agent_extensions",
        &[
            ("workspace_id", "TEXT NOT NULL DEFAULT ''"),
            ("owner_user_id", "TEXT NOT NULL DEFAULT 'user-owner'"),
            ("species", "TEXT NOT NULL DEFAULT 'octopus'"),
            ("display_name", "TEXT NOT NULL DEFAULT '小章'"),
            ("avatar_label", "TEXT NOT NULL DEFAULT 'Octopus mascot'"),
            (
                "summary",
                "TEXT NOT NULL DEFAULT 'Octopus 首席吉祥物，负责卖萌和加油。'",
            ),
            (
                "greeting",
                "TEXT NOT NULL DEFAULT '嗨！我是小章，今天也要加油哦！'",
            ),
            ("mood", "TEXT NOT NULL DEFAULT 'happy'"),
            ("favorite_snack", "TEXT NOT NULL DEFAULT '新鲜小虾'"),
            ("prompt_hints_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("fallback_asset", "TEXT NOT NULL DEFAULT 'octopus'"),
            ("rive_asset", "TEXT"),
            ("state_machine", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
        ],
    )?;
    connection
        .execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_pet_agent_extensions_workspace_owner
             ON pet_agent_extensions (workspace_id, owner_user_id)",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn ensure_pet_projection_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "pet_presence",
        &[
            ("owner_user_id", "TEXT"),
            ("context_scope", "TEXT NOT NULL DEFAULT 'home'"),
            ("project_id", "TEXT"),
        ],
    )?;
    ensure_columns(
        connection,
        "pet_conversation_bindings",
        &[
            ("owner_user_id", "TEXT"),
            ("context_scope", "TEXT NOT NULL DEFAULT 'home'"),
            ("project_id", "TEXT"),
        ],
    )
}

pub(crate) fn ensure_team_record_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "teams",
        &[
            ("avatar_path", "TEXT"),
            ("personality", "TEXT NOT NULL DEFAULT ''"),
            ("tags", "TEXT NOT NULL DEFAULT '[]'"),
            ("prompt", "TEXT NOT NULL DEFAULT ''"),
            ("builtin_tool_keys", "TEXT NOT NULL DEFAULT '[]'"),
            ("skill_ids", "TEXT NOT NULL DEFAULT '[]'"),
            ("mcp_server_names", "TEXT NOT NULL DEFAULT '[]'"),
            ("task_domains", "TEXT NOT NULL DEFAULT '[]'"),
            (
                "manifest_revision",
                "TEXT NOT NULL DEFAULT 'asset-manifest/v2'",
            ),
            ("default_model_strategy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("capability_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("permission_envelope_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("memory_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("delegation_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("approval_preference_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("output_contract_json", "TEXT NOT NULL DEFAULT '{}'"),
            (
                "shared_capability_policy_json",
                "TEXT NOT NULL DEFAULT '{}'",
            ),
            ("leader_ref", "TEXT NOT NULL DEFAULT ''"),
            ("member_refs", "TEXT NOT NULL DEFAULT '[]'"),
            ("team_topology_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("shared_memory_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("mailbox_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("artifact_handoff_policy_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("workflow_affordance_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("worker_concurrency_limit", "INTEGER NOT NULL DEFAULT 1"),
            ("integration_source_json", "TEXT"),
            ("trust_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("dependency_resolution_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("import_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
        ],
    )
}

pub(crate) fn ensure_bundle_asset_descriptor_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS bundle_asset_descriptors (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                project_id TEXT,
                scope TEXT NOT NULL,
                asset_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                source_path TEXT NOT NULL,
                storage_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                byte_size INTEGER NOT NULL,
                manifest_revision TEXT NOT NULL DEFAULT 'asset-manifest/v2',
                task_domains_json TEXT NOT NULL DEFAULT '[]',
                translation_mode TEXT NOT NULL DEFAULT 'native',
                trust_metadata_json TEXT NOT NULL DEFAULT '{}',
                dependency_resolution_json TEXT NOT NULL DEFAULT '[]',
                import_metadata_json TEXT NOT NULL DEFAULT '{}',
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    ensure_columns(
        connection,
        "bundle_asset_descriptors",
        &[
            ("project_id", "TEXT"),
            ("scope", "TEXT NOT NULL DEFAULT 'workspace'"),
            ("asset_kind", "TEXT NOT NULL DEFAULT 'plugin'"),
            ("source_id", "TEXT NOT NULL DEFAULT ''"),
            ("display_name", "TEXT NOT NULL DEFAULT ''"),
            ("source_path", "TEXT NOT NULL DEFAULT ''"),
            ("storage_path", "TEXT NOT NULL DEFAULT ''"),
            ("content_hash", "TEXT NOT NULL DEFAULT ''"),
            ("byte_size", "INTEGER NOT NULL DEFAULT 0"),
            (
                "manifest_revision",
                "TEXT NOT NULL DEFAULT 'asset-manifest/v2'",
            ),
            ("task_domains_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("translation_mode", "TEXT NOT NULL DEFAULT 'native'"),
            ("trust_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("dependency_resolution_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("import_metadata_json", "TEXT NOT NULL DEFAULT '{}'"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
        ],
    )
}

pub(crate) fn ensure_project_assignment_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "projects",
        &[
            ("leader_agent_id", "TEXT"),
            ("manager_user_id", "TEXT"),
            ("preset_code", "TEXT"),
            ("assignments_json", "TEXT"),
            ("resource_directory", "TEXT"),
            ("owner_user_id", "TEXT"),
            ("member_user_ids_json", "TEXT"),
            ("permission_overrides_json", "TEXT"),
            ("linked_workspace_assets_json", "TEXT"),
        ],
    )
}

pub(crate) fn ensure_project_promotion_request_table(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_promotion_requests (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              asset_type TEXT NOT NULL,
              asset_id TEXT NOT NULL,
              requested_by_user_id TEXT NOT NULL,
              submitted_by_owner_user_id TEXT NOT NULL,
              required_workspace_capability TEXT NOT NULL,
              status TEXT NOT NULL,
              reviewed_by_user_id TEXT,
              review_comment TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              reviewed_at INTEGER
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn ensure_project_deletion_request_table(
    connection: &Connection,
) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_deletion_requests (
              id TEXT PRIMARY KEY,
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              requested_by_user_id TEXT NOT NULL,
              status TEXT NOT NULL,
              reason TEXT,
              reviewed_by_user_id TEXT,
              review_comment TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              reviewed_at INTEGER
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn ensure_project_agent_link_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_agent_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              agent_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, agent_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn ensure_project_team_link_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS project_team_links (
              workspace_id TEXT NOT NULL,
              project_id TEXT NOT NULL,
              team_id TEXT NOT NULL,
              linked_at INTEGER NOT NULL,
              PRIMARY KEY (workspace_id, project_id, team_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

pub(crate) fn ensure_project_task_run_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "project_task_runs",
        &[("pending_approval_id", "TEXT")],
    )
}

pub(crate) fn ensure_runtime_config_snapshot_columns(
    connection: &Connection,
) -> Result<(), AppError> {
    let mut stmt = connection
        .prepare("PRAGMA table_info(runtime_config_snapshots)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

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
    let mut stmt = connection
        .prepare("PRAGMA table_info(cost_entries)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

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

pub(crate) fn ensure_resource_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "resources",
        &[
            ("scope", "TEXT"),
            ("visibility", "TEXT"),
            ("owner_user_id", "TEXT"),
            ("storage_path", "TEXT"),
            ("content_type", "TEXT"),
            ("byte_size", "INTEGER"),
            ("preview_kind", "TEXT"),
            ("source_artifact_id", "TEXT"),
        ],
    )
}

pub(crate) fn ensure_knowledge_columns(connection: &Connection) -> Result<(), AppError> {
    ensure_columns(
        connection,
        "knowledge_records",
        &[
            ("scope", "TEXT"),
            ("visibility", "TEXT"),
            ("owner_user_id", "TEXT"),
        ],
    )?;

    connection
        .execute(
            "UPDATE knowledge_records
             SET scope = CASE
                 WHEN project_id IS NULL THEN 'workspace'
                 ELSE 'project'
             END
             WHERE scope IS NULL OR TRIM(scope) = ''",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute(
            "UPDATE knowledge_records
             SET visibility = CASE
                 WHEN COALESCE(scope, '') = 'personal' THEN 'private'
                 ELSE 'public'
             END
             WHERE visibility IS NULL OR TRIM(visibility) = ''",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(crate) fn backfill_project_resource_directories(
    connection: &Connection,
    paths: &WorkspacePaths,
) -> Result<(), AppError> {
    let mut stmt = connection
        .prepare("SELECT id, resource_directory FROM projects")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    for (project_id, stored_directory) in rows {
        let resource_directory = stored_directory
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| paths.default_project_resource_directory(&project_id));
        connection
            .execute(
                "UPDATE projects SET resource_directory = ?2 WHERE id = ?1",
                params![project_id, resource_directory],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        fs::create_dir_all(paths.root.join(&resource_directory))?;
    }

    fs::create_dir_all(&paths.workspace_resources_dir)?;
    Ok(())
}

pub(crate) fn backfill_project_governance(
    connection: &Connection,
    workspace_owner_user_id: Option<&str>,
) -> Result<(), AppError> {
    let resolved_workspace_owner_user_id = workspace_owner_user_id
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    let replace_bootstrap_placeholder = resolved_workspace_owner_user_id.is_some();
    let fallback_owner_user_id = resolved_workspace_owner_user_id
        .clone()
        .unwrap_or_else(|| BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID.to_string());
    let data_policies = load_data_policies(connection)?;
    let selected_project_members = data_policies
        .into_iter()
        .filter(|policy| {
            policy.subject_type == "user"
                && policy.resource_type == "project"
                && policy.scope_type == "selected-projects"
                && policy.effect == "allow"
        })
        .fold(
            std::collections::BTreeMap::<String, Vec<String>>::new(),
            |mut acc, policy| {
                for project_id in policy.project_ids {
                    acc.entry(project_id)
                        .or_default()
                        .push(policy.subject_id.clone());
                }
                acc
            },
        );

    let mut stmt = connection
        .prepare(
            "SELECT id, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json FROM projects",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    for (
        project_id,
        assignments_json,
        stored_owner_user_id,
        stored_member_user_ids_json,
        stored_permission_overrides_json,
        stored_linked_workspace_assets_json,
    ) in rows
    {
        let owner_user_id = stored_owner_user_id
            .filter(|value| !value.trim().is_empty())
            .filter(|value| {
                !(replace_bootstrap_placeholder && value == BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID)
            })
            .unwrap_or_else(|| fallback_owner_user_id.clone());
        let member_user_ids = stored_member_user_ids_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<Vec<String>>)
            .transpose()?
            .unwrap_or_else(|| {
                selected_project_members
                    .get(&project_id)
                    .cloned()
                    .unwrap_or_default()
            })
            .into_iter()
            .filter(|user_id| {
                !(replace_bootstrap_placeholder && user_id == BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID)
            })
            .collect::<Vec<_>>();
        let permission_overrides = stored_permission_overrides_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<ProjectPermissionOverrides>)
            .transpose()?
            .unwrap_or_else(default_project_permission_overrides);
        let linked_workspace_assets = stored_linked_workspace_assets_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<ProjectLinkedWorkspaceAssets>)
            .transpose()?
            .unwrap_or_else(|| {
                let assignments = assignments_json
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
                    .transpose()
                    .ok()
                    .flatten();
                ProjectLinkedWorkspaceAssets {
                    agent_ids: assignments
                        .as_ref()
                        .and_then(|value| value.agents.as_ref())
                        .map(|value| value.agent_ids.clone())
                        .unwrap_or_default(),
                    resource_ids: Vec::new(),
                    tool_source_keys: assignments
                        .as_ref()
                        .and_then(|value| value.tools.as_ref())
                        .map(|value| value.source_keys.clone())
                        .unwrap_or_default(),
                    knowledge_ids: Vec::new(),
                }
            });
        let normalized_members =
            normalized_project_member_user_ids(&owner_user_id, member_user_ids);

        connection
            .execute(
                "UPDATE projects
                 SET owner_user_id = ?2,
                     member_user_ids_json = ?3,
                     permission_overrides_json = ?4,
                     linked_workspace_assets_json = ?5
                 WHERE id = ?1",
                params![
                    project_id,
                    owner_user_id,
                    serde_json::to_string(&normalized_members)?,
                    serde_json::to_string(&permission_overrides)?,
                    serde_json::to_string(&linked_workspace_assets)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}

pub(crate) fn backfill_default_project_assignments(
    connection: &Connection,
) -> Result<(), AppError> {
    let stored_assignments_json = connection
        .query_row(
            "SELECT assignments_json FROM projects WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?
        .flatten();
    let parsed_assignments = stored_assignments_json
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
        .transpose()?;
    let needs_model_backfill = parsed_assignments
        .as_ref()
        .and_then(|assignments| assignments.models.as_ref())
        .is_none_or(|models| {
            models.default_configured_model_id.trim().is_empty()
                || models.configured_model_ids.is_empty()
        });
    if !needs_model_backfill {
        return Ok(());
    }

    let next_assignments = match parsed_assignments {
        Some(mut assignments) => {
            assignments.models = Some(default_project_model_assignments());
            assignments
        }
        None => default_project_assignments(),
    };

    connection
        .execute(
            "UPDATE projects SET assignments_json = ?2 WHERE id = ?1",
            params![
                DEFAULT_PROJECT_ID,
                serde_json::to_string(&next_assignments)?,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

fn drop_legacy_access_control_tables(connection: &Connection) -> Result<(), AppError> {
    for table in ["memberships", "roles", "permissions", "menus"] {
        connection
            .execute(&format!("DROP TABLE IF EXISTS {table}"), [])
            .map_err(|error| AppError::database(error.to_string()))?;
    }
    Ok(())
}

fn reset_legacy_sessions_table(connection: &Connection) -> Result<(), AppError> {
    let table_exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'sessions' LIMIT 1",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(|error| AppError::database(error.to_string()))?
        .is_some();
    if !table_exists {
        return Ok(());
    }

    let expected_columns = [
        "id",
        "workspace_id",
        "user_id",
        "client_app_id",
        "token",
        "status",
        "created_at",
        "expires_at",
    ];
    let mut pragma = connection
        .prepare("PRAGMA table_info(sessions)")
        .map_err(|error| AppError::database(error.to_string()))?;
    let columns = pragma
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))?;

    if columns != expected_columns {
        connection
            .execute("DROP TABLE sessions", [])
            .map_err(|error| AppError::database(error.to_string()))?;
    }

    Ok(())
}
