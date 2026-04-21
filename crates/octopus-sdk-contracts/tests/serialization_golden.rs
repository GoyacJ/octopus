use std::{fs, path::PathBuf};

use octopus_sdk_contracts::{
    AskOption, AskPrompt, AskQuestion, AssistantEvent, ContentBlock, EndReason, EventId, Message,
    RenderBlock, RenderKind, RenderLifecycle, Role, SessionEvent, StopReason, ToolCallId, Usage,
};
use serde::Serialize;
use serde_json::{json, Value};

#[test]
fn serialization_matches_golden_fixtures() {
    for fixture in fixtures() {
        assert_fixture_matches(&fixture);
    }
}

struct FixtureCase {
    name: &'static str,
    value: Value,
}

fn fixtures() -> Vec<FixtureCase> {
    let tool_call_id = ToolCallId("call-123".into());
    let anchor_event_id = EventId("event-anchor".into());
    let render_event_id = EventId("event-render".into());
    let render_parent_id = EventId("event-parent".into());

    let usage = Usage {
        input_tokens: 13,
        output_tokens: 21,
        cache_creation_input_tokens: 34,
        cache_read_input_tokens: 55,
    };

    let tool_use_block = ContentBlock::ToolUse {
        id: tool_call_id.clone(),
        name: "read_file".into(),
        input: json!({
            "encoding": "utf-8",
            "path": "src/lib.rs"
        }),
    };
    let tool_result_block = ContentBlock::ToolResult {
        tool_use_id: tool_call_id.clone(),
        content: vec![ContentBlock::Text {
            text: "file contents".into(),
        }],
        is_error: false,
    };
    let render_block = RenderBlock {
        kind: RenderKind::Record,
        payload: json!({
            "title": "Execution Summary",
            "rows": [
                { "label": "changed_files", "value": "3" },
                { "href": "https://example.com/logs/1", "label": "logs", "value": "view" }
            ]
        }),
        meta: octopus_sdk_contracts::RenderMeta {
            id: render_event_id.clone(),
            parent: Some(render_parent_id),
            ts_ms: 1_713_692_800_123,
        },
    };

    vec![
        fixture_case("usage/basic", &usage),
        fixture_case(
            "assistant_event/text_delta",
            &AssistantEvent::TextDelta("partial output".into()),
        ),
        fixture_case(
            "assistant_event/tool_use",
            &AssistantEvent::ToolUse {
                id: tool_call_id.clone(),
                name: "read_file".into(),
                input: json!({
                    "path": "docs/sdk/14-ui-intent-ir.md",
                    "start_line": 1
                }),
            },
        ),
        fixture_case("assistant_event/usage", &AssistantEvent::Usage(usage)),
        fixture_case(
            "assistant_event/prompt_cache",
            &AssistantEvent::PromptCache(octopus_sdk_contracts::PromptCacheEvent {
                cache_read_input_tokens: 89,
                cache_creation_input_tokens: 144,
                breakpoint_count: 2,
            }),
        ),
        fixture_case(
            "assistant_event/message_stop",
            &AssistantEvent::MessageStop {
                stop_reason: StopReason::ToolUse,
            },
        ),
        fixture_case(
            "session_event/session_started",
            &SessionEvent::SessionStarted {
                config_snapshot_id: "cfg-2026-04-21".into(),
                effective_config_hash: "sha256:abc123".into(),
            },
        ),
        fixture_case(
            "session_event/user_message",
            &SessionEvent::UserMessage(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Summarize the workspace state.".into(),
                }],
            }),
        ),
        fixture_case(
            "session_event/assistant_message",
            &SessionEvent::AssistantMessage(Message {
                role: Role::Assistant,
                content: vec![ContentBlock::Thinking {
                    text: "Need to inspect the latest checkpoint.".into(),
                }],
            }),
        ),
        fixture_case(
            "session_event/tool_executed",
            &SessionEvent::ToolExecuted {
                call: tool_call_id.clone(),
                name: "read_file".into(),
                duration_ms: 287,
                is_error: false,
            },
        ),
        fixture_case(
            "session_event/render",
            &SessionEvent::Render {
                block: render_block.clone(),
                lifecycle: RenderLifecycle::OnToolResult,
            },
        ),
        fixture_case(
            "session_event/ask",
            &SessionEvent::Ask {
                prompt: AskPrompt {
                    kind: "ask-user".into(),
                    questions: vec![AskQuestion {
                        id: "choose-path".into(),
                        question: "Which path should the agent take?".into(),
                        header: "Path".into(),
                        multi_select: false,
                        options: vec![
                            AskOption {
                                id: "stable".into(),
                                label: "Stable".into(),
                                description: "Keep the existing typed contract.".into(),
                                preview: Some("Preserve the current SDK surface.".into()),
                                preview_format: Some("markdown".into()),
                            },
                            AskOption {
                                id: "expand".into(),
                                label: "Expand".into(),
                                description: "Adopt the richer UI intent shape.".into(),
                                preview: None,
                                preview_format: None,
                            },
                        ],
                    }],
                },
            },
        ),
        fixture_case(
            "session_event/checkpoint",
            &SessionEvent::Checkpoint {
                id: "checkpoint-1".into(),
                anchor_event_id,
            },
        ),
        fixture_case(
            "session_event/session_ended",
            &SessionEvent::SessionEnded {
                reason: EndReason::Completed,
            },
        ),
        fixture_case(
            "content_block/text",
            &ContentBlock::Text {
                text: "Rendered answer".into(),
            },
        ),
        fixture_case("content_block/tool_use", &tool_use_block),
        fixture_case("content_block/tool_result", &tool_result_block),
        fixture_case(
            "content_block/thinking",
            &ContentBlock::Thinking {
                text: "Reasoning omitted".into(),
            },
        ),
        fixture_case("render_block/record", &render_block),
    ]
}

fn fixture_case<T>(name: &'static str, value: &T) -> FixtureCase
where
    T: Serialize,
{
    FixtureCase {
        name,
        value: serde_json::to_value(value).expect("fixture should convert to JSON value"),
    }
}

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");

    for segment in name.split('/') {
        path.push(segment);
    }

    path.set_extension("json");
    path
}

fn assert_fixture_matches(fixture: &FixtureCase) {
    let path = fixture_path(fixture.name);
    let actual = format!(
        "{}\n",
        serde_json::to_string_pretty(&fixture.value).expect("fixture value should serialize")
    );

    if !path.exists() {
        let parent = path.parent().expect("fixture path should have a parent");
        fs::create_dir_all(parent).expect("fixture directory should be creatable");
        fs::write(&path, &actual).expect("fixture should be writable on first run");
        return;
    }

    let expected = fs::read_to_string(&path).expect("fixture should be readable");
    assert_eq!(actual, expected, "fixture {} diverged", fixture.name);
}
