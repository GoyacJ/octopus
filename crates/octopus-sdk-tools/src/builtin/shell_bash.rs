use std::{collections::HashMap, process::Stdio, time::Instant};

use async_trait::async_trait;
use octopus_sdk_contracts::ContentBlock;
use serde::Deserialize;
use serde_json::json;
use tokio::process::Command;

use crate::{
    Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec, BASH_MAX_OUTPUT_DEFAULT,
    BASH_MAX_OUTPUT_UPPER_LIMIT,
};

const DEFAULT_TIMEOUT_MS: u64 = 120_000;
const TRUNCATION_HINT: &str = "\n\n[output truncated, use grep/head to inspect specific sections]";

#[derive(Debug, Deserialize)]
struct BashInput {
    command: String,
    timeout_ms: Option<u64>,
}

pub struct BashTool {
    spec: ToolSpec,
}

impl BashTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "bash".into(),
                description: "Run a bash command in the current sandbox working directory.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["command"],
                    "properties": {
                        "command": { "type": "string" },
                        "timeout_ms": { "type": "integer", "minimum": 1 }
                    }
                }),
                category: ToolCategory::Shell,
            },
        }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
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
        let input: BashInput = serde_json::from_value(input)?;
        let timeout_ms = input.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
        let output = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            run_command(&ctx, &input.command),
        )
        .await
        .map_err(|_| ToolError::Timeout {
            message: format!("command exceeded timeout of {timeout_ms} ms"),
        })??;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if !output.stderr.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        let text = truncate_output(&text, max_output_len());

        Ok(ToolResult {
            content: vec![ContentBlock::Text { text }],
            is_error: !output.status.success(),
            duration_ms: started.elapsed().as_millis() as u64,
            render: None,
        })
    }
}

async fn run_command(ctx: &ToolContext, command: &str) -> Result<std::process::Output, ToolError> {
    let mut process = Command::new("bash");
    process
        .arg("-c")
        .arg(command)
        .current_dir(&ctx.sandbox.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(filtered_env(&ctx.sandbox.env_allowlist));
    process
        .output()
        .await
        .map_err(|error| ToolError::Execution {
            message: format!("failed to execute bash command: {error}"),
        })
}

fn filtered_env(allowlist: &[String]) -> HashMap<String, String> {
    std::env::vars()
        .filter(|(key, _)| allowlist.iter().any(|allowed| allowed == key))
        .collect()
}

fn max_output_len() -> usize {
    std::env::var("BASH_MAX_OUTPUT_LENGTH")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(BASH_MAX_OUTPUT_DEFAULT)
        .min(BASH_MAX_OUTPUT_UPPER_LIMIT)
}

fn truncate_output(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.into();
    }

    let mut end = 0_usize;
    for (count, (index, ch)) in text.char_indices().enumerate() {
        if count == limit {
            break;
        }
        end = index + ch.len_utf8();
    }
    format!("{}{}", &text[..end], TRUNCATION_HINT)
}
