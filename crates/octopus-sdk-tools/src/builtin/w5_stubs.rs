use std::{sync::Arc, time::Instant};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    ContentBlock, HookEvent, SubagentOutput, SubagentSpec, SubagentSummary,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    with_task_parent_context, TaskFn, Tool, ToolCategory, ToolContext, ToolError, ToolResult,
    ToolSpec,
};

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

const DEFAULT_TASK_FN_REASON: &str = "TaskFn not injected";

pub struct AgentTool {
    spec: ToolSpec,
    task_fn: Arc<dyn TaskFn>,
    missing_task_fn_reason: Option<String>,
}

impl AgentTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "task".into(),
                description: "[STUB · W5] Spawn and manage subagent execution.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["spec", "input"],
                    "properties": {
                        "spec": { "type": "object" },
                        "input": { "type": "string" }
                    }
                }),
                category: ToolCategory::Subagent,
            },
            task_fn: Arc::new(ErrorTaskFn::new(DEFAULT_TASK_FN_REASON)),
            missing_task_fn_reason: Some(DEFAULT_TASK_FN_REASON.into()),
        }
    }

    #[must_use]
    pub fn with_task_fn(mut self, f: Arc<dyn TaskFn>) -> Self {
        self.task_fn = f;
        self.missing_task_fn_reason = None;
        self
    }
}

impl Default for AgentTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AgentTool {
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
        let started_at = Instant::now();
        if let Some(reason) = &self.missing_task_fn_reason {
            return Ok(error_result(
                reason.clone(),
                started_at.elapsed().as_millis() as u64,
            ));
        }

        let input: AgentToolInput =
            serde_json::from_value(input).map_err(|error| ToolError::Validation {
                message: error.to_string(),
            })?;
        let hooks = Arc::clone(&ctx.hooks);
        let parent_session = ctx.session_id.clone();
        run_subagent_spawn_hook(hooks.as_ref(), &parent_session, &input.spec)
            .await
            .map_err(|message| ToolError::Execution { message })?;

        with_task_parent_context(ctx.session_id.clone(), ctx.tool_call_id.clone(), async {
            match self.task_fn.run(&input.spec, &input.input).await {
                Ok(output) => {
                    run_subagent_return_hook(
                        hooks.as_ref(),
                        &parent_session,
                        subagent_meta(&output),
                    )
                    .await
                    .map_err(|message| ToolError::Execution { message })?;
                    Ok(output_result(
                        output,
                        started_at.elapsed().as_millis() as u64,
                    )?)
                }
                Err(error) => Ok(error_result(
                    error.to_string(),
                    started_at.elapsed().as_millis() as u64,
                )),
            }
        })
        .await
    }
}

#[derive(Debug, Deserialize)]
struct AgentToolInput {
    spec: SubagentSpec,
    input: String,
}

struct ErrorTaskFn {
    #[allow(dead_code)]
    reason: String,
}

impl ErrorTaskFn {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl TaskFn for ErrorTaskFn {
    async fn run(
        &self,
        _spec: &SubagentSpec,
        _input: &str,
    ) -> Result<SubagentOutput, octopus_sdk_contracts::SubagentError> {
        Err(octopus_sdk_contracts::SubagentError::Provider {
            reason: self.reason.clone(),
        })
    }
}

fn output_result(output: SubagentOutput, duration_ms: u64) -> Result<ToolResult, ToolError> {
    let text = match output {
        SubagentOutput::Summary { text, .. } => text,
        SubagentOutput::FileRef { path, bytes, .. } => {
            format!("file: {} ({} bytes)", path.display(), bytes)
        }
        SubagentOutput::Json { value, .. } => serde_json::to_string(&value)?,
    };

    Ok(ToolResult {
        content: vec![ContentBlock::Text { text }],
        is_error: false,
        duration_ms,
        render: None,
    })
}

fn error_result(reason: String, duration_ms: u64) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text { text: reason }],
        is_error: true,
        duration_ms,
        render: None,
    }
}

fn subagent_meta(output: &SubagentOutput) -> SubagentSummary {
    match output {
        SubagentOutput::Summary { meta, .. }
        | SubagentOutput::FileRef { meta, .. }
        | SubagentOutput::Json { meta, .. } => meta.clone(),
    }
}

async fn run_subagent_spawn_hook(
    hooks: &octopus_sdk_hooks::HookRunner,
    parent_session: &octopus_sdk_contracts::SessionId,
    spec: &SubagentSpec,
) -> Result<(), String> {
    let outcome = hooks
        .run(HookEvent::SubagentSpawn {
            parent_session: parent_session.clone(),
            spec: spec.clone(),
        })
        .await
        .map_err(|error| format!("subagent_spawn hook failed: {error}"))?;

    if let Some(reason) = outcome.aborted {
        return Err(reason);
    }

    Ok(())
}

async fn run_subagent_return_hook(
    hooks: &octopus_sdk_hooks::HookRunner,
    parent_session: &octopus_sdk_contracts::SessionId,
    summary: SubagentSummary,
) -> Result<(), String> {
    let outcome = hooks
        .run(HookEvent::SubagentReturn {
            parent_session: parent_session.clone(),
            summary,
        })
        .await
        .map_err(|error| format!("subagent_return hook failed: {error}"))?;

    if let Some(reason) = outcome.aborted {
        return Err(reason);
    }

    Ok(())
}

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
