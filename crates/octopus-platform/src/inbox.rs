use async_trait::async_trait;
use octopus_core::{AppError, InboxItemRecord};

#[async_trait]
pub trait InboxService: Send + Sync {
    async fn list_inbox(&self) -> Result<Vec<InboxItemRecord>, AppError>;
}
