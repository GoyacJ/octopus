use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use async_trait::async_trait;
use octopus_sdk_contracts::{ContentBlock, ToolCallId, ToolCallRequest};
use serde::Deserialize;
use serde_json::json;

use crate::{
    builtin::fs_write::{check_permission, display_path, write_atomic},
    Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec,
};

#[derive(Debug, Deserialize)]
struct FileEditInput {
    path: String,
    old_string: String,
    new_string: String,
    replace_all: Option<bool>,
}

pub struct FileEditTool {
    spec: ToolSpec,
}

impl FileEditTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "edit_file".into(),
                description: "Replace text in a file and persist the change atomically.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["path", "old_string", "new_string"],
                    "properties": {
                        "path": { "type": "string" },
                        "old_string": { "type": "string" },
                        "new_string": { "type": "string" },
                        "replace_all": { "type": "boolean" }
                    }
                }),
                category: ToolCategory::Write,
            },
        }
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileEditTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        false
    }

    async fn execute(
        &self,
        ctx: ToolContext,
        input: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let started = Instant::now();
        let request = ToolCallRequest {
            id: ToolCallId("edit_file".into()),
            name: self.spec.name.clone(),
            input: input.clone(),
        };
        check_permission(&ctx, &request).await?;
        let input: FileEditInput = serde_json::from_value(input)?;
        let path = resolve_existing_path(&ctx.working_dir, &input.path)?;
        let original = fs::read_to_string(&path).map_err(|error| ToolError::Execution {
            message: format!("failed to read {}: {error}", path.display()),
        })?;

        if input.old_string == input.new_string {
            return Err(ToolError::Validation {
                message: "old_string and new_string must differ".into(),
            });
        }
        if !original.contains(&input.old_string) {
            return Err(ToolError::Validation {
                message: format!("{} does not contain the requested text", path.display()),
            });
        }

        let updated = if input.replace_all.unwrap_or(false) {
            original.replace(&input.old_string, &input.new_string)
        } else {
            original.replacen(&input.old_string, &input.new_string, 1)
        };
        write_atomic(&path, &updated)?;

        Ok(ToolResult {
            content: vec![ContentBlock::Text {
                text: format!("edited {}", display_path(&ctx.working_dir, &path)),
            }],
            is_error: false,
            duration_ms: started.elapsed().as_millis() as u64,
            render: None,
        })
    }
}

fn resolve_existing_path(base_dir: &Path, input: &str) -> Result<PathBuf, ToolError> {
    let candidate = if Path::new(input).is_absolute() {
        PathBuf::from(input)
    } else {
        base_dir.join(input)
    };
    dunce::canonicalize(&candidate).map_err(|error| ToolError::Execution {
        message: format!("failed to resolve {}: {error}", candidate.display()),
    })
}
