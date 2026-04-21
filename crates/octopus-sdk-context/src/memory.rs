use async_trait::async_trait;
use octopus_sdk_contracts::{MemoryError, MemoryItem};

#[async_trait]
pub trait MemoryBackend: Send + Sync {
    async fn recall(&self, query: &str) -> Result<Vec<MemoryItem>, MemoryError>;

    async fn commit(&self, item: MemoryItem) -> Result<(), MemoryError>;
}
