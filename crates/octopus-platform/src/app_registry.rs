use async_trait::async_trait;
use octopus_core::{AppError, ClientAppRecord};

#[async_trait]
pub trait AppRegistryService: Send + Sync {
    async fn list_apps(&self) -> Result<Vec<ClientAppRecord>, AppError>;
    async fn register_app(
        &self,
        record: ClientAppRecord,
    ) -> Result<ClientAppRecord, AppError>;
    async fn find_app(&self, app_id: &str) -> Result<Option<ClientAppRecord>, AppError>;
}
