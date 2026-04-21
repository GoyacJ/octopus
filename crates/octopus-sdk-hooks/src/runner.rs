use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use octopus_sdk_contracts::{HookDecision, HookEvent, Message, RewritePayload};
use thiserror::Error;

#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;

    async fn on_event(&self, event: &HookEvent) -> HookDecision;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookSource {
    Plugin { plugin_id: String },
    Workspace,
    Defaults,
    Project,
    Session,
}

#[derive(Clone)]
pub struct HookRegistration {
    pub hook: Arc<dyn Hook>,
    pub source: HookSource,
    pub priority: i32,
    pub name: String,
}

impl HookRegistration {
    #[must_use]
    pub fn new(name: &str, hook: Arc<dyn Hook>, source: HookSource, priority: i32) -> Self {
        Self {
            hook,
            source,
            priority,
            name: name.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HookRunOutcome {
    pub decisions: Vec<(String, HookDecision)>,
    pub final_payload: Option<RewritePayload>,
    pub aborted: Option<String>,
}

#[derive(Debug, Error)]
pub enum HookError {
    #[error("rewrite is not allowed for hook event `{event_kind}`")]
    RewriteNotAllowed { event_kind: &'static str },
    #[error("inject is not allowed for hook event `{event_kind}`")]
    InjectNotAllowed { event_kind: &'static str },
    #[error("hook `{name}` panicked")]
    HookPanic { name: String },
    #[error("hook serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct HookRunner {
    registrations: RwLock<Vec<HookRegistration>>,
}

impl Default for HookRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl HookRunner {
    #[must_use]
    pub fn new() -> Self {
        Self {
            registrations: RwLock::new(Vec::new()),
        }
    }

    pub fn register(&self, name: &str, hook: Arc<dyn Hook>, source: HookSource, priority: i32) {
        let resolved_name = if name.is_empty() {
            hook.name().to_string()
        } else {
            name.to_string()
        };

        self.registrations
            .write()
            .expect("hook registrations lock poisoned")
            .push(HookRegistration::new(
                &resolved_name,
                hook,
                source,
                priority,
            ));
    }

    pub fn unregister_by_source(&self, source: HookSource) -> usize {
        let mut registrations = self
            .registrations
            .write()
            .expect("hook registrations lock poisoned");
        let before = registrations.len();
        registrations.retain(|registration| registration.source != source);
        before - registrations.len()
    }

    pub async fn run(&self, event: HookEvent) -> Result<HookRunOutcome, HookError> {
        let mut current_event = event;
        let mut outcome = HookRunOutcome {
            decisions: Vec::new(),
            final_payload: None,
            aborted: None,
        };

        for registration in self.sorted_registrations() {
            let decision = self.invoke_hook(&registration, current_event.clone()).await?;
            outcome
                .decisions
                .push((registration.name.clone(), decision.clone()));

            match decision {
                HookDecision::Continue => {}
                HookDecision::Rewrite(payload) => {
                    let applied =
                        apply_rewrite(&mut current_event, payload).map_err(|event_kind| {
                            HookError::RewriteNotAllowed { event_kind }
                        })?;
                    outcome.final_payload = Some(applied);
                }
                HookDecision::Abort { reason } => {
                    tracing::debug!(hook = %registration.name, %reason, "hook aborted event");
                    outcome.aborted = Some(reason);
                    break;
                }
                HookDecision::InjectMessage(message) => {
                    if !allows_inject(&current_event) {
                        return Err(HookError::InjectNotAllowed {
                            event_kind: event_kind(&current_event),
                        });
                    }
                    outcome.final_payload = Some(RewritePayload::UserPrompt { message });
                }
            }
        }

        Ok(outcome)
    }

    fn sorted_registrations(&self) -> Vec<HookRegistration> {
        let mut registrations = self
            .registrations
            .read()
            .expect("hook registrations lock poisoned")
            .clone();
        registrations.sort_by(|left, right| {
            source_rank(&left.source)
                .cmp(&source_rank(&right.source))
                .then_with(|| left.priority.cmp(&right.priority))
                .then_with(|| left.name.cmp(&right.name))
        });
        registrations
    }

    async fn invoke_hook(
        &self,
        registration: &HookRegistration,
        event: HookEvent,
    ) -> Result<HookDecision, HookError> {
        let hook = Arc::clone(&registration.hook);
        let name = registration.name.clone();
        tokio::spawn(async move { hook.on_event(&event).await })
            .await
            .map_err(|_| HookError::HookPanic { name })
    }
}

fn event_kind(event: &HookEvent) -> &'static str {
    match event {
        HookEvent::PreToolUse { .. } => "pre_tool_use",
        HookEvent::PostToolUse { .. } => "post_tool_use",
        HookEvent::Stop { .. } => "stop",
        HookEvent::SessionStart { .. } => "session_start",
        HookEvent::SessionEnd { .. } => "session_end",
        HookEvent::UserPromptSubmit { .. } => "user_prompt_submit",
        HookEvent::PreCompact { .. } => "pre_compact",
        HookEvent::PostCompact { .. } => "post_compact",
    }
}

fn allows_inject(event: &HookEvent) -> bool {
    matches!(
        event,
        HookEvent::Stop { .. } | HookEvent::UserPromptSubmit { .. }
    )
}

fn source_rank(source: &HookSource) -> u8 {
    match source {
        HookSource::Plugin { .. } => 0,
        HookSource::Workspace => 1,
        HookSource::Defaults => 2,
        HookSource::Project => 3,
        HookSource::Session => 4,
    }
}

fn apply_rewrite(
    event: &mut HookEvent,
    payload: RewritePayload,
) -> Result<RewritePayload, &'static str> {
    match (event, payload) {
        (HookEvent::PreToolUse { call, .. }, RewritePayload::ToolCall { call: next }) => {
            *call = next.clone();
            Ok(RewritePayload::ToolCall { call: next })
        }
        (HookEvent::PostToolUse { result, .. }, RewritePayload::ToolResult { result: next }) => {
            *result = next.clone();
            Ok(RewritePayload::ToolResult { result: next })
        }
        (HookEvent::UserPromptSubmit { message }, RewritePayload::UserPrompt { message: next }) => {
            *message = next.clone();
            Ok(RewritePayload::UserPrompt { message: next })
        }
        (HookEvent::PreCompact { ctx, .. }, RewritePayload::Compaction { ctx: next }) => {
            *ctx = next.clone();
            Ok(RewritePayload::Compaction { ctx: next })
        }
        (event, _) => Err(event_kind(event)),
    }
}

#[allow(dead_code)]
fn serialize_message(message: &Message) -> Result<String, HookError> {
    Ok(serde_json::to_string(message)?)
}
