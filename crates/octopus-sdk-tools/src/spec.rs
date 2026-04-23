use octopus_sdk_contracts::{ToolCategory, ToolSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutputFormat {
    #[default]
    Concise,
    Detailed,
    Structured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDisplayDescriptor {
    pub title: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub category: ToolCategory,
}

impl ToolSpec {
    #[must_use]
    pub fn to_mcp(&self) -> ToolSchema {
        ToolSchema {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ToolDisplayDescriptor, ToolOutputFormat, ToolSpec};
    use crate::ToolCategory;

    #[test]
    fn tool_spec_to_mcp_reuses_contract_shape() {
        let spec = ToolSpec {
            name: "grep".into(),
            description: "Search files".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" }
                }
            }),
            category: ToolCategory::Read,
        };

        let schema = spec.to_mcp();

        assert_eq!(schema.name, "grep");
        assert_eq!(schema.description, "Search files");
        assert_eq!(
            schema.input_schema["properties"]["pattern"]["type"],
            "string"
        );
    }

    #[test]
    fn tool_contract_metadata_serializes_stable_defaults() {
        assert_eq!(
            serde_json::to_value(ToolOutputFormat::Detailed).expect("format should serialize"),
            json!("detailed")
        );

        let display = ToolDisplayDescriptor {
            title: Some("Search Results".into()),
            summary: Some("Top matches".into()),
        };
        let value = serde_json::to_value(display).expect("display should serialize");
        assert_eq!(value["title"], "Search Results");
        assert_eq!(value["summary"], "Top matches");
    }
}
