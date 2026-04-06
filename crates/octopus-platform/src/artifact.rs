use async_trait::async_trait;
use octopus_core::{AppError, ArtifactRecord};

#[async_trait]
pub trait ArtifactService: Send + Sync {
    async fn list_artifacts(&self) -> Result<Vec<ArtifactRecord>, AppError>;
}
