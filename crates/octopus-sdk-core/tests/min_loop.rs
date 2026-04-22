use std::sync::Arc;

use octopus_sdk_contracts::{AssistantEvent, ContentBlock, Role, StopReason};
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
