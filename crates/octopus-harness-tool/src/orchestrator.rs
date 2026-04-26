use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use futures::{future::join_all, StreamExt};
use harness_contracts::{
    Decision, DecisionScope, PermissionSubject, RequestId, Severity, ToolError, ToolResult,
    ToolUseId,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolResultEnvelope {
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    pub result: Result<ToolResult, ToolError>,
    pub duration: Duration,
    pub progress_emitted: u32,
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
                collect_stream(stream, &mut progress_emitted).await
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

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress(_) => {
                *progress_emitted += 1;
            }
            ToolEvent::Partial(_) => {}
            ToolEvent::Final(result) => {
                final_result = Some(result);
                break;
            }
            ToolEvent::Error(error) => return Err(error),
        }
    }

    final_result
        .ok_or_else(|| ToolError::Internal("tool stream ended without final result".to_owned()))
}
