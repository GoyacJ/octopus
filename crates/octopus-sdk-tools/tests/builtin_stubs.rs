use std::sync::Arc;

use octopus_sdk_contracts::AskAnswer;
use octopus_sdk_tools::{
    builtin::{
        builtin_tool_catalog, register_builtins, AgentTool, SkillTool, TaskGetTool, TaskListTool,
        WebSearchTool,
    },
    Tool, ToolError, ToolRegistry,
};
use tempfile::tempdir;

mod support;

#[tokio::test]
async fn w5_stub_tools_stay_unimplemented() {
    let dir = tempdir().expect("tempdir should exist");
    let ctx = || {
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
        )
    };

    let agent = AgentTool::new();
    assert!(agent.spec().description.starts_with("[STUB · W5]"));
    let result = agent
        .execute(ctx(), serde_json::json!({}))
        .await
        .expect("agent tool should surface a boundary error result");
    assert!(result.is_error);
    assert!(support::text_output(result).contains("TaskFn not injected"));

    for tool in [
        Arc::new(SkillTool::new()) as Arc<dyn Tool>,
        Arc::new(TaskListTool::new()) as Arc<dyn Tool>,
        Arc::new(TaskGetTool::new()) as Arc<dyn Tool>,
    ] {
        assert!(tool.spec().description.starts_with("[STUB · W5]"));
        let error = tool
            .execute(ctx(), serde_json::json!({}))
            .await
            .expect_err("stub should stay unimplemented");

        assert!(matches!(
            error,
            ToolError::NotYetImplemented { week: "W5", .. }
        ));
        assert!(error.as_tool_result().is_error);
    }

    let web_search = WebSearchTool::new();
    let error = web_search
        .execute(ctx(), serde_json::json!({ "query": "octopus sdk" }))
        .await
        .expect_err("web_search should stay unimplemented until a provider exists");
    assert!(matches!(
        error,
        ToolError::NotYetImplemented { week: "W6", .. }
    ));
    assert!(error.as_tool_result().is_error);
}

#[test]
fn live_builtin_registry_and_catalog_hide_stub_only_tools() {
    let mut registry = ToolRegistry::new();
    register_builtins(&mut registry).expect("live builtins should register");

    let catalog = builtin_tool_catalog();
    for name in ["web_search", "task", "skill", "task_list", "task_get"] {
        assert!(
            registry.get(name).is_none(),
            "{name} should stay out of the live builtin registry"
        );
        assert!(
            catalog.resolve(name).is_none(),
            "{name} should stay out of the live builtin catalog"
        );
    }
}
