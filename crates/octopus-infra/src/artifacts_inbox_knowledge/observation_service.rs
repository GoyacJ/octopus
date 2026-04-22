use super::*;
use octopus_core::ProjectTokenUsageProjection;

#[async_trait]
impl ObservationService for InfraObservationService {
    async fn list_trace_events(&self) -> Result<Vec<TraceEventRecord>, AppError> {
        Ok(self
            .state
            .trace_events
            .lock()
            .map_err(|_| AppError::runtime("trace mutex poisoned"))?
            .clone())
    }

    async fn list_audit_records(&self) -> Result<Vec<AuditRecord>, AppError> {
        Ok(self
            .state
            .audit_records
            .lock()
            .map_err(|_| AppError::runtime("audit mutex poisoned"))?
            .clone())
    }

    async fn list_cost_entries(&self) -> Result<Vec<CostLedgerEntry>, AppError> {
        Ok(self
            .state
            .cost_entries
            .lock()
            .map_err(|_| AppError::runtime("cost mutex poisoned"))?
            .clone())
    }

    async fn list_project_token_usage(&self) -> Result<Vec<ProjectTokenUsageProjection>, AppError> {
        let connection = self.state.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT project_id, used_tokens, updated_at
                 FROM project_token_usage_projections
                 ORDER BY used_tokens DESC, project_id ASC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                Ok(ProjectTokenUsageProjection {
                    project_id: row.get(0)?,
                    used_tokens: row.get::<_, i64>(1)?.max(0) as u64,
                    updated_at: row.get::<_, i64>(2)? as u64,
                })
            })
            .map_err(|error| AppError::database(error.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))
    }

    async fn project_used_tokens(&self, project_id: &str) -> Result<u64, AppError> {
        let connection = self.state.open_db()?;
        let used_tokens = connection
            .query_row(
                "SELECT used_tokens
                 FROM project_token_usage_projections
                 WHERE project_id = ?1",
                [project_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?
            .unwrap_or(0);
        Ok(used_tokens.max(0) as u64)
    }

    async fn append_trace(&self, record: TraceEventRecord) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "INSERT INTO trace_events (id, workspace_id, project_id, run_id, session_id, event_kind, title, detail, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.run_id,
                    record.session_id,
                    record.event_kind,
                    record.title,
                    record.detail,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        append_json_line(
            &self
                .state
                .paths
                .runtime_traces_dir
                .join("trace-events.jsonl"),
            &record,
        )?;
        self.state
            .trace_events
            .lock()
            .map_err(|_| AppError::runtime("trace mutex poisoned"))?
            .push(record);
        Ok(())
    }

    async fn append_audit(&self, record: AuditRecord) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "INSERT INTO audit_records (id, workspace_id, project_id, actor_type, actor_id, action, resource, outcome, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.actor_type,
                    record.actor_id,
                    record.action,
                    record.resource,
                    record.outcome,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        append_json_line(
            &self.state.paths.audit_log_dir.join("audit-records.jsonl"),
            &record,
        )?;
        self.state
            .audit_records
            .lock()
            .map_err(|_| AppError::runtime("audit mutex poisoned"))?
            .push(record);
        Ok(())
    }

    async fn append_cost(&self, record: CostLedgerEntry) -> Result<(), AppError> {
        let connection = self.state.open_db()?;
        connection
            .execute(
                "INSERT INTO cost_entries (id, workspace_id, project_id, run_id, configured_model_id, metric, amount, unit, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.run_id,
                    record.configured_model_id,
                    record.metric,
                    record.amount,
                    record.unit,
                    record.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        if record.metric == "tokens" {
            if let (Some(project_id), amount) = (record.project_id.as_deref(), record.amount) {
                if amount > 0 {
                    connection
                        .execute(
                            "INSERT INTO project_token_usage_projections (project_id, used_tokens, updated_at)
                             VALUES (?1, ?2, ?3)
                             ON CONFLICT(project_id)
                             DO UPDATE SET
                               used_tokens = project_token_usage_projections.used_tokens + excluded.used_tokens,
                               updated_at = excluded.updated_at",
                            params![project_id, amount, record.created_at as i64],
                        )
                        .map_err(|error| AppError::database(error.to_string()))?;
                }
            }
        }
        append_json_line(
            &self.state.paths.server_log_dir.join("cost-ledger.jsonl"),
            &record,
        )?;
        self.state
            .cost_entries
            .lock()
            .map_err(|_| AppError::runtime("cost mutex poisoned"))?
            .push(record);
        Ok(())
    }
}
