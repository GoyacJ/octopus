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
pub struct FileReadTool {
    descriptor: ToolDescriptor,
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self {
            descriptor: super::descriptor(
                "FileRead",
                "File read",
                "Read a UTF-8 workspace file.",
                ToolGroup::FileSystem,
                true,
                true,
                false,
                64_000,
                Vec::new(),
                super::object_schema(
                    &["path"],
                    json!({
                        "path": { "type": "string" },
                        "start_line": { "type": "integer", "minimum": 1 },
                        "end_line": { "type": "integer", "minimum": 1 }
                    }),
                ),
            ),
        }
    }
}

#[async_trait]
impl Tool for FileReadTool {
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
        let content = std::fs::read_to_string(path(&input).map_err(validation_error)?)
            .map_err(|error| ToolError::Message(error.to_string()))?;
        let content = slice_lines(
            &content,
            input.get("start_line").and_then(Value::as_u64),
            input.get("end_line").and_then(Value::as_u64),
        );
        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Text(content),
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

fn slice_lines(content: &str, start_line: Option<u64>, end_line: Option<u64>) -> String {
    let start = start_line.unwrap_or(1).max(1);
    let end = end_line.unwrap_or(u64::MAX).max(start);
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_number = index as u64 + 1;
            (line_number >= start && line_number <= end).then_some(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
