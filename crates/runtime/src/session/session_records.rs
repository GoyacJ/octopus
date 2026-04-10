use std::collections::BTreeMap;
use std::path::Path;

use crate::json::JsonValue;
use crate::usage::TokenUsage;

use super::{
    ContentBlock, ConversationMessage, MessageRole, SessionCompaction, SessionError, SessionFork,
};

impl ConversationMessage {
    #[must_use]
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text { text: text.into() }],
            usage: None,
        }
    }

    #[must_use]
    pub fn assistant(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::Assistant,
            blocks,
            usage: None,
        }
    }

    #[must_use]
    pub fn assistant_with_usage(blocks: Vec<ContentBlock>, usage: Option<TokenUsage>) -> Self {
        Self {
            role: MessageRole::Assistant,
            blocks,
            usage,
        }
    }

    #[must_use]
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        tool_name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.into(),
                tool_name: tool_name.into(),
                output: output.into(),
                is_error,
            }],
            usage: None,
        }
    }

    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        object.insert(
            "role".to_string(),
            JsonValue::String(
                match self.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }
                .to_string(),
            ),
        );
        object.insert(
            "blocks".to_string(),
            JsonValue::Array(self.blocks.iter().map(ContentBlock::to_json).collect()),
        );
        if let Some(usage) = self.usage {
            object.insert("usage".to_string(), usage_to_json(usage));
        }
        JsonValue::Object(object)
    }

    pub(super) fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("message must be an object".to_string()))?;
        let role = match object
            .get("role")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| SessionError::Format("missing role".to_string()))?
        {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            other => {
                return Err(SessionError::Format(format!(
                    "unsupported message role: {other}"
                )))
            }
        };
        let blocks = object
            .get("blocks")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| SessionError::Format("missing blocks".to_string()))?
            .iter()
            .map(ContentBlock::from_json)
            .collect::<Result<Vec<_>, _>>()?;
        let usage = object.get("usage").map(usage_from_json).transpose()?;
        Ok(Self {
            role,
            blocks,
            usage,
        })
    }
}

impl ContentBlock {
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        match self {
            Self::Text { text } => {
                object.insert("type".to_string(), JsonValue::String("text".to_string()));
                object.insert("text".to_string(), JsonValue::String(text.clone()));
            }
            Self::ToolUse { id, name, input } => {
                object.insert(
                    "type".to_string(),
                    JsonValue::String("tool_use".to_string()),
                );
                object.insert("id".to_string(), JsonValue::String(id.clone()));
                object.insert("name".to_string(), JsonValue::String(name.clone()));
                object.insert("input".to_string(), JsonValue::String(input.clone()));
            }
            Self::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => {
                object.insert(
                    "type".to_string(),
                    JsonValue::String("tool_result".to_string()),
                );
                object.insert(
                    "tool_use_id".to_string(),
                    JsonValue::String(tool_use_id.clone()),
                );
                object.insert(
                    "tool_name".to_string(),
                    JsonValue::String(tool_name.clone()),
                );
                object.insert("output".to_string(), JsonValue::String(output.clone()));
                object.insert("is_error".to_string(), JsonValue::Bool(*is_error));
            }
        }
        JsonValue::Object(object)
    }

    pub(super) fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("block must be an object".to_string()))?;
        match object
            .get("type")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| SessionError::Format("missing block type".to_string()))?
        {
            "text" => Ok(Self::Text {
                text: required_string(object, "text")?,
            }),
            "tool_use" => Ok(Self::ToolUse {
                id: required_string(object, "id")?,
                name: required_string(object, "name")?,
                input: required_string(object, "input")?,
            }),
            "tool_result" => Ok(Self::ToolResult {
                tool_use_id: required_string(object, "tool_use_id")?,
                tool_name: required_string(object, "tool_name")?,
                output: required_string(object, "output")?,
                is_error: object
                    .get("is_error")
                    .and_then(JsonValue::as_bool)
                    .ok_or_else(|| SessionError::Format("missing is_error".to_string()))?,
            }),
            other => Err(SessionError::Format(format!(
                "unsupported block type: {other}"
            ))),
        }
    }
}

impl SessionCompaction {
    pub fn to_json(&self) -> Result<JsonValue, SessionError> {
        let mut object = BTreeMap::new();
        object.insert(
            "count".to_string(),
            JsonValue::Number(i64::from(self.count)),
        );
        object.insert(
            "removed_message_count".to_string(),
            JsonValue::Number(i64_from_usize(
                self.removed_message_count,
                "removed_message_count",
            )?),
        );
        object.insert(
            "summary".to_string(),
            JsonValue::String(self.summary.clone()),
        );
        Ok(JsonValue::Object(object))
    }

    pub(super) fn to_jsonl_record(&self) -> Result<JsonValue, SessionError> {
        let mut object = BTreeMap::new();
        object.insert(
            "type".to_string(),
            JsonValue::String("compaction".to_string()),
        );
        object.insert(
            "count".to_string(),
            JsonValue::Number(i64::from(self.count)),
        );
        object.insert(
            "removed_message_count".to_string(),
            JsonValue::Number(i64_from_usize(
                self.removed_message_count,
                "removed_message_count",
            )?),
        );
        object.insert(
            "summary".to_string(),
            JsonValue::String(self.summary.clone()),
        );
        Ok(JsonValue::Object(object))
    }

    pub(super) fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("compaction must be an object".to_string()))?;
        Ok(Self {
            count: required_u32(object, "count")?,
            removed_message_count: required_usize(object, "removed_message_count")?,
            summary: required_string(object, "summary")?,
        })
    }
}

impl SessionFork {
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        object.insert(
            "parent_session_id".to_string(),
            JsonValue::String(self.parent_session_id.clone()),
        );
        if let Some(branch_name) = &self.branch_name {
            object.insert(
                "branch_name".to_string(),
                JsonValue::String(branch_name.clone()),
            );
        }
        JsonValue::Object(object)
    }

    pub(super) fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("fork metadata must be an object".to_string()))?;
        Ok(Self {
            parent_session_id: required_string(object, "parent_session_id")?,
            branch_name: object
                .get("branch_name")
                .and_then(JsonValue::as_str)
                .map(ToOwned::to_owned),
        })
    }
}

pub(super) fn message_record(message: &ConversationMessage) -> JsonValue {
    let mut object = BTreeMap::new();
    object.insert("type".to_string(), JsonValue::String("message".to_string()));
    object.insert("message".to_string(), message.to_json());
    JsonValue::Object(object)
}

fn usage_to_json(usage: TokenUsage) -> JsonValue {
    let mut object = BTreeMap::new();
    object.insert(
        "input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.input_tokens)),
    );
    object.insert(
        "output_tokens".to_string(),
        JsonValue::Number(i64::from(usage.output_tokens)),
    );
    object.insert(
        "cache_creation_input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.cache_creation_input_tokens)),
    );
    object.insert(
        "cache_read_input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.cache_read_input_tokens)),
    );
    JsonValue::Object(object)
}

fn usage_from_json(value: &JsonValue) -> Result<TokenUsage, SessionError> {
    let object = value
        .as_object()
        .ok_or_else(|| SessionError::Format("usage must be an object".to_string()))?;
    Ok(TokenUsage {
        input_tokens: required_u32(object, "input_tokens")?,
        output_tokens: required_u32(object, "output_tokens")?,
        cache_creation_input_tokens: required_u32(object, "cache_creation_input_tokens")?,
        cache_read_input_tokens: required_u32(object, "cache_read_input_tokens")?,
    })
}

pub(super) fn required_string(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<String, SessionError> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))
}

pub(super) fn required_u32(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<u32, SessionError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    u32::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

pub(super) fn required_u64(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<u64, SessionError> {
    let value = object
        .get(key)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    required_u64_from_value(value, key)
}

pub(super) fn required_u64_from_value(value: &JsonValue, key: &str) -> Result<u64, SessionError> {
    let value = value
        .as_i64()
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    u64::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

pub(super) fn required_usize(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<usize, SessionError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    usize::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

pub(super) fn i64_from_u64(value: u64, key: &str) -> Result<i64, SessionError> {
    i64::try_from(value)
        .map_err(|_| SessionError::Format(format!("{key} out of range for JSON number")))
}

pub(super) fn i64_from_usize(value: usize, key: &str) -> Result<i64, SessionError> {
    i64::try_from(value)
        .map_err(|_| SessionError::Format(format!("{key} out of range for JSON number")))
}

pub(super) fn workspace_root_to_string(path: &Path) -> Result<String, SessionError> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        SessionError::Format(format!(
            "workspace_root is not valid UTF-8: {}",
            path.display()
        ))
    })
}
