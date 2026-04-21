//! Tool schema contract lands here in Task 3.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use serde_json::{json, to_value};

    use super::ToolSchema;

    #[test]
    fn tool_schema_serializes_expected_fields() {
        let schema = ToolSchema {
            name: "search".to_string(),
            description: "Search docs".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                }
            }),
        };

        let value = to_value(&schema).unwrap();

        assert_eq!(value["name"], json!("search"));
        assert_eq!(value["description"], json!("Search docs"));
        assert_eq!(value["input_schema"]["type"], json!("object"));
    }
}
