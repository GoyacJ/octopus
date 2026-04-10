use super::*;

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
        aggregate.events.push(event.clone());
        self.persist_session(session_id, aggregate)?;
        persistence::append_json_line(&self.runtime_events_path(session_id), &event)?;
        drop(sessions);

        let sender = self.session_sender(session_id)?;
        let _ = sender.send(event);
        Ok(())
    }
}
