use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ToolCallId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: ToolCallId,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: ToolCallId,
        content: Vec<ContentBlock>,
        is_error: bool,
    },
    Thinking {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{ContentBlock, Message, Role};
    use crate::ToolCallId;

    #[test]
    fn message_serializes_role_before_content() {
        let message = Message {
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: "hello".into(),
            }],
        };

        let serialized = serde_json::to_string(&message).expect("message should serialize");

        assert!(serialized.find("\"role\"").expect("role key should exist")
            < serialized.find("\"content\"").expect("content key should exist"));
    }

    #[test]
    fn tool_use_content_block_serializes_snake_case_type_tag() {
        let block = ContentBlock::ToolUse {
            id: ToolCallId("call-1".into()),
            name: "read_file".into(),
            input: json!({ "path": "Cargo.toml" }),
        };

        let value = serde_json::to_value(&block).expect("content block should serialize");

        assert_eq!(value.get("type"), Some(&Value::String("tool_use".into())));
    }
}
