//! Tool capability marker traits.
//!
//! SPEC: docs/architecture/harness/crates/harness-contracts.md §3.4

use std::{any::Any, collections::HashMap, sync::Arc};

use futures::future::BoxFuture;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::{AgentId, SkillId, SkillSourceKind, ToolCapability, ToolError};

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
pub trait SkillRegistryCap: Send + Sync + 'static {
    fn list_summaries(&self, agent: &AgentId, filter: SkillFilter) -> Vec<SkillSummary>;

    fn view(&self, agent: &AgentId, name: &str, full: bool) -> Option<SkillView>;

    fn render(
        &self,
        agent: &AgentId,
        name: String,
        params: Value,
    ) -> BoxFuture<'static, Result<RenderedSkill, ToolError>>;
}
pub trait EmbeddedToolDispatcherCap: Send + Sync + 'static {}
pub trait CodeRuntimeCap: Send + Sync + 'static {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Ready,
    PrerequisiteMissing { env_vars: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub source: SkillSourceKind,
    pub status: SkillStatus,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillFilter {
    pub tag: Option<String>,
    pub category: Option<String>,
    pub include_prerequisite_missing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillView {
    pub summary: SkillSummary,
    pub parameters: Vec<SkillParameterInfo>,
    pub config_keys: Vec<String>,
    pub body_preview: String,
    pub body_full: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillParameterInfo {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct SkillInjectionId(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillInvocationReceipt {
    pub skill_name: String,
    pub injection_id: SkillInjectionId,
    pub bytes_injected: u64,
    pub consumed_config_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RenderedSkill {
    pub skill_id: SkillId,
    pub skill_name: String,
    pub content: String,
    pub shell_invocations: Vec<SkillShellInvocation>,
    pub consumed_config_keys: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillShellInvocation {
    pub command: String,
    pub stdout_truncated: bool,
    pub exit_code: i32,
}

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
