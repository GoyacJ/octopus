use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use harness_contracts::{
    CorrelationId, DecisionScope, Event, PermissionSubject, Redactor, SandboxError,
    SandboxExitStatus, ToolCapability, ToolDescriptor, ToolError, ToolGroup, ToolResult,
    WorkspaceAccess,
};
use harness_permission::PermissionCheck;
use harness_sandbox::{EventSink, ExecContext, ExecSpec, ProcessHandle, StdioSpec};
use serde_json::{json, Value};

use crate::{Tool, ToolContext, ToolEvent, ToolStream, ValidationError};

#[derive(Clone)]
pub struct BashTool {
    descriptor: ToolDescriptor,
}

impl Default for BashTool {
    fn default() -> Self {
        Self {
            descriptor: super::descriptor(
                "Bash",
                "Bash",
                "Execute a shell command through the configured sandbox.",
                ToolGroup::Shell,
                false,
                false,
                true,
                256_000,
                Vec::new(),
                super::object_schema(
                    &["command"],
                    json!({
                        "command": { "type": "string" },
                        "cwd": { "type": "string" }
                    }),
                ),
            ),
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        command(input)?;
        Ok(())
    }

    async fn check_permission(&self, input: &Value, _ctx: &ToolContext) -> PermissionCheck {
        let command = command(input).unwrap_or_default().to_owned();
        let cwd = cwd(input);
        PermissionCheck::AskUser {
            subject: PermissionSubject::CommandExec {
                command: command.clone(),
                argv: Vec::new(),
                cwd: cwd.clone(),
                fingerprint: None,
            },
            scope: DecisionScope::ExactCommand { command, cwd },
        }
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolStream, ToolError> {
        let sandbox = ctx.sandbox.clone().ok_or_else(|| {
            ToolError::CapabilityMissing(ToolCapability::Custom("sandbox_backend".to_owned()))
        })?;
        let command = command(&input).map_err(validation_error)?.to_owned();
        let spec = ExecSpec {
            command,
            cwd: cwd(&input),
            stdin: StdioSpec::Null,
            stdout: StdioSpec::Piped,
            stderr: StdioSpec::Piped,
            workspace_access: WorkspaceAccess::ReadWrite {
                allowed_writable_subpaths: Vec::new(),
            },
            ..ExecSpec::default()
        };
        let exec_ctx = exec_context(&ctx);

        sandbox
            .before_execute(&spec, &exec_ctx)
            .await
            .map_err(ToolError::Sandbox)?;
        let handle = sandbox
            .execute(spec, exec_ctx.clone())
            .await
            .map_err(ToolError::Sandbox)?;
        let ProcessHandle {
            stdout,
            stderr,
            activity,
            ..
        } = handle;
        let stdout = collect_stream(stdout).await?;
        let stderr = collect_stream(stderr).await?;
        let outcome = activity.wait().await.map_err(ToolError::Sandbox)?;
        sandbox
            .after_execute(&outcome, &exec_ctx)
            .await
            .map_err(ToolError::Sandbox)?;

        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Structured(json!({
                "exit_status": exit_status_json(&outcome.exit_status),
                "stdout": stdout,
                "stderr": stderr
            })),
        )])))
    }
}

async fn collect_stream<S, B>(stream: Option<S>) -> Result<String, ToolError>
where
    S: Stream<Item = B>,
    B: AsRef<[u8]>,
{
    let Some(stream) = stream else {
        return Ok(String::new());
    };
    let bytes = stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flat_map(|chunk| chunk.as_ref().to_vec())
        .collect();
    String::from_utf8(bytes).map_err(|error| ToolError::Message(error.to_string()))
}

fn exec_context(ctx: &ToolContext) -> ExecContext {
    ExecContext {
        session_id: ctx.session_id,
        run_id: ctx.run_id,
        tool_use_id: Some(ctx.tool_use_id),
        tenant_id: ctx.tenant_id,
        workspace_root: ctx.workspace_root.clone(),
        correlation_id: CorrelationId::new(),
        event_sink: Arc::new(NullEventSink),
        redactor: Arc::new(harness_contracts::NoopRedactor) as Arc<dyn Redactor>,
    }
}

fn exit_status_json(status: &SandboxExitStatus) -> Value {
    match status {
        SandboxExitStatus::Code(code) => json!({ "code": code }),
        SandboxExitStatus::Signal(signal) => json!({ "signal": signal }),
        SandboxExitStatus::Timeout => json!({ "timeout": true }),
        SandboxExitStatus::InactivityTimeout => json!({ "inactivity_timeout": true }),
        SandboxExitStatus::OutputBudgetExceeded => json!({ "output_budget_exceeded": true }),
        SandboxExitStatus::Cancelled => json!({ "cancelled": true }),
        SandboxExitStatus::BackendError => json!({ "backend_error": true }),
        _ => json!({ "unknown": true }),
    }
}

fn validation_error(error: ValidationError) -> ToolError {
    ToolError::Validation(error.to_string())
}

fn command(input: &Value) -> Result<&str, ValidationError> {
    input
        .get("command")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ValidationError::from("command is required"))
}

fn cwd(input: &Value) -> Option<PathBuf> {
    input.get("cwd").and_then(Value::as_str).map(PathBuf::from)
}

struct NullEventSink;

impl EventSink for NullEventSink {
    fn emit(&self, _event: Event) -> Result<(), SandboxError> {
        Ok(())
    }
}
