use std::sync::Arc;

use octopus_sdk_contracts::{AskAnswer, AskError, SessionEvent};
use octopus_sdk_tools::{
    builtin::{AskUserQuestionTool, TodoWriteTool},
    Tool, ToolError,
};
use tempfile::tempdir;

mod support;

#[tokio::test]
async fn ask_user_question_returns_resolved_answer_text() {
    let dir = tempdir().expect("tempdir should exist");
    let events = Arc::new(support::RecordingEventSink::new());
    let result = AskUserQuestionTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "approve".into(),
                        text: "Proceed with the migration".into(),
                    }),
                }),
                events.clone(),
            ),
            serde_json::json!({
                "promptId": "prompt-1",
                "questions": [{
                    "id": "q-1",
                    "question": "Choose a path",
                    "header": "Path",
                    "multiSelect": false,
                    "options": [
                        { "id": "approve", "label": "Approve", "description": "Proceed", "preview": "Plan A", "previewFormat": "markdown" },
                        { "id": "deny", "label": "Deny", "description": "Stop" }
                    ]
                }]
            }),
        )
        .await
        .expect("ask should succeed");

    assert!(support::text_output(result).contains("Proceed with the migration"));
    assert!(events
        .events()
        .into_iter()
        .any(|event| matches!(event, SessionEvent::Ask { .. })));
}

#[tokio::test]
async fn ask_user_question_rejects_invalid_prompt_shapes() {
    let dir = tempdir().expect("tempdir should exist");
    let error = AskUserQuestionTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "approve".into(),
                        text: "Proceed".into(),
                    }),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({
                "questions": [{
                    "id": "q-1",
                    "question": "Choose a path",
                    "header": "Path",
                    "multiSelect": true,
                    "options": [
                        { "id": "approve", "label": "Approve", "description": "Proceed", "preview": "Plan A", "previewFormat": "markdown" },
                        { "id": "deny", "label": "Deny", "description": "Stop" }
                    ]
                }]
            }),
        )
        .await
        .expect_err("invalid prompt should fail");

    assert!(matches!(error, ToolError::Validation { .. }));
}

#[tokio::test]
async fn ask_user_question_maps_resolver_errors() {
    let dir = tempdir().expect("tempdir should exist");
    let error = AskUserQuestionTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Err(AskError::NotResolvable),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({
                "questions": [{
                    "id": "q-1",
                    "question": "Choose a path",
                    "header": "Path",
                    "multiSelect": false,
                    "options": [
                        { "id": "approve", "label": "Approve", "description": "Proceed" },
                        { "id": "deny", "label": "Deny", "description": "Stop" }
                    ]
                }]
            }),
        )
        .await
        .expect_err("unresolvable prompt should fail");

    assert!(matches!(error, ToolError::Execution { .. }));
}

#[tokio::test]
async fn todo_write_emits_render_event() {
    let dir = tempdir().expect("tempdir should exist");
    let events = Arc::new(support::RecordingEventSink::new());
    let result = TodoWriteTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "approve".into(),
                        text: "Proceed".into(),
                    }),
                }),
                events.clone(),
            ),
            serde_json::json!({
                "todos": [
                    { "content": "Implement transport", "activeForm": "Implementing transport", "status": "in_progress" },
                    { "content": "Verify tests", "activeForm": "Verifying tests", "status": "pending" }
                ]
            }),
        )
        .await
        .expect("todo write should succeed");

    assert_eq!(support::text_output(result), "updated 2 todos");
    assert!(events
        .events()
        .into_iter()
        .any(|event| matches!(event, SessionEvent::Render { .. })));
}
