use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WorkspaceConfigFile {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) avatar_path: Option<String>,
    #[serde(default)]
    pub(crate) avatar_content_type: Option<String>,
    pub(crate) slug: String,
    pub(crate) deployment: String,
    pub(crate) bootstrap_status: String,
    pub(crate) owner_user_id: Option<String>,
    pub(crate) host: String,
    pub(crate) listen_address: String,
    pub(crate) default_project_id: String,
    #[serde(default)]
    pub(crate) mapped_directory: Option<String>,
    #[serde(default)]
    pub(crate) mapped_directory_default: Option<String>,
    #[serde(default = "default_project_default_permissions")]
    pub(crate) project_default_permissions: ProjectDefaultPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppRegistryFile {
    pub(crate) apps: Vec<ClientAppRecord>,
}

pub(crate) fn initialize_workspace_config(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.workspace_config.exists() {
        return Ok(());
    }

    let config = WorkspaceConfigFile {
        id: DEFAULT_WORKSPACE_ID.into(),
        name: "Octopus Local Workspace".into(),
        avatar_path: None,
        avatar_content_type: None,
        slug: "local-workspace".into(),
        deployment: "local".into(),
        bootstrap_status: "setup_required".into(),
        owner_user_id: None,
        host: "127.0.0.1".into(),
        listen_address: "127.0.0.1".into(),
        default_project_id: DEFAULT_PROJECT_ID.into(),
        mapped_directory: None,
        mapped_directory_default: Some(workspace_root_display_path(paths)),
        project_default_permissions: ProjectDefaultPermissions {
            agents: "allow".into(),
            resources: "allow".into(),
            tools: "allow".into(),
            knowledge: "allow".into(),
            tasks: "allow".into(),
        },
    };
    fs::write(&paths.workspace_config, toml::to_string_pretty(&config)?)?;
    Ok(())
}

pub(crate) fn initialize_app_registry(paths: &WorkspacePaths) -> Result<(), AppError> {
    if paths.app_registry_config.exists() {
        return Ok(());
    }

    let registry = AppRegistryFile {
        apps: default_client_apps(),
    };
    fs::write(
        &paths.app_registry_config,
        toml::to_string_pretty(&registry)?,
    )?;
    Ok(())
}
