use async_trait::async_trait;
use futures::stream;
use harness_contracts::{
    ClarifyChannelCap, ClarifyChoice, ClarifyPrompt, DecisionScope, PermissionSubject,
    ToolCapability, ToolDescriptor, ToolError, ToolGroup, ToolResult,
};
use harness_permission::PermissionCheck;
use serde_json::{json, Value};

use crate::{Tool, ToolContext, ToolEvent, ToolStream, ValidationError};

#[derive(Clone)]
pub struct ClarifyTool {
    descriptor: ToolDescriptor,
}

impl Default for ClarifyTool {
    fn default() -> Self {
        Self {
            descriptor: super::descriptor(
                "Clarify",
                "Clarify",
                "Ask the user for clarification through the session channel.",
                ToolGroup::Clarification,
                false,
                false,
                false,
                8_000,
                vec![ToolCapability::ClarifyChannel],
                super::object_schema(
                    &["prompt"],
                    json!({
                        "prompt": { "type": "string" },
                        "choices": { "type": "array" },
                        "multiple": { "type": "boolean" },
                        "timeout_seconds": { "type": "integer", "minimum": 1 }
                    }),
                ),
            ),
        }
    }
}

#[async_trait]
impl Tool for ClarifyTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        prompt(input)?;
        Ok(())
    }

    async fn check_permission(&self, input: &Value, _ctx: &ToolContext) -> PermissionCheck {
        PermissionCheck::AskUser {
            subject: PermissionSubject::ToolInvocation {
                tool: self.descriptor.name.clone(),
                input: input.clone(),
            },
            scope: DecisionScope::ToolName(self.descriptor.name.clone()),
        }
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolStream, ToolError> {
        let channel = ctx.capability::<dyn ClarifyChannelCap>(ToolCapability::ClarifyChannel)?;
        let answer = channel
            .ask(prompt(&input).map_err(validation_error)?)
            .await?;
        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Structured(json!({
                "answer": answer.answer,
                "chosen_ids": answer.chosen_ids
            })),
        )])))
    }
}

fn validation_error(error: ValidationError) -> ToolError {
    ToolError::Validation(error.to_string())
}

fn prompt(input: &Value) -> Result<ClarifyPrompt, ValidationError> {
    let prompt = input
        .get("prompt")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ValidationError::from("prompt is required"))?
        .to_owned();
    let choices = input
        .get("choices")
        .and_then(Value::as_array)
        .map(|choices| {
            choices
                .iter()
                .map(|choice| ClarifyChoice {
                    id: choice
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    label: choice
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    hint: choice
                        .get("hint")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(ClarifyPrompt {
        prompt,
        choices,
        multiple: input
            .get("multiple")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        timeout_seconds: input
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .map(|value| value as u32),
    })
}
