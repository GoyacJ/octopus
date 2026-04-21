use std::time::{Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    ContentBlock, EventId, RenderBlock, RenderKind, RenderLifecycle, RenderMeta, SessionEvent,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

#[derive(Debug, Deserialize)]
struct TodoWriteInput {
    todos: Vec<TodoItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TodoItem {
    content: String,
    active_form: String,
    status: TodoStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl TodoStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

pub struct TodoWriteTool {
    spec: ToolSpec,
}

impl TodoWriteTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "todo_write".into(),
                description: "Update the structured todo list for the current session.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["todos"],
                    "properties": {
                        "todos": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "required": ["content", "activeForm", "status"],
                                "properties": {
                                    "content": { "type": "string" },
                                    "activeForm": { "type": "string" },
                                    "status": {
                                        "type": "string",
                                        "enum": ["pending", "in_progress", "completed"]
                                    }
                                }
                            }
                        }
                    }
                }),
                category: ToolCategory::Write,
            },
        }
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TodoWriteTool {
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
        let input: TodoWriteInput = serde_json::from_value(input)?;
        validate_todos(&input.todos)?;

        let render = RenderBlock {
            kind: RenderKind::Record,
            payload: json!({
                "title": "Todo list",
                "rows": input.todos.iter().map(|todo| json!({
                    "label": todo.status.as_str(),
                    "value": format!("{} ({})", todo.content, todo.active_form),
                })).collect::<Vec<_>>()
            }),
            meta: render_meta(),
        };
        ctx.event_sink.emit(SessionEvent::Render {
            block: render.clone(),
            lifecycle: RenderLifecycle::OnToolResult,
        });

        Ok(ToolResult {
            content: vec![ContentBlock::Text {
                text: format!("updated {} todos", input.todos.len()),
            }],
            is_error: false,
            duration_ms: started.elapsed().as_millis() as u64,
            render: Some(render),
        })
    }
}

fn validate_todos(todos: &[TodoItem]) -> Result<(), ToolError> {
    if todos.is_empty() {
        return Err(ToolError::Validation {
            message: "todos must not be empty".into(),
        });
    }
    if todos.iter().any(|todo| todo.content.trim().is_empty()) {
        return Err(ToolError::Validation {
            message: "todo content must not be empty".into(),
        });
    }
    if todos.iter().any(|todo| todo.active_form.trim().is_empty()) {
        return Err(ToolError::Validation {
            message: "todo activeForm must not be empty".into(),
        });
    }
    Ok(())
}

fn render_meta() -> RenderMeta {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis() as i64;

    RenderMeta {
        id: EventId::new_v4(),
        parent: None,
        ts_ms,
    }
}
