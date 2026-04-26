//! Tool capability marker traits.
//!
//! SPEC: docs/architecture/harness/crates/harness-contracts.md §3.4

use std::{any::Any, collections::HashMap, sync::Arc};

use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ToolCapability, ToolError};

pub trait SubagentRunnerCap: Send + Sync + 'static {}
pub trait TodoStoreCap: Send + Sync + 'static {}
pub trait RunCancellerCap: Send + Sync + 'static {}
pub trait ClarifyChannelCap: Send + Sync + 'static {
    fn ask(&self, prompt: ClarifyPrompt) -> BoxFuture<'static, Result<ClarifyAnswer, ToolError>>;
}

pub trait UserMessengerCap: Send + Sync + 'static {
    fn send(
        &self,
        message: OutboundUserMessage,
    ) -> BoxFuture<'static, Result<UserMessageDelivery, ToolError>>;
}
pub trait BlobReaderCap: Send + Sync + 'static {}
pub trait HookEmitterCap: Send + Sync + 'static {}
pub trait SkillRegistryCap: Send + Sync + 'static {}
pub trait EmbeddedToolDispatcherCap: Send + Sync + 'static {}
pub trait CodeRuntimeCap: Send + Sync + 'static {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClarifyPrompt {
    pub prompt: String,
    pub choices: Vec<ClarifyChoice>,
    pub multiple: bool,
    pub timeout_seconds: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClarifyChoice {
    pub id: String,
    pub label: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClarifyAnswer {
    pub answer: String,
    pub chosen_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OutboundUserMessage {
    pub channel: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UserMessageDelivery {
    pub message_id: String,
    pub delivered: bool,
}

#[derive(Clone, Default)]
pub struct CapabilityRegistry {
    inner: HashMap<ToolCapability, Arc<dyn Any + Send + Sync>>,
}

impl CapabilityRegistry {
    pub fn install<T>(&mut self, capability: ToolCapability, implementation: Arc<T>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.inner.insert(capability, Arc::new(implementation));
    }

    pub fn get<T>(&self, capability: &ToolCapability) -> Option<Arc<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let erased = Arc::clone(self.inner.get(capability)?);
        erased
            .downcast::<Arc<T>>()
            .ok()
            .map(|typed| Arc::clone(typed.as_ref()))
    }
}
