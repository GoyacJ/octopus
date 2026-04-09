use super::*;

pub fn initialize_workspace(root: impl Into<PathBuf>) -> Result<WorkspacePaths, AppError> {
    let paths = WorkspacePaths::new(root);
    paths.ensure_layout()?;
    initialize_workspace_config(&paths)?;
    initialize_app_registry(&paths)?;
    initialize_database(&paths)?;
    seed_defaults(&paths)?;
    Ok(paths)
}

pub fn build_infra_bundle(root: impl Into<PathBuf>) -> Result<InfraBundle, AppError> {
    let paths = initialize_workspace(root)?;
    let state = Arc::new(load_state(paths.clone())?);

    Ok(InfraBundle {
        paths: paths.clone(),
        workspace: Arc::new(InfraWorkspaceService {
            state: Arc::clone(&state),
        }),
        auth: Arc::new(InfraAuthService {
            state: Arc::clone(&state),
        }),
        app_registry: Arc::new(InfraAppRegistryService {
            state: Arc::clone(&state),
        }),
        rbac: Arc::new(InfraRbacService {
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
) -> Result<(), AppError> {
    let config = WorkspaceConfigFile {
        id: workspace.id.clone(),
        name: workspace.name.clone(),
        slug: workspace.slug.clone(),
        deployment: workspace.deployment.clone(),
        bootstrap_status: workspace.bootstrap_status.clone(),
        owner_user_id: workspace.owner_user_id.clone(),
        host: workspace.host.clone(),
        listen_address: workspace.listen_address.clone(),
        default_project_id: workspace.default_project_id.clone(),
    };
    fs::write(path, toml::to_string_pretty(&config)?)?;
    Ok(())
}
