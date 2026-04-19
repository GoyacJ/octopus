use std::collections::BTreeMap;

use api::{ContentBlockDelta, OutputContentBlock, StreamEvent};
use octopus_core::AppError;
use runtime::{AssistantEvent, TokenUsage};

use super::{ModelExecutionDeliverable, RuntimeConversationExecution};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConversationTurnEvent {
    RequestMetadata {
        request_id: Option<String>,
    },
    TextDelta(String),
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    Usage(TokenUsage),
    Stop {
        reason: Option<String>,
    },
}

#[derive(Debug, Default)]
pub(crate) struct ProviderStreamBridge {
    tool_uses: BTreeMap<u32, ToolUseBuffer>,
}

impl ProviderStreamBridge {
    pub(crate) fn request_metadata(
        &mut self,
        request_id: Option<String>,
    ) -> Option<ConversationTurnEvent> {
        if request_id.is_some() {
            return Some(ConversationTurnEvent::RequestMetadata { request_id });
        }
        None
    }

    pub(crate) fn ingest(
        &mut self,
        event: StreamEvent,
    ) -> Result<Vec<ConversationTurnEvent>, AppError> {
        let mut events = Vec::new();
        match event {
            StreamEvent::ContentBlockStart(start) => {
                if let OutputContentBlock::ToolUse { id, name, .. } = start.content_block {
                    self.tool_uses.insert(
                        start.index,
                        ToolUseBuffer {
                            id,
                            name,
                            input: String::new(),
                        },
                    );
                }
            }
            StreamEvent::ContentBlockDelta(delta) => match delta.delta {
                ContentBlockDelta::TextDelta { text } => {
                    if !text.is_empty() {
                        events.push(ConversationTurnEvent::TextDelta(text));
                    }
                }
                ContentBlockDelta::InputJsonDelta { partial_json } => {
                    if let Some(tool_use) = self.tool_uses.get_mut(&delta.index) {
                        tool_use.input.push_str(&partial_json);
                    }
                }
                ContentBlockDelta::ThinkingDelta { .. }
                | ContentBlockDelta::SignatureDelta { .. } => {}
            },
            StreamEvent::ContentBlockStop(stop) => {
                if let Some(tool_use) = self.tool_uses.remove(&stop.index) {
                    events.push(ConversationTurnEvent::ToolUse {
                        id: tool_use.id,
                        name: tool_use.name,
                        input: tool_use.input,
                    });
                }
            }
            StreamEvent::MessageDelta(delta) => {
                events.push(ConversationTurnEvent::Usage(delta.usage.token_usage()));
                events.push(ConversationTurnEvent::Stop {
                    reason: delta.delta.stop_reason,
                });
            }
            StreamEvent::MessageStart(_) | StreamEvent::MessageStop(_) => {}
        }
        Ok(events)
    }
}

pub(crate) fn runtime_conversation_execution_from_turn_events(
    events: Vec<ConversationTurnEvent>,
    deliverables: Vec<ModelExecutionDeliverable>,
) -> RuntimeConversationExecution {
    let events = events
        .into_iter()
        .filter_map(|event| match event {
            ConversationTurnEvent::RequestMetadata { .. } => None,
            ConversationTurnEvent::TextDelta(text) => Some(AssistantEvent::TextDelta(text)),
            ConversationTurnEvent::ToolUse { id, name, input } => {
                Some(AssistantEvent::ToolUse { id, name, input })
            }
            ConversationTurnEvent::Usage(usage) => Some(AssistantEvent::Usage(usage)),
            ConversationTurnEvent::Stop { .. } => Some(AssistantEvent::MessageStop),
        })
        .collect();

    RuntimeConversationExecution {
        events,
        deliverables,
    }
}

#[derive(Debug)]
struct ToolUseBuffer {
    id: String,
    name: String,
    input: String,
}
