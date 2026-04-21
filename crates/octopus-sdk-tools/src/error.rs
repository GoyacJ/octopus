use octopus_sdk_contracts::ContentBlock;
use thiserror::Error;

use crate::ToolResult;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool validation failed: {message}")]
    Validation { message: String },
    #[error("tool permission denied: {message}")]
    Permission { message: String },
    #[error("tool execution failed: {message}")]
    Execution { message: String },
    #[error("tool timed out: {message}")]
    Timeout { message: String },
    #[error("tool was cancelled: {message}")]
    Cancelled { message: String },
    #[error("tool is not yet implemented in {crate_name} ({week})")]
    NotYetImplemented {
        crate_name: &'static str,
        week: &'static str,
    },
    #[error("tool transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("tool serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("tool sandbox error: {reason}")]
    Sandbox { reason: String },
}

impl ToolError {
    #[must_use]
    pub fn as_tool_result(&self) -> ToolResult {
        let remediation = match self {
            Self::Validation { .. } => "fix the tool input and retry",
            Self::Permission { .. } => "adjust permission mode or approve the call",
            Self::Execution { .. } => "inspect the runtime state and retry",
            Self::Timeout { .. } => "retry with a shorter-running request",
            Self::Cancelled { .. } => "rerun the tool if work should continue",
            Self::NotYetImplemented { crate_name, week } => {
                return ToolResult {
                    content: vec![ContentBlock::Text {
                        text: format!(
                            "{self}\nremediation: wait for {week} work in {crate_name} to land"
                        ),
                    }],
                    is_error: true,
                    duration_ms: 0,
                    render: None,
                };
            }
            Self::Transport(_) => "retry after the transport path is healthy",
            Self::Serialization(_) => "inspect the payload schema and retry",
            Self::Sandbox { .. } => "adjust sandbox configuration or run in a looser mode",
        };

        ToolResult {
            content: vec![ContentBlock::Text {
                text: format!("{self}\nremediation: {remediation}"),
            }],
            is_error: true,
            duration_ms: 0,
            render: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegistryError {
    #[error("tool `{0}` is already registered")]
    DuplicateName(String),
    #[error("tool spec is invalid: {0}")]
    InvalidSpec(String),
}

#[cfg(test)]
mod tests {
    use super::{RegistryError, ToolError};

    #[test]
    fn tool_error_becomes_boundary_result() {
        let result = ToolError::Validation {
            message: "missing path".into(),
        }
        .as_tool_result();

        assert!(result.is_error);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            octopus_sdk_contracts::ContentBlock::Text { text } => {
                assert!(text.contains("tool validation failed: missing path"));
                assert!(text.contains("remediation:"));
            }
            other => panic!("expected text block, got {other:?}"),
        }
    }

    #[test]
    fn registry_error_duplicate_name_stays_stable() {
        let error = RegistryError::DuplicateName("grep".into());
        assert_eq!(error.to_string(), "tool `grep` is already registered");
    }
}
