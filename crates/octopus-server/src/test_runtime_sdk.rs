use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use octopus_core::{
    default_connection_stubs, default_host_state, default_preferences, DesktopBackendConnection,
    DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
};
use octopus_infra::build_infra_bundle;
use octopus_persistence::Database;
use octopus_platform::{PlatformServices, RuntimeSdkDeps, RuntimeSdkFactory};
use octopus_sdk::{
    register_builtins, AskAnswer, AskError, AskPrompt, AskResolver, AssistantEvent, ModelError,
    ModelId, ModelProvider, ModelRequest, ModelStream, NoopBackend, NoopTracer, PermissionGate,
    PermissionOutcome, PluginRegistry, ProtocolFamily, ProviderDescriptor, ProviderId, StopReason,
    ToolCallRequest, ToolRegistry,
};
use tokio_stream::iter;

use crate::ServerState;

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct StaticAskResolver;

#[async_trait]
impl AskResolver for StaticAskResolver {
    async fn resolve(&self, prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Ok(AskAnswer {
            prompt_id: prompt_id.into(),
            option_id: "approve".into(),
            text: "approved".into(),
        })
    }
}

struct ScriptedModelProvider;

#[async_trait]
impl ModelProvider for ScriptedModelProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(iter(vec![
            Ok(AssistantEvent::TextDelta("test response".into())),
            Ok(AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            }),
        ])))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

pub(crate) fn test_server_state(root: &Path) -> ServerState {
    let infra = build_infra_bundle(root).expect("infra bundle");
    let store = Arc::new(
        octopus_sdk::SqliteJsonlSessionStore::open(
            &root.join("data/runtime-sdk-tests.db"),
            &root.join("runtime/sdk-events"),
        )
        .expect("session store should open"),
    );
    let mut tools = ToolRegistry::new();
    register_builtins(&mut tools).expect("builtins should register");
    let plugin_registry = PluginRegistry::new();
    let plugins_snapshot = plugin_registry.get_snapshot();
    let runtime = RuntimeSdkFactory::new(RuntimeSdkDeps {
        workspace_id: DEFAULT_WORKSPACE_ID.into(),
        workspace_root: root.to_path_buf(),
        default_model: ModelId("claude-sonnet-4-5".into()),
        default_permission_mode: octopus_sdk::PermissionMode::Default,
        default_token_budget: 8_192,
        session_store: store,
        model_provider: Arc::new(ScriptedModelProvider),
        tool_registry: tools,
        permission_gate: Arc::new(AllowAllGate),
        ask_resolver: Arc::new(StaticAskResolver),
        sandbox_backend: Arc::new(NoopBackend),
        plugin_registry,
        plugins_snapshot,
        tracer: Arc::new(NoopTracer),
        task_fn: None,
    })
    .build()
    .expect("runtime sdk bridge should build");
    let services = PlatformServices {
        workspace: infra.workspace.clone(),
        project_tasks: infra.workspace.clone(),
        access_control: infra.access_control.clone(),
        auth: infra.auth.clone(),
        app_registry: infra.app_registry.clone(),
        authorization: infra.authorization.clone(),
        runtime_session: runtime.clone(),
        runtime_execution: runtime.clone(),
        runtime_config: runtime.clone(),
        runtime_registry: runtime,
        artifact: infra.artifact.clone(),
        inbox: infra.inbox.clone(),
        knowledge: infra.knowledge.clone(),
        observation: infra.observation.clone(),
    };
    let host_notifications_db =
        Database::open(&root.join("data").join("main.db")).expect("host notifications database");

    ServerState {
        services,
        host_notifications_db,
        host_auth_token: "host-test-token".into(),
        transport_security: "loopback".into(),
        idempotency_cache: Arc::new(Mutex::new(HashMap::new())),
        auth_rate_limits: Arc::new(Mutex::new(HashMap::new())),
        host_state: default_host_state("0.1.0-test".into(), true),
        host_connections: default_connection_stubs(),
        host_preferences_path: root.join("config").join("shell-preferences.json"),
        host_workspace_connections_path: root
            .join("config")
            .join("shell-workspace-connections.json"),
        host_default_preferences: default_preferences(DEFAULT_WORKSPACE_ID, DEFAULT_PROJECT_ID),
        backend_connection: DesktopBackendConnection {
            base_url: Some("http://127.0.0.1:43127".into()),
            auth_token: Some("desktop-test-token".into()),
            state: "ready".into(),
            transport: "http".into(),
        },
    }
}
