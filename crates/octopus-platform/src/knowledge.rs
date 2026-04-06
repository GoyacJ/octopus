use async_trait::async_trait;
use octopus_core::{AppError, KnowledgeEntryRecord};

#[async_trait]
pub trait KnowledgeService: Send + Sync {
    async fn list_knowledge(&self) -> Result<Vec<KnowledgeEntryRecord>, AppError>;
}
