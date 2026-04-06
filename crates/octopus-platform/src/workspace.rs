use async_trait::async_trait;
use octopus_core::{AppError, ProjectRecord, SystemBootstrapStatus, WorkspaceSummary};

#[async_trait]
pub trait WorkspaceService: Send + Sync {
    async fn system_bootstrap(&self) -> Result<SystemBootstrapStatus, AppError>;
    async fn workspace_summary(&self) -> Result<WorkspaceSummary, AppError>;
    async fn list_projects(&self) -> Result<Vec<ProjectRecord>, AppError>;
}
