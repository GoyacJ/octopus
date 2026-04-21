use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use async_trait::async_trait;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use octopus_sdk_contracts::ContentBlock;
use serde::Deserialize;
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

const MAX_GLOB_MATCHES: usize = 500;

#[derive(Debug, Deserialize)]
struct GlobInput {
    pattern: String,
    path: Option<String>,
}

pub struct GlobTool {
    spec: ToolSpec,
}

impl GlobTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "glob".into(),
                description: "Expand a glob against the current workspace.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["pattern"],
                    "properties": {
                        "pattern": { "type": "string" },
                        "path": { "type": "string" }
                    }
                }),
                category: ToolCategory::Read,
            },
        }
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
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
        let input: GlobInput = serde_json::from_value(input)?;
        let base_dir = resolve_dir(&ctx.working_dir, input.path.as_deref())?;
        let workspace_root =
            dunce::canonicalize(&ctx.working_dir).unwrap_or_else(|_| ctx.working_dir.clone());
        let mut builder = GlobSetBuilder::new();
        builder.add(
            Glob::new(&input.pattern).map_err(|error| ToolError::Validation {
                message: error.to_string(),
            })?,
        );
        let matcher = builder.build().map_err(|error| ToolError::Validation {
            message: error.to_string(),
        })?;

        let mut paths = WalkBuilder::new(&base_dir)
            .hidden(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .build()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
            .filter_map(|entry| {
                let path = entry.into_path();
                let rel = path.strip_prefix(&base_dir).unwrap_or(path.as_path());
                matcher
                    .is_match(rel)
                    .then(|| display_path(&workspace_root, &path))
            })
            .collect::<Vec<_>>();

        paths.sort();
        let truncated = paths.len() > MAX_GLOB_MATCHES;
        paths.truncate(MAX_GLOB_MATCHES);
        let mut text = paths.join("\n");
        if truncated {
            text.push_str("\n... truncated after 500 paths");
        }

        Ok(ToolResult {
            content: vec![ContentBlock::Text { text }],
            is_error: false,
            duration_ms: started.elapsed().as_millis() as u64,
            render: None,
        })
    }
}

fn resolve_dir(base_dir: &Path, path: Option<&str>) -> Result<PathBuf, ToolError> {
    let candidate = match path {
        Some(path) if Path::new(path).is_absolute() => PathBuf::from(path),
        Some(path) => base_dir.join(path),
        None => base_dir.to_path_buf(),
    };
    dunce::canonicalize(&candidate).map_err(|error| ToolError::Execution {
        message: format!("failed to resolve {}: {error}", candidate.display()),
    })
}

fn display_path(base_dir: &Path, path: &Path) -> String {
    path.strip_prefix(base_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
