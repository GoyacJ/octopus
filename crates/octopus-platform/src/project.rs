use async_trait::async_trait;
use octopus_core::{
    AppError, CreateTaskInterventionRequest, CreateTaskRequest, ProjectTaskInterventionRecord,
    ProjectTaskRecord, ProjectTaskRunRecord, UpdateTaskRequest,
};

#[async_trait]
pub trait ProjectTaskService: Send + Sync {
    async fn list_tasks(&self, project_id: &str) -> Result<Vec<ProjectTaskRecord>, AppError>;
    async fn create_task(
        &self,
        project_id: &str,
        user_id: &str,
        request: CreateTaskRequest,
    ) -> Result<ProjectTaskRecord, AppError>;
    async fn get_task(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<ProjectTaskRecord, AppError>;
    async fn update_task(
        &self,
        project_id: &str,
        task_id: &str,
        user_id: &str,
        request: UpdateTaskRequest,
    ) -> Result<ProjectTaskRecord, AppError>;
    async fn save_task(&self, record: ProjectTaskRecord) -> Result<ProjectTaskRecord, AppError>;
    async fn list_task_runs(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<Vec<ProjectTaskRunRecord>, AppError>;
    async fn save_task_run(
        &self,
        record: ProjectTaskRunRecord,
    ) -> Result<ProjectTaskRunRecord, AppError>;
    async fn list_task_interventions(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<Vec<ProjectTaskInterventionRecord>, AppError>;
    async fn create_task_intervention(
        &self,
        project_id: &str,
        task_id: &str,
        user_id: &str,
        request: CreateTaskInterventionRequest,
    ) -> Result<ProjectTaskInterventionRecord, AppError>;
}
