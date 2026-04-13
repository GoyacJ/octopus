use super::*;

pub(super) fn append_json_line(path: &Path, value: &impl Serialize) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    Ok(())
}

impl RuntimeAdapter {
    pub(super) fn runtime_events_path(&self, session_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_events_dir
            .join(format!("{session_id}.jsonl"))
    }

    pub(super) fn runtime_debug_session_path(&self, session_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join(format!("{session_id}.json"))
    }

    pub(super) fn runtime_debug_events_path(&self, session_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join(format!("{session_id}-events.json"))
    }

    pub(super) fn load_persisted_sessions(&self) -> Result<(), AppError> {
        let connection = self.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT detail_json, manifest_snapshot_ref, session_policy_snapshot_ref
                 FROM runtime_session_projections
                 ORDER BY updated_at DESC, id DESC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|error| AppError::database(error.to_string()))?;

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
        sessions.clear();
        order.clear();

        for row in rows {
            let (detail_json, manifest_snapshot_ref, session_policy_snapshot_ref) =
                row.map_err(|error| AppError::database(error.to_string()))?;
            let mut detail = serde_json::from_str::<RuntimeSessionDetail>(&detail_json)?;
            sync_runtime_session_detail(&mut detail);
            let events = self.load_event_log(&detail.summary.id)?;
            let fallback_manifest_snapshot_ref = format!("{}-manifest", detail.summary.id);
            let fallback_session_policy_snapshot_ref = format!("{}-policy", detail.summary.id);
            order.push(detail.summary.id.clone());
            sessions.insert(
                detail.summary.id.clone(),
                RuntimeAggregate {
                    detail,
                    events,
                    metadata: RuntimeAggregateMetadata {
                        manifest_snapshot_ref: manifest_snapshot_ref
                            .unwrap_or(fallback_manifest_snapshot_ref),
                        session_policy_snapshot_ref: session_policy_snapshot_ref
                            .unwrap_or(fallback_session_policy_snapshot_ref),
                    },
                },
            );
        }

        Ok(())
    }

    pub(super) fn load_event_log(
        &self,
        session_id: &str,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError> {
        let path = self.runtime_events_path(session_id);
        if path.exists() {
            let file = fs::File::open(&path)?;
            let reader = BufReader::new(file);
            let mut events = Vec::new();
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                events.push(serde_json::from_str(&line)?);
            }
            return Ok(events);
        }

        let legacy_path = self.runtime_debug_events_path(session_id);
        if legacy_path.exists() {
            return Ok(serde_json::from_str(&fs::read_to_string(legacy_path)?)?);
        }

        Ok(Vec::new())
    }

    pub(super) fn persist_session(
        &self,
        session_id: &str,
        aggregate: &RuntimeAggregate,
    ) -> Result<(), AppError> {
        if let Some(parent) = self.runtime_debug_session_path(session_id).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            self.runtime_debug_session_path(session_id),
            serde_json::to_vec_pretty(&aggregate.detail)?,
        )?;
        fs::write(
            self.runtime_debug_events_path(session_id),
            serde_json::to_vec_pretty(&aggregate.events)?,
        )?;
        self.persist_runtime_projections(aggregate)?;
        Ok(())
    }

    pub(super) fn persist_runtime_projections(
        &self,
        aggregate: &RuntimeAggregate,
    ) -> Result<(), AppError> {
        let connection = self.open_db()?;
        let summary = &aggregate.detail.summary;
        let run = &aggregate.detail.run;
        let started_from_scope_set = serde_json::to_string(&summary.started_from_scope_set)?;
        let detail_json = serde_json::to_string(&aggregate.detail)?;
        let run_json = serde_json::to_string(run)?;
        let session_capability_snapshot = self
            .load_capability_state_snapshot(summary.capability_state_ref.as_deref())?
            .unwrap_or_default();
        let run_capability_snapshot = self
            .load_capability_state_snapshot(run.capability_state_ref.as_deref())?
            .unwrap_or_default();
        let capability_plan_summary_json = serde_json::to_string(&summary.capability_summary)?;
        let provider_state_summary_json = serde_json::to_string(&summary.provider_state_summary)?;
        let pending_mediation_json = summary
            .pending_mediation
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let last_execution_outcome_json = summary
            .last_execution_outcome
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let granted_tool_count = session_capability_snapshot.granted_tool_count as i64;
        let injected_skill_message_count =
            session_capability_snapshot.injected_skill_message_count as i64;
        let deferred_capability_count =
            aggregate.detail.capability_summary.deferred_tools.len() as i64;
        let hidden_capability_count = aggregate
            .detail
            .capability_summary
            .hidden_capabilities
            .len() as i64;
        let degraded_provider_count = aggregate
            .detail
            .provider_state_summary
            .iter()
            .filter(|provider| provider.degraded)
            .count() as i64;
        let run_capability_plan_summary_json = serde_json::to_string(&run.capability_plan_summary)?;
        let run_provider_state_summary_json = serde_json::to_string(&run.provider_state_summary)?;
        let run_pending_mediation_json = run
            .pending_mediation
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let run_last_execution_outcome_json = run
            .last_execution_outcome
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let run_granted_tool_count = run_capability_snapshot.granted_tool_count as i64;
        let run_injected_skill_message_count =
            run_capability_snapshot.injected_skill_message_count as i64;
        let run_deferred_capability_count = run.capability_plan_summary.deferred_tools.len() as i64;
        let run_hidden_capability_count =
            run.capability_plan_summary.hidden_capabilities.len() as i64;
        let run_degraded_provider_count = run
            .provider_state_summary
            .iter()
            .filter(|provider| provider.degraded)
            .count() as i64;

        connection
            .execute(
                "INSERT OR REPLACE INTO runtime_session_projections
                 (id, conversation_id, project_id, title, session_kind, status, updated_at, last_message_preview,
                  config_snapshot_id, effective_config_hash, started_from_scope_set, selected_actor_ref,
                  manifest_revision, active_run_id, subrun_count, manifest_snapshot_ref,
                  session_policy_snapshot_ref, capability_plan_summary_json, provider_state_summary_json,
                  pending_mediation_json, capability_state_ref, last_execution_outcome_json,
                  granted_tool_count, injected_skill_message_count, deferred_capability_count,
                  hidden_capability_count, degraded_provider_count, detail_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)",
                params![
                    summary.id,
                    summary.conversation_id,
                    summary.project_id,
                    summary.title,
                    summary.session_kind,
                    summary.status,
                    summary.updated_at as i64,
                    summary.last_message_preview,
                    summary.config_snapshot_id,
                    summary.effective_config_hash,
                    started_from_scope_set,
                    aggregate.detail.selected_actor_ref,
                    aggregate.detail.manifest_revision,
                    aggregate.detail.active_run_id,
                    aggregate.detail.subrun_count as i64,
                    aggregate.metadata.manifest_snapshot_ref,
                    aggregate.metadata.session_policy_snapshot_ref,
                    capability_plan_summary_json,
                    provider_state_summary_json,
                    pending_mediation_json,
                    summary.capability_state_ref,
                    last_execution_outcome_json,
                    granted_tool_count,
                    injected_skill_message_count,
                    deferred_capability_count,
                    hidden_capability_count,
                    degraded_provider_count,
                    detail_json,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        connection
            .execute(
                "INSERT OR REPLACE INTO runtime_run_projections
                 (id, session_id, conversation_id, status, current_step, started_at, updated_at,
                  model_id, next_action, config_snapshot_id, effective_config_hash,
                  started_from_scope_set, run_kind, parent_run_id, actor_ref, delegated_by_tool_call_id,
                  approval_state, trace_id, turn_id, capability_plan_summary_json,
                  provider_state_summary_json, pending_mediation_json, capability_state_ref,
                  last_execution_outcome_json, granted_tool_count, injected_skill_message_count,
                  deferred_capability_count, hidden_capability_count, degraded_provider_count, run_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30)",
                params![
                    run.id,
                    run.session_id,
                    run.conversation_id,
                    run.status,
                    run.current_step,
                    run.started_at as i64,
                    run.updated_at as i64,
                    run.model_id,
                    run.next_action,
                    run.config_snapshot_id,
                    run.effective_config_hash,
                    serde_json::to_string(&run.started_from_scope_set)?,
                    run.run_kind,
                    run.parent_run_id,
                    run.actor_ref,
                    run.delegated_by_tool_call_id,
                    run.approval_state,
                    run.trace_context.trace_id,
                    run.trace_context.turn_id,
                    run_capability_plan_summary_json,
                    run_provider_state_summary_json,
                    run_pending_mediation_json,
                    run.capability_state_ref,
                    run_last_execution_outcome_json,
                    run_granted_tool_count,
                    run_injected_skill_message_count,
                    run_deferred_capability_count,
                    run_hidden_capability_count,
                    run_degraded_provider_count,
                    run_json,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        connection
            .execute(
                "DELETE FROM runtime_approval_projections WHERE session_id = ?1",
                [summary.id.as_str()],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if let Some(approval) = aggregate.detail.pending_approval.as_ref() {
            connection
                .execute(
                    "INSERT OR REPLACE INTO runtime_approval_projections
                     (id, session_id, run_id, conversation_id, tool_name, summary, detail,
                      risk_level, created_at, status, approval_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        approval.id,
                        approval.session_id,
                        approval.run_id,
                        approval.conversation_id,
                        approval.tool_name,
                        approval.summary,
                        approval.detail,
                        approval.risk_level,
                        approval.created_at as i64,
                        approval.status,
                        serde_json::to_string(approval)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        Ok(())
    }
}
