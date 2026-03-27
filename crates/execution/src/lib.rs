use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionAction {
    EmitText {
        content: String,
    },
    ConnectorCall {
        tool_name: String,
        arguments: Value,
    },
    FailOnceThenEmitText {
        failure_message: String,
        content: String,
    },
    AlwaysFail {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSuccess {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionFailure {
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Succeeded(ExecutionSuccess),
    Failed(ExecutionFailure),
}

pub struct ExecutionEngine;

impl ExecutionEngine {
    pub fn execute(action: &ExecutionAction, attempt: u32) -> ExecutionOutcome {
        match action {
            ExecutionAction::EmitText { content } => {
                ExecutionOutcome::Succeeded(ExecutionSuccess {
                    content: content.clone(),
                })
            }
            ExecutionAction::ConnectorCall { .. } => ExecutionOutcome::Failed(ExecutionFailure {
                message: "connector_call_must_use_gateway".to_string(),
                retryable: false,
            }),
            ExecutionAction::FailOnceThenEmitText {
                failure_message,
                content,
            } => {
                if attempt == 1 {
                    ExecutionOutcome::Failed(ExecutionFailure {
                        message: failure_message.clone(),
                        retryable: true,
                    })
                } else {
                    ExecutionOutcome::Succeeded(ExecutionSuccess {
                        content: content.clone(),
                    })
                }
            }
            ExecutionAction::AlwaysFail { message } => ExecutionOutcome::Failed(ExecutionFailure {
                message: message.clone(),
                retryable: false,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ExecutionAction, ExecutionEngine, ExecutionOutcome};

    #[test]
    fn fail_once_then_emit_text_only_fails_on_first_attempt() {
        let action = ExecutionAction::FailOnceThenEmitText {
            failure_message: "network_glitch".into(),
            content: "artifact".into(),
        };

        let first = ExecutionEngine::execute(&action, 1);
        let second = ExecutionEngine::execute(&action, 2);

        assert!(matches!(first, ExecutionOutcome::Failed(_)));
        assert!(matches!(second, ExecutionOutcome::Succeeded(_)));
    }

    #[test]
    fn always_fail_never_becomes_retryable() {
        let action = ExecutionAction::AlwaysFail {
            message: "irrecoverable".into(),
        };

        let first = ExecutionEngine::execute(&action, 1);
        assert!(matches!(
            first,
            ExecutionOutcome::Failed(ref failure) if !failure.retryable
        ));
    }

    #[test]
    fn connector_calls_must_be_dispatched_outside_builtin_engine() {
        let action = ExecutionAction::ConnectorCall {
            tool_name: "emit_text".into(),
            arguments: json!({ "content": "hello" }),
        };

        let first = ExecutionEngine::execute(&action, 1);
        assert!(matches!(
            first,
            ExecutionOutcome::Failed(ref failure)
                if failure.message == "connector_call_must_use_gateway" && !failure.retryable
        ));
    }
}
