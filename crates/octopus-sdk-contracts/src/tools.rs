use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Read = 0,
    Write = 1,
    Network = 2,
    Shell = 3,
    Subagent = 4,
    Skill = 5,
    Meta = 6,
}

impl ToolCategory {
    #[must_use]
    pub const fn category_priority(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::ToolCategory;

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
}
