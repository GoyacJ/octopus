use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskOption, AskPrompt, AskQuestion, AskResolver, EventSink,
    PermissionOutcome, SessionEvent, ToolCallId, ToolCallRequest,
};
use octopus_sdk_permissions::ApprovalBroker;
use serde_json::json;

struct RecordingEventSink {
    events: Arc<Mutex<Vec<SessionEvent>>>,
}

impl EventSink for RecordingEventSink {
    fn emit(&self, event: SessionEvent) {
        self.events.lock().expect("events mutex poisoned").push(event);
    }
}

struct FixedAskResolver {
    answer: Result<AskAnswer, AskError>,
}

#[async_trait]
impl AskResolver for FixedAskResolver {
    async fn resolve(&self, _prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        self.answer.clone()
    }
}

fn tool_call() -> ToolCallRequest {
    ToolCallRequest {
        id: ToolCallId("call-approval".into()),
        name: "write_file".into(),
        input: json!({
            "path": "/tmp/secret.txt",
            "content": "do not leak me"
        }),
    }
}

fn prompt() -> AskPrompt {
    AskPrompt {
        kind: "permission-approval".into(),
        questions: vec![AskQuestion {
            id: "permission-call-approval".into(),
            header: "Permission approval".into(),
            question: "Allow 'write_file' (Write) while mode is Default?".into(),
            multi_select: false,
            options: vec![
                AskOption {
                    id: "approve".into(),
                    label: "Approve".into(),
                    description: "Allow this tool call.".into(),
                    preview: None,
                    preview_format: None,
                },
                AskOption {
                    id: "deny".into(),
                    label: "Deny".into(),
                    description: "Reject this tool call.".into(),
                    preview: None,
                    preview_format: None,
                },
            ],
        }],
    }
}

#[tokio::test]
async fn approval_broker_emits_ask_and_allows_approve_answers() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let broker = ApprovalBroker::new(
        Arc::new(RecordingEventSink {
            events: Arc::clone(&events),
        }),
        Arc::new(FixedAskResolver {
            answer: Ok(AskAnswer {
                prompt_id: "approval:call-approval".into(),
                option_id: "approve".into(),
                text: "approve".into(),
            }),
        }),
    );

    let outcome = broker.request_approval(&tool_call(), prompt()).await;

    assert_eq!(outcome, PermissionOutcome::Allow);
    let recorded = events.lock().expect("events mutex poisoned");
    assert_eq!(recorded.len(), 1);
    match &recorded[0] {
        SessionEvent::Ask { prompt } => {
            assert_eq!(prompt.kind, "permission-approval");
            assert!(!prompt.questions[0].question.contains("do not leak me"));
        }
        other => panic!("expected ask event, got {other:?}"),
    }
}

#[tokio::test]
async fn approval_broker_maps_deny_answers_to_permission_denied() {
    let broker = ApprovalBroker::new(
        Arc::new(RecordingEventSink {
            events: Arc::new(Mutex::new(Vec::new())),
        }),
        Arc::new(FixedAskResolver {
            answer: Ok(AskAnswer {
                prompt_id: "approval:call-approval".into(),
                option_id: "deny".into(),
                text: "deny".into(),
            }),
        }),
    );

    let outcome = broker.request_approval(&tool_call(), prompt()).await;

    assert_eq!(
        outcome,
        PermissionOutcome::Deny {
            reason: "tool 'write_file' denied by approval option 'deny'".into(),
        }
    );
}

#[tokio::test]
async fn approval_broker_denies_when_resolver_fails() {
    let broker = ApprovalBroker::new(
        Arc::new(RecordingEventSink {
            events: Arc::new(Mutex::new(Vec::new())),
        }),
        Arc::new(FixedAskResolver {
            answer: Err(AskError::Cancelled),
        }),
    );

    let outcome = broker.request_approval(&tool_call(), prompt()).await;

    assert_eq!(
        outcome,
        PermissionOutcome::Deny {
            reason: "approval failed for 'write_file': ask prompt was cancelled before receiving an answer".into(),
        }
    );
}
