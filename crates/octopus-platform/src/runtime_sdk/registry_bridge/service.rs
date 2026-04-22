use async_trait::async_trait;
use octopus_core::{AppError, ModelCatalogSnapshot};

use crate::runtime::{ModelRegistryService, RuntimeConfigService};
use crate::runtime_sdk::RuntimeSdkBridge;

use super::build_catalog_snapshot;

#[async_trait]
impl ModelRegistryService for RuntimeSdkBridge {
    async fn catalog_snapshot(&self) -> Result<ModelCatalogSnapshot, AppError> {
        let effective = self.get_config().await?;
        build_catalog_snapshot(self, &effective.effective_config)
    }
}
