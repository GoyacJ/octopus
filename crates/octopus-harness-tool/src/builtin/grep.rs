use std::path::PathBuf;
use std::process::Command;

use async_trait::async_trait;
use futures::stream;
use harness_contracts::{
    DecisionScope, PermissionSubject, ToolDescriptor, ToolError, ToolGroup, ToolResult,
};
use harness_permission::PermissionCheck;
use serde_json::{json, Value};

use crate::{Tool, ToolContext, ToolEvent, ToolStream, ValidationError};

#[derive(Clone)]
pub struct GrepTool {
    descriptor: ToolDescriptor,
}

impl Default for GrepTool {
    fn default() -> Self {
        Self {
            descriptor: super::descriptor(
                "Grep",
                "Grep",
                "Search files with ripgrep.",
                ToolGroup::Search,
                true,
                true,
                false,
                64_000,
                Vec::new(),
                super::object_schema(
                    &["path", "pattern"],
                    json!({
                        "path": { "type": "string" },
                        "pattern": { "type": "string" }
                    }),
                ),
            ),
        }
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        path(input)?;
        pattern(input)?;
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
        let pattern = pattern(&input).map_err(validation_error)?;
        let output = Command::new("rg")
            .arg("--line-number")
            .arg("--with-filename")
            .arg("--color")
            .arg("never")
            .arg("--no-heading")
            .arg("--")
            .arg(pattern)
            .arg(root)
            .output()
            .map_err(|error| ToolError::Message(error.to_string()))?;

        if !output.status.success() && output.status.code() != Some(1) {
            return Err(ToolError::Message(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| ToolError::Message(error.to_string()))?;
        let mut matches = stdout.lines().filter_map(parse_rg_line).collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            left["path"]
                .as_str()
                .unwrap_or_default()
                .cmp(right["path"].as_str().unwrap_or_default())
                .then_with(|| {
                    left["line"]
                        .as_u64()
                        .unwrap_or_default()
                        .cmp(&right["line"].as_u64().unwrap_or_default())
                })
        });

        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Structured(Value::Array(matches)),
        )])))
    }
}

fn validation_error(error: ValidationError) -> ToolError {
    ToolError::Validation(error.to_string())
}

fn parse_rg_line(line: &str) -> Option<Value> {
    let mut parts = line.splitn(3, ':');
    let path = parts.next()?;
    let line_number = parts.next()?.parse::<u64>().ok()?;
    let text = parts.next()?.to_owned();
    Some(json!({
        "path": path,
        "line": line_number,
        "text": text
    }))
}

fn path(input: &Value) -> Result<PathBuf, ValidationError> {
    input
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| ValidationError::from("path is required"))
}

fn pattern(input: &Value) -> Result<&str, ValidationError> {
    input
        .get("pattern")
        .and_then(Value::as_str)
        .ok_or_else(|| ValidationError::from("pattern is required"))
}
