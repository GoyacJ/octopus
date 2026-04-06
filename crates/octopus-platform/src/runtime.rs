use async_trait::async_trait;
use octopus_core::{
    AppError, CreateRuntimeSessionInput, ResolveRuntimeApprovalInput, RuntimeBootstrap,
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
    ) -> Result<RuntimeSessionDetail, AppError>;
    async fn get_session(&self, session_id: &str) -> Result<RuntimeSessionDetail, AppError>;
    async fn list_events(
        &self,
        session_id: &str,
        after: Option<&str>,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError>;
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
pub trait ToolExecutionService: Send + Sync {}

#[async_trait]
pub trait AutomationService: Send + Sync {}

#[async_trait]
pub trait RuntimeProjectionService: Send + Sync {}
