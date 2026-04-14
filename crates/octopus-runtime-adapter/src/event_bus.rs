use super::*;

fn resolve_event_mediation_metadata(
    event: &RuntimeEventEnvelope,
) -> (Option<String>, Option<String>, Option<String>) {
    if let Some(approval) = event.approval.as_ref() {
        return (
            approval.approval_layer.clone(),
            approval.target_kind.clone(),
            approval.target_ref.clone(),
        );
    }
    if let Some(challenge) = event.auth_challenge.as_ref() {
        return (
            Some(challenge.approval_layer.clone()),
            Some(challenge.target_kind.clone()),
            Some(challenge.target_ref.clone()),
        );
    }
    if let Some(pending) = event.pending_mediation.as_ref() {
        return (
            pending.approval_layer.clone(),
            Some(pending.target_kind.clone()),
            Some(pending.target_ref.clone()),
        );
    }
    if let Some(run) = event.run.as_ref() {
        if let Some(approval) = run.approval_target.as_ref() {
            return (
                approval.approval_layer.clone(),
                approval.target_kind.clone(),
                approval.target_ref.clone(),
            );
        }
        if let Some(challenge) = run.auth_target.as_ref() {
            return (
                Some(challenge.approval_layer.clone()),
                Some(challenge.target_kind.clone()),
                Some(challenge.target_ref.clone()),
            );
        }
        if let Some(pending) = run.pending_mediation.as_ref() {
            return (
                pending.approval_layer.clone(),
                Some(pending.target_kind.clone()),
                Some(pending.target_ref.clone()),
            );
        }
        if let Some(outcome) = run.last_mediation_outcome.as_ref() {
            return (
                outcome.approval_layer.clone(),
                Some(outcome.target_kind.clone()),
                Some(outcome.target_ref.clone()),
            );
        }
    }
    if let Some(outcome) = event.last_mediation_outcome.as_ref() {
        return (
            outcome.approval_layer.clone(),
            Some(outcome.target_kind.clone()),
            Some(outcome.target_ref.clone()),
        );
    }
    (None, None, None)
}

impl RuntimeAdapter {
    pub(super) async fn emit_event(
        &self,
        session_id: &str,
        mut event: RuntimeEventEnvelope,
    ) -> Result<(), AppError> {
        let mut sessions = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
        let aggregate = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::not_found("runtime session"))?;
        event.sequence = aggregate
            .events
            .last()
            .map(|existing| existing.sequence + 1)
            .unwrap_or(1);
        if event.kind.is_none() {
            event.kind = Some(event.event_type.clone());
        }
        let (approval_layer, target_kind, target_ref) = resolve_event_mediation_metadata(&event);
        if event.approval_layer.is_none() {
            event.approval_layer = approval_layer;
        }
        if event.target_kind.is_none() {
            event.target_kind = target_kind;
        }
        if event.target_ref.is_none() {
            event.target_ref = target_ref;
        }
        aggregate.events.push(event.clone());
        self.persist_session(session_id, aggregate)?;
        persistence::append_json_line(&self.runtime_events_path(session_id), &event)?;
        drop(sessions);

        let sender = self.session_sender(session_id)?;
        let _ = sender.send(event);
        Ok(())
    }
}
