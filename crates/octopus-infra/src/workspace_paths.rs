use std::{fs, path::PathBuf};

use octopus_core::AppError;

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub workspace_config: PathBuf,
    pub app_registry_config: PathBuf,
    pub runtime_config_dir: PathBuf,
    pub runtime_project_config_dir: PathBuf,
    pub runtime_user_config_dir: PathBuf,
    pub db_path: PathBuf,
    pub blobs_dir: PathBuf,
    pub user_avatars_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub knowledge_dir: PathBuf,
    pub inbox_dir: PathBuf,
    pub managed_skills_dir: PathBuf,
    pub project_data_dir: PathBuf,
    pub runtime_sessions_dir: PathBuf,
    pub runtime_events_dir: PathBuf,
    pub runtime_traces_dir: PathBuf,
    pub runtime_approvals_dir: PathBuf,
    pub runtime_cache_dir: PathBuf,
    pub audit_log_dir: PathBuf,
    pub server_log_dir: PathBuf,
}

impl WorkspacePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let config_dir = root.join("config");
        let data_dir = root.join("data");
        let runtime_dir = root.join("runtime");
        let logs_dir = root.join("logs");
        let tmp_dir = root.join("tmp");
        let runtime_config_dir = config_dir.join("runtime");
        let runtime_project_config_dir = runtime_config_dir.join("projects");
        let runtime_user_config_dir = runtime_config_dir.join("users");
        let blobs_dir = data_dir.join("blobs");
        let user_avatars_dir = blobs_dir.join("avatars");
        let artifacts_dir = data_dir.join("artifacts");
        let knowledge_dir = data_dir.join("knowledge");
        let inbox_dir = data_dir.join("inbox");
        let managed_skills_dir = data_dir.join("skills");
        let project_data_dir = data_dir.join("projects");
        let runtime_sessions_dir = runtime_dir.join("sessions");
        let runtime_events_dir = runtime_dir.join("events");
        let runtime_traces_dir = runtime_dir.join("traces");
        let runtime_approvals_dir = runtime_dir.join("approvals");
        let runtime_cache_dir = runtime_dir.join("cache");
        let audit_log_dir = logs_dir.join("audit");
        let server_log_dir = logs_dir.join("server");

        Self {
            workspace_config: config_dir.join("workspace.toml"),
            app_registry_config: config_dir.join("app-registry.toml"),
            runtime_config_dir,
            runtime_project_config_dir,
            runtime_user_config_dir,
            db_path: data_dir.join("main.db"),
            root,
            config_dir,
            data_dir,
            runtime_dir,
            logs_dir,
            tmp_dir,
            blobs_dir,
            user_avatars_dir,
            artifacts_dir,
            knowledge_dir,
            inbox_dir,
            managed_skills_dir,
            project_data_dir,
            runtime_sessions_dir,
            runtime_events_dir,
            runtime_traces_dir,
            runtime_approvals_dir,
            runtime_cache_dir,
            audit_log_dir,
            server_log_dir,
        }
    }

    pub fn ensure_layout(&self) -> Result<(), AppError> {
        for path in [
            &self.root,
            &self.config_dir,
            &self.runtime_config_dir,
            &self.runtime_project_config_dir,
            &self.runtime_user_config_dir,
            &self.data_dir,
            &self.runtime_dir,
            &self.logs_dir,
            &self.tmp_dir,
            &self.blobs_dir,
            &self.user_avatars_dir,
            &self.artifacts_dir,
            &self.knowledge_dir,
            &self.inbox_dir,
            &self.managed_skills_dir,
            &self.project_data_dir,
            &self.runtime_sessions_dir,
            &self.runtime_events_dir,
            &self.runtime_traces_dir,
            &self.runtime_approvals_dir,
            &self.runtime_cache_dir,
            &self.audit_log_dir,
            &self.server_log_dir,
        ] {
            fs::create_dir_all(path)?;
        }

        Ok(())
    }

    pub fn project_dir(&self, project_id: &str) -> PathBuf {
        self.project_data_dir.join(project_id)
    }

    pub fn project_skills_root(&self, project_id: &str) -> PathBuf {
        self.project_dir(project_id).join("skills")
    }
}
