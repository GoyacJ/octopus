use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use chrono::Utc;
use futures::{future::join_all, StreamExt};
use harness_contracts::{
    BlobMeta, BlobRetention, BlobStore, BudgetMetric, Decision, DecisionScope, Event,
    OverflowAction, OverflowMetadata, PermissionSubject, RequestId, Severity, ToolCapability,
    ToolError, ToolResult, ToolResultOffloadedEvent, ToolResultPart, ToolUseId,
};
use harness_permission::{PermissionCheck, PermissionContext, PermissionRequest};
use serde_json::Value;
use tokio::sync::Semaphore;

use crate::{ToolContext, ToolEvent, ToolPool};

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    pub input: Value,
}

#[derive(Clone)]
pub struct OrchestratorContext {
    pub pool: ToolPool,
    pub tool_context: ToolContext,
    pub permission_context: PermissionContext,
    pub blob_store: Option<Arc<dyn BlobStore>>,
    pub event_emitter: Arc<dyn ToolEventEmitter>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolResultEnvelope {
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    pub result: Result<ToolResult, ToolError>,
    pub overflow: Option<OverflowMetadata>,
    pub duration: Duration,
    pub progress_emitted: u32,
}

pub trait ToolEventEmitter: Send + Sync + 'static {
    fn emit(&self, event: Event);
}

#[derive(Debug, Default)]
pub struct NoopToolEventEmitter;

impl ToolEventEmitter for NoopToolEventEmitter {
    fn emit(&self, _event: Event) {}
}

#[derive(Clone)]
pub struct ToolOrchestrator {
    concurrency_limit: usize,
}

impl Default for ToolOrchestrator {
    fn default() -> Self {
        Self::new(10)
    }
}

impl ToolOrchestrator {
    pub fn new(concurrency_limit: usize) -> Self {
        Self {
            concurrency_limit: concurrency_limit.max(1),
        }
    }

    pub async fn dispatch(
        &self,
        calls: Vec<ToolCall>,
        ctx: OrchestratorContext,
    ) -> Vec<ToolResultEnvelope> {
        let mut results = Vec::with_capacity(calls.len());
        let mut index = 0;

        while index < calls.len() {
            if self.is_concurrency_safe(&ctx.pool, &calls[index]) {
                let start = index;
                while index < calls.len() && self.is_concurrency_safe(&ctx.pool, &calls[index]) {
                    index += 1;
                }
                results.extend(
                    self.dispatch_parallel(calls[start..index].to_vec(), ctx.clone())
                        .await,
                );
            } else {
                results.push(Self::dispatch_one(calls[index].clone(), ctx.clone()).await);
                index += 1;
            }
        }

        results
    }

    fn is_concurrency_safe(&self, pool: &ToolPool, call: &ToolCall) -> bool {
        pool.get(&call.tool_name)
            .map(|tool| tool.descriptor().properties.is_concurrency_safe)
            .unwrap_or(true)
    }

    async fn dispatch_parallel(
        &self,
        calls: Vec<ToolCall>,
        ctx: OrchestratorContext,
    ) -> Vec<ToolResultEnvelope> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        join_all(calls.into_iter().map(|call| {
            let semaphore = Arc::clone(&semaphore);
            let ctx = ctx.clone();
            async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .expect("tool dispatch semaphore closed");
                Self::dispatch_one(call, ctx).await
            }
        }))
        .await
    }

    async fn dispatch_one(call: ToolCall, ctx: OrchestratorContext) -> ToolResultEnvelope {
        let started = Instant::now();
        let tool_use_id = call.tool_use_id;
        let tool_name = call.tool_name.clone();
        let mut progress_emitted = 0;

        let mut overflow = None;
        let result = async {
            if ctx.tool_context.interrupt.is_interrupted() {
                return Err(ToolError::Interrupted);
            }

            let tool = ctx.pool.get(&call.tool_name).ok_or_else(|| {
                ToolError::Internal(format!("tool not found: {}", call.tool_name))
            })?;

            let mut tool_ctx = ctx.tool_context.clone();
            tool_ctx.tool_use_id = call.tool_use_id;

            tool.validate(&call.input, &tool_ctx)
                .await
                .map_err(|error| ToolError::Validation(error.to_string()))?;

            let permission_check = tool.check_permission(&call.input, &tool_ctx).await;
            let decision = match permission_check {
                PermissionCheck::Denied { reason } => {
                    return Err(ToolError::PermissionDenied(reason));
                }
                check => {
                    let request = permission_request(&call, &ctx, check);
                    tool_ctx
                        .permission_broker
                        .decide(request, ctx.permission_context.clone())
                        .await
                }
            };

            if !decision_allows(&decision) {
                return Err(ToolError::PermissionDenied(format!(
                    "permission denied: {decision:?}"
                )));
            }

            let execute_and_collect = async {
                let stream = tool.execute(call.input, tool_ctx.clone()).await?;
                let result = collect_stream(stream, &mut progress_emitted).await?;
                apply_result_budget(
                    result,
                    &tool.descriptor().budget,
                    &ctx,
                    call.tool_use_id,
                    &mut overflow,
                )
                .await
            };

            match tool.descriptor().properties.long_running.as_ref() {
                Some(policy) => {
                    match tokio::time::timeout(policy.hard_timeout, execute_and_collect).await {
                        Ok(result) => result,
                        Err(_elapsed) => {
                            tool_ctx.interrupt.interrupt();
                            Err(ToolError::Timeout)
                        }
                    }
                }
                None => execute_and_collect.await,
            }
        }
        .await;

        ToolResultEnvelope {
            tool_use_id,
            tool_name,
            result,
            overflow,
            duration: started.elapsed(),
            progress_emitted,
        }
    }
}

fn permission_request(
    call: &ToolCall,
    ctx: &OrchestratorContext,
    check: PermissionCheck,
) -> PermissionRequest {
    let (subject, severity, scope_hint) = match check {
        PermissionCheck::Allowed => (
            PermissionSubject::ToolInvocation {
                tool: call.tool_name.clone(),
                input: call.input.clone(),
            },
            Severity::Info,
            DecisionScope::ToolName(call.tool_name.clone()),
        ),
        PermissionCheck::AskUser { subject, scope } => (subject, Severity::Medium, scope),
        PermissionCheck::DangerousCommand { pattern, severity } => (
            PermissionSubject::DangerousCommand {
                command: call.tool_name.clone(),
                pattern_id: pattern,
                severity,
            },
            severity,
            DecisionScope::ToolName(call.tool_name.clone()),
        ),
        PermissionCheck::Denied { reason } => (
            PermissionSubject::Custom {
                kind: "denied".to_owned(),
                payload: Value::String(reason),
            },
            Severity::High,
            DecisionScope::ToolName(call.tool_name.clone()),
        ),
    };

    PermissionRequest {
        request_id: RequestId::new(),
        tenant_id: ctx.tool_context.tenant_id,
        session_id: ctx.tool_context.session_id,
        tool_use_id: call.tool_use_id,
        tool_name: call.tool_name.clone(),
        subject,
        severity,
        scope_hint,
        created_at: Utc::now(),
    }
}

fn decision_allows(decision: &Decision) -> bool {
    matches!(
        decision,
        Decision::AllowOnce | Decision::AllowSession | Decision::AllowPermanent
    )
}

async fn collect_stream(
    mut stream: crate::ToolStream,
    progress_emitted: &mut u32,
) -> Result<ToolResult, ToolError> {
    let mut final_result = None;
    let mut text_partials = String::new();

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress(_) => {
                *progress_emitted += 1;
            }
            ToolEvent::Partial(part) => {
                if let harness_contracts::MessagePart::Text(text) = part {
                    text_partials.push_str(&text);
                }
            }
            ToolEvent::Final(result) => {
                final_result = Some(result);
                break;
            }
            ToolEvent::Error(error) => return Err(error),
        }
    }

    let result = final_result
        .ok_or_else(|| ToolError::Internal("tool stream ended without final result".to_owned()))?;
    Ok(if text_partials.is_empty() {
        result
    } else {
        match result {
            ToolResult::Text(text) => ToolResult::Text(format!("{text_partials}{text}")),
            ToolResult::Mixed(parts) => {
                let mut combined = Vec::with_capacity(parts.len() + 1);
                combined.push(ToolResultPart::Text {
                    text: text_partials,
                });
                combined.extend(parts);
                ToolResult::Mixed(combined)
            }
            other => {
                let mut parts = vec![ToolResultPart::Text {
                    text: text_partials,
                }];
                parts.extend(tool_result_to_parts(other));
                ToolResult::Mixed(parts)
            }
        }
    })
}

async fn apply_result_budget(
    result: ToolResult,
    budget: &harness_contracts::ResultBudget,
    ctx: &OrchestratorContext,
    tool_use_id: ToolUseId,
    overflow: &mut Option<OverflowMetadata>,
) -> Result<ToolResult, ToolError> {
    let Some(text) = budgeted_text(&result) else {
        return Ok(result);
    };
    let original_size = measure(&text, budget.metric);
    if original_size <= budget.limit {
        return Ok(result);
    }

    match budget.on_overflow {
        OverflowAction::Truncate => Ok(ToolResult::Text(truncate_by_metric(
            &text,
            budget.metric,
            budget.limit,
        ))),
        OverflowAction::Offload => {
            let blob_store = ctx
                .blob_store
                .as_ref()
                .ok_or(ToolError::CapabilityMissing(ToolCapability::BlobReader))?;
            let bytes = Bytes::from(text.clone());
            let content_hash = *blake3::hash(&bytes).as_bytes();
            let meta = BlobMeta {
                content_type: Some("text/plain; charset=utf-8".to_owned()),
                size: bytes.len() as u64,
                content_hash,
                created_at: Utc::now(),
                retention: BlobRetention::SessionScoped(ctx.tool_context.session_id),
            };
            let blob_ref = blob_store
                .put(ctx.tool_context.tenant_id, bytes, meta)
                .await
                .map_err(|error| ToolError::OffloadFailed(error.to_string()))?;
            let head = take_chars(&text, budget.preview_head_chars as usize);
            let tail = take_tail_chars(&text, budget.preview_tail_chars as usize);
            let metadata = OverflowMetadata {
                blob_ref: blob_ref.clone(),
                head_chars: head.chars().count() as u32,
                tail_chars: tail.chars().count() as u32,
                original_size,
                original_metric: budget.metric,
                effective_limit: budget.limit,
            };
            ctx.event_emitter
                .emit(Event::ToolResultOffloaded(ToolResultOffloadedEvent {
                    tool_use_id,
                    run_id: ctx.tool_context.run_id,
                    blob_ref: blob_ref.clone(),
                    original_metric: budget.metric,
                    original_size,
                    effective_limit: budget.limit,
                    head_chars: metadata.head_chars,
                    tail_chars: metadata.tail_chars,
                    at: Utc::now(),
                }));
            *overflow = Some(metadata);
            Ok(ToolResult::Mixed(vec![
                ToolResultPart::Text { text: head },
                ToolResultPart::Blob {
                    content_type: "text/plain; charset=utf-8".to_owned(),
                    blob_ref,
                    summary: Some(
                        "tool result exceeded budget; full content was offloaded".to_owned(),
                    ),
                },
                ToolResultPart::Text { text: tail },
            ]))
        }
        _ => Err(ToolError::ResultTooLarge {
            original: original_size,
            limit: budget.limit,
            metric: budget.metric,
        }),
    }
}

fn budgeted_text(result: &ToolResult) -> Option<String> {
    match result {
        ToolResult::Text(text) => Some(text.clone()),
        ToolResult::Structured(value) => serde_json::to_string(value).ok(),
        ToolResult::Mixed(parts) => {
            let mut text = String::new();
            for part in parts {
                match part {
                    ToolResultPart::Text { text: part_text } => text.push_str(part_text),
                    ToolResultPart::Structured { value, .. } => {
                        text.push_str(&serde_json::to_string(value).ok()?);
                    }
                    ToolResultPart::Code { text: code, .. } => text.push_str(code),
                    _ => {}
                }
            }
            Some(text)
        }
        _ => None,
    }
}

fn measure(text: &str, metric: BudgetMetric) -> u64 {
    match metric {
        BudgetMetric::Bytes => text.len() as u64,
        BudgetMetric::Lines => text.lines().count() as u64,
        _ => text.chars().count() as u64,
    }
}

fn take_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

fn take_tail_chars(text: &str, count: usize) -> String {
    let mut chars = text.chars().rev().take(count).collect::<Vec<_>>();
    chars.reverse();
    chars.into_iter().collect()
}

fn truncate_by_metric(text: &str, metric: BudgetMetric, limit: u64) -> String {
    match metric {
        BudgetMetric::Bytes => take_bytes(text, limit as usize),
        BudgetMetric::Lines => text
            .lines()
            .take(limit as usize)
            .collect::<Vec<_>>()
            .join("\n"),
        _ => take_chars(text, limit as usize),
    }
}

fn take_bytes(text: &str, count: usize) -> String {
    if text.len() <= count {
        return text.to_owned();
    }
    let mut end = count;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn tool_result_to_parts(result: ToolResult) -> Vec<ToolResultPart> {
    match result {
        ToolResult::Text(text) => vec![ToolResultPart::Text { text }],
        ToolResult::Structured(value) => vec![ToolResultPart::Structured {
            value,
            schema_ref: None,
        }],
        ToolResult::Blob {
            content_type,
            blob_ref,
        } => vec![ToolResultPart::Blob {
            content_type,
            blob_ref,
            summary: None,
        }],
        ToolResult::Mixed(parts) => parts,
        _ => vec![ToolResultPart::Text {
            text: String::new(),
        }],
    }
}
