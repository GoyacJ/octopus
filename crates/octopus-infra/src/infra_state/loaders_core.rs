use super::*;

pub(crate) fn load_state(paths: WorkspacePaths) -> Result<InfraState, AppError> {
    let workspace_file: WorkspaceConfigFile =
        toml::from_str(&fs::read_to_string(&paths.workspace_config)?)?;
    let workspace_avatar_path = workspace_file.avatar_path.clone();
    let workspace_avatar_content_type = workspace_file.avatar_content_type.clone();
    let mut workspace = WorkspaceSummary {
        id: workspace_file.id,
        name: workspace_file.name,
        avatar: stored_avatar_data_url(
            &paths,
            workspace_file.avatar_path.as_deref(),
            workspace_file.avatar_content_type.as_deref(),
        ),
        slug: workspace_file.slug,
        deployment: workspace_file.deployment,
        bootstrap_status: workspace_file.bootstrap_status,
        owner_user_id: workspace_file.owner_user_id,
        host: workspace_file.host,
        listen_address: workspace_file.listen_address,
        default_project_id: workspace_file.default_project_id,
        mapped_directory: stored_mapped_directory(workspace_file.mapped_directory.as_deref()),
        mapped_directory_default: workspace_file
            .mapped_directory_default
            .filter(|value| !value.trim().is_empty())
            .or_else(|| Some(workspace_root_display_path(&paths))),
        project_default_permissions: workspace_file.project_default_permissions,
    };

    let app_registry: AppRegistryFile =
        toml::from_str(&fs::read_to_string(&paths.app_registry_config)?)?;
    let connection = crate::persistence::open_connection(&paths)?;
    ensure_default_owner_role_permissions(&connection)?;
    backfill_project_resource_directories(&connection, &paths)?;
    backfill_default_project_assignments(&connection)?;
    let users = load_users(&connection)?;
    let owner_user_id = users
        .iter()
        .find(|user| {
            resolve_effective_role_ids(&connection, &user.record.id)
                .map(|(role_ids, _)| {
                    role_ids
                        .iter()
                        .any(|role_id| role_id == SYSTEM_OWNER_ROLE_ID)
                })
                .unwrap_or(false)
        })
        .map(|user| user.record.id.clone());
    let expected_bootstrap_status = if owner_user_id.is_some() {
        "ready"
    } else {
        "setup_required"
    };
    let workspace_needs_normalize = workspace.bootstrap_status != expected_bootstrap_status
        || workspace.owner_user_id != owner_user_id;
    if workspace_needs_normalize {
        workspace.bootstrap_status = expected_bootstrap_status.into();
        workspace.owner_user_id = owner_user_id;
        bootstrap::save_workspace_config_file(
            &paths.workspace_config,
            &workspace,
            workspace_avatar_path.as_deref(),
            workspace_avatar_content_type.as_deref(),
        )?;
    }
    backfill_project_governance(&connection, workspace.owner_user_id.as_deref())?;
    for user in &users {
        ensure_personal_pet_for_user(&connection, &workspace.id, &user.record.id)?;
    }
    let projects = load_projects(&connection)?;
    let project_promotion_requests = load_project_promotion_requests(&connection)?;
    let project_deletion_requests = load_project_deletion_requests(&connection)?;
    let sessions = load_sessions(&connection)?;
    let resources = load_resources(&connection)?;
    let knowledge_records = load_knowledge_records(&connection)?;
    let project_tasks = load_project_tasks(&connection)?;
    let project_task_runs = load_project_task_runs(&connection)?;
    let project_task_interventions = load_project_task_interventions(&connection)?;
    let project_task_scheduler_claims = load_project_task_scheduler_claims(&connection)?;
    let artifacts = load_artifact_records(&connection)?;
    let agents = load_agents(&connection)?;
    let project_agent_links = load_project_agent_links(&connection)?;
    let teams = load_teams(&connection)?;
    let project_team_links = load_project_team_links(&connection)?;
    let model_catalog = load_model_catalog(&connection)?;
    let provider_credentials = load_provider_credentials(&connection)?;
    let tools = load_tools(&connection)?;
    let trace_events = load_trace_events(&connection)?;
    let audit_records = load_audit_records(&connection)?;
    let cost_entries = load_cost_entries(&connection)?;
    let pet_extensions = load_pet_agent_extensions(&connection)?;
    let pet_presences = load_pet_presences(&connection)?;
    let pet_bindings = load_pet_bindings(&connection)?;

    Ok(InfraState {
        paths,
        workspace: Mutex::new(workspace),
        workspace_avatar_path: Mutex::new(workspace_avatar_path),
        workspace_avatar_content_type: Mutex::new(workspace_avatar_content_type),
        users: Mutex::new(users),
        apps: Mutex::new(app_registry.apps),
        sessions: Mutex::new(sessions),
        projects: Mutex::new(projects),
        project_promotion_requests: Mutex::new(project_promotion_requests),
        project_deletion_requests: Mutex::new(project_deletion_requests),
        resources: Mutex::new(resources),
        knowledge_records: Mutex::new(knowledge_records),
        project_tasks: Mutex::new(project_tasks),
        project_task_runs: Mutex::new(project_task_runs),
        project_task_interventions: Mutex::new(project_task_interventions),
        project_task_scheduler_claims: Mutex::new(project_task_scheduler_claims),
        agents: Mutex::new(agents),
        project_agent_links: Mutex::new(project_agent_links),
        teams: Mutex::new(teams),
        project_team_links: Mutex::new(project_team_links),
        model_catalog: Mutex::new(model_catalog),
        provider_credentials: Mutex::new(provider_credentials),
        tools: Mutex::new(tools),
        artifacts: Mutex::new(artifacts),
        inbox: Mutex::new(Vec::new()),
        trace_events: Mutex::new(trace_events),
        audit_records: Mutex::new(audit_records),
        cost_entries: Mutex::new(cost_entries),
        pet_extensions: Mutex::new(pet_extensions),
        pet_presences: Mutex::new(pet_presences),
        pet_bindings: Mutex::new(pet_bindings),
    })
}

pub(crate) fn load_users(connection: &Connection) -> Result<Vec<StoredUser>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, username, display_name, avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash,
                    status, password_hash, password_state, created_at, updated_at
             FROM users",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StoredUser {
                record: UserRecord {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    avatar_path: row.get(3)?,
                    avatar_content_type: row.get(4)?,
                    avatar_byte_size: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
                    avatar_content_hash: row.get(6)?,
                    status: row.get(7)?,
                    password_state: row.get(9)?,
                    created_at: row.get::<_, i64>(10)? as u64,
                    updated_at: row.get::<_, i64>(11)? as u64,
                },
                password_hash: row.get(8)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}
