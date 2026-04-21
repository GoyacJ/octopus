use std::{pin::Pin, time::Instant};

use async_trait::async_trait;
use octopus_sdk_contracts::ContentBlock;
use serde::Deserialize;
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

const MAX_SLEEP_MS: u64 = 60_000;

#[derive(Debug, Deserialize)]
struct SleepInput {
    ms: u64,
}

pub struct SleepTool {
    spec: ToolSpec,
}

impl SleepTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "sleep".into(),
                description: "Wait for a bounded duration without holding a shell process open."
                    .into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["ms"],
                    "properties": {
                        "ms": { "type": "integer", "minimum": 0, "maximum": MAX_SLEEP_MS }
                    }
                }),
                category: ToolCategory::Meta,
            },
        }
    }
}

impl Default for SleepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SleepTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn execute(
        &self,
        ctx: ToolContext,
        input: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let started = Instant::now();
        let input: SleepInput = serde_json::from_value(input)?;
        if input.ms > MAX_SLEEP_MS {
            return Err(ToolError::Validation {
                message: format!(
                    "ms {} exceeds maximum allowed sleep of {MAX_SLEEP_MS}ms",
                    input.ms
                ),
            });
        }

        let sleep = tokio::time::sleep(std::time::Duration::from_millis(input.ms));
        let mut sleep = Pin::from(Box::new(sleep));
        tokio::select! {
            () = &mut sleep => {}
            () = ctx.cancellation.cancelled() => {
                return Err(ToolError::Cancelled { message: "sleep was cancelled before completion".into() });
            }
        }

        Ok(ToolResult {
            content: vec![ContentBlock::Text {
                text: format!("slept for {} ms", input.ms),
            }],
            is_error: false,
            duration_ms: started.elapsed().as_millis() as u64,
            render: None,
        })
    }
}
