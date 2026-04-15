use super::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(crate) struct PendingCapabilityExecution {
    pub(crate) capability: Option<tools::CapabilitySpec>,
    pub(crate) outcome: runtime::ToolExecutionOutcome,
    pub(crate) mediation_request: Option<approval_broker::MediationRequest>,
    pub(crate) execution_events: Vec<tools::CapabilityExecutionEvent>,
}

fn capability_dispatch_kind_label(dispatch_kind: tools::CapabilityDispatchKind) -> &'static str {
    match dispatch_kind {
        tools::CapabilityDispatchKind::BuiltinOrPlugin => "builtin_or_plugin",
        tools::CapabilityDispatchKind::RuntimeCapability => "runtime_capability",
    }
}

fn capability_concurrency_policy_label(policy: tools::CapabilityConcurrencyPolicy) -> &'static str {
    match policy {
        tools::CapabilityConcurrencyPolicy::ParallelRead => "parallel_read",
        tools::CapabilityConcurrencyPolicy::Serialized => "serialized",
    }
}

fn dispatch_runtime_capability(
    capability_runtime: &tools::CapabilityRuntime,
    managed_mcp_runtime: Option<&Arc<Mutex<tools::ManagedMcpRuntime>>>,
    visible_capabilities: &[tools::CapabilitySpec],
    tool_name: &str,
    input: Value,
    current_dir: Option<&Path>,
) -> Result<runtime::ToolExecutionOutcome, runtime::ToolError> {
    let capability = visible_capabilities
        .iter()
        .find(|capability| capability.display_name == tool_name)
        .ok_or_else(|| {
            runtime::ToolError::new(format!("runtime capability `{tool_name}` is unavailable"))
        })?;
    match capability.execution_kind {
        tools::CapabilityExecutionKind::Tool => {
            let Some(managed_mcp_runtime) = managed_mcp_runtime else {
                return Err(runtime::ToolError::new(format!(
                    "runtime capability `{tool_name}` is unavailable without a managed runtime"
                )));
            };
            managed_mcp_runtime
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .execute_tool(tool_name, input)
                .map(|output| runtime::ToolExecutionOutcome::Allow { output })
        }
        tools::CapabilityExecutionKind::PromptSkill => {
            let Some(managed_mcp_runtime) = managed_mcp_runtime else {
                return Err(runtime::ToolError::new(format!(
                    "prompt capability `{tool_name}` is unavailable without a managed runtime"
                )));
            };
            managed_mcp_runtime
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .execute_prompt_skill(capability, Some(input))
                .map_err(runtime::ToolError::new)
                .and_then(|output| {
                    serde_json::to_string_pretty(&output)
                        .map(|output| runtime::ToolExecutionOutcome::Allow { output })
                        .map_err(|error| runtime::ToolError::new(error.to_string()))
                })
        }
        tools::CapabilityExecutionKind::Resource => capability_runtime
            .read_resource(capability, input, current_dir)
            .map(|output| runtime::ToolExecutionOutcome::Allow { output }),
    }
}

fn runtime_capability_mediation_decision(
    request: &tools::CapabilityExecutionRequest,
) -> tools::CapabilityMediationDecision {
    if request.requires_auth {
        tools::CapabilityMediationDecision::RequireAuth(Some(format!(
            "tool `{}` requires auth before execution can continue",
            request.tool_name
        )))
    } else if request.requires_approval {
        tools::CapabilityMediationDecision::RequireApproval(Some(format!(
            "tool `{}` requires approval before execution can continue",
            request.tool_name
        )))
    } else {
        tools::CapabilityMediationDecision::Allow
    }
}

fn capability_call_mediation_request(
    session_id: &str,
    conversation_id: &str,
    run_id: &str,
    tool_use: &agent_runtime_core::RuntimePendingToolUse,
    request: &tools::CapabilityExecutionRequest,
    visible_capabilities: &[tools::CapabilitySpec],
    outcome: &runtime::ToolExecutionOutcome,
    created_at: u64,
) -> approval_broker::MediationRequest {
    let capability = visible_capabilities
        .iter()
        .find(|capability| capability.display_name == request.tool_name);
    let target_ref = format!("capability-call:{run_id}:{}", tool_use.tool_use_id);
    let detail = match outcome {
        runtime::ToolExecutionOutcome::RequireApproval { reason }
        | runtime::ToolExecutionOutcome::RequireAuth { reason } => {
            reason.clone().unwrap_or_else(|| {
                if request.requires_auth {
                    format!(
                        "Resolve auth for `{}` before this capability call can continue.",
                        request.tool_name
                    )
                } else {
                    format!(
                        "Approve `{}` before this capability call can continue.",
                        request.tool_name
                    )
                }
            })
        }
        _ => format!("tool `{}` is blocked", request.tool_name),
    };
    approval_broker::MediationRequest {
        session_id: session_id.to_string(),
        conversation_id: conversation_id.to_string(),
        run_id: run_id.to_string(),
        tool_name: request.tool_name.clone(),
        summary: if request.requires_auth {
            format!("{} requires auth", request.tool_name)
        } else {
            format!("{} requires approval", request.tool_name)
        },
        detail: detail.clone(),
        mediation_kind: if request.requires_auth {
            "auth".into()
        } else {
            "approval".into()
        },
        approval_layer: "capability-call".into(),
        target_kind: "capability-call".into(),
        target_ref,
        capability_id: Some(request.capability_id.clone()),
        dispatch_kind: capability_dispatch_kind_label(request.dispatch_kind).into(),
        provider_key: capability.and_then(|capability| capability.provider_key.clone()),
        concurrency_policy: capability_concurrency_policy_label(request.concurrency_policy).into(),
        input: tool_use.input.clone(),
        required_permission: Some(request.required_permission.as_str().to_string()),
        escalation_reason: Some(detail),
        requires_approval: request.requires_approval,
        requires_auth: request.requires_auth,
        created_at,
        risk_level: if request.requires_auth {
            "medium".into()
        } else {
            "high".into()
        },
        checkpoint_ref: None,
    }
}

pub(crate) fn execute_pending_tool_use(
    adapter: &RuntimeAdapter,
    capability_runtime: &tools::CapabilityRuntime,
    managed_mcp_runtime: Option<&Arc<Mutex<tools::ManagedMcpRuntime>>>,
    visible_capabilities: &[tools::CapabilitySpec],
    planned_tool_names: &BTreeSet<String>,
    capability_store: &tools::SessionCapabilityStore,
    session_id: &str,
    conversation_id: &str,
    run_id: &str,
    tool_use: &agent_runtime_core::RuntimePendingToolUse,
    pending_mcp_servers: Option<Vec<String>>,
    mcp_degraded: Option<runtime::McpDegradedReport>,
) -> Result<PendingCapabilityExecution, AppError> {
    let mediation_capture = Arc::new(Mutex::new(None));
    let execution_capture = Arc::new(Mutex::new(Vec::new()));
    let capability = visible_capabilities
        .iter()
        .find(|capability| capability.display_name == tool_use.tool_name)
        .cloned();
    capability_runtime.set_execution_hook({
        let execution_capture = Arc::clone(&execution_capture);
        move |event| {
            execution_capture
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(event);
        }
    });
    capability_runtime.set_mediation_hook({
        let mediation_capture = Arc::clone(&mediation_capture);
        move |request| {
            let decision = runtime_capability_mediation_decision(request);
            let mut slot = mediation_capture
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *slot = Some((request.clone(), decision.clone()));
            decision
        }
    });
    let session_state = capability_store.snapshot();
    let outcome = capability_runtime.execute_tool_with_outcome(
        &tool_use.tool_name,
        tool_use.input.clone(),
        tools::CapabilityPlannerInput::new(Some(planned_tool_names), Some(&session_state))
            .with_current_dir(Some(adapter.state.paths.root.as_path())),
        capability_store,
        pending_mcp_servers,
        mcp_degraded,
        {
            let capability_runtime = capability_runtime.clone();
            let managed_mcp_runtime = managed_mcp_runtime.cloned();
            let visible_capabilities = visible_capabilities.to_vec();
            move |_dispatch_kind, tool_name, input| {
                dispatch_runtime_capability(
                    &capability_runtime,
                    managed_mcp_runtime.as_ref(),
                    &visible_capabilities,
                    tool_name,
                    input,
                    Some(adapter.state.paths.root.as_path()),
                )
            }
        },
    );
    capability_runtime.clear_execution_hook();
    capability_runtime.clear_mediation_hook();

    let execution_events = execution_capture
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let mediation_request = match &outcome {
        runtime::ToolExecutionOutcome::RequireApproval { .. }
        | runtime::ToolExecutionOutcome::RequireAuth { .. } => {
            let (request, _decision) = mediation_capture
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
                .ok_or_else(|| {
                    AppError::runtime(
                        "runtime capability mediation did not capture a blocked request",
                    )
                })?;
            Some(capability_call_mediation_request(
                session_id,
                conversation_id,
                run_id,
                tool_use,
                &request,
                visible_capabilities,
                &outcome,
                timestamp_now(),
            ))
        }
        _ => None,
    };

    Ok(PendingCapabilityExecution {
        capability,
        outcome,
        mediation_request,
        execution_events,
    })
}
