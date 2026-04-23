use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    ContentBlock, DeclSource, HookDecision, HookEvent, HookPoint, Message, ModelProviderDecl,
    PluginSourceTag, SessionId, SkillDecl, ToolCategory, ToolDecl,
};
use octopus_sdk_hooks::{Hook, HookSource};
use octopus_sdk_plugin::{
    PluginApi, PluginCompat, PluginComponent, PluginDiscoveryConfig, PluginDiscoveryRoot,
    PluginError, PluginHookRegistration, PluginLifecycle, PluginManifest, PluginRegistry,
    PluginRuntime, PluginRuntimeCatalog, PluginToolRegistration,
};
use octopus_sdk_tools::{Tool, ToolContext, ToolResult, ToolSpec};
use serde_json::json;
use tempfile::tempdir;

struct NoopTool {
    spec: ToolSpec,
}

impl NoopTool {
    fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "noop-tool".into(),
                description: "No-op tool".into(),
                input_schema: json!({ "type": "object" }),
                category: ToolCategory::Read,
            },
        }
    }
}

#[async_trait]
impl Tool for NoopTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _input: serde_json::Value,
    ) -> Result<ToolResult, octopus_sdk_tools::ToolError> {
        Ok(ToolResult {
            content: vec![ContentBlock::Text { text: "ok".into() }],
            ..ToolResult::default()
        })
    }
}

struct NoopHook {
    hits: Arc<AtomicUsize>,
}

#[async_trait]
impl Hook for NoopHook {
    fn name(&self) -> &str {
        "noop-hook"
    }

    async fn on_event(&self, event: &HookEvent) -> HookDecision {
        if matches!(event, HookEvent::SessionStart { .. }) {
            self.hits.fetch_add(1, Ordering::SeqCst);
        }
        HookDecision::Continue
    }
}

struct NoopPlugin {
    manifest: PluginManifest,
    hook_hits: Arc<AtomicUsize>,
}

impl NoopPlugin {
    fn new(hook_hits: Arc<AtomicUsize>) -> Self {
        Self {
            manifest: PluginManifest {
                id: "example-noop-tool".into(),
                version: "0.1.0".into(),
                git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
                source: PluginSourceTag::Local,
                compat: PluginCompat {
                    plugin_api: "^1.0.0".into(),
                },
                components: vec![
                    PluginComponent::Tool(tool_decl()),
                    PluginComponent::Hook(hook_decl()),
                    PluginComponent::Skill(skill_decl()),
                    PluginComponent::ModelProvider(model_provider_decl()),
                ],
            },
            hook_hits,
        }
    }
}

impl PluginRuntime for NoopPlugin {
    fn register(&self, api: &mut PluginApi<'_>) -> Result<(), PluginError> {
        api.register_tool(PluginToolRegistration {
            decl: tool_decl(),
            tool: Arc::new(NoopTool::new()),
        })?;
        api.register_hook(PluginHookRegistration {
            decl: hook_decl(),
            hook: Arc::new(NoopHook {
                hits: Arc::clone(&self.hook_hits),
            }),
            source: HookSource::Plugin {
                plugin_id: "example-noop-tool".into(),
            },
            priority: 10,
        })?;
        api.register_skill_decl(skill_decl())?;
        api.register_model_provider_decl(model_provider_decl())?;
        Ok(())
    }
}

#[test]
fn test_register_noop_plugin() {
    let mut registry = PluginRegistry::new();
    let plugin = NoopPlugin::new(Arc::new(AtomicUsize::new(0)));
    let root = plugin_root(&plugin.manifest);
    let plugin_id = plugin.manifest.id.clone();
    let runtimes = runtime_catalog(&plugin_id, Arc::new(plugin));

    PluginLifecycle::run(&mut registry, &config_for(root.path()), &runtimes)
        .expect("plugin should register");

    let snapshot = registry.get_snapshot();
    assert_eq!(snapshot.plugins.len(), 1);
    assert_eq!(snapshot.plugins[0].id, "example-noop-tool");
    assert_eq!(snapshot.plugins[0].source, PluginSourceTag::Local);
    assert_eq!(snapshot.plugins[0].components_count, 4);
}

#[tokio::test]
async fn test_register_api_unidirectional() {
    let hook_hits = Arc::new(AtomicUsize::new(0));
    let mut registry = PluginRegistry::new();
    let plugin = NoopPlugin::new(Arc::clone(&hook_hits));
    let root = plugin_root(&plugin.manifest);
    let plugin_id = plugin.manifest.id.clone();
    let runtimes = runtime_catalog(&plugin_id, Arc::new(plugin));

    PluginLifecycle::run(&mut registry, &config_for(root.path()), &runtimes)
        .expect("plugin should register");

    assert!(registry.tools().get("noop-tool").is_some());

    let outcome = registry
        .hooks()
        .run(HookEvent::SessionStart {
            session: SessionId("session-1".into()),
        })
        .await
        .expect("hook runner should execute");

    assert_eq!(hook_hits.load(Ordering::SeqCst), 1);
    assert_eq!(outcome.decisions.len(), 1);
    assert_eq!(outcome.decisions[0].0, "noop-hook");
}

#[test]
fn test_plugin_register_once() {
    let mut registry = PluginRegistry::new();
    let plugin = NoopPlugin::new(Arc::new(AtomicUsize::new(0)));
    let root = plugin_root(&plugin.manifest);
    let plugin_id = plugin.manifest.id.clone();
    let runtimes = runtime_catalog(&plugin_id, Arc::new(plugin));

    PluginLifecycle::run(&mut registry, &config_for(root.path()), &runtimes)
        .expect("first register should pass");
    let error = PluginLifecycle::run(&mut registry, &config_for(root.path()), &runtimes)
        .expect_err("second register should reject duplicate plugin");

    assert_eq!(
        error,
        PluginError::DuplicateId {
            id: "example-noop-tool".into(),
        }
    );
}

fn config_for(root: &Path) -> PluginDiscoveryConfig {
    PluginDiscoveryConfig {
        roots: vec![PluginDiscoveryRoot::local(root.to_path_buf())],
        allow: Vec::new(),
        deny: Vec::new(),
    }
}

fn runtime_catalog(plugin_id: &str, runtime: Arc<dyn PluginRuntime>) -> PluginRuntimeCatalog {
    let mut runtimes = PluginRuntimeCatalog::new();
    runtimes
        .register_local(plugin_id, runtime)
        .expect("runtime ids should stay unique");
    runtimes
}

fn plugin_root(manifest: &PluginManifest) -> tempfile::TempDir {
    let root = tempdir().expect("tempdir should exist");
    let plugin_dir = root.path().join(&manifest.id);
    fs::create_dir_all(&plugin_dir).expect("plugin dir should exist");
    fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");
    root
}

fn tool_decl() -> ToolDecl {
    ToolDecl {
        id: "noop-tool".into(),
        name: "noop-tool".into(),
        description: "No-op tool".into(),
        schema: json!({ "type": "object" }),
        source: DeclSource::Plugin {
            plugin_id: "example-noop-tool".into(),
        },
    }
}

fn hook_decl() -> octopus_sdk_contracts::HookDecl {
    octopus_sdk_contracts::HookDecl {
        id: "noop-hook".into(),
        point: HookPoint::SessionStart,
        source: DeclSource::Plugin {
            plugin_id: "example-noop-tool".into(),
        },
    }
}

fn skill_decl() -> SkillDecl {
    SkillDecl {
        id: "noop-skill".into(),
        manifest_path: "skills/noop/manifest.md".into(),
    }
}

fn model_provider_decl() -> ModelProviderDecl {
    ModelProviderDecl {
        id: "noop-provider".into(),
        provider_ref: "noop.provider".into(),
    }
}

#[allow(dead_code)]
fn user_message(text: &str) -> Message {
    Message {
        role: octopus_sdk_contracts::Role::User,
        content: vec![ContentBlock::Text { text: text.into() }],
    }
}
