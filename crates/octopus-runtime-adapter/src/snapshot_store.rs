use super::*;

impl RuntimeAdapter {
    pub(super) fn persist_config_snapshot(
        &self,
        snapshot: &RuntimeConfigSnapshotSummary,
    ) -> Result<(), AppError> {
        self.open_db()?
            .execute(
                "INSERT OR REPLACE INTO runtime_config_snapshots
                 (id, effective_config_hash, started_from_scope_set, source_refs, created_at, effective_config_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    snapshot.id,
                    snapshot.effective_config_hash,
                    serde_json::to_string(&snapshot.started_from_scope_set)?,
                    serde_json::to_string(&snapshot.source_refs)?,
                    snapshot.created_at as i64,
                    snapshot
                        .effective_config
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .config_snapshots
            .lock()
            .map_err(|_| AppError::runtime("runtime config snapshots mutex poisoned"))?
            .insert(
                snapshot.id.clone(),
                snapshot
                    .effective_config
                    .clone()
                    .unwrap_or_else(|| json!({})),
            );
        Ok(())
    }

    pub(super) fn load_persisted_config_snapshots(&self) -> Result<(), AppError> {
        let connection = self.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT id, effective_config_json
                 FROM runtime_config_snapshots",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let payload: Option<String> = row.get(1)?;
                Ok((id, payload))
            })
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut snapshots = self
            .state
            .config_snapshots
            .lock()
            .map_err(|_| AppError::runtime("runtime config snapshots mutex poisoned"))?;
        snapshots.clear();

        for row in rows {
            let (id, payload) = row.map_err(|error| AppError::database(error.to_string()))?;
            let parsed = payload
                .as_deref()
                .map(serde_json::from_str::<Value>)
                .transpose()?
                .unwrap_or_else(|| json!({}));
            snapshots.insert(id, parsed);
        }

        Ok(())
    }
}
