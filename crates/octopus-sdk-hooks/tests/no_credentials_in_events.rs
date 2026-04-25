use std::sync::Arc;

use async_trait::async_trait;
use octopus_sdk_contracts::{
    HookDecision, HookEvent, SessionEvent, ToolCallId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_hooks::runner::{Hook, HookRunner, HookSource};
use serde_json::json;

const SECRET_VALUE: &str = "secret-xyz";

struct SecretRewriteHook;

#[async_trait]
impl Hook for SecretRewriteHook {
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "secret-rewrite"
    }

    async fn on_event(&self, event: &HookEvent) -> HookDecision {
        match event {
            HookEvent::PreToolUse { call, .. } => {
                HookDecision::Rewrite(octopus_sdk_contracts::RewritePayload::ToolCall {
                    call: ToolCallRequest {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        input: json!({
                            "command": "curl https://example.invalid",
                            "api_key": SECRET_VALUE,
                        }),
                    },
                })
            }
            _ => HookDecision::Continue,
        }
    }
}

#[tokio::test]
async fn tool_executed_events_do_not_serialize_secret_input() {
    let runner = HookRunner::new();
    runner.register(
        "rewrite",
        Arc::new(SecretRewriteHook),
        HookSource::Workspace,
        10,
    );

    let outcome = runner
        .run(HookEvent::PreToolUse {
            call: ToolCallRequest {
                id: ToolCallId("call-secret".into()),
                name: "shell_exec".into(),
                input: json!({ "command": "echo safe" }),
            },
            category: ToolCategory::Shell,
        })
        .await
        .expect("hook run should succeed");

    let rewritten_call = match outcome.final_payload {
        Some(octopus_sdk_contracts::RewritePayload::ToolCall { call }) => call,
        other => panic!("expected rewritten tool call, got {other:?}"),
    };

    let event = SessionEvent::ToolExecuted {
        call: rewritten_call.id,
        name: rewritten_call.name,
        duration_ms: 7,
        is_error: false,
    };
    let serialized = serde_json::to_string(&event).expect("event should serialize");

    assert!(!serialized.contains(SECRET_VALUE));
    assert!(!serialized.contains("api_key"));
}
