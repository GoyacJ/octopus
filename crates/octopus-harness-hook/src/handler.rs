use async_trait::async_trait;
use harness_contracts::{HookError, HookEventKind, HookFailureMode};

use crate::{HookContext, HookEvent, HookOutcome};

#[async_trait]
pub trait HookHandler: Send + Sync + 'static {
    fn handler_id(&self) -> &str;

    fn interested_events(&self) -> &[HookEventKind];

    fn priority(&self) -> i32 {
        0
    }

    fn failure_mode(&self) -> HookFailureMode {
        HookFailureMode::FailOpen
    }

    async fn handle(&self, event: HookEvent, ctx: HookContext) -> Result<HookOutcome, HookError>;
}
