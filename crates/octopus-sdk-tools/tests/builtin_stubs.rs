use std::sync::Arc;

use octopus_sdk_contracts::AskAnswer;
use octopus_sdk_tools::{
    builtin::{AgentTool, SkillTool, TaskGetTool, TaskListTool},
    Tool, ToolError,
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

    for tool in [
        Arc::new(AgentTool::new()) as Arc<dyn Tool>,
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
}
