use super::*;

pub(crate) fn apply_core_schema_batch(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            r"
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
            ",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}
