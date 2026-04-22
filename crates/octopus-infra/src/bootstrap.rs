use super::*;

pub fn initialize_workspace(root: impl Into<PathBuf>) -> Result<WorkspacePaths, AppError> {
    let paths = WorkspacePaths::new(root);
    paths.ensure_layout()?;
    initialize_workspace_config(&paths)?;
    initialize_app_registry(&paths)?;
    let database = open_workspace_database(&paths)?;
    initialize_database(&database)?;
    seed_defaults(&database, &paths)?;
    Ok(paths)
}

pub fn build_infra_bundle(root: impl Into<PathBuf>) -> Result<InfraBundle, AppError> {
    let paths = initialize_workspace(root)?;
    let database = open_workspace_database(&paths)?;
    let state = Arc::new(load_state(paths.clone(), database)?);

    Ok(InfraBundle {
        paths: paths.clone(),
        workspace: Arc::new(InfraWorkspaceService {
            state: Arc::clone(&state),
        }),
        access_control: Arc::new(InfraAccessControlService {
            state: Arc::clone(&state),
        }),
        auth: Arc::new(InfraAuthService {
            state: Arc::clone(&state),
        }),
        app_registry: Arc::new(InfraAppRegistryService {
            state: Arc::clone(&state),
        }),
        authorization: Arc::new(InfraAuthorizationService {
            _state: Arc::clone(&state),
        }),
        artifact: Arc::new(InfraArtifactService {
            state: Arc::clone(&state),
        }),
        inbox: Arc::new(InfraInboxService {
            state: Arc::clone(&state),
        }),
        knowledge: Arc::new(InfraKnowledgeService {
            state: Arc::clone(&state),
        }),
        observation: Arc::new(InfraObservationService { state }),
    })
}

pub(super) fn save_workspace_config_file(
    path: &Path,
    workspace: &WorkspaceSummary,
    avatar_path: Option<&str>,
    avatar_content_type: Option<&str>,
) -> Result<(), AppError> {
    let config = WorkspaceConfigFile {
        id: workspace.id.clone(),
        name: workspace.name.clone(),
        avatar_path: avatar_path.map(str::to_string),
        avatar_content_type: avatar_content_type.map(str::to_string),
        slug: workspace.slug.clone(),
        deployment: workspace.deployment.clone(),
        bootstrap_status: workspace.bootstrap_status.clone(),
        owner_user_id: workspace.owner_user_id.clone(),
        host: workspace.host.clone(),
        listen_address: workspace.listen_address.clone(),
        default_project_id: workspace.default_project_id.clone(),
        mapped_directory: workspace.mapped_directory.clone(),
        mapped_directory_default: workspace.mapped_directory_default.clone(),
        project_default_permissions: workspace.project_default_permissions.clone(),
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string_pretty(&config)?)?;
    Ok(())
}

pub(super) fn relocate_workspace_root(
    current_root: &Path,
    target_root: &Path,
    shell_root: &Path,
    workspace: &WorkspaceSummary,
    avatar_path: Option<&str>,
    avatar_content_type: Option<&str>,
) -> Result<(), AppError> {
    if current_root == target_root {
        save_workspace_config_file(
            &WorkspacePaths::new(current_root).workspace_config,
            workspace,
            avatar_path,
            avatar_content_type,
        )?;
        save_workspace_config_file(
            &WorkspacePaths::new(shell_root).workspace_config,
            workspace,
            avatar_path,
            avatar_content_type,
        )?;
        return Ok(());
    }

    if target_root.starts_with(current_root) || current_root.starts_with(target_root) {
        return Err(AppError::invalid_input(
            "mapped directory must not contain the current workspace root or be nested inside it",
        ));
    }

    if target_root.exists() {
        let mut entries = fs::read_dir(target_root)?;
        if entries.next().transpose()?.is_some() {
            return Err(AppError::invalid_input(
                "mapped directory must point to a missing or empty directory",
            ));
        }
        fs::remove_dir(target_root)?;
    }

    if let Some(parent) = target_root.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(current_root, target_root).map_err(|error| {
        if matches!(error.kind(), std::io::ErrorKind::CrossesDevices) {
            AppError::invalid_input(
                "mapped directory must stay on the same filesystem as the current workspace root",
            )
        } else {
            AppError::runtime(format!(
                "failed to move workspace root from {} to {}: {error}",
                current_root.display(),
                target_root.display(),
            ))
        }
    })?;

    let target_paths = WorkspacePaths::new(target_root);
    save_workspace_config_file(
        &target_paths.workspace_config,
        workspace,
        avatar_path,
        avatar_content_type,
    )?;

    let shell_paths = WorkspacePaths::new(shell_root);
    save_workspace_config_file(
        &shell_paths.workspace_config,
        workspace,
        avatar_path,
        avatar_content_type,
    )?;

    Ok(())
}
