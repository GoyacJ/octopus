use std::{
    collections::HashSet,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    AskError, AskPrompt, AskQuestion, ContentBlock, EventId, RenderBlock, RenderKind, RenderMeta,
    SessionEvent, ToolCallId,
};
use serde::Deserialize;
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

const ASK_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskUserQuestionInput {
    prompt_id: Option<String>,
    kind: Option<String>,
    questions: Vec<AskQuestion>,
}

pub struct AskUserQuestionTool {
    spec: ToolSpec,
}

impl AskUserQuestionTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "ask_user_question".into(),
                description: "Ask the host a structured question and wait for a selected answer."
                    .into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["questions"],
                    "properties": {
                        "promptId": { "type": "string" },
                        "kind": { "type": "string" },
                        "questions": {
                            "type": "array",
                            "minItems": 1,
                            "maxItems": 4,
                            "items": {
                                "type": "object",
                                "required": ["id", "question", "header", "multiSelect", "options"],
                                "properties": {
                                    "id": { "type": "string" },
                                    "question": { "type": "string" },
                                    "header": { "type": "string" },
                                    "multiSelect": { "type": "boolean" },
                                    "options": {
                                        "type": "array",
                                        "minItems": 2,
                                        "maxItems": 4,
                                        "items": {
                                            "type": "object",
                                            "required": ["id", "label", "description"],
                                            "properties": {
                                                "id": { "type": "string" },
                                                "label": { "type": "string" },
                                                "description": { "type": "string" },
                                                "preview": { "type": "string" },
                                                "previewFormat": { "type": "string", "enum": ["markdown", "html"] }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }),
                category: ToolCategory::Meta,
            },
        }
    }
}

impl Default for AskUserQuestionTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AskUserQuestionTool {
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
        let input: AskUserQuestionInput = serde_json::from_value(input)?;
        validate_prompt(&input.questions)?;

        let prompt = AskPrompt {
            kind: input.kind.unwrap_or_else(|| "ask-user".into()),
            questions: input.questions,
        };
        let prompt_id = input.prompt_id.unwrap_or_else(|| ToolCallId::new_v4().0);
        ctx.event_sink.emit(SessionEvent::Ask {
            prompt: prompt.clone(),
        });

        let answer = tokio::select! {
            () = ctx.cancellation.cancelled() => {
                return Err(ToolError::Cancelled { message: "ask prompt was cancelled before an answer arrived".into() });
            }
            answer = tokio::time::timeout(
                Duration::from_secs(ASK_TIMEOUT_SECS),
                ctx.ask_resolver.resolve(&prompt_id, &prompt),
            ) => {
                answer
                    .map_err(|_| ToolError::Timeout { message: format!("ask prompt exceeded timeout of {ASK_TIMEOUT_SECS} seconds") })??
            }
        };

        let render = RenderBlock {
            kind: RenderKind::Record,
            payload: json!({
                "title": "User answered",
                "rows": [{
                    "label": prompt.questions.first().map_or("answer", |question| question.header.as_str()),
                    "value": answer.text
                }]
            }),
            meta: render_meta(),
        };

        Ok(ToolResult {
            content: vec![ContentBlock::Text {
                text: format!("{} ({})", answer.text, answer.option_id),
            }],
            is_error: false,
            duration_ms: started.elapsed().as_millis() as u64,
            render: Some(render),
        })
    }
}

fn validate_prompt(questions: &[AskQuestion]) -> Result<(), ToolError> {
    if !(1..=4).contains(&questions.len()) {
        return Err(ToolError::Validation {
            message: "questions must contain between 1 and 4 items".into(),
        });
    }

    let mut unique_questions = HashSet::new();
    for question in questions {
        if !unique_questions.insert(question.question.trim().to_string()) {
            return Err(ToolError::Validation {
                message: "question text must be unique".into(),
            });
        }
        if !(2..=4).contains(&question.options.len()) {
            return Err(ToolError::Validation {
                message: format!(
                    "question `{}` must contain between 2 and 4 options",
                    question.id
                ),
            });
        }

        let mut unique_labels = HashSet::new();
        for option in &question.options {
            if !unique_labels.insert(option.label.trim().to_string()) {
                return Err(ToolError::Validation {
                    message: format!(
                        "question `{}` contains duplicate option labels",
                        question.id
                    ),
                });
            }
            let preview_pair = option.preview.is_some() == option.preview_format.is_some();
            if !preview_pair {
                return Err(ToolError::Validation {
                    message: format!(
                        "question `{}` has an option with mismatched preview fields",
                        question.id
                    ),
                });
            }
            if option.preview.is_some() && question.multi_select {
                return Err(ToolError::Validation {
                    message: format!(
                        "question `{}` cannot use preview with multiSelect=true",
                        question.id
                    ),
                });
            }
        }
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

impl From<AskError> for ToolError {
    fn from(value: AskError) -> Self {
        match value {
            AskError::NotResolvable => Self::Execution {
                message: AskError::NotResolvable.to_string(),
            },
            AskError::Timeout => Self::Timeout {
                message: AskError::Timeout.to_string(),
            },
            AskError::Cancelled => Self::Cancelled {
                message: AskError::Cancelled.to_string(),
            },
        }
    }
}
