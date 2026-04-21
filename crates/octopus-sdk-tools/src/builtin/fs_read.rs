use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

use async_trait::async_trait;
use octopus_sdk_contracts::ContentBlock;
use serde::Deserialize;
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

const MAX_READ_LINES: usize = 2_000;
const MAX_READ_BYTES: usize = 500 * 1024;

#[derive(Debug, Deserialize)]
struct FileReadInput {
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

pub struct FileReadTool {
    spec: ToolSpec,
}

impl FileReadTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "read_file".into(),
                description: "Read a text file with stable inline line numbers.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": { "type": "string" },
                        "offset": { "type": "integer", "minimum": 0 },
                        "limit": { "type": "integer", "minimum": 0 }
                    }
                }),
                category: ToolCategory::Read,
            },
        }
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
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
        let input: FileReadInput = serde_json::from_value(input)?;
        let path = resolve_existing_path(&ctx.working_dir, &input.path)?;
        ensure_text_file(&path)?;

        let content = fs::read_to_string(&path).map_err(|error| ToolError::Execution {
            message: format!("failed to read {}: {error}", path.display()),
        })?;
        let lines = content.lines().collect::<Vec<_>>();
        let start = input.offset.unwrap_or(0).min(lines.len());
        let end = input.limit.map_or(lines.len(), |limit| {
            start.saturating_add(limit).min(lines.len())
        });
        let (body, truncated) = render_numbered_lines(&lines[start..end], start + 1);

        let mut text = body;
        if truncated {
            text.push_str("\n... truncated after 2000 lines or 512000 bytes");
        }

        Ok(ToolResult {
            content: vec![ContentBlock::Text { text }],
            is_error: false,
            duration_ms: started.elapsed().as_millis() as u64,
            render: None,
        })
    }
}

fn resolve_existing_path(base_dir: &Path, input: &str) -> Result<PathBuf, ToolError> {
    let candidate = if Path::new(input).is_absolute() {
        PathBuf::from(input)
    } else {
        base_dir.join(input)
    };
    dunce::canonicalize(&candidate).map_err(|error| ToolError::Execution {
        message: format!("failed to resolve {}: {error}", candidate.display()),
    })
}

fn ensure_text_file(path: &Path) -> Result<(), ToolError> {
    let mut file = fs::File::open(path).map_err(|error| ToolError::Execution {
        message: format!("failed to open {}: {error}", path.display()),
    })?;
    let mut chunk = [0_u8; 8_192];
    let read = file
        .read(&mut chunk)
        .map_err(|error| ToolError::Execution {
            message: format!("failed to inspect {}: {error}", path.display()),
        })?;
    if chunk[..read].contains(&0) {
        return Err(ToolError::Validation {
            message: format!("{} appears to be binary", path.display()),
        });
    }
    Ok(())
}

fn render_numbered_lines(lines: &[&str], start_line: usize) -> (String, bool) {
    let mut rendered = Vec::new();
    let mut used = 0_usize;

    for (index, line) in lines.iter().enumerate() {
        let entry = format!("{:06}|{line}", start_line + index);
        let next = used + entry.len() + usize::from(!rendered.is_empty());
        if rendered.len() == MAX_READ_LINES || next > MAX_READ_BYTES {
            return (rendered.join("\n"), true);
        }
        used = next;
        rendered.push(entry);
    }

    (rendered.join("\n"), false)
}
