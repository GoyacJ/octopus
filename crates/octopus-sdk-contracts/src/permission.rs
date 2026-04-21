use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{AskPrompt, ToolCallId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: ToolCallId,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    Plan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOutcome {
    Allow,
    Deny { reason: String },
    AskApproval { prompt: AskPrompt },
    // W4 to add: RequireAuth { prompt: AskPrompt }
}

#[async_trait]
pub trait PermissionGate: Send + Sync {
    async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome;
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{PermissionMode, PermissionOutcome, ToolCallRequest};
    use crate::{AskOption, AskPrompt, AskQuestion, ToolCallId};

    #[test]
    fn permission_mode_bypass_permissions_round_trips() {
        let value = serde_json::to_value(PermissionMode::BypassPermissions)
            .expect("permission mode should serialize");

        assert_eq!(value, Value::String("bypass_permissions".into()));

        let roundtrip: PermissionMode =
            serde_json::from_value(value).expect("permission mode should deserialize");
        assert_eq!(roundtrip, PermissionMode::BypassPermissions);
    }

    #[test]
    fn permission_outcome_ask_approval_keeps_prompt_payload() {
        let outcome = PermissionOutcome::AskApproval {
            prompt: AskPrompt {
                kind: "ask-user".into(),
                questions: vec![AskQuestion {
                    id: "question-1".into(),
                    question: "Proceed?".into(),
                    header: "Approval".into(),
                    multi_select: false,
                    options: vec![AskOption {
                        id: "approve".into(),
                        label: "Approve".into(),
                        description: "Allow the call".into(),
                        preview: None,
                        preview_format: None,
                    }],
                }],
            },
        };

        let value = serde_json::to_value(&outcome).expect("outcome should serialize");

        assert_eq!(
            value["ask_approval"]["prompt"]["questions"][0]["id"],
            "question-1"
        );
    }

    #[test]
    fn tool_call_request_round_trips_minimal_shape() {
        let call = ToolCallRequest {
            id: ToolCallId("call-1".into()),
            name: "bash".into(),
            input: json!({ "command": "pwd" }),
        };

        let value = serde_json::to_value(&call).expect("tool call request should serialize");

        assert_eq!(value["id"], "call-1");
        assert_eq!(value["name"], "bash");
        assert_eq!(value["input"]["command"], "pwd");
    }
}
