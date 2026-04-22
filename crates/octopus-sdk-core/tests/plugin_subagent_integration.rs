use std::{
    fs,
    sync::Arc,
};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    AssistantEvent, ContentBlock, DeclSource, HookDecision, HookEvent, HookPoint, PermissionMode,
    PluginSourceTag, Role, StopReason, SubagentOutput, SubagentSpec, TaskBudget, ToolCallId,
    ToolCategory, ToolDecl,
};
use octopus_sdk_core::SubmitTurnInput;
use octopus_sdk_hooks::{Hook, HookSource};
use octopus_sdk_plugin::{
    Plugin, PluginApi, PluginCompat, PluginComponent, PluginDiscoveryConfig,
    PluginHookRegistration, PluginLifecycle, PluginManifest, PluginRegistry, PluginToolRegistration,
};
use octopus_sdk_session::SessionStore;
use octopus_sdk_tools::{TaskFn, Tool, ToolContext, ToolError, ToolResult, ToolSpec};
use serde_json::json;

mod support;

struct PluginTool;

#[async_trait]
impl Tool for PluginTool {
    fn spec(&self) -> &ToolSpec {
        static SPEC: std::sync::OnceLock<ToolSpec> = std::sync::OnceLock::new();
        SPEC.get_or_init(|| ToolSpec {
            name: "noop-tool".into(),
            description: "noop".into(),
            input_schema: json!({ "type": "object" }),
            category: ToolCategory::Read,
        })
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _input: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Ok(ToolResult {
            content: vec![ContentBlock::Text { text: "ok".into() }],
            ..ToolResult::default()
        })
    }
}

struct SessionStartHook;

#[async_trait]
impl Hook for SessionStartHook {
    fn name(&self) -> &str {
        "session-start-hook"
    }

    async fn on_event(&self, _event: &HookEvent) -> HookDecision {
        HookDecision::Continue
    }
}

struct NoopPlugin {
    manifest: PluginManifest,
}

impl NoopPlugin {
    fn new() -> Self {
        Self {
            manifest: PluginManifest {
                id: "example-noop-tool".into(),
                version: "0.1.0".into(),
                git_sha: None,
                compat: PluginCompat {
                    plugin_api: "^1.0.0".into(),
                },
                components: vec![
                    PluginComponent::Tool(ToolDecl {
                        id: "noop-tool".into(),
                        name: "noop-tool".into(),
                        description: "noop".into(),
                        schema: json!({ "type": "object" }),
                        source: DeclSource::Plugin {
                            plugin_id: "example-noop-tool".into(),
                        },
                    }),
                    PluginComponent::Hook(octopus_sdk_contracts::HookDecl {
                        id: "session-start-hook".into(),
                        point: HookPoint::SessionStart,
                        source: DeclSource::Plugin {
                            plugin_id: "example-noop-tool".into(),
                        },
                    }),
                ],
                source: PluginSourceTag::Local,
            },
        }
    }
}

impl Plugin for NoopPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn register(&self, api: &mut PluginApi<'_>) -> Result<(), octopus_sdk_plugin::PluginError> {
        api.register_tool(PluginToolRegistration {
            decl: ToolDecl {
                id: "noop-tool".into(),
                name: "noop-tool".into(),
                description: "noop".into(),
                schema: json!({ "type": "object" }),
                source: DeclSource::Plugin {
                    plugin_id: "example-noop-tool".into(),
                },
            },
            tool: Arc::new(PluginTool),
        })?;
        api.register_hook(PluginHookRegistration {
            decl: octopus_sdk_contracts::HookDecl {
                id: "session-start-hook".into(),
                point: HookPoint::SessionStart,
                source: DeclSource::Plugin {
                    plugin_id: "example-noop-tool".into(),
                },
            },
            hook: Arc::new(SessionStartHook),
            source: HookSource::Plugin {
                plugin_id: "example-noop-tool".into(),
            },
            priority: 10,
        })?;
        Ok(())
    }
}

struct StaticTaskFn;

#[async_trait]
impl TaskFn for StaticTaskFn {
    async fn run(
        &self,
        _spec: &SubagentSpec,
        input: &str,
    ) -> Result<SubagentOutput, octopus_sdk_contracts::SubagentError> {
        Ok(SubagentOutput::Summary {
            text: format!("subagent: {input}"),
            meta: octopus_sdk_contracts::SubagentSummary {
                session_id: octopus_sdk_contracts::SessionId("subagent-1".into()),
                turns: 1,
                tokens_used: 3,
                duration_ms: 1,
                trace_id: "trace-subagent".into(),
            },
        })
    }
}

#[tokio::test]
async fn test_builder_uses_supplied_plugin_registry() {
    let (root, store) = support::temp_store();
    let mut registry = PluginRegistry::new();
    let plugin = NoopPlugin::new();
    let plugin_root = plugin_root(plugin.manifest());
    let plugins: Vec<Box<dyn Plugin>> = vec![Box::new(plugin)];

    PluginLifecycle::run(
        &mut registry,
        &PluginDiscoveryConfig {
            roots: vec![plugin_root.path().to_path_buf()],
            allow: Vec::new(),
            deny: Vec::new(),
        },
        &plugins,
    )
    .expect("plugin lifecycle should populate registry");
    let supplied_snapshot = registry.get_snapshot();

    let runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![vec![]])),
        store.clone(),
    )
    .with_plugin_registry(registry)
    .with_plugins_snapshot(supplied_snapshot.clone())
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    let snapshot = store
        .snapshot(&handle.session_id)
        .await
        .expect("snapshot should exist");

    assert_eq!(snapshot.plugins_snapshot, supplied_snapshot);
}

#[tokio::test]
async fn test_agent_tool_uses_orchestrator() {
    let (root, store) = support::temp_store();
    let runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![
            vec![
                AssistantEvent::ToolUse {
                    id: ToolCallId("task-1".into()),
                    name: "task".into(),
                    input: json!({
                        "spec": sample_spec("worker-1"),
                        "input": "delegate this"
                    }),
                },
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::ToolUse,
                },
            ],
            vec![
                AssistantEvent::TextDelta("subagent finished".into()),
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        ])),
        store,
    )
    .with_task_fn(Arc::new(StaticTaskFn))
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("do the task"),
        })
        .await
        .expect("task tool should complete");

    let events = support::collect_events(&runtime, &handle.session_id).await;
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::ToolExecuted { name, .. } if name == "task"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::AssistantMessage(message)
            if message.role == Role::Tool
                && message.content.iter().any(|block| matches!(block, ContentBlock::ToolResult { content, .. } if content.iter().any(|child| matches!(child, ContentBlock::Text { text } if text.contains("subagent: delegate this")))))
    )));
}

fn sample_spec(id: &str) -> SubagentSpec {
    SubagentSpec {
        id: id.into(),
        system_prompt: "Be concise.".into(),
        allowed_tools: vec!["noop-tool".into()],
        model_role: "subagent-default".into(),
        permission_mode: PermissionMode::Default,
        task_budget: TaskBudget {
            total: 100,
            completion_threshold: 0.9,
        },
        max_turns: 2,
        depth: 1,
    }
}

fn plugin_root(manifest: &PluginManifest) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir should exist");
    let plugin_dir = root.path().join(&manifest.id);
    fs::create_dir_all(&plugin_dir).expect("plugin dir should exist");
    fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(manifest).expect("manifest should serialize"),
    )
    .expect("manifest should write");
    root
}
