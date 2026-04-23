use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use octopus_core::AppError;
use octopus_sdk::{
    builtin::AgentTool, ModelProvider, PermissionGate, PluginRegistry, SessionStore, SubagentError,
    SubagentOutput, SubagentSpec, TaskFn, ToolRegistry,
};
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_subagent::{OrchestratorWorkers, ParentSessionContext};
use octopus_sdk_tools::{current_task_parent_session, Tool};

const LIVE_TASK_MAX_CONCURRENCY: usize = 5;
const MISSING_PARENT_SESSION_REASON: &str = "task parent session missing from tool context";
const UNINITIALIZED_TASK_FN_REASON: &str = "live task runtime not initialized";

pub(crate) fn build_live_task_fn(
    session_store: Arc<dyn SessionStore>,
    model_provider: Arc<dyn ModelProvider>,
    tool_registry: &ToolRegistry,
    permission_gate: Arc<dyn PermissionGate>,
    plugin_registry: &PluginRegistry,
    workspace_root: &std::path::Path,
) -> Result<Arc<dyn TaskFn>, AppError> {
    let deferred_task_fn = Arc::new(DeferredTaskFn::default());
    let live_tools = Arc::new(build_live_task_registry(
        tool_registry,
        plugin_registry,
        deferred_task_fn.clone() as Arc<dyn TaskFn>,
    )?);
    let live_task_fn: Arc<dyn TaskFn> = Arc::new(LiveTaskFn {
        session_store,
        model_provider,
        tools: live_tools,
        permission_gate,
        scratchpad: DurableScratchpad::new(workspace_root.to_path_buf()),
    });

    deferred_task_fn.install(Arc::clone(&live_task_fn))?;
    Ok(live_task_fn)
}

fn build_live_task_registry(
    base: &ToolRegistry,
    plugins: &PluginRegistry,
    task_fn: Arc<dyn TaskFn>,
) -> Result<ToolRegistry, AppError> {
    let mut merged = ToolRegistry::new();
    let mut inserted_task_tool = false;

    for (name, tool) in base.iter() {
        if name == "task" {
            merged
                .register(task_tool(task_fn.clone()))
                .map_err(|error| AppError::runtime(error.to_string()))?;
            inserted_task_tool = true;
            continue;
        }

        merged
            .register(Arc::clone(tool))
            .map_err(|error| AppError::runtime(error.to_string()))?;
    }

    for (name, tool) in plugins.tools().iter() {
        if name == "task" {
            if !inserted_task_tool {
                merged
                    .register(task_tool(task_fn.clone()))
                    .map_err(|error| AppError::runtime(error.to_string()))?;
                inserted_task_tool = true;
            }
            continue;
        }

        merged
            .register(Arc::clone(tool))
            .map_err(|error| AppError::runtime(error.to_string()))?;
    }

    if !inserted_task_tool {
        merged
            .register(task_tool(task_fn))
            .map_err(|error| AppError::runtime(error.to_string()))?;
    }

    Ok(merged)
}

fn task_tool(task_fn: Arc<dyn TaskFn>) -> Arc<dyn Tool> {
    Arc::new(AgentTool::new().with_task_fn(task_fn))
}

struct LiveTaskFn {
    session_store: Arc<dyn SessionStore>,
    model_provider: Arc<dyn ModelProvider>,
    tools: Arc<ToolRegistry>,
    permission_gate: Arc<dyn PermissionGate>,
    scratchpad: DurableScratchpad,
}

#[async_trait]
impl TaskFn for LiveTaskFn {
    async fn run(&self, spec: &SubagentSpec, input: &str) -> Result<SubagentOutput, SubagentError> {
        let parent_session_id =
            current_task_parent_session().ok_or_else(missing_parent_session_error)?;
        let workers = OrchestratorWorkers::new(
            ParentSessionContext {
                session_id: parent_session_id,
                session_store: Arc::clone(&self.session_store),
                model: Arc::clone(&self.model_provider),
                tools: Arc::clone(&self.tools),
                permissions: Arc::clone(&self.permission_gate),
                scratchpad: self.scratchpad.clone(),
            },
            LIVE_TASK_MAX_CONCURRENCY,
        );

        workers.run_worker(spec.clone(), input.to_owned()).await
    }
}

#[derive(Default)]
struct DeferredTaskFn {
    inner: OnceLock<Arc<dyn TaskFn>>,
}

impl DeferredTaskFn {
    fn install(&self, task_fn: Arc<dyn TaskFn>) -> Result<(), AppError> {
        self.inner
            .set(task_fn)
            .map_err(|_| AppError::runtime("live task runtime already initialized"))
    }
}

#[async_trait]
impl TaskFn for DeferredTaskFn {
    async fn run(&self, spec: &SubagentSpec, input: &str) -> Result<SubagentOutput, SubagentError> {
        let Some(task_fn) = self.inner.get().cloned() else {
            return Err(uninitialized_task_fn_error());
        };

        task_fn.run(spec, input).await
    }
}

fn missing_parent_session_error() -> SubagentError {
    SubagentError::Provider {
        reason: MISSING_PARENT_SESSION_REASON.into(),
    }
}

fn uninitialized_task_fn_error() -> SubagentError {
    SubagentError::Provider {
        reason: UNINITIALIZED_TASK_FN_REASON.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    use octopus_sdk::{
        AssistantEvent, ContentBlock, EventId, EventRange, ModelError, ModelRequest, ModelStream,
        PermissionMode, PermissionOutcome, PluginsSnapshot, ProviderDescriptor, ProviderId,
        SessionEvent, SessionId, StopReason, TaskBudget, ToolCallRequest, Usage,
    };
    use octopus_sdk_session::{EventStream, SessionError, SessionSnapshot};
    use octopus_sdk_tools::{
        with_task_parent_session, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec,
    };

    struct AllowAllGate;

    #[async_trait]
    impl PermissionGate for AllowAllGate {
        async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
            PermissionOutcome::Allow
        }
    }

    struct EchoProvider;

    #[async_trait]
    impl ModelProvider for EchoProvider {
        async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError> {
            let text = req
                .messages
                .first()
                .and_then(|message| message.content.first())
                .and_then(|block| match block {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "empty".into());

            Ok(Box::pin(futures::stream::iter(vec![
                Ok(AssistantEvent::TextDelta(format!("summary: {text}"))),
                Ok(AssistantEvent::Usage(Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                })),
                Ok(AssistantEvent::MessageStop {
                    stop_reason: StopReason::EndTurn,
                }),
            ])))
        }

        fn describe(&self) -> ProviderDescriptor {
            ProviderDescriptor {
                id: ProviderId("mock".into()),
                supported_families: vec![octopus_sdk::ProtocolFamily::VendorNative],
                catalog_version: "test".into(),
            }
        }
    }

    struct DummyTool {
        spec: ToolSpec,
    }

    impl DummyTool {
        fn new(name: &str) -> Self {
            Self {
                spec: ToolSpec {
                    name: name.into(),
                    description: format!("{name} tool"),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {}
                    }),
                    category: ToolCategory::Read,
                },
            }
        }
    }

    #[async_trait]
    impl Tool for DummyTool {
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
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::default())
        }
    }

    #[tokio::test]
    async fn live_task_fn_uses_current_parent_session_and_runs_worker() {
        let root = tempfile::tempdir().expect("tempdir should exist");
        let store = Arc::new(InMemorySessionStore::default());
        let mut tools = ToolRegistry::new();
        tools
            .register(Arc::new(DummyTool::new("ToolA")))
            .expect("tool should register");
        let task_fn = build_live_task_fn(
            store.clone(),
            Arc::new(EchoProvider),
            &tools,
            Arc::new(AllowAllGate),
            &PluginRegistry::new(),
            root.path(),
        )
        .expect("live task fn should build");

        let output = with_task_parent_session(
            SessionId("parent-live-task".into()),
            task_fn.run(&sample_spec("worker-1"), "build summary"),
        )
        .await
        .expect("task fn should run with scoped parent session");

        let SubagentOutput::Summary { text, meta } = output else {
            panic!("expected summary output");
        };
        assert_eq!(text, "summary: build summary");
        assert_eq!(meta.session_id.0, "child-0");
    }

    fn sample_spec(id: &str) -> SubagentSpec {
        SubagentSpec {
            id: id.into(),
            system_prompt: "Be concise.".into(),
            allowed_tools: vec!["ToolA".into()],
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

    #[derive(Default)]
    struct InMemorySessionStore {
        next_event: AtomicU64,
        next_child: AtomicU64,
    }

    #[async_trait]
    impl SessionStore for InMemorySessionStore {
        async fn append(
            &self,
            _id: &SessionId,
            _event: SessionEvent,
        ) -> Result<EventId, SessionError> {
            Ok(EventId(format!(
                "event-{}",
                self.next_event.fetch_add(1, Ordering::Relaxed)
            )))
        }

        async fn append_session_started(
            &self,
            _id: &SessionId,
            _working_dir: std::path::PathBuf,
            _permission_mode: PermissionMode,
            _model: String,
            _config_snapshot_id: String,
            _effective_config_hash: String,
            _token_budget: u32,
            _plugins_snapshot: Option<PluginsSnapshot>,
        ) -> Result<EventId, SessionError> {
            Ok(EventId(format!(
                "event-{}",
                self.next_event.fetch_add(1, Ordering::Relaxed)
            )))
        }

        async fn new_child_session(
            &self,
            _parent_id: &SessionId,
            _spec: &SubagentSpec,
        ) -> Result<SessionId, SessionError> {
            Ok(SessionId(format!(
                "child-{}",
                self.next_child.fetch_add(1, Ordering::Relaxed)
            )))
        }

        async fn stream(
            &self,
            _id: &SessionId,
            _range: EventRange,
        ) -> Result<EventStream, SessionError> {
            Ok(Box::pin(futures::stream::empty()))
        }

        async fn snapshot(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
            Err(SessionError::NotFound)
        }

        async fn fork(&self, _id: &SessionId, _from: EventId) -> Result<SessionId, SessionError> {
            Err(SessionError::NotFound)
        }

        async fn wake(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
            Err(SessionError::NotFound)
        }
    }
}
