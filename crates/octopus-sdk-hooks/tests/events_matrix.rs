use std::sync::Arc;

use async_trait::async_trait;
use octopus_sdk_contracts::{
    CompactionCtx, CompactionResult, CompactionStrategyTag, ContentBlock, EndReason, EventId,
    HookDecision, HookEvent, HookToolResult, Message, RenderBlock, RenderKind, RenderMeta, Role,
    SessionId, ToolCallId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_hooks::runner::{Hook, HookError, HookRunner, HookSource};
use serde_json::json;

struct StaticHook {
    name: String,
    handler: Arc<dyn Fn(&HookEvent) -> HookDecision + Send + Sync>,
}

impl StaticHook {
    fn new<F>(name: &str, handler: F) -> Self
    where
        F: Fn(&HookEvent) -> HookDecision + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            handler: Arc::new(handler),
        }
    }
}

#[async_trait]
impl Hook for StaticHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn on_event(&self, event: &HookEvent) -> HookDecision {
        (self.handler)(event)
    }
}

#[tokio::test]
async fn hook_event_matrix_is_stable() {
    let injected = sample_message("injected");
    let rewrite_message = sample_message("rewritten");

    for case in event_cases() {
        let continue_runner = HookRunner::new();
        continue_runner.register(
            "continue",
            Arc::new(StaticHook::new("continue", |_| HookDecision::Continue)),
            HookSource::Workspace,
            10,
        );
        let continue_outcome = continue_runner
            .run(case.event.clone())
            .await
            .expect("continue path should not fail");
        assert_eq!(continue_outcome.decisions.len(), 1, "{}", case.kind);
        assert_eq!(continue_outcome.aborted, None, "{}", case.kind);

        let abort_runner = HookRunner::new();
        abort_runner.register(
            "abort",
            Arc::new(StaticHook::new("abort", |_| HookDecision::Abort {
                reason: "blocked".into(),
            })),
            HookSource::Workspace,
            10,
        );
        let abort_outcome = abort_runner
            .run(case.event.clone())
            .await
            .expect("abort path should still return outcome");
        assert_eq!(abort_outcome.aborted, Some("blocked".into()), "{}", case.kind);
        assert_eq!(abort_outcome.decisions.len(), 1, "{}", case.kind);

        let rewrite_runner = HookRunner::new();
        let rewrite_decision = case
            .rewrite_payload
            .clone()
            .unwrap_or_else(|| invalid_rewrite_payload(&rewrite_message));
        rewrite_runner.register(
            "rewrite",
            Arc::new(StaticHook::new("rewrite", move |_| {
                HookDecision::Rewrite(rewrite_decision.clone())
            })),
            HookSource::Workspace,
            10,
        );
        match case.rewrite_payload.clone() {
            Some(expected) => {
                let rewrite_outcome = rewrite_runner
                    .run(case.event.clone())
                    .await
                    .expect("rewrite should succeed for allowed events");
                assert_eq!(
                    rewrite_outcome.final_payload,
                    Some(expected),
                    "{}",
                    case.kind
                );
            }
            None => {
                let err = rewrite_runner
                    .run(case.event.clone())
                    .await
                    .expect_err("rewrite should fail for disallowed events");
                assert!(matches!(
                    err,
                    HookError::RewriteNotAllowed { event_kind } if event_kind == case.kind
                ));
            }
        }

        let inject_runner = HookRunner::new();
        inject_runner.register(
            "inject",
            Arc::new(StaticHook::new("inject", {
                let injected = injected.clone();
                move |_| HookDecision::InjectMessage(injected.clone())
            })),
            HookSource::Workspace,
            10,
        );
        if case.inject_allowed {
            let inject_outcome = inject_runner
                .run(case.event.clone())
                .await
                .expect("inject should succeed for allowed events");
            assert_eq!(
                inject_outcome.final_payload,
                Some(octopus_sdk_contracts::RewritePayload::UserPrompt {
                    message: injected.clone(),
                }),
                "{}",
                case.kind
            );
        } else {
            let err = inject_runner
                .run(case.event.clone())
                .await
                .expect_err("inject should fail for disallowed events");
            assert!(matches!(
                err,
                HookError::InjectNotAllowed { event_kind } if event_kind == case.kind
            ));
        }
    }
}

#[derive(Clone)]
struct EventCase {
    kind: &'static str,
    event: HookEvent,
    rewrite_payload: Option<octopus_sdk_contracts::RewritePayload>,
    inject_allowed: bool,
}

fn event_cases() -> Vec<EventCase> {
    vec![
        EventCase {
            kind: "pre_tool_use",
            event: sample_pre_tool_use(),
            rewrite_payload: Some(octopus_sdk_contracts::RewritePayload::ToolCall {
                call: ToolCallRequest {
                    id: ToolCallId("call-2".into()),
                    name: "shell_exec".into(),
                    input: json!({ "command": "echo rewritten" }),
                },
            }),
            inject_allowed: false,
        },
        EventCase {
            kind: "post_tool_use",
            event: sample_post_tool_use(),
            rewrite_payload: Some(octopus_sdk_contracts::RewritePayload::ToolResult {
                result: HookToolResult {
                    content: vec![ContentBlock::Text {
                        text: "rewritten".into(),
                    }],
                    is_error: false,
                    duration_ms: 3,
                    render: None,
                },
            }),
            inject_allowed: false,
        },
        EventCase {
            kind: "stop",
            event: sample_stop(),
            rewrite_payload: None,
            inject_allowed: true,
        },
        EventCase {
            kind: "session_start",
            event: sample_session_start(),
            rewrite_payload: None,
            inject_allowed: false,
        },
        EventCase {
            kind: "session_end",
            event: sample_session_end(),
            rewrite_payload: None,
            inject_allowed: false,
        },
        EventCase {
            kind: "user_prompt_submit",
            event: sample_user_prompt_submit(),
            rewrite_payload: Some(octopus_sdk_contracts::RewritePayload::UserPrompt {
                message: sample_message("rewritten"),
            }),
            inject_allowed: true,
        },
        EventCase {
            kind: "pre_compact",
            event: sample_pre_compact(),
            rewrite_payload: Some(octopus_sdk_contracts::RewritePayload::Compaction {
                ctx: CompactionCtx {
                    session: SessionId("session-1".into()),
                    strategy: CompactionStrategyTag::ClearToolResults,
                    threshold: 0.8,
                    tokens_current: 800,
                    tokens_budget: 1000,
                },
            }),
            inject_allowed: false,
        },
        EventCase {
            kind: "post_compact",
            event: sample_post_compact(),
            rewrite_payload: None,
            inject_allowed: false,
        },
    ]
}

fn invalid_rewrite_payload(message: &Message) -> octopus_sdk_contracts::RewritePayload {
    octopus_sdk_contracts::RewritePayload::UserPrompt {
        message: message.clone(),
    }
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

fn sample_session_end() -> HookEvent {
    HookEvent::SessionEnd {
        session: SessionId("session-1".into()),
        reason: EndReason::Normal,
    }
}

fn sample_user_prompt_submit() -> HookEvent {
    HookEvent::UserPromptSubmit {
        message: sample_message("hello"),
    }
}

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
