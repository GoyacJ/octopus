use std::path::{Path, PathBuf};
use std::sync::Arc;

use harness_context::ContextEngine;
use harness_contracts::{BlobStore, CapabilityRegistry, Event, MessageId, RunId, ToolCapability};
use harness_hook::HookDispatcher;
use harness_journal::EventStore;
use harness_model::{ApiMode, ModelProvider};
use harness_permission::PermissionBroker;
use harness_sandbox::SandboxBackend;
use harness_tool::ToolPool;

use crate::{EngineError, EngineId, EngineRunner, EventStream, RunContext, SessionHandle};

#[derive(Debug, Clone)]
pub struct SteeringMerge {
    pub body: String,
    pub applied_event: Event,
    pub already_persisted: bool,
}

#[async_trait::async_trait]
pub trait SteeringDrain: Send + Sync + 'static {
    async fn drain_and_merge(
        &self,
        session: &SessionHandle,
        run_id: RunId,
        merged_into_message_id: MessageId,
    ) -> Result<Option<SteeringMerge>, EngineError>;
}

#[derive(Clone)]
pub struct Engine {
    id: EngineId,
    pub(crate) event_store: Arc<dyn EventStore>,
    pub(crate) context: ContextEngine,
    pub(crate) hooks: HookDispatcher,
    pub(crate) model: Arc<dyn ModelProvider>,
    pub(crate) tools: ToolPool,
    pub(crate) permission_broker: Arc<dyn PermissionBroker>,
    pub(crate) workspace_root: PathBuf,
    pub(crate) model_id: String,
    pub(crate) api_mode: ApiMode,
    pub(crate) system_prompt: Option<String>,
    pub(crate) sandbox: Option<Arc<dyn SandboxBackend>>,
    pub(crate) cap_registry: Arc<CapabilityRegistry>,
    pub(crate) blob_store: Option<Arc<dyn BlobStore>>,
    pub(crate) max_iterations: u32,
    pub(crate) steering_drain: Option<Arc<dyn SteeringDrain>>,
}

#[derive(Clone)]
pub struct EngineBuilder {
    id: EngineId,
    event_store: Option<Arc<dyn EventStore>>,
    context: Option<ContextEngine>,
    hooks: Option<HookDispatcher>,
    model: Option<Arc<dyn ModelProvider>>,
    tools: Option<ToolPool>,
    permission_broker: Option<Arc<dyn PermissionBroker>>,
    workspace_root: Option<PathBuf>,
    model_id: Option<String>,
    api_mode: ApiMode,
    system_prompt: Option<String>,
    sandbox: Option<Arc<dyn SandboxBackend>>,
    cap_registry: Option<Arc<CapabilityRegistry>>,
    cap_overrides: CapabilityRegistry,
    blob_store: Option<Arc<dyn BlobStore>>,
    max_iterations: u32,
    steering_drain: Option<Arc<dyn SteeringDrain>>,
}

impl Engine {
    #[must_use]
    pub fn builder() -> EngineBuilder {
        EngineBuilder::default()
    }

    #[must_use]
    pub fn engine_id(&self) -> EngineId {
        self.id.clone()
    }

    #[must_use]
    pub fn into_builder(self) -> EngineBuilder {
        EngineBuilder {
            id: self.id,
            event_store: Some(self.event_store),
            context: Some(self.context),
            hooks: Some(self.hooks),
            model: Some(self.model),
            tools: Some(self.tools),
            permission_broker: Some(self.permission_broker),
            workspace_root: Some(self.workspace_root),
            model_id: Some(self.model_id),
            api_mode: self.api_mode,
            system_prompt: self.system_prompt,
            sandbox: self.sandbox,
            cap_registry: Some(self.cap_registry),
            cap_overrides: CapabilityRegistry::default(),
            blob_store: self.blob_store,
            max_iterations: self.max_iterations,
            steering_drain: self.steering_drain,
        }
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self {
            id: EngineId::new("default-engine"),
            event_store: None,
            context: None,
            hooks: None,
            model: None,
            tools: None,
            permission_broker: None,
            workspace_root: None,
            model_id: None,
            api_mode: ApiMode::Messages,
            system_prompt: None,
            sandbox: None,
            cap_registry: None,
            cap_overrides: CapabilityRegistry::default(),
            blob_store: None,
            max_iterations: 25,
            steering_drain: None,
        }
    }
}

impl EngineBuilder {
    #[must_use]
    pub fn with_engine_id(mut self, id: EngineId) -> Self {
        self.id = id;
        self
    }

    #[must_use]
    pub fn with_event_store(mut self, event_store: Arc<dyn EventStore>) -> Self {
        self.event_store = Some(event_store);
        self
    }

    #[must_use]
    pub fn with_context(mut self, context: ContextEngine) -> Self {
        self.context = Some(context);
        self
    }

    #[must_use]
    pub fn with_hooks(mut self, hooks: HookDispatcher) -> Self {
        self.hooks = Some(hooks);
        self
    }

    #[must_use]
    pub fn with_model(mut self, model: Arc<dyn ModelProvider>) -> Self {
        self.model = Some(model);
        self
    }

    #[must_use]
    pub fn with_tools(mut self, tools: ToolPool) -> Self {
        self.tools = Some(tools);
        self
    }

    #[must_use]
    pub fn with_permission_broker(mut self, permission_broker: Arc<dyn PermissionBroker>) -> Self {
        self.permission_broker = Some(permission_broker);
        self
    }

    #[must_use]
    pub fn with_workspace_root(mut self, workspace_root: impl AsRef<Path>) -> Self {
        self.workspace_root = Some(workspace_root.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    #[must_use]
    pub fn with_api_mode(mut self, api_mode: ApiMode) -> Self {
        self.api_mode = api_mode;
        self
    }

    #[must_use]
    pub fn with_system_prompt(mut self, system_prompt: Option<impl Into<String>>) -> Self {
        self.system_prompt = system_prompt.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_sandbox(mut self, sandbox: Arc<dyn SandboxBackend>) -> Self {
        self.sandbox = Some(sandbox);
        self
    }

    #[must_use]
    pub fn with_cap_registry(mut self, cap_registry: Arc<CapabilityRegistry>) -> Self {
        self.cap_registry = Some(cap_registry);
        self
    }

    #[must_use]
    pub fn with_capability<T>(mut self, capability: ToolCapability, implementation: Arc<T>) -> Self
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.cap_overrides.install(capability, implementation);
        self
    }

    #[must_use]
    pub fn with_blob_store(mut self, blob_store: Arc<dyn BlobStore>) -> Self {
        self.blob_store = Some(blob_store);
        self
    }

    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: u32) -> Self {
        self.max_iterations = max_iterations.max(1);
        self
    }

    #[must_use]
    pub fn with_steering_drain(mut self, steering_drain: Arc<dyn SteeringDrain>) -> Self {
        self.steering_drain = Some(steering_drain);
        self
    }

    pub fn build(self) -> Result<Engine, harness_contracts::EngineError> {
        let event_store = self.event_store.ok_or_else(|| {
            harness_contracts::EngineError::Message("event store missing".to_owned())
        })?;
        let context = self.context.ok_or_else(|| {
            harness_contracts::EngineError::Message("context engine missing".to_owned())
        })?;
        let hooks = self.hooks.ok_or_else(|| {
            harness_contracts::EngineError::Message("hook dispatcher missing".to_owned())
        })?;
        let model = self.model.ok_or_else(|| {
            harness_contracts::EngineError::Message("model provider missing".to_owned())
        })?;
        let tools = self.tools.ok_or_else(|| {
            harness_contracts::EngineError::Message("tool pool missing".to_owned())
        })?;
        let permission_broker = self.permission_broker.ok_or_else(|| {
            harness_contracts::EngineError::Message("permission broker missing".to_owned())
        })?;
        let workspace_root = self.workspace_root.ok_or_else(|| {
            harness_contracts::EngineError::Message("workspace root missing".to_owned())
        })?;
        let model_id = self.model_id.ok_or_else(|| {
            harness_contracts::EngineError::Message("model id missing".to_owned())
        })?;
        let cap_registry = crate::capability_assembly::assemble_capability_registry(
            self.cap_registry.as_ref(),
            self.blob_store.as_ref(),
            &self.cap_overrides,
        );
        validate_tool_capabilities(&tools, &cap_registry)?;

        Ok(Engine {
            id: self.id,
            event_store,
            context,
            hooks,
            model,
            tools,
            permission_broker,
            workspace_root,
            model_id,
            api_mode: self.api_mode,
            system_prompt: self.system_prompt,
            sandbox: self.sandbox,
            cap_registry,
            blob_store: self.blob_store,
            max_iterations: self.max_iterations,
            steering_drain: self.steering_drain,
        })
    }
}

#[cfg(feature = "steering")]
#[async_trait::async_trait]
impl SteeringDrain for harness_session::Session {
    async fn drain_and_merge(
        &self,
        _session: &SessionHandle,
        run_id: RunId,
        merged_into_message_id: MessageId,
    ) -> Result<Option<SteeringMerge>, EngineError> {
        Ok(self
            .drain_and_merge_into(run_id, Some(merged_into_message_id))
            .await
            .map_err(|error| EngineError::Message(error.to_string()))?
            .map(|message| SteeringMerge {
                body: message.body,
                applied_event: message.applied_event,
                already_persisted: true,
            }))
    }
}

fn validate_tool_capabilities(
    tools: &ToolPool,
    cap_registry: &CapabilityRegistry,
) -> Result<(), harness_contracts::EngineError> {
    for tool in tools.iter() {
        let descriptor = tool.descriptor();
        for capability in &descriptor.required_capabilities {
            if !cap_registry.contains(capability) {
                return Err(harness_contracts::EngineError::Message(format!(
                    "missing required capability {capability} for tool {}",
                    descriptor.name
                )));
            }
        }
    }

    Ok(())
}

#[async_trait::async_trait]
impl EngineRunner for Engine {
    async fn run(
        &self,
        session: SessionHandle,
        input: harness_contracts::TurnInput,
        ctx: RunContext,
    ) -> Result<EventStream, harness_contracts::EngineError> {
        crate::turn::run_turn(self, session, input, ctx).await
    }

    fn engine_id(&self) -> EngineId {
        self.engine_id()
    }
}
