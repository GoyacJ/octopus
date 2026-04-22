use futures::StreamExt;
use octopus_sdk_contracts::{AssistantEvent, Message, Role, SessionEvent, SessionId};
use octopus_sdk_session::{EventRange, SessionStore};

use crate::RuntimeError;

pub(crate) fn message_event(message: Message) -> SessionEvent {
    match message.role {
        Role::User => SessionEvent::UserMessage(message),
        Role::System | Role::Assistant | Role::Tool => SessionEvent::AssistantMessage(message),
    }
}

pub(crate) async fn load_transcript(
    store: &dyn SessionStore,
    session_id: &SessionId,
) -> Result<Vec<Message>, RuntimeError> {
    let mut stream = store.stream(session_id, EventRange::default()).await?;
    let mut transcript = Vec::new();

    while let Some(event) = stream.next().await {
        match event? {
            SessionEvent::UserMessage(message) | SessionEvent::AssistantMessage(message) => {
                if !is_usage_marker_message(&message)? {
                    transcript.push(message);
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
