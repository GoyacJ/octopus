use std::sync::Arc;

use octopus_sdk_contracts::AskAnswer;
use octopus_sdk_tools::{
    builtin::{SleepTool, WebFetchTool, WebSearchTool},
    Tool, ToolError,
};
use tempfile::tempdir;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

mod support;

#[tokio::test]
async fn web_fetch_strips_html_into_readable_text() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(
                    "<html><body><h1>Title</h1><script>drop()</script><p>Hello <b>world</b></p></body></html>",
                ),
        )
        .mount(&server)
        .await;

    let dir = tempdir().expect("tempdir should exist");
    let result = WebFetchTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "ok".into(),
                        text: "ok".into(),
                    }),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({ "url": format!("{}/page", server.uri()) }),
        )
        .await
        .expect("fetch should succeed");

    let text = support::text_output(result);
    assert!(text.contains("Title"));
    assert!(text.contains("Hello world"));
    assert!(!text.contains("drop()"));
}

#[tokio::test]
async fn web_fetch_truncates_large_plain_text() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/large"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/plain")
                .set_body_string("x".repeat(31_000)),
        )
        .mount(&server)
        .await;

    let dir = tempdir().expect("tempdir should exist");
    let result = WebFetchTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "ok".into(),
                        text: "ok".into(),
                    }),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({ "url": format!("{}/large", server.uri()) }),
        )
        .await
        .expect("fetch should succeed");

    assert!(support::text_output(result).contains("[content truncated after 30000 chars]"));
}

#[tokio::test]
async fn web_search_reports_w6_stub_boundary() {
    let dir = tempdir().expect("tempdir should exist");
    let error = WebSearchTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "ok".into(),
                        text: "ok".into(),
                    }),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({ "query": "latest mcp spec" }),
        )
        .await
        .expect_err("web search should stay stubbed");

    assert!(matches!(
        error,
        ToolError::NotYetImplemented {
            crate_name: "octopus-sdk-tools::web_search",
            week: "W6"
        }
    ));
    assert!(WebSearchTool::new()
        .spec()
        .description
        .starts_with("[STUB · W6]"));
    assert!(error.as_tool_result().is_error);
}

#[tokio::test]
async fn sleep_waits_with_bounded_input() {
    let dir = tempdir().expect("tempdir should exist");
    let result = SleepTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "ok".into(),
                        text: "ok".into(),
                    }),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({ "ms": 1 }),
        )
        .await
        .expect("sleep should succeed");

    assert_eq!(support::text_output(result), "slept for 1 ms");
}

#[tokio::test]
async fn sleep_rejects_values_over_limit() {
    let dir = tempdir().expect("tempdir should exist");
    let error = SleepTool::new()
        .execute(
            support::tool_context(
                dir.path(),
                Arc::new(support::StubAskResolver {
                    answer: Ok(AskAnswer {
                        prompt_id: "prompt-1".into(),
                        option_id: "ok".into(),
                        text: "ok".into(),
                    }),
                }),
                Arc::new(support::RecordingEventSink::new()),
            ),
            serde_json::json!({ "ms": 60_001 }),
        )
        .await
        .expect_err("sleep should reject oversized input");

    assert!(matches!(error, ToolError::Validation { .. }));
}
