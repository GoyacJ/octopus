use super::*;
use octopus_core::RuntimeTargetPolicyDecision;

#[derive(Debug, Clone)]
pub(crate) struct RunContext {
    pub(crate) session_id: String,
    pub(crate) conversation_id: String,
    pub(crate) run_id: String,
    pub(crate) actor_manifest: actor_manifest::CompiledActorManifest,
    pub(crate) requested_permission_mode: String,
    pub(crate) resolved_target: ResolvedExecutionTarget,
    pub(crate) configured_model: ConfiguredModelRecord,
    pub(crate) capability_plan_summary: RuntimeCapabilityPlanSummary,
    pub(crate) provider_state_summary: Vec<RuntimeCapabilityProviderState>,
    pub(crate) auth_state_summary: RuntimeAuthStateSummary,
    pub(crate) policy_decision_summary: RuntimePolicyDecisionSummary,
    pub(crate) provider_auth_policy_decision: Option<RuntimeTargetPolicyDecision>,
    pub(crate) capability_state_ref: String,
    pub(crate) memory_selection: memory_selector::RuntimeMemorySelection,
    pub(crate) pending_memory_proposal: Option<RuntimeMemoryProposal>,
    pub(crate) trace_context: RuntimeTraceContext,
    pub(crate) now: u64,
}

impl RuntimeAdapter {
    pub(crate) async fn build_run_context(
        &self,
        session_id: &str,
        input: &SubmitRuntimeTurnInput,
        now: u64,
    ) -> Result<RunContext, AppError> {
        let (
            conversation_id,
            project_id,
            session_policy_snapshot_ref,
            run_id,
            capability_state_ref,
        ) = {
            let sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;
            (
                aggregate.detail.summary.conversation_id.clone(),
                aggregate.detail.summary.project_id.clone(),
                aggregate.metadata.session_policy_snapshot_ref.clone(),
                aggregate.detail.run.id.clone(),
                aggregate
                    .detail
                    .run
                    .capability_state_ref
                    .clone()
                    .or_else(|| aggregate.detail.capability_state_ref.clone()),
            )
        };
        let session_policy = self.load_session_policy_snapshot(&session_policy_snapshot_ref)?;
        let requested_permission_mode =
            self.narrow_permission_mode(&session_policy, input.permission_mode.as_deref())?;
        let actor_manifest =
            self.load_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref)?;
        let (resolved_target, configured_model) =
            self.resolve_submit_execution(&session_policy, input)?;
        let capability_state_ref = capability_state_ref
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("{run_id}-capability-state"));
        let capability_store = self.load_capability_store(Some(&capability_state_ref))?;
        let capability_projection = self
            .project_capability_state_async(
                &actor_manifest,
                &session_policy,
                &session_policy.config_snapshot_id,
                capability_state_ref.clone(),
                &capability_store,
            )
            .await?;
        let memory_selection =
            self.select_runtime_memory(&session_policy, &project_id, &run_id, now, input)?;
        let pending_memory_proposal = memory_writer::build_memory_proposal(
            session_id,
            &run_id,
            &project_id,
            &memory_runtime::parse_memory_policy(&session_policy.memory_policy),
            input,
            &memory_selection.selected_memory,
        );
        let provider_auth_policy_decision = session_policy
            .target_decisions
            .get(&format!("provider-auth:{}", actor_manifest.actor_ref()))
            .cloned();

        Ok(RunContext {
            session_id: session_id.to_string(),
            conversation_id,
            run_id,
            actor_manifest,
            requested_permission_mode,
            resolved_target,
            configured_model,
            capability_plan_summary: capability_projection.plan_summary,
            provider_state_summary: capability_projection.provider_state_summary,
            auth_state_summary: capability_projection.auth_state_summary,
            policy_decision_summary: capability_projection.policy_decision_summary,
            provider_auth_policy_decision,
            capability_state_ref: capability_projection.capability_state_ref,
            memory_selection,
            pending_memory_proposal,
            trace_context: trace_context::runtime_trace_context(session_id, None),
            now,
        })
    }
}
