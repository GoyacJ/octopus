use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum McpError {
    #[error("mcp transport failed: {message}")]
    Transport { message: String },
    #[error("mcp protocol violation: {message}")]
    Protocol { message: String },
    #[error("mcp request timed out: {message}")]
    Timeout { message: String },
    #[error("mcp handshake failed: {message}")]
    Handshake { message: String },
    #[error("mcp server `{server_id}` crashed with exit code {exit_code:?}")]
    ServerCrashed {
        server_id: String,
        exit_code: Option<i32>,
    },
    #[error("mcp tool not found: {name}")]
    ToolNotFound { name: String },
    #[error("mcp invalid response: {body_preview}")]
    InvalidResponse { body_preview: String },
}

impl From<reqwest::Error> for McpError {
    fn from(value: reqwest::Error) -> Self {
        Self::Transport {
            message: value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::McpError;

    #[test]
    fn server_crashed_serializes_server_and_exit_code() {
        let error = McpError::ServerCrashed {
            server_id: "sdk".into(),
            exit_code: Some(137),
        };

        let value = serde_json::to_value(&error).expect("error should serialize");

        assert_eq!(value["ServerCrashed"]["server_id"], "sdk");
        assert_eq!(value["ServerCrashed"]["exit_code"], 137);
    }

    #[test]
    fn transport_error_display_is_stable() {
        let error = McpError::Transport {
            message: "connection reset".into(),
        };

        assert_eq!(error.to_string(), "mcp transport failed: connection reset");
    }

    #[test]
    fn invalid_response_keeps_preview_payload() {
        let error = McpError::InvalidResponse {
            body_preview: json!({ "status": "bad" }).to_string(),
        };

        assert!(error.to_string().contains("\"status\":\"bad\""));
    }
}
