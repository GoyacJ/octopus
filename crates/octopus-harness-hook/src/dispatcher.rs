use std::panic::AssertUnwindSafe;
use std::time::{Duration, Instant};

use futures::FutureExt;
use harness_contracts::{
    Decision, HookError, HookEventKind, HookFailureCauseKind, HookFailureMode,
    HookOutcomeDiscriminant, InconsistentReason, TransportFailureKind,
};

use crate::{
    ContextPatch, HookContext, HookEvent, HookOutcome, HookRegistrySnapshot, PreToolUseOutcome,
};

#[derive(Clone)]
pub struct HookDispatcher {
    snapshot: HookRegistrySnapshot,
}

impl HookDispatcher {
    pub fn new(snapshot: HookRegistrySnapshot) -> Self {
        Self { snapshot }
    }

    pub async fn dispatch(
        &self,
        event: HookEvent,
        ctx: HookContext,
    ) -> Result<DispatchResult, HookError> {
        if ctx.replay_mode == crate::ReplayMode::Audit {
            return Ok(DispatchResult::default());
        }

        let kind = event.kind();
        let handlers = self.snapshot.handlers_for(kind.clone());
        if handlers.is_empty() {
            return Ok(DispatchResult::default());
        }

        if kind == HookEventKind::PreToolUse {
            return Ok(self.dispatch_pre_tool_use(event, ctx, handlers).await);
        }

        let mut result = DispatchResult::default();
        let mut current = HookOutcome::Continue;

        for handler in handlers {
            let started = Instant::now();
            let outcome = AssertUnwindSafe(handler.handle(event.clone(), ctx.clone()))
                .catch_unwind()
                .await;
            let duration = started.elapsed();

            let outcome = match outcome {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(error)) => {
                    let failure = failure_record(
                        handler.handler_id(),
                        handler.failure_mode(),
                        duration,
                        failure_cause_from_error(error),
                    );
                    if apply_failure(&mut result, failure) == FailureAction::Block {
                        return Ok(result);
                    }
                    continue;
                }
                Err(panic) => {
                    let failure = failure_record(
                        handler.handler_id(),
                        handler.failure_mode(),
                        duration,
                        HookFailureCause::Panicked {
                            snippet: panic_snippet(panic),
                        },
                    );
                    if apply_failure(&mut result, failure) == FailureAction::Block {
                        return Ok(result);
                    }
                    continue;
                }
            };

            if let Some(cause) = unsupported_for(kind.clone(), &outcome) {
                let failure = failure_record(
                    handler.handler_id(),
                    handler.failure_mode(),
                    duration,
                    cause,
                );
                if apply_failure(&mut result, failure) == FailureAction::Block {
                    return Ok(result);
                }
                continue;
            }

            result.trail.push(HookInvocationRecord {
                handler_id: handler.handler_id().to_owned(),
                outcome: outcome.clone(),
                duration,
            });

            match outcome {
                HookOutcome::Continue => {}
                HookOutcome::Block { .. } => {
                    result.final_outcome = outcome;
                    return Ok(result);
                }
                _ => current = outcome,
            }
        }

        result.final_outcome = current;
        Ok(result)
    }

    async fn dispatch_pre_tool_use(
        &self,
        event: HookEvent,
        ctx: HookContext,
        handlers: Vec<std::sync::Arc<dyn crate::HookHandler>>,
    ) -> DispatchResult {
        let original_event = event;
        let mut event = original_event.clone();
        let mut result = DispatchResult::default();
        let mut accumulator = PreToolUseAccumulator::default();

        for handler in handlers {
            let started = Instant::now();
            let outcome = AssertUnwindSafe(handler.handle(event.clone(), ctx.clone()))
                .catch_unwind()
                .await;
            let duration = started.elapsed();

            let outcome = match outcome {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(error)) => {
                    let failure = failure_record(
                        handler.handler_id(),
                        handler.failure_mode(),
                        duration,
                        failure_cause_from_error(error),
                    );
                    apply_pre_tool_use_failure(&mut result, failure);
                    return result;
                }
                Err(panic) => {
                    let failure = failure_record(
                        handler.handler_id(),
                        handler.failure_mode(),
                        duration,
                        HookFailureCause::Panicked {
                            snippet: panic_snippet(panic),
                        },
                    );
                    apply_pre_tool_use_failure(&mut result, failure);
                    return result;
                }
            };

            if let Some(cause) = unsupported_for(HookEventKind::PreToolUse, &outcome) {
                let failure = failure_record(
                    handler.handler_id(),
                    handler.failure_mode(),
                    duration,
                    cause,
                );
                apply_pre_tool_use_failure(&mut result, failure);
                return result;
            }

            let mut trail_outcome = outcome.clone();
            match outcome {
                HookOutcome::Continue => {}
                HookOutcome::Block { reason } => {
                    result.trail.push(HookInvocationRecord {
                        handler_id: handler.handler_id().to_owned(),
                        outcome: HookOutcome::Block {
                            reason: reason.clone(),
                        },
                        duration,
                    });
                    result.final_outcome = HookOutcome::Block { reason };
                    return result;
                }
                HookOutcome::PreToolUse(pre_tool_use) => {
                    if let Err(reason) = pre_tool_use.validate() {
                        let failure = failure_record(
                            handler.handler_id(),
                            handler.failure_mode(),
                            duration,
                            HookFailureCause::Inconsistent { reason },
                        );
                        apply_pre_tool_use_failure(&mut result, failure);
                        return result;
                    }

                    if let Some(reason) = pre_tool_use.block.clone() {
                        trail_outcome = HookOutcome::Block {
                            reason: reason.clone(),
                        };
                        result.trail.push(HookInvocationRecord {
                            handler_id: handler.handler_id().to_owned(),
                            outcome: trail_outcome,
                            duration,
                        });
                        result.final_outcome = HookOutcome::Block { reason };
                        return result;
                    }

                    accumulator.merge(handler.handler_id(), handler.priority(), pre_tool_use);
                    if let Some(input) = accumulator.rewrite_input.clone() {
                        event = replace_pre_tool_use_input(&original_event, input);
                    }
                }
                _ => unreachable!("unsupported pre-tool-use outcomes are handled above"),
            }

            result.trail.push(HookInvocationRecord {
                handler_id: handler.handler_id().to_owned(),
                outcome: trail_outcome,
                duration,
            });
        }

        result.final_outcome = accumulator.into_outcome();
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchResult {
    pub final_outcome: HookOutcome,
    pub trail: Vec<HookInvocationRecord>,
    pub failures: Vec<HookFailureRecord>,
}

impl Default for DispatchResult {
    fn default() -> Self {
        Self {
            final_outcome: HookOutcome::Continue,
            trail: Vec::new(),
            failures: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HookInvocationRecord {
    pub handler_id: String,
    pub outcome: HookOutcome,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HookFailureRecord {
    pub handler_id: String,
    pub mode: HookFailureMode,
    pub cause: HookFailureCause,
    pub duration: Duration,
    pub cause_kind: HookFailureCauseKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HookFailureCause {
    Unsupported {
        kind: HookOutcomeDiscriminant,
    },
    Inconsistent {
        reason: InconsistentReason,
    },
    Panicked {
        snippet: String,
    },
    Timeout,
    Transport {
        kind: TransportFailureKind,
        detail: String,
    },
    Unauthorized {
        capability: String,
    },
}

#[derive(Default)]
struct PreToolUseAccumulator {
    rewrite_input: Option<serde_json::Value>,
    override_permission: Option<PermissionOverride>,
    additional_context: Option<ContextPatch>,
}

impl PreToolUseAccumulator {
    fn merge(&mut self, handler_id: &str, priority: i32, outcome: PreToolUseOutcome) {
        if let Some(input) = outcome.rewrite_input {
            self.rewrite_input = Some(input);
        }
        if let Some(decision) = outcome.override_permission {
            self.override_permission = Some(match self.override_permission.take() {
                Some(existing) => existing.winner(PermissionOverride {
                    handler_id: handler_id.to_owned(),
                    priority,
                    decision,
                }),
                None => PermissionOverride {
                    handler_id: handler_id.to_owned(),
                    priority,
                    decision,
                },
            });
        }
        if outcome.additional_context.is_some() {
            self.additional_context = outcome.additional_context;
        }
    }

    fn into_outcome(self) -> HookOutcome {
        if self.rewrite_input.is_none()
            && self.override_permission.is_none()
            && self.additional_context.is_none()
        {
            return HookOutcome::Continue;
        }

        HookOutcome::PreToolUse(PreToolUseOutcome {
            rewrite_input: self.rewrite_input,
            override_permission: self.override_permission.map(|winner| winner.decision),
            additional_context: self.additional_context,
            block: None,
        })
    }
}

struct PermissionOverride {
    handler_id: String,
    priority: i32,
    decision: Decision,
}

impl PermissionOverride {
    fn winner(self, other: Self) -> Self {
        match other.priority.cmp(&self.priority) {
            std::cmp::Ordering::Greater => other,
            std::cmp::Ordering::Less => self,
            std::cmp::Ordering::Equal => {
                match (is_deny(&other.decision), is_deny(&self.decision)) {
                    (true, false) => other,
                    (false, true) => self,
                    _ if other.handler_id < self.handler_id => other,
                    _ => self,
                }
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FailureAction {
    Continue,
    Block,
}

fn apply_failure(result: &mut DispatchResult, failure: HookFailureRecord) -> FailureAction {
    let action = if failure.mode == HookFailureMode::FailClosed {
        result.final_outcome = HookOutcome::Block {
            reason: format!("hook handler {} failed", failure.handler_id),
        };
        FailureAction::Block
    } else {
        FailureAction::Continue
    };
    result.failures.push(failure);
    action
}

fn apply_pre_tool_use_failure(result: &mut DispatchResult, failure: HookFailureRecord) {
    if failure.mode == HookFailureMode::FailClosed {
        result.final_outcome = HookOutcome::Block {
            reason: format!("hook handler {} failed", failure.handler_id),
        };
    } else {
        result.final_outcome = HookOutcome::Continue;
    }
    result.failures.push(failure);
}

fn failure_record(
    handler_id: &str,
    mode: HookFailureMode,
    duration: Duration,
    cause: HookFailureCause,
) -> HookFailureRecord {
    let cause_kind = match &cause {
        HookFailureCause::Unsupported { .. } => HookFailureCauseKind::Unsupported,
        HookFailureCause::Inconsistent { .. } => HookFailureCauseKind::Inconsistent,
        HookFailureCause::Panicked { .. } => HookFailureCauseKind::Panicked,
        HookFailureCause::Timeout => HookFailureCauseKind::Timeout,
        HookFailureCause::Transport { .. } => HookFailureCauseKind::Transport,
        HookFailureCause::Unauthorized { .. } => HookFailureCauseKind::Unauthorized,
    };

    HookFailureRecord {
        handler_id: handler_id.to_owned(),
        mode,
        cause,
        duration,
        cause_kind,
    }
}

fn unsupported_for(kind: HookEventKind, outcome: &HookOutcome) -> Option<HookFailureCause> {
    let allowed = match kind {
        HookEventKind::UserPromptSubmit => matches!(
            outcome,
            HookOutcome::Continue | HookOutcome::RewriteInput(_) | HookOutcome::Block { .. }
        ),
        HookEventKind::PreToolUse => matches!(
            outcome,
            HookOutcome::Continue | HookOutcome::Block { .. } | HookOutcome::PreToolUse(_)
        ),
        HookEventKind::PostToolUse
        | HookEventKind::PostToolUseFailure
        | HookEventKind::SessionStart
        | HookEventKind::SubagentStart
        | HookEventKind::SubagentStop
        | HookEventKind::PostToolSearchMaterialize => {
            matches!(outcome, HookOutcome::Continue | HookOutcome::AddContext(_))
        }
        HookEventKind::PermissionRequest => matches!(
            outcome,
            HookOutcome::Continue | HookOutcome::OverridePermission(_)
        ),
        HookEventKind::TransformToolResult | HookEventKind::TransformTerminalOutput => {
            matches!(outcome, HookOutcome::Continue | HookOutcome::Transform(_))
        }
        HookEventKind::Elicitation
        | HookEventKind::PreToolSearch
        | HookEventKind::PreApiRequest => {
            matches!(outcome, HookOutcome::Continue | HookOutcome::Block { .. })
        }
        HookEventKind::PreLlmCall => {
            matches!(
                outcome,
                HookOutcome::Continue | HookOutcome::RewriteInput(_)
            )
        }
        _ => matches!(outcome, HookOutcome::Continue),
    };

    (!allowed).then(|| HookFailureCause::Unsupported {
        kind: outcome_discriminant(outcome),
    })
}

fn outcome_discriminant(outcome: &HookOutcome) -> HookOutcomeDiscriminant {
    match outcome {
        HookOutcome::Continue => HookOutcomeDiscriminant::Continue,
        HookOutcome::Block { .. } => HookOutcomeDiscriminant::Block,
        HookOutcome::PreToolUse(_) => HookOutcomeDiscriminant::PreToolUse,
        HookOutcome::RewriteInput(_) => HookOutcomeDiscriminant::RewriteInput,
        HookOutcome::OverridePermission(_) => HookOutcomeDiscriminant::OverridePermission,
        HookOutcome::AddContext(_) => HookOutcomeDiscriminant::AddContext,
        HookOutcome::Transform(_) => HookOutcomeDiscriminant::Transform,
    }
}

fn replace_pre_tool_use_input(event: &HookEvent, input: serde_json::Value) -> HookEvent {
    match event {
        HookEvent::PreToolUse {
            tool_use_id,
            tool_name,
            ..
        } => HookEvent::PreToolUse {
            tool_use_id: *tool_use_id,
            tool_name: tool_name.clone(),
            input,
        },
        _ => event.clone(),
    }
}

fn panic_snippet(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    "handler panicked".to_owned()
}

fn is_deny(decision: &Decision) -> bool {
    matches!(decision, Decision::DenyOnce | Decision::DenyPermanent)
}

fn failure_cause_from_error(error: HookError) -> HookFailureCause {
    match error {
        HookError::Timeout { .. } => HookFailureCause::Timeout,
        HookError::Inconsistent { reason, .. } => HookFailureCause::Inconsistent { reason },
        HookError::Unsupported { kind, .. } => HookFailureCause::Unsupported { kind },
        HookError::Transport { kind, detail } => HookFailureCause::Transport { kind, detail },
        HookError::Unauthorized(capability) => HookFailureCause::Unauthorized { capability },
        HookError::ProtocolParse(detail) => HookFailureCause::Transport {
            kind: TransportFailureKind::ProtocolVersionMismatch,
            detail,
        },
        HookError::Panicked { snippet, .. } => HookFailureCause::Panicked { snippet },
        HookError::Message(message) | HookError::HandlerError { cause: message, .. } => {
            HookFailureCause::Panicked { snippet: message }
        }
        _ => HookFailureCause::Panicked {
            snippet: "unknown hook error".to_owned(),
        },
    }
}
