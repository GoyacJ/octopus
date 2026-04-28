use harness_contracts::{RunId, SessionId, TenantId, TurnInput};

use crate::CancellationToken;

pub type EngineError = harness_contracts::EngineError;
pub type EventStream = harness_journal::EventStream;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EngineId(String);

impl EngineId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EngineId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SessionHandle {
    pub tenant_id: TenantId,
    pub session_id: SessionId,
}

#[derive(Clone)]
pub struct RunContext {
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub run_id: RunId,
    pub cancellation: CancellationToken,
}

impl RunContext {
    #[must_use]
    pub fn new(tenant_id: TenantId, session_id: SessionId, run_id: RunId) -> Self {
        Self {
            tenant_id,
            session_id,
            run_id,
            cancellation: CancellationToken::new(),
        }
    }

    #[must_use]
    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }
}

#[async_trait::async_trait]
pub trait EngineRunner: Send + Sync + 'static {
    async fn run(
        &self,
        session: SessionHandle,
        input: TurnInput,
        ctx: RunContext,
    ) -> Result<EventStream, EngineError>;

    fn engine_id(&self) -> EngineId;
}
