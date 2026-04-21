use async_trait::async_trait;
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

macro_rules! define_stub_tool {
    ($tool:ident, $tool_name:literal, $description:literal, $category:expr, $crate_name:literal) => {
        pub struct $tool {
            spec: ToolSpec,
        }

        impl $tool {
            #[must_use]
            pub fn new() -> Self {
                Self {
                    spec: ToolSpec {
                        name: $tool_name.into(),
                        description: concat!("[STUB · W5] ", $description).into(),
                        input_schema: json!({ "type": "object", "properties": {} }),
                        category: $category,
                    },
                }
            }
        }

        impl Default for $tool {
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait]
        impl Tool for $tool {
            fn spec(&self) -> &ToolSpec {
                &self.spec
            }

            fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
                false
            }

            async fn execute(
                &self,
                _ctx: ToolContext,
                _input: serde_json::Value,
            ) -> Result<ToolResult, ToolError> {
                Err(ToolError::NotYetImplemented {
                    crate_name: $crate_name,
                    week: "W5",
                })
            }
        }
    };
}

define_stub_tool!(
    AgentTool,
    "task",
    "Spawn and manage subagent execution.",
    ToolCategory::Subagent,
    "octopus-sdk-subagent"
);
define_stub_tool!(
    SkillTool,
    "skill",
    "Resolve and activate a named skill package.",
    ToolCategory::Skill,
    "octopus-sdk-subagent"
);
define_stub_tool!(
    TaskListTool,
    "task_list",
    "List background tasks tracked by the host runtime.",
    ToolCategory::Meta,
    "octopus-sdk-tools::task_registry"
);
define_stub_tool!(
    TaskGetTool,
    "task_get",
    "Read a single background task snapshot from the host runtime.",
    ToolCategory::Meta,
    "octopus-sdk-tools::task_registry"
);
