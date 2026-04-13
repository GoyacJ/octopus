use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use runtime::{McpDegradedReport, ToolError, ToolExecutionOutcome};
use serde_json::Value;

use crate::tool_registry::ToolSearchOutput;

pub use super::events::{
    CapabilityExecutionEvent, CapabilityExecutionPhase, CapabilityExecutionRequest,
    CapabilityMediationDecision,
};
use super::planner::CapabilityPlannerInput;
use super::provider::{
    CapabilityConcurrencyPolicy, CapabilityExecutionKind, CapabilitySourceKind, CapabilitySpec,
};
pub use super::provider::CapabilityRuntime;
use super::state::{CapabilityActivation, SessionCapabilityStore};

type CapabilityExecutionHook = Arc<dyn Fn(CapabilityExecutionEvent) + Send + Sync>;
type CapabilityMediationHook =
    Arc<dyn Fn(&CapabilityExecutionRequest) -> CapabilityMediationDecision + Send + Sync>;
type PromptSkillExecutor = Arc<
    dyn Fn(
            &CapabilitySpec,
            Option<Value>,
            Option<&Path>,
        ) -> Result<crate::SkillExecutionResult, String>
        + Send
        + Sync,
>;
type ResourceExecutor =
    Arc<dyn Fn(&CapabilitySpec, Value, Option<&Path>) -> Result<String, String> + Send + Sync>;

#[derive(Default)]
struct CapabilityExecutorInner {
    serialized_gate: Mutex<()>,
    execution_hook: Mutex<Option<CapabilityExecutionHook>>,
    mediation_hook: Mutex<Option<CapabilityMediationHook>>,
    prompt_skill_executors: Mutex<BTreeMap<String, PromptSkillExecutor>>,
    resource_executors: Mutex<BTreeMap<String, ResourceExecutor>>,
}

#[derive(Clone, Default)]
pub struct CapabilityExecutor {
    inner: Arc<CapabilityExecutorInner>,
}

impl std::fmt::Debug for CapabilityExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapabilityExecutor").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityDispatchKind {
    BuiltinOrPlugin,
    RuntimeCapability,
}

impl CapabilityExecutor {
    pub fn register_prompt_skill_executor<F>(&self, key: impl Into<String>, executor: F)
    where
        F: Fn(
                &CapabilitySpec,
                Option<Value>,
                Option<&Path>,
            ) -> Result<crate::SkillExecutionResult, String>
            + Send
            + Sync
            + 'static,
    {
        self.inner
            .prompt_skill_executors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key.into(), Arc::new(executor));
    }

    pub fn register_resource_executor<F>(&self, key: impl Into<String>, executor: F)
    where
        F: Fn(&CapabilitySpec, Value, Option<&Path>) -> Result<String, String>
            + Send
            + Sync
            + 'static,
    {
        self.inner
            .resource_executors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key.into(), Arc::new(executor));
    }

    #[must_use]
    pub fn has_prompt_skill_executor(&self, capability: &CapabilitySpec) -> bool {
        if !matches!(
            capability.execution_kind,
            CapabilityExecutionKind::PromptSkill
        ) {
            return false;
        }

        if matches!(
            capability.source_kind,
            CapabilitySourceKind::LocalSkill | CapabilitySourceKind::BundledSkill
        ) {
            return true;
        }

        capability
            .executor_key
            .as_ref()
            .map(|key| {
                self.inner
                    .prompt_skill_executors
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .contains_key(key)
            })
            .unwrap_or(false)
    }

    #[must_use]
    pub fn has_resource_executor(&self, capability: &CapabilitySpec) -> bool {
        if capability.execution_kind != CapabilityExecutionKind::Resource {
            return false;
        }

        capability
            .executor_key
            .as_ref()
            .map(|key| {
                self.inner
                    .resource_executors
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .contains_key(key)
            })
            .unwrap_or(false)
    }

    pub(crate) fn execute_prompt_skill(
        &self,
        capability: &CapabilitySpec,
        arguments: Option<Value>,
        current_dir: Option<&Path>,
    ) -> Result<crate::SkillExecutionResult, crate::skill_runtime::SkillExecutionFailure> {
        let Some(executor_key) = capability.executor_key.as_ref() else {
            return Err(crate::skill_runtime::SkillExecutionFailure {
                message: format!(
                    "skill `{}` does not have a runtime executor yet",
                    capability.display_name
                ),
                state_updates: Vec::new(),
            });
        };

        let executor = self
            .inner
            .prompt_skill_executors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(executor_key)
            .cloned()
            .ok_or_else(|| crate::skill_runtime::SkillExecutionFailure {
                message: format!(
                    "skill `{}` does not have a runtime executor yet",
                    capability.display_name
                ),
                state_updates: Vec::new(),
            })?;

        executor(capability, arguments, current_dir).map_err(|message| {
            crate::skill_runtime::SkillExecutionFailure {
                message,
                state_updates: Vec::new(),
            }
        })
    }

    pub fn read_resource(
        &self,
        capability: &CapabilitySpec,
        input: Value,
        current_dir: Option<&Path>,
    ) -> Result<String, ToolError> {
        let Some(executor_key) = capability.executor_key.as_ref() else {
            return Err(ToolError::new(format!(
                "resource `{}` does not have a runtime executor",
                capability.display_name
            )));
        };

        let executor = self
            .inner
            .resource_executors
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(executor_key)
            .cloned()
            .ok_or_else(|| {
                ToolError::new(format!(
                    "resource `{}` does not have a runtime executor",
                    capability.display_name
                ))
            })?;

        executor(capability, input, current_dir).map_err(ToolError::new)
    }

    pub fn set_execution_hook<F>(&self, hook: F)
    where
        F: Fn(CapabilityExecutionEvent) + Send + Sync + 'static,
    {
        let mut slot = self
            .inner
            .execution_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *slot = Some(Arc::new(hook));
    }

    pub fn clear_execution_hook(&self) {
        let mut slot = self
            .inner
            .execution_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *slot = None;
    }

    pub fn set_mediation_hook<F>(&self, hook: F)
    where
        F: Fn(&CapabilityExecutionRequest) -> CapabilityMediationDecision + Send + Sync + 'static,
    {
        let mut slot = self
            .inner
            .mediation_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *slot = Some(Arc::new(hook));
    }

    pub fn clear_mediation_hook(&self) {
        let mut slot = self
            .inner
            .mediation_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *slot = None;
    }

    pub fn activate_search_selection(
        &self,
        query: &str,
        output: &ToolSearchOutput,
        store: &SessionCapabilityStore,
    ) {
        if !query.trim().to_ascii_lowercase().starts_with("select:") {
            return;
        }
        store.mutate(|state| {
            for tool_name in output.matches() {
                state.activate(CapabilityActivation::tool(tool_name.clone()));
            }
        });
    }

    pub fn apply_skill_execution_result(
        &self,
        result: &crate::SkillExecutionResult,
        store: &SessionCapabilityStore,
    ) {
        store.apply_skill_execution_result(result);
    }

    pub fn apply_skill_state_updates(
        &self,
        updates: &[crate::SkillStateUpdate],
        store: &SessionCapabilityStore,
    ) {
        store.apply_skill_state_updates(updates);
    }

    fn execution_hook(&self) -> Option<CapabilityExecutionHook> {
        self.inner
            .execution_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn mediation_hook(&self) -> Option<CapabilityMediationHook> {
        self.inner
            .mediation_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn emit_event(&self, event: CapabilityExecutionEvent) {
        if let Some(hook) = self.execution_hook() {
            hook(event);
        }
    }

    fn mediation_decision(
        &self,
        request: &CapabilityExecutionRequest,
        store: &SessionCapabilityStore,
    ) -> CapabilityMediationDecision {
        let (approval_resolved, auth_resolved) = store.with_state(|state| {
            (
                state.is_tool_approved(&request.tool_name),
                state.is_tool_auth_resolved(&request.tool_name),
            )
        });
        let needs_mediation = (request.requires_approval && !approval_resolved)
            || (request.requires_auth && !auth_resolved);
        if !needs_mediation {
            return CapabilityMediationDecision::Allow;
        }

        let Some(hook) = self.mediation_hook() else {
            return CapabilityMediationDecision::Allow;
        };
        hook(request)
    }

    fn handle_mediation_decision(
        &self,
        request: &CapabilityExecutionRequest,
        decision: CapabilityMediationDecision,
        store: &SessionCapabilityStore,
    ) -> Option<ToolExecutionOutcome> {
        match decision {
            CapabilityMediationDecision::Allow => {
                store.clear_tool_pending(&request.tool_name);
                if request.requires_approval {
                    store.approve_tool(request.tool_name.clone());
                }
                if request.requires_auth {
                    store.resolve_tool_auth(request.tool_name.clone());
                }
                None
            }
            CapabilityMediationDecision::RequireApproval(reason) => {
                store.mark_tool_pending(request.tool_name.clone());
                self.emit_event(CapabilityExecutionEvent::from_request(
                    request,
                    CapabilityExecutionPhase::BlockedApproval,
                    reason.clone(),
                ));
                Some(ToolExecutionOutcome::RequireApproval { reason })
            }
            CapabilityMediationDecision::RequireAuth(reason) => {
                store.mark_tool_pending(request.tool_name.clone());
                self.emit_event(CapabilityExecutionEvent::from_request(
                    request,
                    CapabilityExecutionPhase::BlockedAuth,
                    reason.clone(),
                ));
                Some(ToolExecutionOutcome::RequireAuth { reason })
            }
            CapabilityMediationDecision::Deny(reason) => {
                self.emit_event(CapabilityExecutionEvent::from_request(
                    request,
                    CapabilityExecutionPhase::Denied,
                    Some(reason.clone()),
                ));
                Some(ToolExecutionOutcome::Deny { reason })
            }
        }
    }

    fn classify_dispatch_error(tool_name: &str, error: ToolError) -> ToolExecutionOutcome {
        let message = error.to_string();
        let normalized = message.to_ascii_lowercase();
        if normalized.contains("cancelled") {
            return ToolExecutionOutcome::Cancelled {
                reason: Some(message),
            };
        }
        if normalized.contains("interrupted")
            || normalized.contains("broken pipe")
            || normalized.contains("connection")
            || normalized.contains("transport")
        {
            return ToolExecutionOutcome::Interrupted {
                reason: Some(message),
            };
        }
        if normalized.contains("degraded")
            || (normalized.contains("mcp") && normalized.contains("failed"))
            || (normalized.contains("provider") && normalized.contains("unavailable"))
        {
            return ToolExecutionOutcome::Degraded {
                reason: Some(message),
            };
        }
        ToolExecutionOutcome::Failed {
            message: format!("{tool_name}: {message}"),
        }
    }

    fn emit_terminal_event(
        &self,
        request: &CapabilityExecutionRequest,
        outcome: &ToolExecutionOutcome,
    ) {
        let (phase, detail) = match outcome {
            ToolExecutionOutcome::Allow { .. } => (CapabilityExecutionPhase::Completed, None),
            ToolExecutionOutcome::RequireApproval { reason } => {
                (CapabilityExecutionPhase::BlockedApproval, reason.clone())
            }
            ToolExecutionOutcome::RequireAuth { reason } => {
                (CapabilityExecutionPhase::BlockedAuth, reason.clone())
            }
            ToolExecutionOutcome::Deny { reason } => {
                (CapabilityExecutionPhase::Denied, Some(reason.clone()))
            }
            ToolExecutionOutcome::Cancelled { reason } => {
                (CapabilityExecutionPhase::Cancelled, reason.clone())
            }
            ToolExecutionOutcome::Interrupted { reason } => {
                (CapabilityExecutionPhase::Interrupted, reason.clone())
            }
            ToolExecutionOutcome::Degraded { reason } => {
                (CapabilityExecutionPhase::Degraded, reason.clone())
            }
            ToolExecutionOutcome::Failed { message } => {
                (CapabilityExecutionPhase::Failed, Some(message.clone()))
            }
        };
        self.emit_event(CapabilityExecutionEvent::from_request(request, phase, detail));
    }

    fn with_concurrency_gate<T>(
        &self,
        policy: CapabilityConcurrencyPolicy,
        f: impl FnOnce() -> Result<T, ToolError>,
    ) -> Result<T, ToolError> {
        match policy {
            CapabilityConcurrencyPolicy::ParallelRead => f(),
            CapabilityConcurrencyPolicy::Serialized => {
                let _guard = self
                    .inner
                    .serialized_gate
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                f()
            }
        }
    }

    pub fn execute_tool_with_outcome<F>(
        &self,
        runtime: &CapabilityRuntime,
        tool_name: &str,
        input: Value,
        planner_input: CapabilityPlannerInput<'_>,
        store: &SessionCapabilityStore,
        pending_mcp_servers: Option<Vec<String>>,
        mcp_degraded: Option<McpDegradedReport>,
        mut dispatch: F,
    ) -> ToolExecutionOutcome
    where
        F: FnMut(CapabilityDispatchKind, &str, Value) -> Result<ToolExecutionOutcome, ToolError>,
    {
        let surface = runtime
            .surface_projection(planner_input)
            .map_err(ToolError::new);
        let surface = match surface {
            Ok(surface) => surface,
            Err(error) => return Self::classify_dispatch_error(tool_name, error),
        };
        let capability = surface
            .visible_tools
            .into_iter()
            .find(|capability| capability.display_name == tool_name)
            .ok_or_else(|| ToolError::new(format!(
                    "tool `{tool_name}` is not enabled in the current capability surface"
                )));
        let capability = match capability {
            Ok(capability) => capability,
            Err(error) => return Self::classify_dispatch_error(tool_name, error),
        };
        let request = CapabilityExecutionRequest {
            tool_name: tool_name.to_string(),
            capability_id: capability.capability_id.clone(),
            dispatch_kind: dispatch_kind_for_capability(&capability),
            required_permission: capability.permission_profile.required_permission,
            concurrency_policy: capability.concurrency_policy,
            requires_auth: capability.invocation_policy.requires_auth,
            requires_approval: capability.invocation_policy.requires_approval,
            input: input.clone(),
        };
        if let Some(outcome) =
            self.handle_mediation_decision(&request, self.mediation_decision(&request, store), store)
        {
            return outcome;
        }
        self.emit_event(CapabilityExecutionEvent::from_request(
            &request,
            CapabilityExecutionPhase::Started,
            None,
        ));

        let result = self.with_concurrency_gate(request.concurrency_policy, || match tool_name {
            "ToolSearch" => {
                let input: crate::ToolSearchInput = serde_json::from_value(input)
                    .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
                let output = runtime.search(
                    &input.query,
                    input.max_results.unwrap_or(5),
                    planner_input,
                    pending_mcp_servers,
                    mcp_degraded,
                );
                self.activate_search_selection(&input.query, &output, store);
                serde_json::to_string_pretty(&output)
                    .map(|output| ToolExecutionOutcome::Allow { output })
                    .map_err(|error| ToolError::new(error.to_string()))
            }
            "SkillDiscovery" => {
                let input: crate::SkillDiscoveryInput = serde_json::from_value(input)
                    .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
                let output = runtime.skill_discovery(
                    &input.query,
                    input.max_results.unwrap_or(5),
                    planner_input,
                );
                serde_json::to_string_pretty(&output)
                    .map(|output| ToolExecutionOutcome::Allow { output })
                    .map_err(|error| ToolError::new(error.to_string()))
            }
            "SkillTool" => {
                let input: crate::SkillToolInput = serde_json::from_value(input)
                    .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
                match runtime.execute_skill_detailed(&input.skill, input.arguments, planner_input) {
                    Ok(output) => {
                        self.apply_skill_execution_result(&output, store);
                        serde_json::to_string_pretty(&output)
                            .map(|output| ToolExecutionOutcome::Allow { output })
                            .map_err(|error| ToolError::new(error.to_string()))
                    }
                    Err(failure) => {
                        self.apply_skill_state_updates(&failure.state_updates, store);
                        Err(ToolError::new(failure.message))
                    }
                }
            }
            _ => match request.dispatch_kind {
                CapabilityDispatchKind::BuiltinOrPlugin => {
                    runtime
                        .execute_local_tool(tool_name, &input)
                        .map(|output| ToolExecutionOutcome::Allow { output })
                }
                CapabilityDispatchKind::RuntimeCapability =>
                    dispatch(CapabilityDispatchKind::RuntimeCapability, tool_name, input),
            },
        });
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(error) => Self::classify_dispatch_error(tool_name, error),
        };
        self.emit_terminal_event(&request, &outcome);
        outcome
    }

    pub fn execute_tool<F>(
        &self,
        runtime: &CapabilityRuntime,
        tool_name: &str,
        input: Value,
        planner_input: CapabilityPlannerInput<'_>,
        store: &SessionCapabilityStore,
        pending_mcp_servers: Option<Vec<String>>,
        mcp_degraded: Option<McpDegradedReport>,
        mut dispatch: F,
    ) -> Result<String, ToolError>
    where
        F: FnMut(CapabilityDispatchKind, &str, Value) -> Result<String, ToolError>,
    {
        self.execute_tool_with_outcome(
            runtime,
            tool_name,
            input,
            planner_input,
            store,
            pending_mcp_servers,
            mcp_degraded,
            move |dispatch_kind, tool_name, input| {
                dispatch(dispatch_kind, tool_name, input)
                    .map(|output| ToolExecutionOutcome::Allow { output })
            },
        )
        .into_result(tool_name)
    }
}

#[must_use]
pub(crate) fn dispatch_kind_for_capability(capability: &CapabilitySpec) -> CapabilityDispatchKind {
    match capability.source_kind {
        CapabilitySourceKind::RuntimeTool
        | CapabilitySourceKind::McpTool
        | CapabilitySourceKind::McpPrompt
        | CapabilitySourceKind::McpResource => CapabilityDispatchKind::RuntimeCapability,
        CapabilitySourceKind::Builtin
        | CapabilitySourceKind::PluginTool
        | CapabilitySourceKind::LocalSkill
        | CapabilitySourceKind::BundledSkill
        | CapabilitySourceKind::PluginSkill => CapabilityDispatchKind::BuiltinOrPlugin,
    }
}
