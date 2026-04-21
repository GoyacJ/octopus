use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{
    AskPrompt, ContentBlock, EventId, Message, PromptCacheEvent, RenderBlock, RenderLifecycle,
    Role, ToolCallId, Usage,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Completed,
    Interrupted,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssistantEvent {
    TextDelta(String),
    ToolUse {
        id: ToolCallId,
        name: String,
        input: Value,
    },
    Usage(Usage),
    PromptCache(PromptCacheEvent),
    MessageStop {
        stop_reason: StopReason,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AssistantEventRepr {
    TextDelta {
        text: String,
    },
    ToolUse {
        id: ToolCallId,
        name: String,
        input: Value,
    },
    Usage {
        input_tokens: u32,
        output_tokens: u32,
        cache_creation_input_tokens: u32,
        cache_read_input_tokens: u32,
    },
    PromptCache {
        cache_read_input_tokens: u32,
        cache_creation_input_tokens: u32,
        breakpoint_count: u32,
    },
    MessageStop {
        stop_reason: StopReason,
    },
}

impl From<&AssistantEvent> for AssistantEventRepr {
    fn from(value: &AssistantEvent) -> Self {
        match value {
            AssistantEvent::TextDelta(text) => Self::TextDelta { text: text.clone() },
            AssistantEvent::ToolUse { id, name, input } => Self::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            },
            AssistantEvent::Usage(usage) => Self::Usage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_creation_input_tokens: usage.cache_creation_input_tokens,
                cache_read_input_tokens: usage.cache_read_input_tokens,
            },
            AssistantEvent::PromptCache(event) => Self::PromptCache {
                cache_read_input_tokens: event.cache_read_input_tokens,
                cache_creation_input_tokens: event.cache_creation_input_tokens,
                breakpoint_count: event.breakpoint_count,
            },
            AssistantEvent::MessageStop { stop_reason } => Self::MessageStop {
                stop_reason: stop_reason.clone(),
            },
        }
    }
}

impl From<AssistantEventRepr> for AssistantEvent {
    fn from(value: AssistantEventRepr) -> Self {
        match value {
            AssistantEventRepr::TextDelta { text } => Self::TextDelta(text),
            AssistantEventRepr::ToolUse { id, name, input } => Self::ToolUse { id, name, input },
            AssistantEventRepr::Usage {
                input_tokens,
                output_tokens,
                cache_creation_input_tokens,
                cache_read_input_tokens,
            } => Self::Usage(Usage {
                input_tokens,
                output_tokens,
                cache_creation_input_tokens,
                cache_read_input_tokens,
            }),
            AssistantEventRepr::PromptCache {
                cache_read_input_tokens,
                cache_creation_input_tokens,
                breakpoint_count,
            } => Self::PromptCache(PromptCacheEvent {
                cache_read_input_tokens,
                cache_creation_input_tokens,
                breakpoint_count,
            }),
            AssistantEventRepr::MessageStop { stop_reason } => Self::MessageStop { stop_reason },
        }
    }
}

impl Serialize for AssistantEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        AssistantEventRepr::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AssistantEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(AssistantEventRepr::deserialize(deserializer)?.into())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SessionEventRepr {
    SessionStarted {
        config_snapshot_id: String,
        effective_config_hash: String,
    },
    UserMessage {
        role: Role,
        content: Vec<ContentBlock>,
    },
    AssistantMessage {
        role: Role,
        content: Vec<ContentBlock>,
    },
    ToolExecuted {
        call: ToolCallId,
        name: String,
        duration_ms: u64,
        is_error: bool,
    },
    Render {
        block: RenderBlock,
        lifecycle: RenderLifecycle,
    },
    Ask {
        prompt: AskPrompt,
    },
    Checkpoint {
        id: String,
        anchor_event_id: EventId,
    },
    SessionEnded {
        reason: EndReason,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    SessionStarted {
        config_snapshot_id: String,
        effective_config_hash: String,
    },
    UserMessage(Message),
    AssistantMessage(Message),
    ToolExecuted {
        call: ToolCallId,
        name: String,
        duration_ms: u64,
        is_error: bool,
    },
    Render {
        block: RenderBlock,
        lifecycle: RenderLifecycle,
    },
    Ask {
        prompt: AskPrompt,
    },
    Checkpoint {
        id: String,
        anchor_event_id: EventId,
    },
    SessionEnded {
        reason: EndReason,
    },
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: SessionEvent);
}

impl From<&SessionEvent> for SessionEventRepr {
    fn from(value: &SessionEvent) -> Self {
        match value {
            SessionEvent::SessionStarted {
                config_snapshot_id,
                effective_config_hash,
            } => Self::SessionStarted {
                config_snapshot_id: config_snapshot_id.clone(),
                effective_config_hash: effective_config_hash.clone(),
            },
            SessionEvent::UserMessage(message) => Self::UserMessage {
                role: message.role.clone(),
                content: message.content.clone(),
            },
            SessionEvent::AssistantMessage(message) => Self::AssistantMessage {
                role: message.role.clone(),
                content: message.content.clone(),
            },
            SessionEvent::ToolExecuted {
                call,
                name,
                duration_ms,
                is_error,
            } => Self::ToolExecuted {
                call: call.clone(),
                name: name.clone(),
                duration_ms: *duration_ms,
                is_error: *is_error,
            },
            SessionEvent::Render { block, lifecycle } => Self::Render {
                block: block.clone(),
                lifecycle: lifecycle.clone(),
            },
            SessionEvent::Ask { prompt } => Self::Ask {
                prompt: prompt.clone(),
            },
            SessionEvent::Checkpoint {
                id,
                anchor_event_id,
            } => Self::Checkpoint {
                id: id.clone(),
                anchor_event_id: anchor_event_id.clone(),
            },
            SessionEvent::SessionEnded { reason } => Self::SessionEnded {
                reason: reason.clone(),
            },
        }
    }
}

impl From<SessionEventRepr> for SessionEvent {
    fn from(value: SessionEventRepr) -> Self {
        match value {
            SessionEventRepr::SessionStarted {
                config_snapshot_id,
                effective_config_hash,
            } => Self::SessionStarted {
                config_snapshot_id,
                effective_config_hash,
            },
            SessionEventRepr::UserMessage { role, content } => {
                Self::UserMessage(Message { role, content })
            }
            SessionEventRepr::AssistantMessage { role, content } => {
                Self::AssistantMessage(Message { role, content })
            }
            SessionEventRepr::ToolExecuted {
                call,
                name,
                duration_ms,
                is_error,
            } => Self::ToolExecuted {
                call,
                name,
                duration_ms,
                is_error,
            },
            SessionEventRepr::Render { block, lifecycle } => Self::Render { block, lifecycle },
            SessionEventRepr::Ask { prompt } => Self::Ask { prompt },
            SessionEventRepr::Checkpoint {
                id,
                anchor_event_id,
            } => Self::Checkpoint {
                id,
                anchor_event_id,
            },
            SessionEventRepr::SessionEnded { reason } => Self::SessionEnded { reason },
        }
    }
}

impl Serialize for SessionEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SessionEventRepr::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SessionEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(SessionEventRepr::deserialize(deserializer)?.into())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::{json, Value};

    use super::{AssistantEvent, EventSink, SessionEvent, StopReason};
    use crate::{EventId, ToolCallId, Usage};

    #[test]
    fn session_started_serializes_kind_before_payload_fields() {
        let event = SessionEvent::SessionStarted {
            config_snapshot_id: "cfg-1".into(),
            effective_config_hash: "hash-1".into(),
        };

        let serialized = serde_json::to_string(&event).expect("session event should serialize");

        assert!(serialized.starts_with("{\"kind\":\"session_started\""));
    }

    #[test]
    fn assistant_tool_use_preserves_json_input_payload() {
        let event = AssistantEvent::ToolUse {
            id: ToolCallId("call-1".into()),
            name: "search".into(),
            input: json!({ "query": "octopus" }),
        };

        let value = serde_json::to_value(&event).expect("assistant event should serialize");

        assert_eq!(value.get("kind"), Some(&Value::String("tool_use".into())));
        assert_eq!(value.get("input"), Some(&json!({ "query": "octopus" })));
    }

    #[test]
    fn assistant_usage_flattens_usage_payload_under_usage_kind() {
        let event = AssistantEvent::Usage(Usage {
            input_tokens: 3,
            output_tokens: 5,
            cache_creation_input_tokens: 7,
            cache_read_input_tokens: 11,
        });

        let value = serde_json::to_value(&event).expect("assistant event should serialize");

        assert_eq!(value.get("kind"), Some(&Value::String("usage".into())));
        assert_eq!(
            value.get("input_tokens"),
            Some(&Value::Number(3_u32.into()))
        );
        assert_eq!(
            value.get("output_tokens"),
            Some(&Value::Number(5_u32.into()))
        );
    }

    #[test]
    fn message_stop_serializes_stop_reason_in_snake_case() {
        let event = AssistantEvent::MessageStop {
            stop_reason: StopReason::EndTurn,
        };

        let value = serde_json::to_value(&event).expect("assistant event should serialize");

        assert_eq!(
            value.get("stop_reason"),
            Some(&Value::String("end_turn".into()))
        );
    }

    #[test]
    fn event_sink_is_object_safe() {
        struct NoopSink;

        impl EventSink for NoopSink {
            fn emit(&self, _event: SessionEvent) {}
        }

        let sink: Arc<dyn EventSink> = Arc::new(NoopSink);
        sink.emit(SessionEvent::Checkpoint {
            id: "checkpoint-1".into(),
            anchor_event_id: EventId("event-1".into()),
        });
    }
}
