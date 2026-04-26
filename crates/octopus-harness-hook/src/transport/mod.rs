#[cfg(feature = "in-process")]
mod in_process;

use async_trait::async_trait;
use harness_contracts::HookError;

use crate::{HookContext, HookEvent, HookOutcome};

#[derive(Clone)]
pub struct HookPayload {
    pub event: HookEvent,
    pub ctx: HookContext,
}

pub type HookOutput = Result<HookOutcome, HookError>;

#[async_trait]
pub trait HookTransport: Send + Sync + 'static {
    async fn invoke(&self, payload: HookPayload) -> HookOutput;
}

#[cfg(feature = "in-process")]
pub use in_process::*;
