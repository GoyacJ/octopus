use std::path::PathBuf;

use async_trait::async_trait;
use futures::stream;
use harness_contracts::{
    DecisionScope, PermissionSubject, ToolDescriptor, ToolError, ToolGroup, ToolResult,
};
use harness_permission::PermissionCheck;
use serde_json::{json, Value};

use crate::{Tool, ToolContext, ToolEvent, ToolStream, ValidationError};

#[derive(Clone)]
pub struct ListDirTool {
    descriptor: ToolDescriptor,
}

impl Default for ListDirTool {
    fn default() -> Self {
        Self {
            descriptor: super::descriptor(
                "ListDir",
                "List directory",
                "List workspace directory entries.",
                ToolGroup::FileSystem,
                true,
                true,
                false,
                32_000,
                Vec::new(),
                super::object_schema(
                    &["path"],
                    json!({
                        "path": { "type": "string" },
                        "include_hidden": { "type": "boolean" }
                    }),
                ),
            ),
        }
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        path(input)?;
        Ok(())
    }

    async fn check_permission(&self, input: &Value, _ctx: &ToolContext) -> PermissionCheck {
        PermissionCheck::AskUser {
            subject: PermissionSubject::ToolInvocation {
                tool: self.descriptor.name.clone(),
                input: input.clone(),
            },
            scope: DecisionScope::PathPrefix(path(input).unwrap_or_default()),
        }
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolStream, ToolError> {
        let root = path(&input).map_err(validation_error)?;
        let include_hidden = input
            .get("include_hidden")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut entries = Vec::new();
        for entry in
            std::fs::read_dir(&root).map_err(|error| ToolError::Message(error.to_string()))?
        {
            let entry = entry.map_err(|error| ToolError::Message(error.to_string()))?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !include_hidden && file_name.starts_with('.') {
                continue;
            }
            let meta = entry
                .metadata()
                .map_err(|error| ToolError::Message(error.to_string()))?;
            entries.push(json!({
                "path": file_name,
                "kind": if meta.is_dir() { "dir" } else { "file" },
                "size": meta.len()
            }));
        }
        entries.sort_by(|left, right| {
            left["path"]
                .as_str()
                .unwrap_or_default()
                .cmp(right["path"].as_str().unwrap_or_default())
        });
        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Structured(Value::Array(entries)),
        )])))
    }
}

fn validation_error(error: ValidationError) -> ToolError {
    ToolError::Validation(error.to_string())
}

fn path(input: &Value) -> Result<PathBuf, ValidationError> {
    input
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| ValidationError::from("path is required"))
}
