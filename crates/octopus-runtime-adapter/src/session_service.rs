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

    async fn create_session(
        &self,
        input: CreateRuntimeSessionInput,
        user_id: &str,
    ) -> Result<RuntimeSessionDetail, AppError> {
        let session_id = format!("rt-{}", Uuid::new_v4());
        let conversation_id = if input.conversation_id.is_empty() {
            format!("conv-{}", Uuid::new_v4())
        } else {
            input.conversation_id
        };
        let run_id = format!("run-{}", Uuid::new_v4());
        let now = timestamp_now();
        let project_id = input.project_id.clone();
        let snapshot = self
            .current_config_snapshot(optional_project_id(&project_id).as_deref(), Some(user_id))?;
        self.persist_config_snapshot(&snapshot)?;
        let manifest = self.compile_actor_manifest(&input.selected_actor_ref)?;
        let session_policy = self.compile_session_policy(
            &session_id,
            &manifest,
            &snapshot,
            input.selected_configured_model_id.as_deref(),
            &input.execution_permission_mode,
        )?;
        self.persist_actor_manifest_snapshot(&session_policy.manifest_snapshot_ref, &manifest)?;
        self.persist_session_policy_snapshot(
            &session_policy.session_policy_snapshot_ref,
            &session_policy,
        )?;
        let memory_summary = manifest.memory_summary();
        let capability_projection = self.project_capability_state(
            &manifest,
            &snapshot.id,
            format!("{run_id}-capability-state"),
            &tools::SessionCapabilityStore::default(),
        )?;
        let capability_summary = capability_projection.plan_summary.clone();

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
                memory_summary,
                capability_summary: capability_summary.clone(),
                provider_state_summary: capability_projection.provider_state_summary.clone(),
                pending_mediation: None,
                capability_state_ref: Some(capability_projection.capability_state_ref.clone()),
                last_execution_outcome: None,
            },
            selected_actor_ref: input.selected_actor_ref.clone(),
            manifest_revision: manifest.manifest_revision().to_string(),
            session_policy: session_policy.contract_snapshot(),
            active_run_id: run_id.clone(),
            subrun_count: 0,
            memory_summary: manifest.memory_summary(),
            capability_summary: capability_summary.clone(),
            provider_state_summary: capability_projection.provider_state_summary.clone(),
            pending_mediation: None,
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
                approval_state: "not-required".into(),
                usage_summary: RuntimeUsageSummary::default(),
                artifact_refs: Vec::new(),
                trace_context: trace_context::runtime_trace_context(&session_id, None),
                checkpoint: RuntimeRunCheckpoint {
                    serialized_session: json!({}),
                    current_iteration_index: 0,
                    usage_summary: RuntimeUsageSummary::default(),
                    pending_approval: None,
                    compaction_metadata: json!({}),
                    pending_mediation: None,
                    capability_state_ref: Some(capability_projection.capability_state_ref.clone()),
                    capability_plan_summary: capability_summary.clone(),
                    last_execution_outcome: None,
                },
                capability_plan_summary: capability_summary,
                provider_state_summary: capability_projection.provider_state_summary.clone(),
                pending_mediation: None,
                capability_state_ref: Some(capability_projection.capability_state_ref),
                last_execution_outcome: None,
                resolved_target: None,
                requested_actor_kind: Some(manifest.actor_kind_label().into()),
                requested_actor_id: Some(input.selected_actor_ref.clone()),
                resolved_actor_kind: Some(manifest.actor_kind_label().into()),
                resolved_actor_id: Some(input.selected_actor_ref.clone()),
                resolved_actor_label: Some(manifest.label().to_string()),
            },
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
        self.persist_session(&session_id, &aggregate)?;

        let event = RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.session.updated".into(),
            kind: Some("runtime.session.updated".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&detail.summary.project_id),
            session_id: session_id.clone(),
            conversation_id,
            run_id: Some(detail.run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "summary": detail.summary.clone(),
                "run": detail.run.clone(),
            })),
            run: Some(detail.run.clone()),
            message: None,
            trace: None,
            approval: None,
            decision: None,
            summary: Some(detail.summary.clone()),
            error: None,
            capability_plan_summary: Some(detail.summary.capability_summary.clone()),
            provider_state_summary: Some(detail.summary.provider_state_summary.clone()),
            pending_mediation: detail.summary.pending_mediation.clone(),
            capability_state_ref: detail.summary.capability_state_ref.clone(),
            last_execution_outcome: detail.summary.last_execution_outcome.clone(),
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

        let _ = fs::remove_file(self.runtime_debug_session_path(session_id));
        let _ = fs::remove_file(self.runtime_debug_events_path(session_id));

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
