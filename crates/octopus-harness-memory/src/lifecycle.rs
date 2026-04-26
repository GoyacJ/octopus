use async_trait::async_trait;
use harness_contracts::{
    ContentHash, MemoryError, MemorySessionCtx, MemoryWriteAction, MemoryWriteTarget, MessageView,
    SessionId, SessionSummaryView, UserMessageView,
};

use crate::MemoryStore;

#[async_trait]
pub trait MemoryLifecycle: Send + Sync + 'static {
    async fn initialize(&self, ctx: &MemorySessionCtx<'_>) -> Result<(), MemoryError> {
        let _ = ctx;
        Ok(())
    }

    async fn on_turn_start(
        &self,
        turn: u32,
        message: &UserMessageView<'_>,
    ) -> Result<(), MemoryError> {
        let _ = (turn, message);
        Ok(())
    }

    async fn on_pre_compress(
        &self,
        messages: &[MessageView<'_>],
    ) -> Result<Option<String>, MemoryError> {
        let _ = messages;
        Ok(None)
    }

    async fn on_memory_write(
        &self,
        action: MemoryWriteAction,
        target: &MemoryWriteTarget,
        content_hash: ContentHash,
    ) -> Result<(), MemoryError> {
        let _ = (action, target, content_hash);
        Ok(())
    }

    async fn on_delegation(
        &self,
        task: &str,
        result: &str,
        child_session: SessionId,
    ) -> Result<(), MemoryError> {
        let _ = (task, result, child_session);
        Ok(())
    }

    async fn on_session_end(
        &self,
        ctx: &MemorySessionCtx<'_>,
        summary: &SessionSummaryView<'_>,
    ) -> Result<(), MemoryError> {
        let _ = (ctx, summary);
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), MemoryError> {
        Ok(())
    }
}

pub trait MemoryProvider: MemoryStore + MemoryLifecycle {}

impl<T: MemoryStore + MemoryLifecycle> MemoryProvider for T {}
