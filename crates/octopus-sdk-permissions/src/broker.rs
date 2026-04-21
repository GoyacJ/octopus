//! Approval broker for permission prompts.

use std::sync::Arc;

use octopus_sdk_contracts::{
    AskPrompt, AskResolver, EventSink, PermissionOutcome, SessionEvent, ToolCallRequest,
};

#[derive(Clone)]
pub struct ApprovalBroker {
    event_sink: Arc<dyn EventSink>,
    ask_resolver: Arc<dyn AskResolver>,
}

impl ApprovalBroker {
    #[must_use]
    pub fn new(event_sink: Arc<dyn EventSink>, ask_resolver: Arc<dyn AskResolver>) -> Self {
        Self {
            event_sink,
            ask_resolver,
        }
    }

    pub async fn request_approval(
        &self,
        call: &ToolCallRequest,
        prompt: AskPrompt,
    ) -> PermissionOutcome {
        let prompt_id = format!("approval:{}", call.id.0);
        self.event_sink.emit(SessionEvent::Ask {
            prompt: prompt.clone(),
        });

        match self.ask_resolver.resolve(&prompt_id, &prompt).await {
            Ok(answer) if is_approval_answer(answer.option_id.as_str()) => PermissionOutcome::Allow,
            Ok(answer) => PermissionOutcome::Deny {
                reason: format!(
                    "tool '{}' denied by approval option '{}'",
                    call.name, answer.option_id
                ),
            },
            Err(error) => PermissionOutcome::Deny {
                reason: format!("approval failed for '{}': {error}", call.name),
            },
        }
    }
}

fn is_approval_answer(option_id: &str) -> bool {
    matches!(option_id, "approve" | "allow" | "ok" | "yes")
}
