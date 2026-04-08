use async_trait::async_trait;
use octopus_core::{
    AppError, CreateRuntimeSessionInput, ModelCatalogSnapshot, ResolveRuntimeApprovalInput,
    RuntimeBootstrap, RuntimeConfigPatch, RuntimeConfigValidationResult,
    RuntimeConfiguredModelProbeInput, RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig,
    RuntimeEventEnvelope, RuntimeRunSnapshot, RuntimeSessionDetail, RuntimeSessionSummary,
    SubmitRuntimeTurnInput,
};

#[async_trait]
pub trait RuntimeSessionService: Send + Sync {
    async fn bootstrap(&self) -> Result<RuntimeBootstrap, AppError>;
    async fn list_sessions(&self) -> Result<Vec<RuntimeSessionSummary>, AppError>;
    async fn create_session(
        &self,
        input: CreateRuntimeSessionInput,
        user_id: &str,
    ) -> Result<RuntimeSessionDetail, AppError>;
    async fn get_session(&self, session_id: &str) -> Result<RuntimeSessionDetail, AppError>;
    async fn list_events(
        &self,
        session_id: &str,
        after: Option<&str>,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait RuntimeExecutionService: Send + Sync {
    async fn submit_turn(
        &self,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError>;
    async fn resolve_approval(
        &self,
        session_id: &str,
        approval_id: &str,
        input: ResolveRuntimeApprovalInput,
    ) -> Result<RuntimeRunSnapshot, AppError>;
    async fn subscribe_events(
        &self,
        session_id: &str,
    ) -> Result<tokio::sync::broadcast::Receiver<RuntimeEventEnvelope>, AppError>;
}

#[async_trait]
pub trait RuntimeConfigService: Send + Sync {
    async fn get_config(&self) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn get_project_config(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn get_user_config(&self, user_id: &str) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn validate_config(
        &self,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError>;
    async fn validate_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError>;
    async fn validate_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError>;
    async fn probe_configured_model(
        &self,
        input: RuntimeConfiguredModelProbeInput,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError>;
    async fn save_config(
        &self,
        scope: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn save_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError>;
    async fn save_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError>;
}

#[async_trait]
pub trait ModelRegistryService: Send + Sync {
    async fn catalog_snapshot(&self) -> Result<ModelCatalogSnapshot, AppError>;
}

#[async_trait]
pub trait ToolExecutionService: Send + Sync {}

#[async_trait]
pub trait AutomationService: Send + Sync {}

#[async_trait]
pub trait RuntimeProjectionService: Send + Sync {}
