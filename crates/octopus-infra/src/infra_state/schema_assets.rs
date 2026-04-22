use super::*;

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

pub(crate) fn ensure_pet_projection_columns(connection: &Connection) -> Result<(), AppError> {
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
    )?;

    Ok(())
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

pub(crate) fn write_agent_record(
    connection: &Connection,
    record: &AgentRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };

    let sql = format!(
        "{verb} INTO agents (
            id, workspace_id, project_id, scope, owner_user_id, asset_role, name, avatar_path, personality, tags, prompt,
            builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
            default_model_strategy_json, capability_policy_json, permission_envelope_json,
            memory_policy_json, delegation_policy_json, approval_preference_json,
            output_contract_json, shared_capability_policy_json, integration_source_json,
            trust_metadata_json, dependency_resolution_json, import_metadata_json,
            description, status, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16,
            ?17, ?18, ?19,
            ?20, ?21, ?22,
            ?23, ?24, ?25,
            ?26, ?27, ?28,
            ?29, ?30, ?31
        )"
    );

    connection
        .execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.owner_user_id,
                record.asset_role,
                record.name,
                record.avatar_path,
                record.personality,
                json_string(&record.tags)?,
                record.prompt,
                json_string(&record.builtin_tool_keys)?,
                json_string(&record.skill_ids)?,
                json_string(&record.mcp_server_names)?,
                json_string(&record.task_domains)?,
                record.manifest_revision,
                json_string(&record.default_model_strategy)?,
                json_string(&record.capability_policy)?,
                json_string(&record.permission_envelope)?,
                json_string(&record.memory_policy)?,
                json_string(&record.delegation_policy)?,
                json_string(&record.approval_preference)?,
                json_string(&record.output_contract)?,
                json_string(&record.shared_capability_policy)?,
                record
                    .integration_source
                    .as_ref()
                    .map(json_string)
                    .transpose()?,
                json_string(&record.trust_metadata)?,
                json_string(&record.dependency_resolution)?,
                json_string(&record.import_metadata)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(crate) fn write_team_record(
    connection: &Connection,
    record: &TeamRecord,
    replace: bool,
) -> Result<(), AppError> {
    let member_refs_json = json_string(&record.member_refs)?;
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };

    let sql = format!(
        "{verb} INTO teams (
            id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt,
            builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
            default_model_strategy_json, capability_policy_json, permission_envelope_json,
            memory_policy_json, delegation_policy_json, approval_preference_json,
            output_contract_json, shared_capability_policy_json, leader_ref, member_refs,
            team_topology_json, shared_memory_policy_json, mailbox_policy_json,
            artifact_handoff_policy_json, workflow_affordance_json, worker_concurrency_limit,
            integration_source_json, trust_metadata_json, dependency_resolution_json,
            import_metadata_json, description, status, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17,
            ?18, ?19, ?20,
            ?21, ?22, ?23, ?24,
            ?25, ?26, ?27,
            ?28, ?29, ?30,
            ?31, ?32, ?33,
            ?34, ?35, ?36, ?37
        )"
    );

    connection
        .execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                json_string(&record.task_domains)?,
                record.manifest_revision,
                json_string(&record.default_model_strategy)?,
                json_string(&record.capability_policy)?,
                json_string(&record.permission_envelope)?,
                json_string(&record.memory_policy)?,
                json_string(&record.delegation_policy)?,
                json_string(&record.approval_preference)?,
                json_string(&record.output_contract)?,
                json_string(&record.shared_capability_policy)?,
                record.leader_ref,
                member_refs_json,
                json_string(&record.team_topology)?,
                json_string(&record.shared_memory_policy)?,
                json_string(&record.mailbox_policy)?,
                json_string(&record.artifact_handoff_policy)?,
                json_string(&record.workflow_affordance)?,
                record.worker_concurrency_limit as i64,
                record
                    .integration_source
                    .as_ref()
                    .map(json_string)
                    .transpose()?,
                json_string(&record.trust_metadata)?,
                json_string(&record.dependency_resolution)?,
                json_string(&record.import_metadata)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(crate) fn write_bundle_asset_descriptor_record(
    connection: &Connection,
    record: &BundleAssetDescriptorRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    let sql = format!(
        "{verb} INTO bundle_asset_descriptors (
            id, workspace_id, project_id, scope, asset_kind, source_id, display_name, source_path,
            storage_path, content_hash, byte_size, manifest_revision, task_domains_json,
            translation_mode, trust_metadata_json, dependency_resolution_json,
            import_metadata_json, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
            ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16,
            ?17, ?18
        )"
    );

    connection
        .execute(
            &sql,
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.asset_kind,
                record.source_id,
                record.display_name,
                record.source_path,
                record.storage_path,
                record.content_hash,
                record.byte_size as i64,
                record.manifest_revision,
                json_string(&record.task_domains)?,
                record.translation_mode,
                json_string(&record.trust_metadata)?,
                json_string(&record.dependency_resolution)?,
                json_string(&record.import_metadata)?,
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}
