use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    ContentBlock, HookEvent, PermissionOutcome, RewritePayload, ToolCallId, ToolCallRequest,
};
use serde::Deserialize;
use serde_json::json;
use tempfile::NamedTempFile;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

#[derive(Debug, Deserialize)]
struct FileWriteInput {
    path: String,
    content: String,
}

pub struct FileWriteTool {
    spec: ToolSpec,
}

impl FileWriteTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "write_file".into(),
                description: "Write a file atomically inside the current workspace.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["path", "content"],
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" }
                    }
                }),
                category: ToolCategory::Write,
            },
        }
    }
}

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileWriteTool {
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
            id: ToolCallId("write_file".into()),
            name: self.spec.name.clone(),
            input: input.clone(),
        };
        check_permission(&ctx, &request).await?;
        let input: FileWriteInput = serde_json::from_value(input)?;
        let (path, content) =
            run_pre_file_write_hooks(&ctx, &request, &input.path, &input.content).await?;
        write_atomic(&path, &content)?;
        run_post_file_write_hooks(&ctx, &request, &path).await?;

        Ok(ToolResult {
            content: vec![ContentBlock::Text {
                text: format!("wrote {}", display_path(&ctx.working_dir, &path)),
            }],
            is_error: false,
            duration_ms: started.elapsed().as_millis() as u64,
            render: None,
        })
    }
}

pub(crate) async fn check_permission(
    ctx: &ToolContext,
    request: &ToolCallRequest,
) -> Result<(), ToolError> {
    match ctx.permissions.check(request).await {
        PermissionOutcome::Allow => Ok(()),
        PermissionOutcome::Deny { reason } => Err(ToolError::Permission { message: reason }),
        PermissionOutcome::AskApproval { prompt } => Err(ToolError::Permission {
            message: format!("approval required for {}", prompt.kind),
        }),
        PermissionOutcome::RequireAuth { prompt } => Err(ToolError::Permission {
            message: format!("authentication required for {}", prompt.kind),
        }),
    }
}

pub(crate) fn resolve_path_allow_missing(base_dir: &Path, input: &str) -> PathBuf {
    let candidate = if Path::new(input).is_absolute() {
        PathBuf::from(input)
    } else {
        base_dir.join(input)
    };
    if let Ok(path) = dunce::canonicalize(&candidate) {
        return path;
    }
    candidate
}

pub(crate) async fn run_pre_file_write_hooks(
    ctx: &ToolContext,
    request: &ToolCallRequest,
    input_path: &str,
    input_content: &str,
) -> Result<(PathBuf, String), ToolError> {
    let path = resolve_path_allow_missing(&ctx.working_dir, input_path);
    let outcome = ctx
        .hooks
        .run(HookEvent::PreFileWrite {
            call: request.clone(),
            path: path.display().to_string(),
            content: input_content.to_string(),
        })
        .await
        .map_err(|error| ToolError::Execution {
            message: format!("pre_file_write hook failed: {error}"),
        })?;

    if let Some(reason) = outcome.aborted {
        return Err(ToolError::Execution { message: reason });
    }

    match outcome.final_payload {
        Some(RewritePayload::FileWrite { path, content }) => {
            Ok((resolve_path_allow_missing(&ctx.working_dir, &path), content))
        }
        _ => Ok((path, input_content.to_string())),
    }
}

pub(crate) async fn run_post_file_write_hooks(
    ctx: &ToolContext,
    request: &ToolCallRequest,
    path: &Path,
) -> Result<(), ToolError> {
    let outcome = ctx
        .hooks
        .run(HookEvent::PostFileWrite {
            call: request.clone(),
            path: path.display().to_string(),
        })
        .await
        .map_err(|error| ToolError::Execution {
            message: format!("post_file_write hook failed: {error}"),
        })?;

    if let Some(reason) = outcome.aborted {
        return Err(ToolError::Execution { message: reason });
    }

    Ok(())
}

pub(crate) fn write_atomic(path: &Path, content: &str) -> Result<(), ToolError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| ToolError::Execution {
            message: format!("failed to create {}: {error}", parent.display()),
        })?;
        let mut file = NamedTempFile::new_in(parent).map_err(|error| ToolError::Execution {
            message: format!(
                "failed to allocate temp file in {}: {error}",
                parent.display()
            ),
        })?;
        file.write_all(content.as_bytes())
            .map_err(|error| ToolError::Execution {
                message: format!("failed to write temp file for {}: {error}", path.display()),
            })?;
        file.persist(path).map_err(|error| ToolError::Execution {
            message: format!("failed to persist {}: {}", path.display(), error.error),
        })?;
        return Ok(());
    }

    Err(ToolError::Execution {
        message: format!("{} has no parent directory", path.display()),
    })
}

pub(crate) fn display_path(base_dir: &Path, path: &Path) -> String {
    path.strip_prefix(base_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
