use futures::StreamExt;
use octopus_sdk_contracts::{
    AssistantEvent, CompactionStrategyTag, EventId, Message, Role, SessionEvent, SessionId,
};
use octopus_sdk_session::{EventRange, SessionStore};

use crate::RuntimeError;

#[derive(Debug, Clone, Default)]
pub(crate) struct TranscriptState {
    pub messages: Vec<Message>,
    pub event_ids: Vec<EventId>,
}

impl TranscriptState {
    pub(crate) fn push(&mut self, event_id: EventId, message: Message) {
        self.event_ids.push(event_id);
        self.messages.push(message);
    }
}

pub(crate) fn message_event(message: Message) -> SessionEvent {
    match message.role {
        Role::User => SessionEvent::UserMessage(message),
        Role::System | Role::Assistant | Role::Tool => SessionEvent::AssistantMessage(message),
    }
}

pub(crate) async fn load_transcript(
    store: &dyn SessionStore,
    session_id: &SessionId,
) -> Result<TranscriptState, RuntimeError> {
    let mut stream = store
        .stream_records(session_id, EventRange::default())
        .await?;
    let mut records = Vec::new();

    while let Some(record) = stream.next().await {
        records.push(record?);
    }

    let compaction_checkpoint = records.iter().rev().find_map(|record| match &record.event {
        SessionEvent::Checkpoint {
            anchor_event_id,
            compaction: Some(compaction),
            ..
        } if matches!(compaction.strategy, CompactionStrategyTag::Summarize)
            && !compaction.summary.trim().is_empty() =>
        {
            Some((
                record.event_id.clone(),
                anchor_event_id.clone(),
                compaction.summary.clone(),
            ))
        }
        _ => None,
    });
    let mut transcript = TranscriptState::default();
    let mut replay_after_anchor = compaction_checkpoint.is_none();
    let mut found_anchor = replay_after_anchor;

    if let Some((checkpoint_event_id, _, summary)) = &compaction_checkpoint {
        transcript.push(
            checkpoint_event_id.clone(),
            Message {
                role: Role::System,
                content: vec![octopus_sdk_contracts::ContentBlock::Text {
                    text: summary.clone(),
                }],
            },
        );
    }

    for record in records {
        if let Some((_, anchor_event_id, _)) = &compaction_checkpoint {
            if !replay_after_anchor {
                if &record.event_id == anchor_event_id {
                    replay_after_anchor = true;
                    found_anchor = true;
                }
                continue;
            }
        }

        match record.event {
            SessionEvent::UserMessage(message) | SessionEvent::AssistantMessage(message) => {
                if !is_usage_marker_message(&message)? {
                    transcript.push(record.event_id, message);
                }
            }
            SessionEvent::SessionStarted { .. }
            | SessionEvent::SessionPluginsSnapshot { .. }
            | SessionEvent::ToolExecuted { .. }
            | SessionEvent::Render { .. }
            | SessionEvent::Ask { .. }
            | SessionEvent::Checkpoint { .. }
            | SessionEvent::SessionEnded { .. } => {}
        }
    }

    if !found_anchor {
        return Err(RuntimeError::Session(octopus_sdk_session::SessionError::Corrupted {
            reason: "compaction_anchor_event_not_found_during_replay".into(),
        }));
    }

    Ok(transcript)
}

fn is_usage_marker_message(message: &Message) -> Result<bool, RuntimeError> {
    if message.content.len() != 1 {
        return Ok(false);
    }

    let octopus_sdk_contracts::ContentBlock::Text { text } = &message.content[0] else {
        return Ok(false);
    };

    match serde_json::from_str::<AssistantEvent>(text) {
        Ok(AssistantEvent::Usage(_)) => Ok(true),
        Ok(_) | Err(_) => Ok(false),
    }
}
