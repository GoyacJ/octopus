use std::sync::Arc;

use octopus_sdk_contracts::{
    AskOption, AskPrompt, AskQuestion, AssistantEvent, ContentBlock, PermissionOutcome, Role,
    StopReason,
};
use octopus_sdk_core::SubmitTurnInput;
use serde_json::json;

mod support;

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
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::Render { .. }
    )));
    let snapshot = runtime
        .snapshot(&handle.session_id)
        .await
        .expect("snapshot should load");
    assert_eq!(snapshot.usage.input_tokens, 12);
    assert_eq!(snapshot.usage.output_tokens, 8);
}

#[tokio::test]
async fn test_bash_tool_round_trip() {
    let (root, store) = support::temp_store();
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

    let resumed_provider = Arc::new(support::ScriptedModelProvider::new(vec![vec![
        AssistantEvent::TextDelta("resumed reply".into()),
        AssistantEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        },
    ]]));
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

    let resumed_request = resumed_provider
        .requests()
        .into_iter()
        .next()
        .expect("resumed request should be captured");
    let request_text = flatten_request_text(&resumed_request.messages);

    assert!(request_text.contains("SUMMARY: folded early turns"));
    assert!(request_text.contains("continue after resume"));
    assert!(!request_text.contains("very long prefix user prompt that must be folded"));
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
