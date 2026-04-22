use futures::StreamExt;
use octopus_sdk_contracts::{
    AssistantEvent, ContentBlock, EventId, Message, RenderBlock, RenderKind, RenderMeta, Role,
    StopReason, ToolCallRequest, Usage,
};
use octopus_sdk_model::ModelStream;
use octopus_sdk_observability::{TraceSpan, TraceValue, Tracer, UsageLedger};
use serde_json::json;

use crate::RuntimeError;

pub(crate) struct AssistantTurn {
    pub message: Option<Message>,
    pub tool_calls: Vec<ToolCallRequest>,
    pub stop_reason: StopReason,
    pub rendered_text: String,
    pub usage: Usage,
}

pub(crate) async fn collect_assistant_turn(
    mut stream: ModelStream,
    tracer: &dyn Tracer,
    usage_ledger: &UsageLedger,
) -> Result<AssistantTurn, RuntimeError> {
    let mut rendered_text = String::new();
    let mut blocks = Vec::new();
    let mut tool_calls = Vec::new();
    let mut usage = Usage::default();
    let mut stop_reason = StopReason::EndTurn;

    while let Some(event) = stream.next().await {
        let event = event?;
        tracer.record(trace_for_assistant_event(&event));
        usage_ledger.record_assistant_event(&event);

        match event {
            AssistantEvent::TextDelta(delta) => {
                rendered_text.push_str(&delta);
            }
            AssistantEvent::ToolUse { id, name, input } => {
                tool_calls.push(ToolCallRequest {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                });
                blocks.push(ContentBlock::ToolUse { id, name, input });
            }
            AssistantEvent::Usage(next_usage) => {
                usage = next_usage;
            }
            AssistantEvent::PromptCache(_) => {}
            AssistantEvent::MessageStop {
                stop_reason: next_reason,
            } => {
                stop_reason = next_reason;
                break;
            }
        }
    }

    if !rendered_text.is_empty() {
        blocks.insert(
            0,
            ContentBlock::Text {
                text: rendered_text.clone(),
            },
        );
    }

    Ok(AssistantTurn {
        message: (!blocks.is_empty()).then_some(Message {
            role: Role::Assistant,
            content: blocks,
        }),
        tool_calls,
        stop_reason,
        rendered_text,
        usage,
    })
}

pub(crate) fn usage_message(usage: Usage) -> Result<Message, RuntimeError> {
    Ok(Message {
        role: Role::Assistant,
        content: vec![ContentBlock::Text {
            text: serde_json::to_string(&AssistantEvent::Usage(usage))?,
        }],
    })
}

pub(crate) fn text_render_block(rendered_text: &str, parent: Option<EventId>) -> RenderBlock {
    RenderBlock {
        kind: RenderKind::Markdown,
        payload: json!({ "text": rendered_text }),
        meta: RenderMeta {
            id: EventId::new_v4(),
            parent,
            ts_ms: now_ms(),
        },
    }
}

fn trace_for_assistant_event(event: &AssistantEvent) -> TraceSpan {
    let kind = match event {
        AssistantEvent::TextDelta(_) => "text_delta",
        AssistantEvent::ToolUse { .. } => "tool_use",
        AssistantEvent::Usage(_) => "usage",
        AssistantEvent::PromptCache(_) => "prompt_cache",
        AssistantEvent::MessageStop { .. } => "message_stop",
    };

    TraceSpan::new("assistant_event").with_field("kind", TraceValue::String(kind.into()))
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should stay after unix epoch")
        .as_millis() as i64
}
