use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use async_trait::async_trait;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use octopus_sdk_contracts::ContentBlock;
use regex::RegexBuilder;
use serde::Deserialize;
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

const MAX_GREP_MATCHES: usize = 100;

#[derive(Debug, Deserialize)]
struct GrepInput {
    pattern: String,
    path: Option<String>,
    glob: Option<String>,
    output_mode: Option<String>,
    case_insensitive: Option<bool>,
    line_numbers: Option<bool>,
}

pub struct GrepTool {
    spec: ToolSpec,
}

impl GrepTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "grep".into(),
                description: "Run a regex search across workspace files.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["pattern"],
                    "properties": {
                        "pattern": { "type": "string" },
                        "path": { "type": "string" },
                        "glob": { "type": "string" },
                        "output_mode": { "type": "string", "enum": ["files_with_matches", "content"] },
                        "case_insensitive": { "type": "boolean" },
                        "line_numbers": { "type": "boolean" }
                    }
                }),
                category: ToolCategory::Read,
            },
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
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
        let input: GrepInput = serde_json::from_value(input)?;
        let base_dir = resolve_dir(&ctx.working_dir, input.path.as_deref())?;
        let workspace_root =
            dunce::canonicalize(&ctx.working_dir).unwrap_or_else(|_| ctx.working_dir.clone());
        let regex = RegexBuilder::new(&input.pattern)
            .case_insensitive(input.case_insensitive.unwrap_or(false))
            .build()
            .map_err(|error| ToolError::Validation {
                message: error.to_string(),
            })?;
        let glob = build_glob(input.glob.as_deref())?;
        let content_mode = input.output_mode.as_deref() == Some("content");
        let line_numbers = input.line_numbers.unwrap_or(true);

        let mut rows = Vec::new();
        let mut files = Vec::new();
        let mut matches = 0_usize;
        for entry in WalkBuilder::new(&base_dir)
            .hidden(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .build()
            .filter_map(Result::ok)
        {
            if matches == MAX_GREP_MATCHES || !entry.file_type().is_some_and(|kind| kind.is_file())
            {
                continue;
            }
            let path = entry.into_path();
            if !matches_glob(glob.as_ref(), &base_dir, &path) {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let display = display_path(&workspace_root, &path);
            let mut file_matched = false;
            for (index, line) in content.lines().enumerate() {
                if matches == MAX_GREP_MATCHES {
                    break;
                }
                if regex.is_match(line) {
                    matches += 1;
                    file_matched = true;
                    if content_mode {
                        let prefix = if line_numbers {
                            format!("{display}:{}:", index + 1)
                        } else {
                            format!("{display}:")
                        };
                        rows.push(format!("{prefix}{line}"));
                    }
                }
            }
            if file_matched {
                files.push(display);
            }
        }

        files.sort();
        files.dedup();
        let mut text = if content_mode {
            rows.join("\n")
        } else {
            files.join("\n")
        };
        if matches == MAX_GREP_MATCHES {
            text.push_str("\n... truncated after 100 matches");
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

fn build_glob(pattern: Option<&str>) -> Result<Option<GlobSet>, ToolError> {
    let Some(pattern) = pattern else {
        return Ok(None);
    };
    let mut builder = GlobSetBuilder::new();
    builder.add(Glob::new(pattern).map_err(|error| ToolError::Validation {
        message: error.to_string(),
    })?);
    builder
        .build()
        .map(Some)
        .map_err(|error| ToolError::Validation {
            message: error.to_string(),
        })
}

fn matches_glob(glob: Option<&GlobSet>, base_dir: &Path, path: &Path) -> bool {
    let Some(glob) = glob else { return true };
    glob.is_match(path.strip_prefix(base_dir).unwrap_or(path))
}

fn display_path(base_dir: &Path, path: &Path) -> String {
    path.strip_prefix(base_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
