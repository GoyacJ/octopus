use std::sync::Arc;

use octopus_sdk_context::{PromptCtx, SystemPromptBuilder};
use octopus_sdk_contracts::{
    HookDecision, HookEvent, Message, RenderLifecycle, SessionEvent, StopReason,
};
use octopus_sdk_model::{CacheControlStrategy, ModelRequest, ModelRole};
use octopus_sdk_observability::{TraceSpan, TraceValue};

use crate::{
    assistant_projection::{collect_assistant_turn, text_render_block, usage_message},
    runtime::{RuntimeInner, SessionRuntimeState},
    session_boot::{load_transcript, message_event, TranscriptState},
    tool_dispatch::{execute_tool_round, DispatchContext},
    RuntimeError, SubmitTurnInput,
};

const MAX_BRAIN_LOOP_ITERATIONS: usize = 4;

pub(crate) async fn submit_turn(
    inner: Arc<RuntimeInner>,
    session: SessionRuntimeState,
    input: SubmitTurnInput,
    cancellation: tokio_util::sync::CancellationToken,
) -> Result<(), RuntimeError> {
    inner
        .session_store
        .append(&input.session_id, message_event(input.message.clone()))
        .await?;
    let mut transcript = load_transcript(inner.session_store.as_ref(), &input.session_id).await?;

    for iteration in 0..MAX_BRAIN_LOOP_ITERATIONS {
        if cancellation.is_cancelled() {
            return Err(RuntimeError::Cancelled);
        }

        inner.tracer.record(
            TraceSpan::new("brain_loop_iteration")
                .with_field("session_id", TraceValue::String(input.session_id.0.clone()))
                .with_field("iteration", TraceValue::U64(iteration as u64)),
        );

        let request = build_request(
            &session,
            &input.session_id,
            &inner.tool_registry,
            &inner.prompt_builder,
            &transcript.messages,
        );
        let stream = inner.model_provider.complete(request).await?;
        let turn = collect_assistant_turn(stream, inner.tracer.as_ref(), inner.usage_ledger.as_ref())
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
                            block: text_render_block(&turn.rendered_text, Some(event_id)),
                            lifecycle: RenderLifecycle::OnToolResult,
                        },
                    )
                    .await?;
            }
        }

        if turn.stop_reason == StopReason::ToolUse && !turn.tool_calls.is_empty() {
            execute_tool_round(
                DispatchContext {
                    session_id: &input.session_id,
                    working_dir: &session.working_dir,
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
                    token_budget: session.token_budget,
                },
                &turn.tool_calls,
                &mut transcript,
            )
            .await?;
            continue;
        }

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

    if let Some(octopus_sdk_contracts::RewritePayload::UserPrompt { message }) = outcome.final_payload
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
    ModelRequest {
        model: session.model.clone(),
        system_prompt: prompt_builder.build(&PromptCtx {
            session: session_id.clone(),
            mode: session.permission_mode,
            project_root: session.working_dir.clone(),
            tools,
        }),
        messages: transcript.to_vec(),
        tools: tools
            .schemas_sorted()
            .into_iter()
            .map(octopus_sdk_tools::ToolSpec::to_mcp)
            .collect(),
        role: ModelRole::Main,
        cache_breakpoints: Vec::new(),
        response_format: None,
        thinking: None,
        cache_control: CacheControlStrategy::None,
        max_tokens: None,
        temperature: None,
        stream: true,
    }
}
