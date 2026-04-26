use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use harness_contracts::ModelError;

use crate::{ModelProvider, ModelRequest};

#[async_trait]
pub trait AuxModelProvider: Send + Sync + 'static {
    fn inner(&self) -> Arc<dyn ModelProvider>;
    fn aux_options(&self) -> AuxOptions;

    async fn call_aux(&self, task: AuxTask, req: ModelRequest) -> Result<String, ModelError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxTask {
    Compact,
    Summarize,
    Classify,
    PermissionAdvisory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxOptions {
    pub max_concurrency: usize,
    pub per_task_timeout: Duration,
    pub fail_open: bool,
}

impl Default for AuxOptions {
    fn default() -> Self {
        Self {
            max_concurrency: 4,
            per_task_timeout: Duration::from_secs(30),
            fail_open: true,
        }
    }
}
