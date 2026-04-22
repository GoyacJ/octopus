use std::sync::Arc;

use octopus_sdk_context::SystemPromptBuilder;
use octopus_sdk_contracts::{AskResolver, PermissionGate, PluginsSnapshot, SecretVault};
use octopus_sdk_model::ModelProvider;
use octopus_sdk_observability::{NoopTracer, Tracer, UsageLedger};
use octopus_sdk_plugin::PluginRegistry;
use octopus_sdk_sandbox::SandboxBackend;
use octopus_sdk_session::SessionStore;
use octopus_sdk_tools::{TaskFn, ToolRegistry};

use crate::{
    plugin_boot::{materialize_tool_registry, resolve_plugins_snapshot},
    runtime::{AgentRuntime, RuntimeInner},
    RuntimeError,
};

pub struct AgentRuntimeBuilder {
    session_store: Option<Arc<dyn SessionStore>>,
    model_provider: Option<Arc<dyn ModelProvider>>,
    secret_vault: Option<Arc<dyn SecretVault>>,
    tool_registry: Option<ToolRegistry>,
    permission_gate: Option<Arc<dyn PermissionGate>>,
    ask_resolver: Option<Arc<dyn AskResolver>>,
    sandbox_backend: Option<Arc<dyn SandboxBackend>>,
    plugin_registry: PluginRegistry,
    plugins_snapshot: Option<PluginsSnapshot>,
    tracer: Arc<dyn Tracer>,
    task_fn: Option<Arc<dyn TaskFn>>,
}

impl Default for AgentRuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRuntimeBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            session_store: None,
            model_provider: None,
            secret_vault: None,
            tool_registry: None,
            permission_gate: None,
            ask_resolver: None,
            sandbox_backend: None,
            plugin_registry: PluginRegistry::new(),
            plugins_snapshot: None,
            tracer: Arc::new(NoopTracer),
            task_fn: None,
        }
    }

    #[must_use]
    pub fn with_session_store(mut self, session_store: Arc<dyn SessionStore>) -> Self {
        self.session_store = Some(session_store);
        self
    }

    #[must_use]
    pub fn with_model_provider(mut self, model_provider: Arc<dyn ModelProvider>) -> Self {
        self.model_provider = Some(model_provider);
        self
    }

    #[must_use]
    pub fn with_secret_vault(mut self, secret_vault: Arc<dyn SecretVault>) -> Self {
        self.secret_vault = Some(secret_vault);
        self
    }

    #[must_use]
    pub fn with_tool_registry(mut self, tool_registry: ToolRegistry) -> Self {
        self.tool_registry = Some(tool_registry);
        self
    }

    #[must_use]
    pub fn with_permission_gate(mut self, permission_gate: Arc<dyn PermissionGate>) -> Self {
        self.permission_gate = Some(permission_gate);
        self
    }

    #[must_use]
    pub fn with_ask_resolver(mut self, ask_resolver: Arc<dyn AskResolver>) -> Self {
        self.ask_resolver = Some(ask_resolver);
        self
    }

    #[must_use]
    pub fn with_sandbox_backend(mut self, sandbox_backend: Arc<dyn SandboxBackend>) -> Self {
        self.sandbox_backend = Some(sandbox_backend);
        self
    }

    #[must_use]
    pub fn with_plugin_registry(mut self, plugin_registry: PluginRegistry) -> Self {
        self.plugin_registry = plugin_registry;
        self
    }

    #[must_use]
    pub fn with_plugins_snapshot(mut self, plugins_snapshot: PluginsSnapshot) -> Self {
        self.plugins_snapshot = Some(plugins_snapshot);
        self
    }

    #[must_use]
    pub fn with_tracer(mut self, tracer: Arc<dyn Tracer>) -> Self {
        self.tracer = tracer;
        self
    }

    #[must_use]
    pub fn with_task_fn(mut self, task_fn: Arc<dyn TaskFn>) -> Self {
        self.task_fn = Some(task_fn);
        self
    }

    pub fn build(self) -> Result<AgentRuntime, RuntimeError> {
        let session_store = self
            .session_store
            .ok_or(RuntimeError::MissingBuilderField {
                field: "session_store",
            })?;
        let model_provider = self
            .model_provider
            .ok_or(RuntimeError::MissingBuilderField {
                field: "model_provider",
            })?;
        let secret_vault = self.secret_vault.ok_or(RuntimeError::MissingBuilderField {
            field: "secret_vault",
        })?;
        let tool_registry = self
            .tool_registry
            .ok_or(RuntimeError::MissingBuilderField {
                field: "tool_registry",
            })?;
        let permission_gate = self
            .permission_gate
            .ok_or(RuntimeError::MissingBuilderField {
                field: "permission_gate",
            })?;
        let ask_resolver = self.ask_resolver.ok_or(RuntimeError::MissingBuilderField {
            field: "ask_resolver",
        })?;
        let sandbox_backend = self
            .sandbox_backend
            .ok_or(RuntimeError::MissingBuilderField {
                field: "sandbox_backend",
            })?;
        let plugins_snapshot =
            resolve_plugins_snapshot(&self.plugin_registry, self.plugins_snapshot)?;
        let merged_tools = materialize_tool_registry(
            &tool_registry,
            &self.plugin_registry,
            self.task_fn.as_ref(),
        )?;

        Ok(AgentRuntime::new(Arc::new(RuntimeInner {
            session_store,
            model_provider,
            secret_vault,
            tool_registry: merged_tools,
            permission_gate,
            ask_resolver,
            sandbox_backend,
            plugin_registry: Arc::new(self.plugin_registry),
            plugins_snapshot,
            tracer: self.tracer,
            usage_ledger: Arc::new(UsageLedger::new()),
            prompt_builder: SystemPromptBuilder::new(),
            sessions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            active_runs: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        })))
    }
}
