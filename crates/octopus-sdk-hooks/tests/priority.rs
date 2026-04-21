use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    CompactionCtx, CompactionResult, CompactionStrategyTag, ContentBlock, EndReason, EventId,
    HookDecision, HookEvent, HookToolResult, Message, RenderBlock, RenderKind, RenderMeta, Role,
    SessionId, ToolCallId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_hooks::runner::{Hook, HookError, HookRunner, HookSource};
use serde_json::json;

struct TestHook {
    name: String,
    record: Arc<Mutex<Vec<String>>>,
    handler: Arc<dyn Fn(&HookEvent) -> HookDecision + Send + Sync>,
}

impl TestHook {
    fn new<F>(name: &str, record: Arc<Mutex<Vec<String>>>, handler: F) -> Self
    where
        F: Fn(&HookEvent) -> HookDecision + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            record,
            handler: Arc::new(handler),
        }
    }
}

#[async_trait]
impl Hook for TestHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn on_event(&self, event: &HookEvent) -> HookDecision {
        self.record
            .lock()
            .expect("record lock should work")
            .push(self.name.clone());
        (self.handler)(event)
    }
}

#[tokio::test]
async fn test_source_ordering() {
    let runner = HookRunner::new();
    let record = Arc::new(Mutex::new(Vec::new()));

    runner.register(
        "session",
        Arc::new(TestHook::new("session", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Session,
        100,
    );
    runner.register(
        "defaults",
        Arc::new(TestHook::new("defaults", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Defaults,
        100,
    );
    runner.register(
        "plugin",
        Arc::new(TestHook::new("plugin", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Plugin {
            plugin_id: "plugin-a".into(),
        },
        100,
    );
    runner.register(
        "project",
        Arc::new(TestHook::new("project", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Project,
        100,
    );
    runner.register(
        "workspace",
        Arc::new(TestHook::new("workspace", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Workspace,
        100,
    );

    let outcome = runner.run(sample_pre_tool_use()).await.expect("run should work");

    assert_eq!(outcome.aborted, None);
    assert_eq!(
        *record.lock().expect("record lock should work"),
        vec!["plugin", "workspace", "defaults", "project", "session"]
    );
}

#[tokio::test]
async fn unregister_by_source_only_removes_matching_plugin_id() {
    let runner = HookRunner::new();
    let record = Arc::new(Mutex::new(Vec::new()));

    runner.register(
        "plugin-a",
        Arc::new(TestHook::new("plugin-a", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Plugin {
            plugin_id: "plugin-a".into(),
        },
        100,
    );
    runner.register(
        "plugin-b",
        Arc::new(TestHook::new("plugin-b", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Plugin {
            plugin_id: "plugin-b".into(),
        },
        100,
    );
    runner.register(
        "workspace",
        Arc::new(TestHook::new("workspace", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Workspace,
        100,
    );

    assert_eq!(
        runner.unregister_by_source(HookSource::Plugin {
            plugin_id: "plugin-a".into(),
        }),
        1
    );

    let outcome = runner.run(sample_pre_tool_use()).await.expect("run should work");

    assert_eq!(outcome.aborted, None);
    assert_eq!(
        *record.lock().expect("record lock should work"),
        vec!["plugin-b", "workspace"]
    );
}

#[tokio::test]
async fn continues_and_priority_then_name_are_stable() {
    let runner = HookRunner::new();
    let record = Arc::new(Mutex::new(Vec::new()));

    runner.register(
        "z-last",
        Arc::new(TestHook::new("z-last", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Workspace,
        20,
    );
    runner.register(
        "a-first",
        Arc::new(TestHook::new("a-first", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Workspace,
        10,
    );
    runner.register(
        "b-second",
        Arc::new(TestHook::new("b-second", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Workspace,
        10,
    );

    let outcome = runner.run(sample_pre_tool_use()).await.expect("run should work");

    assert_eq!(
        outcome
            .decisions
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        vec!["a-first", "b-second", "z-last"]
    );
    assert_eq!(
        *record.lock().expect("record lock should work"),
        vec!["a-first", "b-second", "z-last"]
    );
    assert_eq!(outcome.final_payload, None);
}

#[tokio::test]
async fn rewrite_chain_updates_final_payload() {
    let runner = HookRunner::new();
    let record = Arc::new(Mutex::new(Vec::new()));

    runner.register(
        "rewrite-1",
        Arc::new(TestHook::new("rewrite-1", Arc::clone(&record), |_| {
            HookDecision::Rewrite(octopus_sdk_contracts::RewritePayload::ToolCall {
                call: ToolCallRequest {
                    id: ToolCallId("call-2".into()),
                    name: "shell_exec".into(),
                    input: json!({ "command": "echo one" }),
                },
            })
        })),
        HookSource::Workspace,
        10,
    );
    runner.register(
        "rewrite-2",
        Arc::new(TestHook::new("rewrite-2", Arc::clone(&record), |event| match event {
            HookEvent::PreToolUse { call, .. } => {
                assert_eq!(call.input["command"], "echo one");
                HookDecision::Rewrite(octopus_sdk_contracts::RewritePayload::ToolCall {
                    call: ToolCallRequest {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        input: json!({ "command": "echo two" }),
                    },
                })
            }
            _ => HookDecision::Continue,
        })),
        HookSource::Workspace,
        20,
    );

    let outcome = runner.run(sample_pre_tool_use()).await.expect("run should work");

    assert_eq!(
        outcome.final_payload,
        Some(octopus_sdk_contracts::RewritePayload::ToolCall {
            call: ToolCallRequest {
                id: ToolCallId("call-2".into()),
                name: "shell_exec".into(),
                input: json!({ "command": "echo two" }),
            },
        })
    );
}

#[tokio::test]
async fn abort_short_circuits_following_hooks() {
    let runner = HookRunner::new();
    let record = Arc::new(Mutex::new(Vec::new()));

    runner.register(
        "first",
        Arc::new(TestHook::new("first", Arc::clone(&record), |_| {
            HookDecision::Abort {
                reason: "stop-here".into(),
            }
        })),
        HookSource::Workspace,
        10,
    );
    runner.register(
        "second",
        Arc::new(TestHook::new("second", Arc::clone(&record), |_| {
            HookDecision::Continue
        })),
        HookSource::Workspace,
        20,
    );

    let outcome = runner.run(sample_pre_tool_use()).await.expect("run should work");

    assert_eq!(outcome.aborted, Some("stop-here".into()));
    assert_eq!(
        outcome
            .decisions
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        vec!["first"]
    );
    assert_eq!(*record.lock().expect("record lock should work"), vec!["first"]);
}

#[tokio::test]
async fn inject_message_only_allowed_for_stop_and_user_prompt_submit() {
    let runner = HookRunner::new();
    let record = Arc::new(Mutex::new(Vec::new()));
    let injected = sample_message("added by hook");

    runner.register(
        "inject",
        Arc::new(TestHook::new("inject", Arc::clone(&record), {
            let injected = injected.clone();
            move |_| HookDecision::InjectMessage(injected.clone())
        })),
        HookSource::Workspace,
        10,
    );

    let stop_outcome = runner.run(sample_stop()).await.expect("stop run should work");
    assert_eq!(
        stop_outcome.final_payload,
        Some(octopus_sdk_contracts::RewritePayload::UserPrompt {
            message: injected.clone(),
        })
    );

    let err = runner
        .run(sample_session_start())
        .await
        .expect_err("session_start inject should fail");
    assert!(matches!(
        err,
        HookError::InjectNotAllowed {
            event_kind: "session_start"
        }
    ));
}

fn sample_pre_tool_use() -> HookEvent {
    HookEvent::PreToolUse {
        call: ToolCallRequest {
            id: ToolCallId("call-1".into()),
            name: "shell_exec".into(),
            input: json!({ "command": "echo original" }),
        },
        category: ToolCategory::Shell,
    }
}

#[allow(dead_code)]
fn sample_post_tool_use() -> HookEvent {
    HookEvent::PostToolUse {
        call: ToolCallRequest {
            id: ToolCallId("call-1".into()),
            name: "shell_exec".into(),
            input: json!({ "command": "echo original" }),
        },
        result: HookToolResult {
            content: vec![ContentBlock::Text {
                text: "done".into(),
            }],
            is_error: false,
            duration_ms: 12,
            render: Some(RenderBlock {
                kind: RenderKind::Text,
                payload: json!({ "text": "done" }),
                meta: RenderMeta {
                    id: EventId("event-1".into()),
                    parent: None,
                    ts_ms: 1,
                },
            }),
        },
    }
}

fn sample_stop() -> HookEvent {
    HookEvent::Stop {
        session: SessionId("session-1".into()),
    }
}

fn sample_session_start() -> HookEvent {
    HookEvent::SessionStart {
        session: SessionId("session-1".into()),
    }
}

#[allow(dead_code)]
fn sample_session_end() -> HookEvent {
    HookEvent::SessionEnd {
        session: SessionId("session-1".into()),
        reason: EndReason::Normal,
    }
}

#[allow(dead_code)]
fn sample_user_prompt_submit() -> HookEvent {
    HookEvent::UserPromptSubmit {
        message: sample_message("hello"),
    }
}

#[allow(dead_code)]
fn sample_pre_compact() -> HookEvent {
    HookEvent::PreCompact {
        session: SessionId("session-1".into()),
        ctx: CompactionCtx {
            session: SessionId("session-1".into()),
            strategy: CompactionStrategyTag::Summarize,
            threshold: 0.9,
            tokens_current: 900,
            tokens_budget: 1000,
        },
    }
}

#[allow(dead_code)]
fn sample_post_compact() -> HookEvent {
    HookEvent::PostCompact {
        session: SessionId("session-1".into()),
        result: CompactionResult {
            summary: "summary".into(),
            folded_turn_ids: vec![EventId("event-1".into())],
            tool_results_cleared: 1,
            tokens_before: 1000,
            tokens_after: 200,
            strategy: CompactionStrategyTag::Summarize,
        },
    }
}

fn sample_message(text: &str) -> Message {
    Message {
        role: Role::User,
        content: vec![ContentBlock::Text { text: text.into() }],
    }
}
