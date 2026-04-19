use super::*;

pub(crate) fn default_session_kind(session_kind: Option<String>) -> String {
    session_kind.unwrap_or_else(|| "project".into())
}

#[async_trait]
impl RuntimeSessionService for RuntimeAdapter {
    async fn bootstrap(&self) -> Result<RuntimeBootstrap, AppError> {
        let documents = self.resolve_documents(None, None)?;
        let registry = self.effective_registry(&documents)?;
        Ok(RuntimeBootstrap {
            provider: registry.default_provider_config(),
            sessions: self.list_sessions().await?,
        })
    }

    async fn list_sessions(&self) -> Result<Vec<RuntimeSessionSummary>, AppError> {
        let sessions = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let order = self
            .state
            .order
            .lock()
            .map_err(|_| AppError::runtime("runtime order mutex poisoned"))?;

        Ok(order
            .iter()
            .filter_map(|session_id| {
                sessions
                    .get(session_id)
                    .map(|aggregate| aggregate.detail.summary.clone())
            })
            .collect())
    }

    async fn create_session_with_owner_ceiling(
        &self,
        input: CreateRuntimeSessionInput,
        user_id: &str,
        owner_permission_ceiling: Option<&str>,
    ) -> Result<RuntimeSessionDetail, AppError> {
        let session_id = format!("rt-{}", Uuid::new_v4());
        let conversation_id = if input.conversation_id.is_empty() {
            format!("conv-{}", Uuid::new_v4())
        } else {
            input.conversation_id
        };
        let run_id = format!("run-{}", Uuid::new_v4());
        let now = timestamp_now();
        let project_id = input.project_id.clone().unwrap_or_default();
        let snapshot = self
            .current_config_snapshot(optional_project_id(&project_id).as_deref(), Some(user_id))?;
        self.persist_config_snapshot(&snapshot)?;
        let manifest = self.compile_actor_manifest(&input.selected_actor_ref)?;
        let session_policy = self
            .compile_session_policy(
                &session_id,
                &manifest,
                &snapshot,
                input.selected_configured_model_id.as_deref(),
                &input.execution_permission_mode,
                user_id,
                optional_project_id(&project_id).as_deref(),
                owner_permission_ceiling,
            )
            .await?;
        self.validate_session_creation_execution(&session_policy)?;
        self.persist_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref, &manifest)?;
        self.persist_session_policy_snapshot(
            &session_policy.session_policy_snapshot_ref,
            &session_policy,
        )?;
        let memory_summary = manifest.memory_summary();
        let capability_projection = self
            .project_capability_state_async(
                &manifest,
                &session_policy,
                &snapshot.id,
                format!("{run_id}-capability-state"),
                &tools::SessionCapabilityStore::default(),
            )
            .await?;
        let capability_summary = capability_projection.plan_summary.clone();
        let memory_selection_summary = RuntimeMemorySelectionSummary::default();
        let memory_state_ref = memory_runtime::runtime_memory_state_ref(&run_id, now);

        let mut detail = RuntimeSessionDetail {
            summary: RuntimeSessionSummary {
                id: session_id.clone(),
                conversation_id: conversation_id.clone(),
                project_id,
                title: input.title,
                session_kind: default_session_kind(input.session_kind),
                status: "draft".into(),
                updated_at: now,
                last_message_preview: None,
                config_snapshot_id: snapshot.id.clone(),
                effective_config_hash: snapshot.effective_config_hash.clone(),
                started_from_scope_set: snapshot.started_from_scope_set.clone(),
                selected_actor_ref: input.selected_actor_ref.clone(),
                manifest_revision: manifest.manifest_revision().to_string(),
                session_policy: session_policy.contract_snapshot(),
                active_run_id: run_id.clone(),
                subrun_count: 0,
                workflow: None,
                pending_mailbox: None,
                background_run: None,
                memory_summary,
                memory_selection_summary: memory_selection_summary.clone(),
                pending_memory_proposal_count: 0,
                memory_state_ref: memory_state_ref.clone(),
                capability_summary: capability_summary.clone(),
                provider_state_summary: capability_projection.provider_state_summary.clone(),
                auth_state_summary: capability_projection.auth_state_summary.clone(),
                pending_mediation: None,
                policy_decision_summary: capability_projection.policy_decision_summary.clone(),
                capability_state_ref: Some(capability_projection.capability_state_ref.clone()),
                last_execution_outcome: None,
            },
            selected_actor_ref: input.selected_actor_ref.clone(),
            manifest_revision: manifest.manifest_revision().to_string(),
            session_policy: session_policy.contract_snapshot(),
            active_run_id: run_id.clone(),
            subrun_count: 0,
            workflow: None,
            pending_mailbox: None,
            background_run: None,
            memory_summary: manifest.memory_summary(),
            memory_selection_summary,
            pending_memory_proposal_count: 0,
            memory_state_ref: memory_state_ref.clone(),
            capability_summary: capability_summary.clone(),
            provider_state_summary: capability_projection.provider_state_summary.clone(),
            auth_state_summary: capability_projection.auth_state_summary.clone(),
            pending_mediation: None,
            policy_decision_summary: capability_projection.policy_decision_summary.clone(),
            capability_state_ref: Some(capability_projection.capability_state_ref.clone()),
            last_execution_outcome: None,
            run: RuntimeRunSnapshot {
                id: run_id.clone(),
                session_id: session_id.clone(),
                conversation_id: conversation_id.clone(),
                status: "draft".into(),
                current_step: "ready".into(),
                started_at: now,
                updated_at: now,
                selected_memory: Vec::new(),
                freshness_summary: Some(RuntimeMemoryFreshnessSummary::default()),
                pending_memory_proposal: None,
                memory_state_ref: memory_state_ref.clone(),
                configured_model_id: None,
                configured_model_name: None,
                model_id: None,
                consumed_tokens: None,
                next_action: Some("submit_turn".into()),
                config_snapshot_id: snapshot.id.clone(),
                effective_config_hash: snapshot.effective_config_hash,
                started_from_scope_set: snapshot.started_from_scope_set,
                run_kind: "primary".into(),
                parent_run_id: None,
                actor_ref: input.selected_actor_ref.clone(),
                delegated_by_tool_call_id: None,
                workflow_run: None,
                workflow_run_detail: None,
                mailbox_ref: None,
                handoff_ref: None,
                background_state: None,
                worker_dispatch: None,
                approval_state: "not-required".into(),
                approval_target: None,
                auth_target: None,
                usage_summary: RuntimeUsageSummary::default(),
                artifact_refs: Vec::new(),
                deliverable_refs: Vec::new(),
                trace_context: trace_context::runtime_trace_context(&session_id, None),
                checkpoint: RuntimeRunCheckpoint {
                    approval_layer: None,
                    broker_decision: None,
                    capability_id: None,
                    checkpoint_artifact_ref: None,
                    current_iteration_index: 0,
                    tool_name: None,
                    dispatch_kind: None,
                    concurrency_policy: None,
                    input: None,
                    usage_summary: RuntimeUsageSummary::default(),
                    pending_approval: None,
                    pending_auth_challenge: None,
                    pending_mediation: None,
                    provider_key: None,
                    reason: None,
                    required_permission: None,
                    requires_approval: None,
                    requires_auth: None,
                    target_kind: None,
                    target_ref: None,
                    capability_state_ref: Some(capability_projection.capability_state_ref.clone()),
                    capability_plan_summary: capability_summary.clone(),
                    last_execution_outcome: None,
                    last_mediation_outcome: None,
                },
                capability_plan_summary: capability_summary,
                provider_state_summary: capability_projection.provider_state_summary.clone(),
                pending_mediation: None,
                capability_state_ref: Some(capability_projection.capability_state_ref),
                last_execution_outcome: None,
                last_mediation_outcome: None,
                resolved_target: None,
                requested_actor_kind: Some(manifest.actor_kind_label().into()),
                requested_actor_id: Some(input.selected_actor_ref.clone()),
                resolved_actor_kind: Some(manifest.actor_kind_label().into()),
                resolved_actor_id: Some(input.selected_actor_ref.clone()),
                resolved_actor_label: Some(manifest.label().to_string()),
            },
            subruns: Vec::new(),
            handoffs: Vec::new(),
            messages: Vec::new(),
            trace: Vec::new(),
            pending_approval: None,
        };
        sync_runtime_session_detail(&mut detail);
        let aggregate = RuntimeAggregate {
            detail: detail.clone(),
            events: Vec::new(),
            metadata: RuntimeAggregateMetadata {
                manifest_snapshot_ref: session_policy.manifest_snapshot_ref.clone(),
                session_policy_snapshot_ref: session_policy.session_policy_snapshot_ref.clone(),
                primary_run_serialized_session: json!({}),
                pending_deliverables: BTreeMap::new(),
                subrun_states: BTreeMap::new(),
            },
        };

        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?
            .insert(session_id.clone(), aggregate.clone());
        self.state
            .order
            .lock()
            .map_err(|_| AppError::runtime("runtime order mutex poisoned"))?
            .insert(0, session_id.clone());
        self.persist_runtime_projections(&aggregate)?;

        let policy_event = RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "policy.session_compiled".into(),
            kind: Some("policy.session_compiled".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&detail.summary.project_id),
            session_id: session_id.clone(),
            conversation_id: detail.summary.conversation_id.clone(),
            run_id: Some(detail.run.id.clone()),
            emitted_at: now,
            sequence: 0,
            outcome: Some("compiled".into()),
            run: Some(detail.run.clone()),
            summary: Some(detail.summary.clone()),
            capability_plan_summary: Some(detail.summary.capability_summary.clone()),
            provider_state_summary: Some(detail.summary.provider_state_summary.clone()),
            pending_mediation: detail.summary.pending_mediation.clone(),
            capability_state_ref: detail.summary.capability_state_ref.clone(),
            last_execution_outcome: detail.summary.last_execution_outcome.clone(),
            last_mediation_outcome: detail.run.last_mediation_outcome.clone(),
            ..Default::default()
        };
        self.emit_event(&session_id, policy_event).await?;

        let event = RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.session.updated".into(),
            kind: Some("runtime.session.updated".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&detail.summary.project_id),
            session_id: session_id.clone(),
            conversation_id,
            run_id: Some(detail.run.id.clone()),
            parent_run_id: None,
            emitted_at: now,
            sequence: 0,
            iteration: None,
            workflow_run_id: None,
            workflow_step_id: None,
            actor_ref: Some(detail.run.actor_ref.clone()),
            tool_use_id: None,
            outcome: None,
            approval_layer: None,
            target_kind: None,
            target_ref: None,
            run: Some(detail.run.clone()),
            message: None,
            memory_proposal: None,
            memory_selection_summary: Some(detail.summary.memory_selection_summary.clone()),
            freshness_summary: detail.run.freshness_summary.clone(),
            selected_memory: Some(detail.run.selected_memory.clone()),
            trace: None,
            approval: None,
            auth_challenge: None,
            decision: None,
            summary: Some(detail.summary.clone()),
            error: None,
            capability_plan_summary: Some(detail.summary.capability_summary.clone()),
            provider_state_summary: Some(detail.summary.provider_state_summary.clone()),
            pending_mediation: detail.summary.pending_mediation.clone(),
            capability_state_ref: detail.summary.capability_state_ref.clone(),
            last_execution_outcome: detail.summary.last_execution_outcome.clone(),
            last_mediation_outcome: detail.run.last_mediation_outcome.clone(),
        };
        self.emit_event(&session_id, event).await?;

        Ok(detail)
    }

    async fn get_session(&self, session_id: &str) -> Result<RuntimeSessionDetail, AppError> {
        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?
            .get(session_id)
            .map(|aggregate| aggregate.detail.clone())
            .ok_or_else(|| AppError::not_found("runtime session"))
    }

    async fn get_deliverable_detail(
        &self,
        deliverable_id: &str,
    ) -> Result<DeliverableDetail, AppError> {
        RuntimeAdapter::get_deliverable_detail(self, deliverable_id)?
            .ok_or_else(|| AppError::not_found(format!("deliverable `{deliverable_id}`")))
    }

    async fn list_deliverable_versions(
        &self,
        deliverable_id: &str,
    ) -> Result<Vec<DeliverableVersionSummary>, AppError> {
        RuntimeAdapter::list_deliverable_versions(self, deliverable_id)
    }

    async fn get_deliverable_version_content(
        &self,
        deliverable_id: &str,
        version: u32,
    ) -> Result<DeliverableVersionContent, AppError> {
        RuntimeAdapter::get_deliverable_version_content(self, deliverable_id, version)?.ok_or_else(
            || AppError::not_found(format!("deliverable version `{deliverable_id}`:{version}")),
        )
    }

    async fn create_deliverable_version(
        &self,
        deliverable_id: &str,
        input: CreateDeliverableVersionInput,
    ) -> Result<DeliverableDetail, AppError> {
        RuntimeAdapter::create_deliverable_version(self, deliverable_id, input).await
    }

    async fn promote_deliverable(
        &self,
        deliverable_id: &str,
        input: PromoteDeliverableInput,
    ) -> Result<DeliverableDetail, AppError> {
        RuntimeAdapter::promote_deliverable(self, deliverable_id, input).await
    }

    async fn list_events(
        &self,
        session_id: &str,
        after: Option<&str>,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError> {
        let sessions = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        if let Some(after_id) = after {
            let position = aggregate
                .events
                .iter()
                .position(|event| event.id == after_id)
                .map(|index| index + 1)
                .unwrap_or(0);
            return Ok(aggregate.events[position..].to_vec());
        }

        Ok(aggregate.events.clone())
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), AppError> {
        let mut sessions = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let mut order = self
            .state
            .order
            .lock()
            .map_err(|_| AppError::runtime("runtime order mutex poisoned"))?;

        sessions.remove(session_id);
        order.retain(|id| id != session_id);

        let _ = fs::remove_file(self.runtime_events_path(session_id));

        let connection = self.open_db()?;
        connection
            .execute(
                "DELETE FROM runtime_session_projections WHERE id = ?1",
                [session_id],
            )
            .map_err(|e| AppError::database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::default_session_kind;

    #[test]
    fn defaults_session_kind_to_project() {
        assert_eq!(default_session_kind(None), "project".to_string());
        assert_eq!(
            default_session_kind(Some("review".into())),
            "review".to_string()
        );
    }
}
