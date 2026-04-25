use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    AskOption, AskPrompt, AskQuestion, AssistantEvent, CacheBreakpoint, CacheTtl, ContentBlock,
    HookDecision, HookEvent, PermissionOutcome, Role, StopReason,
};
use octopus_sdk_core::SubmitTurnInput;
use octopus_sdk_hooks::{Hook, HookSource};
use octopus_sdk_model::{CacheControlStrategy, ModelError};
use octopus_sdk_observability::{TraceSpan, TraceValue, Tracer};
use octopus_sdk_plugin::PluginRegistry;
use serde_json::json;

mod support;

struct RecordingTracer {
    spans: Mutex<Vec<TraceSpan>>,
}

impl RecordingTracer {
    fn new() -> Self {
        Self {
            spans: Mutex::new(Vec::new()),
        }
    }

    fn spans(&self) -> Vec<TraceSpan> {
        self.spans
            .lock()
            .expect("spans lock should stay available")
            .clone()
    }
}

impl Tracer for RecordingTracer {
    fn record(&self, span: TraceSpan) {
        self.spans
            .lock()
            .expect("spans lock should stay available")
            .push(span);
    }
}

struct InjectOnceStopHook {
    injected: Mutex<bool>,
    message: &'static str,
}

#[async_trait]
impl Hook for InjectOnceStopHook {
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "inject-once-stop"
    }

    async fn on_event(&self, event: &HookEvent) -> HookDecision {
        if !matches!(event, HookEvent::Stop { .. }) {
            return HookDecision::Continue;
        }

        let mut injected = self
            .injected
            .lock()
            .expect("stop hook state should stay available");
        if *injected {
            return HookDecision::Continue;
        }
        *injected = true;

        HookDecision::InjectMessage(support::text_message(self.message))
    }
}

#[tokio::test]
async fn test_end_turn_without_tools() {
    let (root, store) = support::temp_store();
    let runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![vec![
            AssistantEvent::TextDelta("assistant reply".into()),
            AssistantEvent::Usage(support::usage(12, 8)),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ]])),
        store,
    )
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("hello"),
        })
        .await
        .expect("turn should complete");

    let events = support::collect_events(&runtime, &handle.session_id).await;
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::AssistantMessage(message)
            if message.role == Role::Assistant
                && message.content.iter().any(|block| matches!(block, ContentBlock::Text { text } if text == "assistant reply"))
    )));
    assert!(events
        .iter()
        .any(|event| matches!(event, octopus_sdk_contracts::SessionEvent::Render { .. })));
    let snapshot = runtime
        .snapshot(&handle.session_id)
        .await
        .expect("snapshot should load");
    assert_eq!(snapshot.usage.input_tokens, 12);
    assert_eq!(snapshot.usage.output_tokens, 8);
}

#[tokio::test]
async fn test_main_request_sets_prompt_cache_metadata() {
    let (root, store) = support::temp_store();
    let provider = Arc::new(support::ScriptedModelProvider::new(vec![vec![
        AssistantEvent::TextDelta("cached reply".into()),
        AssistantEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        },
    ]]));
    let runtime = support::runtime_builder(provider.clone(), store)
        .build()
        .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("hello"),
        })
        .await
        .expect("turn should complete");

    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].cache_breakpoints,
        vec![
            CacheBreakpoint {
                position: 0,
                ttl: CacheTtl::OneHour,
            },
            CacheBreakpoint {
                position: 1,
                ttl: CacheTtl::FiveMinutes,
            },
            CacheBreakpoint {
                position: 2,
                ttl: CacheTtl::FiveMinutes,
            },
        ]
    );
    assert_eq!(
        requests[0].cache_control,
        CacheControlStrategy::PromptCaching {
            breakpoints: vec!["system", "tools", "first_user"],
        }
    );
}

#[tokio::test]
async fn test_bash_tool_round_trip() {
    let (root, store) = support::temp_store();
    let tracer = Arc::new(RecordingTracer::new());
    let runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![
            vec![
                AssistantEvent::ToolUse {
                    id: octopus_sdk_contracts::ToolCallId("call-bash".into()),
                    name: "bash".into(),
                    input: json!({ "command": "printf 'bash ok'" }),
                },
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::ToolUse,
                },
            ],
            vec![
                AssistantEvent::TextDelta("tool complete".into()),
                AssistantEvent::MessageStop {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        ])),
        store,
    )
    .with_tracer(tracer.clone())
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("run bash"),
        })
        .await
        .expect("tool round trip should complete");

    let events = support::collect_events(&runtime, &handle.session_id).await;
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::ToolExecuted { name, is_error, .. }
            if name == "bash" && !is_error
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::AssistantMessage(message)
            if message.role == Role::Tool
                && message.content.iter().any(|block| matches!(block, ContentBlock::ToolResult { content, .. } if content.iter().any(|child| matches!(child, ContentBlock::Text { text } if text.contains("bash ok")))))
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::AssistantMessage(message)
            if message.role == Role::Assistant
                && message.content.iter().any(|block| matches!(block, ContentBlock::Text { text } if text == "tool complete"))
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::Render { lifecycle, blocks }
            if lifecycle.phase == octopus_sdk_contracts::RenderPhase::OnToolProgress
                && blocks.iter().any(|block| block.payload.to_string().contains("sandbox_ready"))
    )));

    let expected_trace_id = format!("trace:{}", handle.session_id.0);
    let expected_parent_span_id = format!("session:{}", handle.session_id.0);
    let spans = tracer.spans();
    assert!(spans.iter().any(|span| {
        span.name == "tool_execution"
            && span.trace_id.as_deref() == Some(expected_trace_id.as_str())
            && span.span_id.as_deref() == Some("tool:call-bash")
            && span.parent_span_id.as_deref() == Some(expected_parent_span_id.as_str())
            && span.agent_role.as_deref() == Some("main")
            && span.fields.get("tool_call_id") == Some(&TraceValue::String("call-bash".into()))
            && span.fields.get("permission_mode") == Some(&TraceValue::String("Default".into()))
            && span.fields.get("permission_decision") == Some(&TraceValue::String("Allow".into()))
            && span.fields.get("sandbox_backend") == Some(&TraceValue::String("noop".into()))
            && span.fields.get("config_snapshot_id") == Some(&TraceValue::String("cfg-1".into()))
    }));
}

#[tokio::test]
async fn test_stop_hook_continues_with_injected_message() {
    let (root, store) = support::temp_store();
    let provider = Arc::new(support::ScriptedModelProvider::new(vec![
        vec![
            AssistantEvent::TextDelta("first answer".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
        vec![
            AssistantEvent::TextDelta("continued answer".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
    ]));
    let plugin_registry = PluginRegistry::new();
    plugin_registry.hooks().register(
        "inject-stop",
        Arc::new(InjectOnceStopHook {
            injected: Mutex::new(false),
            message: "continue from stop hook",
        }),
        HookSource::Session,
        0,
    );
    let runtime = support::runtime_builder(provider.clone(), store)
        .with_plugin_registry(plugin_registry)
        .build()
        .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("hello"),
        })
        .await
        .expect("stop hook continuation should complete");

    let events = support::collect_events(&runtime, &handle.session_id).await;
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::AssistantMessage(message)
            if message.role == Role::Assistant
                && message.content.iter().any(|block| matches!(block, ContentBlock::Text { text } if text == "continued answer"))
    )));

    let requests = provider.requests();
    assert_eq!(requests.len(), 2);
    assert!(flatten_request_text(&requests[1].messages).contains("continue from stop hook"));
}

#[tokio::test]
async fn test_resume_replays_compaction_summary_instead_of_folded_prefix() {
    let (root, store) = support::temp_store();
    let initial_provider = Arc::new(support::ScriptedModelProvider::new(vec![
        vec![
            AssistantEvent::ToolUse {
                id: octopus_sdk_contracts::ToolCallId("call-bash-compact".into()),
                name: "bash".into(),
                input: json!({ "command": "printf 'tool payload that should disappear'" }),
            },
            AssistantEvent::MessageStop {
                stop_reason: StopReason::ToolUse,
            },
        ],
        vec![
            AssistantEvent::TextDelta("SUMMARY: folded early turns".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
        vec![
            AssistantEvent::TextDelta("post compact reply".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
    ]));
    let runtime = support::runtime_builder(initial_provider, store.clone())
        .build()
        .expect("runtime should build");

    let mut start = support::start_input(&root);
    start.token_budget = 1;
    let handle = runtime
        .start_session(start)
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("very long prefix user prompt that must be folded"),
        })
        .await
        .expect("initial turn should compact transcript");

    let initial_events = support::collect_events(&runtime, &handle.session_id).await;
    assert!(initial_events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::Checkpoint {
            compaction: Some(result),
            ..
        } if result.summary == "SUMMARY: folded early turns"
    )));

    let resumed_provider = Arc::new(support::ScriptedModelProvider::new(vec![
        vec![
            AssistantEvent::TextDelta("R".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
        vec![
            AssistantEvent::TextDelta("resumed reply".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
    ]));
    let resumed_runtime = support::runtime_builder(resumed_provider.clone(), store)
        .build()
        .expect("runtime should build");
    resumed_runtime
        .resume(&handle.session_id)
        .await
        .expect("resume should succeed");
    resumed_runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("continue after resume"),
        })
        .await
        .expect("resumed turn should complete");

    let resumed_requests = resumed_provider.requests();
    assert_eq!(resumed_requests.len(), 2);
    assert_eq!(resumed_requests[0].model.0, "compact");
    assert_eq!(resumed_requests[1].model.0, "test-model");

    let request_text = flatten_request_text(&resumed_requests[1].messages);

    assert!(request_text.contains('R'));
    assert!(request_text.contains("continue after resume"));
    assert!(!request_text.contains("very long prefix user prompt that must be folded"));
}

#[tokio::test]
async fn test_token_budget_compaction_continues_sampling() {
    let (root, store) = support::temp_store();
    let provider = Arc::new(support::ScriptedModelProvider::new(vec![
        vec![
            AssistantEvent::TextDelta("seed reply".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
        vec![
            AssistantEvent::TextDelta("S".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
        vec![
            AssistantEvent::TextDelta("after budget continuation".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
    ]));
    let runtime = support::runtime_builder(provider.clone(), store)
        .build()
        .expect("runtime should build");

    let mut start = support::start_input(&root);
    start.token_budget = 1;
    let handle = runtime
        .start_session(start)
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("seed enough history for budget continuation"),
        })
        .await
        .expect("seed turn should complete");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message(
                "a very long prompt that should force compaction before sampling",
            ),
        })
        .await
        .expect("budget continuation should complete");

    let requests = provider.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].model.0, "test-model");
    assert_eq!(requests[1].model.0, "compact");
    assert_eq!(requests[2].model.0, "test-model");

    let resumed_text = flatten_request_text(&requests[2].messages);
    assert!(resumed_text.contains('S'));
    assert!(
        resumed_text.contains("a very long prompt that should force compaction before sampling")
    );
}

#[tokio::test]
async fn test_permission_approval_executes_tool_after_approve() {
    let (root, store) = support::temp_store();
    let provider = Arc::new(support::ScriptedModelProvider::new(vec![
        vec![
            AssistantEvent::ToolUse {
                id: octopus_sdk_contracts::ToolCallId("call-bash-approval".into()),
                name: "bash".into(),
                input: json!({ "command": "printf 'approved tool'" }),
            },
            AssistantEvent::MessageStop {
                stop_reason: StopReason::ToolUse,
            },
        ],
        vec![
            AssistantEvent::TextDelta("after approval".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
    ]));
    let runtime = support::runtime_builder_with_controls(
        provider,
        store,
        Arc::new(support::FixedPermissionGate {
            outcome: PermissionOutcome::AskApproval {
                prompt: approval_prompt("permission-approval"),
            },
        }),
        Arc::new(support::FixedAskResolver {
            option_id: "approve",
            text: "approved",
        }),
    )
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("run guarded bash"),
        })
        .await
        .expect("approved tool should execute");

    let events = support::collect_events(&runtime, &handle.session_id).await;
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::Ask { prompt } if prompt.kind == "permission-approval"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::ToolExecuted { name, is_error, .. }
            if name == "bash" && !is_error
    )));
}

#[tokio::test]
async fn test_prompt_too_long_recovers_via_compaction() {
    let (root, store) = support::temp_store();
    let provider = Arc::new(support::ScriptedModelProvider::with_turns(vec![
        support::ScriptedTurn::Error(ModelError::PromptTooLong {
            estimated_tokens: 256,
            max: 16,
        }),
        support::ScriptedTurn::Events(vec![
            AssistantEvent::TextDelta("S".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ]),
        support::ScriptedTurn::Events(vec![
            AssistantEvent::TextDelta("after prompt-too-long recovery".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ]),
    ]));
    let runtime = support::runtime_builder(provider.clone(), store)
        .build()
        .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("oversized request payload for recovery"),
        })
        .await
        .expect("prompt-too-long recovery should complete");

    let requests = provider.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].model.0, "test-model");
    assert_eq!(requests[1].model.0, "compact");
    assert_eq!(requests[2].model.0, "test-model");

    let resumed_text = flatten_request_text(&requests[2].messages);
    assert!(resumed_text.contains('S'));
    assert!(!resumed_text.contains("oversized request payload for recovery"));
}

#[tokio::test]
async fn test_permission_denial_returns_tool_error_without_execution() {
    let (root, store) = support::temp_store();
    let provider = Arc::new(support::ScriptedModelProvider::new(vec![
        vec![
            AssistantEvent::ToolUse {
                id: octopus_sdk_contracts::ToolCallId("call-bash-deny".into()),
                name: "bash".into(),
                input: json!({ "command": "printf 'should not run'" }),
            },
            AssistantEvent::MessageStop {
                stop_reason: StopReason::ToolUse,
            },
        ],
        vec![
            AssistantEvent::TextDelta("handled denied tool".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ],
    ]));
    let runtime = support::runtime_builder_with_controls(
        provider,
        store,
        Arc::new(support::FixedPermissionGate {
            outcome: PermissionOutcome::RequireAuth {
                prompt: approval_prompt("require-auth"),
            },
        }),
        Arc::new(support::FixedAskResolver {
            option_id: "deny",
            text: "denied",
        }),
    )
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("run denied bash"),
        })
        .await
        .expect("denied tool should still complete loop");

    let events = support::collect_events(&runtime, &handle.session_id).await;
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::Ask { prompt } if prompt.kind == "require-auth"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::ToolExecuted { name, is_error, .. }
            if name == "bash" && *is_error
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::AssistantMessage(message)
            if message.role == Role::Tool
                && message.content.iter().any(|block| matches!(block, ContentBlock::ToolResult { content, is_error, .. }
                    if *is_error && content.iter().any(|child| matches!(child, ContentBlock::Text { text } if text.contains("denied")))))
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::Render { lifecycle, blocks }
            if lifecycle.phase == octopus_sdk_contracts::RenderPhase::OnToolRejected
                && blocks.iter().any(|block| {
                    block.payload
                        .to_string()
                        .contains("retry after approving the request")
                })
    )));
}

#[tokio::test]
async fn test_overloaded_request_retries_once_and_recovers() {
    let (root, store) = support::temp_store();
    let provider = Arc::new(support::ScriptedModelProvider::with_turns(vec![
        support::ScriptedTurn::Error(ModelError::Overloaded {
            retry_after_ms: Some(0),
        }),
        support::ScriptedTurn::Events(vec![
            AssistantEvent::TextDelta("after overload retry".into()),
            AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            },
        ]),
    ]));
    let runtime = support::runtime_builder(provider.clone(), store)
        .build()
        .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("retry after overload"),
        })
        .await
        .expect("overflow retry should recover");

    let requests = provider.requests();
    assert_eq!(requests.len(), 2);

    let events = support::collect_events(&runtime, &handle.session_id).await;
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::AssistantMessage(message)
            if message.role == Role::Assistant
                && message.content.iter().any(|block| matches!(block, ContentBlock::Text { text } if text == "after overload retry"))
    )));
}

fn approval_prompt(kind: &str) -> AskPrompt {
    AskPrompt {
        kind: kind.into(),
        questions: vec![AskQuestion {
            id: "approval-q".into(),
            header: "Approval".into(),
            question: "Proceed?".into(),
            multi_select: false,
            options: vec![
                AskOption {
                    id: "approve".into(),
                    label: "Approve".into(),
                    description: "Allow the tool call.".into(),
                    preview: None,
                    preview_format: None,
                },
                AskOption {
                    id: "deny".into(),
                    label: "Deny".into(),
                    description: "Reject the tool call.".into(),
                    preview: None,
                    preview_format: None,
                },
            ],
        }],
    }
}

fn flatten_request_text(messages: &[octopus_sdk_contracts::Message]) -> String {
    let mut parts = Vec::new();

    for message in messages {
        for block in &message.content {
            match block {
                ContentBlock::Text { text } => parts.push(text.clone()),
                ContentBlock::ToolResult { content, .. } => {
                    let joined = content
                        .iter()
                        .filter_map(|child| match child {
                            ContentBlock::Text { text } => Some(text.as_str()),
                            ContentBlock::Thinking { .. }
                            | ContentBlock::ToolUse { .. }
                            | ContentBlock::ToolResult { .. } => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !joined.is_empty() {
                        parts.push(joined);
                    }
                }
                ContentBlock::Thinking { .. } | ContentBlock::ToolUse { .. } => {}
            }
        }
    }

    parts.join("\n")
}
