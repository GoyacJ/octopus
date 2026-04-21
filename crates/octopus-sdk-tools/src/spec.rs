use octopus_sdk_contracts::ToolSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Read,
    Write,
    Network,
    Shell,
    Subagent,
    Skill,
    Meta,
}

impl ToolCategory {
    #[must_use]
    pub const fn category_priority(self) -> u8 {
        match self {
            Self::Read => 0,
            Self::Write => 1,
            Self::Network => 2,
            Self::Shell => 3,
            Self::Subagent => 4,
            Self::Skill => 5,
            Self::Meta => 6,
        }
    }
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
    use serde_json::{json, Value};

    use super::{ToolCategory, ToolSpec};

    #[test]
    fn tool_category_round_trips_all_variants() {
        let categories = [
            (ToolCategory::Read, "read"),
            (ToolCategory::Write, "write"),
            (ToolCategory::Network, "network"),
            (ToolCategory::Shell, "shell"),
            (ToolCategory::Subagent, "subagent"),
            (ToolCategory::Skill, "skill"),
            (ToolCategory::Meta, "meta"),
        ];

        for (category, encoded) in categories {
            let value = serde_json::to_value(category).expect("category should serialize");
            assert_eq!(value, Value::String(encoded.into()));

            let roundtrip: ToolCategory =
                serde_json::from_value(value).expect("category should deserialize");
            assert_eq!(roundtrip, category);
        }
    }

    #[test]
    fn tool_category_priority_matches_prompt_cache_contract() {
        let priorities = [
            (ToolCategory::Read, 0),
            (ToolCategory::Write, 1),
            (ToolCategory::Network, 2),
            (ToolCategory::Shell, 3),
            (ToolCategory::Subagent, 4),
            (ToolCategory::Skill, 5),
            (ToolCategory::Meta, 6),
        ];

        for (category, priority) in priorities {
            assert_eq!(category.category_priority(), priority);
        }
    }

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
}
