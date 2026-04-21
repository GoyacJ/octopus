use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, EventSink, PermissionGate, PermissionMode,
    SessionEvent, ToolCallId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_permissions::{
    ApprovalBroker, DefaultPermissionGate, PermissionBehavior, PermissionPolicy, PermissionRule,
    PermissionRuleSource,
};
use serde_json::json;

const SECRET_VALUE: &str = "secret-xyz";

struct RecordingEventSink {
    events: Arc<Mutex<Vec<SessionEvent>>>,
}

impl EventSink for RecordingEventSink {
    fn emit(&self, event: SessionEvent) {
        self.events
            .lock()
            .expect("events mutex poisoned")
            .push(event);
    }
}

struct FixedAskResolver;

#[async_trait]
impl AskResolver for FixedAskResolver {
    async fn resolve(&self, _prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Ok(AskAnswer {
            prompt_id: "approval:call-secret".into(),
            option_id: "approve".into(),
            text: "approve".into(),
        })
    }
}

#[tokio::test]
async fn approval_events_do_not_serialize_secret_input() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let gate = DefaultPermissionGate {
        policy: PermissionPolicy::from_sources(vec![PermissionRule {
            source: PermissionRuleSource::Session,
            behavior: PermissionBehavior::Ask,
            tool_name: "shell_exec".into(),
            rule_content: Some("approval required".into()),
        }]),
        mode: PermissionMode::Default,
        broker: Arc::new(ApprovalBroker::new(
            Arc::new(RecordingEventSink {
                events: Arc::clone(&events),
            }),
            Arc::new(FixedAskResolver),
        )),
        category_resolver: Arc::new(|_| ToolCategory::Shell),
    };

    let outcome = gate
        .check(&ToolCallRequest {
            id: ToolCallId("call-secret".into()),
            name: "shell_exec".into(),
            input: json!({
                "command": "curl https://example.invalid",
                "api_key": SECRET_VALUE,
            }),
        })
        .await;

    assert_eq!(outcome, octopus_sdk_contracts::PermissionOutcome::Allow);

    let serialized = events
        .lock()
        .expect("events mutex poisoned")
        .iter()
        .map(|event| serde_json::to_string(event).expect("event should serialize"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(!serialized.contains(SECRET_VALUE));
    assert!(!serialized.contains("api_key"));
}
