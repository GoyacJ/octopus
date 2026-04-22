use std::{fs, sync::Arc};

use async_trait::async_trait;
use octopus_platform::{
    ModelRegistryService, RuntimeConfigService, RuntimeSdkDeps, RuntimeSdkFactory,
};
use octopus_sdk::{
    register_builtins, AskAnswer, AskError, AskPrompt, AskResolver, AssistantEvent, ModelError,
    ModelId, ModelProvider, ModelRequest, ModelStream, NoopBackend, PermissionGate,
    PermissionOutcome, PluginRegistry, ProviderDescriptor, ProviderId, ToolCallRequest,
    ToolRegistry,
};
use rusqlite::Connection;
use serde_json::json;

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
        Ok(Box::pin(futures::stream::iter(
            vec![Ok(AssistantEvent::TextDelta("ok".into()))].into_iter(),
        )))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![octopus_sdk::ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

fn build_bridge(root: &std::path::Path) -> Arc<octopus_platform::RuntimeSdkBridge> {
    let store = Arc::new(
        octopus_sdk::SqliteJsonlSessionStore::open(
            &root.join("data/main.db"),
            &root.join("runtime/events"),
        )
        .expect("session store should open"),
    );
    let mut tools = ToolRegistry::new();
    register_builtins(&mut tools).expect("builtins should register");
    let plugin_registry = PluginRegistry::new();
    let plugins_snapshot = plugin_registry.get_snapshot();

    RuntimeSdkFactory::new(RuntimeSdkDeps {
        workspace_id: "ws-local".into(),
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
        tracer: Arc::new(octopus_sdk::NoopTracer),
        task_fn: None,
    })
    .build()
    .expect("bridge should build")
}

#[tokio::test]
async fn runtime_config_bridge_saves_managed_credentials_and_redacts_sources() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let bridge = build_bridge(root.path());

    let saved = bridge
        .save_config(
            "workspace",
            octopus_core::RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "configuredModels": {
                        "configured-model": {
                            "configuredModelId": "configured-model",
                            "name": "Configured Model",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "enabled": true,
                            "source": "workspace"
                        }
                    }
                }),
                configured_model_credentials: vec![
                    octopus_core::RuntimeConfiguredModelCredentialInput {
                        configured_model_id: "configured-model".into(),
                        api_key: "sk-test-managed".into(),
                    },
                ],
            },
        )
        .await
        .expect("save runtime config");

    let workspace_source = saved
        .sources
        .iter()
        .find(|source| source.scope == "workspace")
        .expect("workspace source");
    let stored_ref = workspace_source
        .document
        .as_ref()
        .and_then(|document| {
            document
                .pointer("/configuredModels/configured-model/credentialRef")
                .and_then(serde_json::Value::as_str)
        })
        .expect("managed credential ref");
    assert!(stored_ref.starts_with("secret-ref:workspace:ws-local:configured-model:"));
    assert!(saved.secret_references.iter().any(|reference| {
        reference.path == "configuredModels.configured-model.credentialRef"
            && reference.reference.as_deref() == Some(stored_ref)
            && reference.status == "reference-present"
    }));

    let stored = fs::read_to_string(root.path().join("config/runtime/workspace.json"))
        .expect("workspace config should persist");
    assert!(stored.contains(stored_ref));
    assert!(!stored.contains("sk-test-managed"));
}

#[tokio::test]
async fn runtime_config_bridge_merges_user_workspace_and_project_sources() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    fs::create_dir_all(root.path().join("config/runtime/projects")).expect("project config dir");
    fs::create_dir_all(root.path().join("config/runtime/users")).expect("user config dir");
    fs::write(
        root.path().join("config/runtime/workspace.json"),
        serde_json::to_vec_pretty(&json!({
            "configuredModels": {
                "workspace-model": {
                    "configuredModelId": "workspace-model",
                    "name": "Workspace Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "env:ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }))
        .expect("workspace json"),
    )
    .expect("write workspace config");
    fs::write(
        root.path().join("config/runtime/users/user-1.json"),
        serde_json::to_vec_pretty(&json!({
            "model": "gpt-5.4-mini"
        }))
        .expect("user json"),
    )
    .expect("write user config");
    fs::write(
        root.path().join("config/runtime/projects/project-1.json"),
        serde_json::to_vec_pretty(&json!({
            "defaultSelections": {
                "conversation": {
                    "configuredModelId": "workspace-model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "surface": "conversation"
                }
            }
        }))
        .expect("project json"),
    )
    .expect("write project config");

    let bridge = build_bridge(root.path());
    let effective = bridge
        .get_project_config("project-1", "user-1")
        .await
        .expect("project config");

    assert_eq!(effective.sources.len(), 3);
    assert_eq!(
        effective
            .effective_config
            .pointer("/defaultSelections/conversation/configuredModelId")
            .and_then(serde_json::Value::as_str),
        Some("workspace-model")
    );
    assert_eq!(
        effective
            .effective_config
            .pointer("/configuredModels/workspace-model/credentialRef")
            .and_then(serde_json::Value::as_str),
        Some("env:ANTHROPIC_API_KEY")
    );
}

#[tokio::test]
async fn runtime_config_bridge_builds_catalog_snapshot_with_usage() {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let bridge = build_bridge(root.path());

    bridge
        .save_config(
            "workspace",
            octopus_core::RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "configuredModels": {
                        "quota-model": {
                            "configuredModelId": "quota-model",
                            "name": "Quota Model",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "credentialRef": "env:ANTHROPIC_API_KEY",
                            "enabled": true,
                            "source": "workspace",
                            "budgetPolicy": {
                                "accountingMode": "provider_reported",
                                "totalBudgetTokens": 1200,
                                "reservationStrategy": "fixed"
                            }
                        }
                    }
                }),
                configured_model_credentials: Vec::new(),
            },
        )
        .await
        .expect("save runtime config");

    let connection =
        Connection::open(root.path().join("data/main.db")).expect("open config usage database");
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS configured_model_budget_projections (
                configured_model_id TEXT PRIMARY KEY,
                settled_tokens INTEGER NOT NULL
            );",
        )
        .expect("create usage table");
    connection
        .execute(
            "INSERT INTO configured_model_budget_projections (configured_model_id, settled_tokens)
             VALUES (?1, ?2)",
            rusqlite::params!["quota-model", 42_i64],
        )
        .expect("insert usage projection");

    let snapshot = bridge.catalog_snapshot().await.expect("catalog snapshot");
    let quota_model = snapshot
        .configured_models
        .iter()
        .find(|model| model.configured_model_id == "quota-model")
        .expect("quota model");
    assert_eq!(quota_model.token_usage.used_tokens, 42);
    assert_eq!(
        snapshot
            .default_selections
            .get("conversation")
            .map(|selection| selection.model_id.as_str()),
        Some("claude-sonnet-4-5")
    );
    assert!(snapshot
        .models
        .iter()
        .any(|model| model.model_id == "MiniMax-M2.7"));
}
