use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use octopus_sdk::{
    default_backend_for_host, register_builtins, AgentRuntimeBuilder, AnthropicMessagesAdapter,
    AskAnswer, AskError, AskPrompt, AskResolver, DefaultModelProvider, GeminiNativeAdapter,
    ModelCatalog, ModelId, ModelProvider, NoopTracer, OpenAiChatAdapter, OpenAiResponsesAdapter,
    PermissionGate, PermissionMode, PermissionOutcome, PluginRegistry, PluginsSnapshot,
    ProtocolAdapter, ProtocolFamily, SandboxBackend, SessionStore, SqliteJsonlSessionStore, TaskFn,
    ToolCallRequest, ToolRegistry, Tracer, VendorNativeAdapter,
};

use super::{RuntimeSdkBridge, RuntimeSdkPaths, RuntimeSdkState, RuntimeSecretVault};

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct NoopAskResolver;

#[async_trait]
impl AskResolver for NoopAskResolver {
    async fn resolve(&self, _prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Err(AskError::NotResolvable)
    }
}

pub struct RuntimeSdkDeps {
    pub workspace_id: String,
    pub workspace_root: PathBuf,
    pub default_model: ModelId,
    pub default_permission_mode: PermissionMode,
    pub default_token_budget: u32,
    pub session_store: Arc<dyn SessionStore>,
    pub model_provider: Arc<dyn ModelProvider>,
    pub tool_registry: ToolRegistry,
    pub permission_gate: Arc<dyn PermissionGate>,
    pub ask_resolver: Arc<dyn AskResolver>,
    pub sandbox_backend: Arc<dyn SandboxBackend>,
    pub plugin_registry: PluginRegistry,
    pub plugins_snapshot: PluginsSnapshot,
    pub tracer: Arc<dyn Tracer>,
    pub task_fn: Option<Arc<dyn TaskFn>>,
}

impl RuntimeSdkDeps {
    #[must_use]
    pub fn minimal(
        workspace_id: impl Into<String>,
        workspace_root: PathBuf,
        default_model: ModelId,
        session_store: Arc<dyn SessionStore>,
        model_provider: Arc<dyn ModelProvider>,
        tool_registry: ToolRegistry,
        permission_gate: Arc<dyn PermissionGate>,
        ask_resolver: Arc<dyn AskResolver>,
        sandbox_backend: Arc<dyn SandboxBackend>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            workspace_root,
            default_model,
            default_permission_mode: PermissionMode::Default,
            default_token_budget: 8_192,
            session_store,
            model_provider,
            tool_registry,
            permission_gate,
            ask_resolver,
            sandbox_backend,
            plugin_registry: PluginRegistry::new(),
            plugins_snapshot: PluginsSnapshot::default(),
            tracer: Arc::new(NoopTracer),
            task_fn: None,
        }
    }
}

pub struct RuntimeSdkFactory {
    deps: RuntimeSdkDeps,
}

impl RuntimeSdkFactory {
    #[must_use]
    pub fn new(deps: RuntimeSdkDeps) -> Self {
        Self { deps }
    }

    pub fn build_live(
        workspace_id: impl Into<String>,
        workspace_root: PathBuf,
        default_model: impl Into<String>,
    ) -> Result<Arc<RuntimeSdkBridge>, octopus_core::AppError> {
        let workspace_id = workspace_id.into();
        let paths = RuntimeSdkPaths::new(&workspace_root);
        paths.ensure_layout()?;
        let database = paths.database()?;
        let secret_vault = RuntimeSecretVault::open(&workspace_id, &paths, database.clone())?;
        let session_store = Arc::new(
            SqliteJsonlSessionStore::open(&paths.db_path, &workspace_root.join("runtime/events"))
                .map_err(|error| octopus_core::AppError::runtime(error.to_string()))?,
        );
        let mut tool_registry = ToolRegistry::new();
        register_builtins(&mut tool_registry)
            .map_err(|error| octopus_core::AppError::runtime(error.to_string()))?;
        let plugin_registry = PluginRegistry::new();
        let plugins_snapshot = plugin_registry.get_snapshot();

        Self::new(RuntimeSdkDeps {
            workspace_id,
            workspace_root,
            default_model: ModelId(default_model.into()),
            default_permission_mode: PermissionMode::Default,
            default_token_budget: 8_192,
            session_store,
            model_provider: build_live_model_provider(secret_vault.clone()),
            tool_registry,
            permission_gate: Arc::new(AllowAllGate),
            ask_resolver: Arc::new(NoopAskResolver),
            sandbox_backend: default_backend_for_host(),
            plugin_registry,
            plugins_snapshot,
            tracer: Arc::new(NoopTracer),
            task_fn: None,
        })
        .build_with_parts(paths, secret_vault)
    }

    pub fn build(self) -> Result<Arc<RuntimeSdkBridge>, octopus_core::AppError> {
        let paths = RuntimeSdkPaths::new(&self.deps.workspace_root);
        paths.ensure_layout()?;
        let database = paths.database()?;
        let secret_vault =
            RuntimeSecretVault::open(&self.deps.workspace_id, &paths, database.clone())?;
        self.build_with_parts(paths, secret_vault)
    }

    fn build_with_parts(
        self,
        paths: RuntimeSdkPaths,
        secret_vault: Arc<RuntimeSecretVault>,
    ) -> Result<Arc<RuntimeSdkBridge>, octopus_core::AppError> {
        let RuntimeSdkDeps {
            workspace_id,
            workspace_root,
            default_model,
            default_permission_mode,
            default_token_budget,
            session_store,
            model_provider,
            tool_registry,
            permission_gate,
            ask_resolver,
            sandbox_backend,
            plugin_registry,
            plugins_snapshot,
            tracer,
            task_fn,
        } = self.deps;
        let builder = AgentRuntimeBuilder::new()
            .with_session_store(Arc::clone(&session_store))
            .with_model_provider(model_provider)
            .with_secret_vault(secret_vault.clone())
            .with_tool_registry(tool_registry)
            .with_permission_gate(permission_gate)
            .with_ask_resolver(ask_resolver)
            .with_sandbox_backend(sandbox_backend)
            .with_plugin_registry(plugin_registry)
            .with_plugins_snapshot(plugins_snapshot)
            .with_tracer(tracer);
        let builder = match task_fn {
            Some(task_fn) => builder.with_task_fn(task_fn),
            None => builder,
        };
        let runtime = Arc::new(
            builder
                .build()
                .map_err(|error| octopus_core::AppError::runtime(error.to_string()))?,
        );

        Ok(Arc::new(RuntimeSdkBridge::new(RuntimeSdkState {
            workspace_id,
            workspace_root,
            paths,
            default_model: default_model.0,
            default_permission_mode,
            default_token_budget,
            runtime,
            secret_vault,
            sessions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            order: tokio::sync::Mutex::new(Vec::new()),
            broadcasters: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        })))
    }
}

fn build_live_model_provider(secret_vault: Arc<RuntimeSecretVault>) -> Arc<dyn ModelProvider> {
    Arc::new(DefaultModelProvider::new(
        Arc::new(ModelCatalog::new_builtin()),
        default_protocol_adapters(),
        reqwest::Client::new(),
        secret_vault,
    ))
}

fn default_protocol_adapters() -> HashMap<ProtocolFamily, Arc<dyn ProtocolAdapter>> {
    HashMap::from([
        (
            ProtocolFamily::AnthropicMessages,
            Arc::new(AnthropicMessagesAdapter) as Arc<dyn ProtocolAdapter>,
        ),
        (
            ProtocolFamily::OpenAiChat,
            Arc::new(OpenAiChatAdapter) as Arc<dyn ProtocolAdapter>,
        ),
        (
            ProtocolFamily::OpenAiResponses,
            Arc::new(OpenAiResponsesAdapter) as Arc<dyn ProtocolAdapter>,
        ),
        (
            ProtocolFamily::GeminiNative,
            Arc::new(GeminiNativeAdapter) as Arc<dyn ProtocolAdapter>,
        ),
        (
            ProtocolFamily::VendorNative,
            Arc::new(VendorNativeAdapter) as Arc<dyn ProtocolAdapter>,
        ),
    ])
}
