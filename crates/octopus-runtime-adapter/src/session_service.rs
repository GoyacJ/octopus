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

        let detail = RuntimeSessionDetail {
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
            },
            run: RuntimeRunSnapshot {
                id: run_id,
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
                resolved_target: None,
                requested_actor_kind: None,
                requested_actor_id: None,
                resolved_actor_kind: None,
                resolved_actor_id: None,
                resolved_actor_label: None,
            },
            messages: Vec::new(),
            trace: Vec::new(),
            pending_approval: None,
        };
        let aggregate = RuntimeAggregate {
            detail: detail.clone(),
            events: Vec::new(),
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
