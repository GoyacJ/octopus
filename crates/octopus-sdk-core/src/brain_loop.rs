use std::sync::Arc;

use octopus_sdk_context::{Compactor, PromptCtx, SessionView, SystemPromptBuilder};
use octopus_sdk_contracts::{
    CacheBreakpoint, CacheTtl, CompactionStrategyTag, HookDecision, HookEvent, Message,
    RenderLifecycle, SessionEvent, StopReason,
};
use octopus_sdk_model::{CacheControlStrategy, ModelError, ModelRequest, ModelRole};
use octopus_sdk_observability::{
    brain_iteration_span_id, session_span_id, session_trace_id, TraceSpan, TraceValue,
};
use octopus_sdk_tools::ToolSurfaceState;

use crate::{
    assistant_projection::{
        collect_assistant_turn, text_render_block, usage_message, AssistantTraceContext,
    },
    runtime::{RuntimeInner, SessionRuntimeState},
    session_boot::{
        estimate_tokens, load_transcript, message_event, persist_compaction_checkpoint,
        TranscriptState,
    },
    tool_dispatch::{execute_tool_round, DispatchContext},
    RuntimeError, SubmitTurnInput,
};

const MAX_BRAIN_LOOP_ITERATIONS: usize = 4;
const MAX_OVERFLOW_RETRIES_PER_TURN: usize = 1;
const CONTINUATION_COMPACTION_THRESHOLD: f32 = 1.0;

pub(crate) async fn submit_turn(
    inner: Arc<RuntimeInner>,
    session: SessionRuntimeState,
    input: SubmitTurnInput,
    cancellation: tokio_util::sync::CancellationToken,
) -> Result<(), RuntimeError> {
    if let Some(restored) = &session.pending_restore {
        inner
            .session_store
            .append(
                &input.session_id,
                message_event(restored.as_system_message()),
            )
            .await?;
    }
    inner
        .session_store
        .append(&input.session_id, message_event(input.message.clone()))
        .await?;
    let mut transcript = load_transcript(inner.session_store.as_ref(), &input.session_id).await?;
    let mut iteration = 0usize;
    let mut overflow_retries = 0usize;
    let mut budget_compacted = false;

    while iteration < MAX_BRAIN_LOOP_ITERATIONS {
        if cancellation.is_cancelled() {
            return Err(RuntimeError::Cancelled);
        }

        if !budget_compacted
            && maybe_compact_for_continuation(
                inner.as_ref(),
                &input.session_id,
                &mut transcript,
                session.token_budget,
            )
            .await?
        {
            budget_compacted = true;
            continue;
        }

        let trace_id = session_trace_id(&input.session_id.0);
        let iteration_span_id = brain_iteration_span_id(&input.session_id.0, iteration);
        let model_version = inner.model_provider.describe().catalog_version;
        inner.tracer.record(
            TraceSpan::new("brain_loop_iteration")
                .with_trace_id(trace_id.clone())
                .with_span_id(iteration_span_id.clone())
                .with_parent_span_id(session_span_id(&input.session_id.0))
                .with_agent_role("main")
                .with_field("session_id", TraceValue::String(input.session_id.0.clone()))
                .with_field("iteration", TraceValue::U64(iteration as u64))
                .with_field("model_id", TraceValue::String(session.model.0.clone()))
                .with_field("model_version", TraceValue::String(model_version.clone()))
                .with_field(
                    "config_snapshot_id",
                    TraceValue::String(session.config_snapshot_id.clone()),
                ),
        );

        let request = build_request(
            &session,
            &input.session_id,
            &inner.tool_registry,
            &inner.prompt_builder,
            &transcript.messages,
        );
        run_sampling_hook(
            inner.as_ref(),
            HookEvent::PreSampling {
                session: input.session_id.clone(),
            },
        )
        .await?;
        iteration += 1;
        let stream = match inner.model_provider.complete(request).await {
            Ok(stream) => {
                overflow_retries = 0;
                stream
            }
            Err(ModelError::Overloaded { retry_after_ms }) => {
                if overflow_retries >= MAX_OVERFLOW_RETRIES_PER_TURN {
                    return Err(ModelError::Overloaded { retry_after_ms }.into());
                }
                overflow_retries += 1;
                if let Some(delay_ms) = retry_after_ms.filter(|ms| *ms > 0) {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms.min(25))).await;
                }
                continue;
            }
            Err(error @ ModelError::PromptTooLong { max, .. }) => {
                if compact_transcript(inner.as_ref(), &input.session_id, &mut transcript, max, 0.0)
                    .await?
                {
                    continue;
                }
                return Err(error.into());
            }
            Err(error) => return Err(error.into()),
        };
        let turn = collect_assistant_turn(
            stream,
            inner.tracer.as_ref(),
            inner.usage_ledger.as_ref(),
            &AssistantTraceContext {
                trace_id: &trace_id,
                parent_span_id: &iteration_span_id,
                session_id: &input.session_id.0,
                model_id: &session.model.0,
                model_version: &model_version,
                config_snapshot_id: &session.config_snapshot_id,
            },
        )
        .await?;
        run_sampling_hook(
            inner.as_ref(),
            HookEvent::PostSampling {
                session: input.session_id.clone(),
                stop_reason: turn.stop_reason.clone(),
            },
        )
        .await?;

        if turn.usage != octopus_sdk_contracts::Usage::default() {
            inner
                .session_store
                .append(&input.session_id, message_event(usage_message(turn.usage)?))
                .await?;
        }

        let mut last_assistant_event_id = None;
        if let Some(message) = turn.message.clone() {
            let event_id = inner
                .session_store
                .append(&input.session_id, message_event(message.clone()))
                .await?;
            last_assistant_event_id = Some(event_id.clone());
            transcript.push(event_id.clone(), message);

            if !turn.rendered_text.is_empty() {
                inner
                    .session_store
                    .append(
                        &input.session_id,
                        SessionEvent::Render {
                            blocks: vec![text_render_block(&turn.rendered_text, Some(event_id))],
                            lifecycle: RenderLifecycle::assistant_message(),
                        },
                    )
                    .await?;
            }
        }

        if turn.stop_reason == StopReason::ToolUse && !turn.tool_calls.is_empty() {
            budget_compacted |= execute_tool_round(
                DispatchContext {
                    session_id: &input.session_id,
                    working_dir: &session.working_dir,
                    permission_mode: session.permission_mode,
                    permissions: Arc::clone(&inner.permission_gate),
                    ask_resolver: Arc::clone(&inner.ask_resolver),
                    secret_vault: Arc::clone(&inner.secret_vault),
                    session_store: Arc::clone(&inner.session_store),
                    tool_registry: &inner.tool_registry,
                    plugin_registry: Arc::clone(&inner.plugin_registry),
                    sandbox_backend: Arc::clone(&inner.sandbox_backend),
                    tracer: Arc::clone(&inner.tracer),
                    cancellation: cancellation.clone(),
                    model_provider: Arc::clone(&inner.model_provider),
                    model_id: session.model.0.clone(),
                    model_version: model_version.clone(),
                    config_snapshot_id: session.config_snapshot_id.clone(),
                    token_budget: session.token_budget,
                },
                &turn.tool_calls,
                &mut transcript,
            )
            .await?;
            continue;
        }

        match turn.stop_reason {
            StopReason::MaxTokens => continue,
            StopReason::EndTurn => {
                if maybe_inject_stop_message(
                    inner.as_ref(),
                    &input.session_id,
                    &mut transcript,
                    last_assistant_event_id,
                )
                .await?
                {
                    continue;
                }

                return Ok(());
            }
            StopReason::StopSequence | StopReason::ToolUse => return Ok(()),
        }
    }

    inner
        .session_store
        .append(
            &input.session_id,
            SessionEvent::SessionEnded {
                reason: octopus_sdk_contracts::EndReason::MaxTurns,
            },
        )
        .await?;
    Ok(())
}

async fn maybe_compact_for_continuation(
    inner: &RuntimeInner,
    session_id: &octopus_sdk_contracts::SessionId,
    transcript: &mut TranscriptState,
    token_budget: u32,
) -> Result<bool, RuntimeError> {
    if transcript.messages.len() <= 1 {
        return Ok(false);
    }

    compact_transcript(
        inner,
        session_id,
        transcript,
        token_budget,
        CONTINUATION_COMPACTION_THRESHOLD,
    )
    .await
}

async fn maybe_inject_stop_message(
    inner: &RuntimeInner,
    session_id: &octopus_sdk_contracts::SessionId,
    transcript: &mut TranscriptState,
    _last_assistant_event_id: Option<octopus_sdk_contracts::EventId>,
) -> Result<bool, RuntimeError> {
    let outcome = inner
        .plugin_registry
        .hooks()
        .run(HookEvent::Stop {
            session: session_id.clone(),
        })
        .await
        .map_err(|error| RuntimeError::Hook(error.to_string()))?;

    if let Some(reason) = outcome.aborted {
        inner
            .session_store
            .append(
                session_id,
                SessionEvent::SessionEnded {
                    reason: octopus_sdk_contracts::EndReason::Error(reason),
                },
            )
            .await?;
        return Ok(false);
    }

    if let Some(octopus_sdk_contracts::RewritePayload::UserPrompt { message }) =
        outcome.final_payload
    {
        let event_id = inner
            .session_store
            .append(session_id, message_event(message.clone()))
            .await?;
        transcript.push(event_id, message);
        return Ok(true);
    }

    Ok(matches!(
        outcome.decisions.last().map(|(_, decision)| decision),
        Some(HookDecision::InjectMessage(_))
    ))
}

fn build_request(
    session: &SessionRuntimeState,
    session_id: &octopus_sdk_contracts::SessionId,
    tools: &octopus_sdk_tools::ToolRegistry,
    prompt_builder: &SystemPromptBuilder,
    transcript: &[Message],
) -> ModelRequest {
    let tool_surface = tools.assemble_surface(&ToolSurfaceState::default());
    let request_tools = tool_surface.request_tools();
    let has_user_message = transcript
        .iter()
        .any(|message| matches!(message.role, octopus_sdk_contracts::Role::User));
    let has_tools = !request_tools.is_empty();
    let cache_control = prompt_cache_control(has_tools, has_user_message);

    ModelRequest {
        model: session.model.clone(),
        system_prompt: prompt_builder.build(&PromptCtx {
            session: session_id.clone(),
            mode: session.permission_mode,
            project_root: session.working_dir.clone(),
            tools: &tool_surface,
        }),
        messages: transcript.to_vec(),
        tools: request_tools,
        role: ModelRole::Main,
        cache_breakpoints: prompt_cache_breakpoints(has_tools, has_user_message),
        response_format: None,
        thinking: None,
        cache_control,
        max_tokens: None,
        temperature: None,
        stream: true,
    }
}

async fn run_sampling_hook(inner: &RuntimeInner, event: HookEvent) -> Result<(), RuntimeError> {
    let outcome = inner
        .plugin_registry
        .hooks()
        .run(event)
        .await
        .map_err(|error| RuntimeError::Hook(error.to_string()))?;

    if let Some(reason) = outcome.aborted {
        return Err(RuntimeError::Hook(reason));
    }

    Ok(())
}

async fn compact_transcript(
    inner: &RuntimeInner,
    session_id: &octopus_sdk_contracts::SessionId,
    transcript: &mut TranscriptState,
    token_budget: u32,
    threshold: f32,
) -> Result<bool, RuntimeError> {
    let estimated_tokens = estimate_tokens(&transcript.messages);
    let usage_ratio = f64::from(estimated_tokens) / f64::from(token_budget.max(1));
    if token_budget == 0 || usage_ratio < f64::from(threshold) {
        return Ok(false);
    }

    let event_ids = std::mem::take(&mut transcript.event_ids);
    let mut view = SessionView {
        messages: &mut transcript.messages,
        tokens: estimated_tokens,
        tokens_budget: token_budget,
        event_ids,
    };
    let compactor = Compactor::new(
        threshold,
        CompactionStrategyTag::Hybrid,
        Arc::clone(&inner.model_provider),
    );
    let result = compactor
        .maybe_compact(&mut view)
        .await
        .map_err(|error| RuntimeError::Hook(error.to_string()))?;

    if let Some(result) = result {
        if !result.summary.trim().is_empty() {
            persist_compaction_checkpoint(
                inner.session_store.as_ref(),
                session_id,
                view.event_ids.as_mut_slice(),
                &result,
            )
            .await?;
        }
        let _ = inner
            .plugin_registry
            .hooks()
            .run(HookEvent::PostCompact {
                session: session_id.clone(),
                result,
            })
            .await;
        transcript.event_ids = view.event_ids;
        return Ok(true);
    }

    transcript.event_ids = view.event_ids;
    Ok(false)
}

fn prompt_cache_breakpoints(has_tools: bool, has_user_message: bool) -> Vec<CacheBreakpoint> {
    let mut breakpoints = vec![CacheBreakpoint {
        position: 0,
        ttl: CacheTtl::OneHour,
    }];
    if has_tools {
        breakpoints.push(CacheBreakpoint {
            position: 1,
            ttl: CacheTtl::FiveMinutes,
        });
    }
    if has_user_message {
        breakpoints.push(CacheBreakpoint {
            position: usize::from(has_tools) + 1,
            ttl: CacheTtl::FiveMinutes,
        });
    }
    breakpoints
}

fn prompt_cache_control(has_tools: bool, has_user_message: bool) -> CacheControlStrategy {
    let mut breakpoints = vec!["system"];
    if has_tools {
        breakpoints.push("tools");
    }
    if has_user_message {
        breakpoints.push("first_user");
    }
    CacheControlStrategy::PromptCaching { breakpoints }
}
