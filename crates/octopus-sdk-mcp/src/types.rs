use octopus_sdk_contracts::{ContentBlock, ToolSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

impl From<ToolSchema> for McpTool {
    fn from(value: ToolSchema) -> Self {
        Self {
            name: value.name,
            description: value.description,
            input_schema: value.input_schema,
        }
    }
}

impl From<McpTool> for ToolSchema {
    fn from(value: McpTool) -> Self {
        Self {
            name: value.name,
            description: value.description,
            input_schema: value.input_schema,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<ContentBlock>,
    #[serde(rename = "isError")]
    pub is_error: bool,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{McpPrompt, McpResource, McpTool, McpToolResult};
    use octopus_sdk_contracts::{ContentBlock, ToolSchema};

    #[test]
    fn mcp_tool_round_trips_with_wire_field_names() {
        let tool = McpTool {
            name: "grep".into(),
            description: "Search files".into(),
            input_schema: json!({ "type": "object" }),
        };

        let encoded = serde_json::to_value(&tool).expect("tool should serialize");
        let decoded: McpTool =
            serde_json::from_value(encoded.clone()).expect("tool should deserialize");

        assert_eq!(encoded["inputSchema"]["type"], "object");
        assert_eq!(decoded.name, "grep");
    }

    #[test]
    fn tool_schema_converts_to_and_from_mcp_tool() {
        let schema = ToolSchema {
            name: "read_file".into(),
            description: "Read a file".into(),
            input_schema: json!({
                "type": "object",
                "properties": { "path": { "type": "string" } }
            }),
        };

        let tool = McpTool::from(schema.clone());
        let roundtrip = ToolSchema::from(tool);

        assert_eq!(roundtrip, schema);
    }

    #[test]
    fn prompt_resource_and_tool_result_round_trip() {
        let prompt = McpPrompt {
            name: "summarize".into(),
            description: Some("Summarize the current buffer".into()),
        };
        let resource = McpResource {
            uri: "file:///tmp/report.md".into(),
            name: "report".into(),
            description: Some("Current report".into()),
            mime_type: Some("text/markdown".into()),
        };
        let result = McpToolResult {
            content: vec![ContentBlock::Text {
                text: "done".into(),
            }],
            is_error: false,
        };

        let prompt_value = serde_json::to_value(&prompt).expect("prompt should serialize");
        let resource_value = serde_json::to_value(&resource).expect("resource should serialize");
        let result_value = serde_json::to_value(&result).expect("result should serialize");

        assert_eq!(prompt_value["name"], "summarize");
        assert_eq!(resource_value["mimeType"], "text/markdown");
        assert_eq!(result_value["isError"], false);
    }
}
