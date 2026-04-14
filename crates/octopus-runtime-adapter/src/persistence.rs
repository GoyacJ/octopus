use super::*;
use serde::de::DeserializeOwned;

pub(super) fn append_json_line(path: &Path, value: &impl Serialize) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedMailboxBody {
    session_id: String,
    run_id: String,
    conversation_id: String,
    summary: RuntimeMailboxSummary,
    handoff_refs: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedHandoffEnvelope {
    session_id: String,
    run_id: String,
    conversation_id: String,
    parent_run_id: Option<String>,
    delegated_by_tool_call_id: Option<String>,
    sender_actor_ref: String,
    receiver_actor_ref: String,
    mailbox_ref: String,
    artifact_refs: Vec<String>,
    handoff_state: String,
    updated_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedWorkflowState {
    summary: RuntimeWorkflowSummary,
    detail: RuntimeWorkflowRunDetail,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedBackgroundState {
    summary: RuntimeBackgroundRunSummary,
}

fn bool_to_sql(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256-{:x}", hasher.finalize())
}

fn pending_mediation_projection_fields(
    pending: Option<&RuntimePendingMediationSummary>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let Some(pending) = pending else {
        return (None, None, None, None, None, None);
    };

    (
        Some(pending.mediation_kind.clone()),
        Some(pending.target_kind.clone()),
        Some(pending.target_ref.clone()),
        pending.approval_layer.clone(),
        pending.provider_key.clone(),
        pending.checkpoint_ref.clone(),
    )
}

fn last_mediation_projection_fields(
    outcome: Option<&RuntimeMediationOutcome>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
) {
    let Some(outcome) = outcome else {
        return (None, None, None, None);
    };

    (
        Some(outcome.outcome.clone()),
        Some(outcome.target_kind.clone()),
        Some(outcome.target_ref.clone()),
        outcome.resolved_at.map(|value| value as i64),
    )
}

fn auth_challenge_state(run: &RuntimeRunSnapshot) -> Option<String> {
    run.auth_target
        .as_ref()
        .map(|challenge| challenge.status.clone())
        .or_else(|| {
            run.checkpoint
                .pending_auth_challenge
                .as_ref()
                .map(|challenge| challenge.status.clone())
        })
}

fn approval_lineage_json(
    pending_approval: Option<&ApprovalRequestRecord>,
    auth_target: Option<&RuntimeAuthChallengeSummary>,
    last_outcome: Option<&RuntimeMediationOutcome>,
) -> Result<Option<String>, AppError> {
    let mut lineage = Vec::new();

    if let Some(approval) = pending_approval {
        lineage.push(json!({
            "id": approval.id,
            "kind": "approval",
            "status": approval.status,
            "targetKind": approval.target_kind,
            "targetRef": approval.target_ref,
        }));
    }
    if let Some(challenge) = auth_target {
        lineage.push(json!({
            "id": challenge.id,
            "kind": "auth",
            "status": challenge.status,
            "targetKind": challenge.target_kind,
            "targetRef": challenge.target_ref,
        }));
    }
    if let Some(outcome) = last_outcome {
        lineage.push(json!({
            "id": outcome.mediation_id,
            "kind": outcome.mediation_kind,
            "status": outcome.outcome,
            "targetKind": outcome.target_kind,
            "targetRef": outcome.target_ref,
        }));
    }

    if lineage.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::to_string(&lineage)?))
    }
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

    fn runtime_mailbox_body_path(&self, mailbox_ref: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join("mailboxes")
            .join(format!("{mailbox_ref}.json"))
    }

    fn runtime_handoff_envelope_path(&self, handoff_ref: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join("handoffs")
            .join(format!("{handoff_ref}.json"))
    }

    fn runtime_workflow_state_path(&self, workflow_run_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join("workflows")
            .join(format!("{workflow_run_id}.json"))
    }

    fn runtime_background_state_path(&self, run_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join("background")
            .join(format!("{run_id}.json"))
    }

    pub(super) fn runtime_memory_body_path(&self, memory_id: &str) -> PathBuf {
        self.state
            .paths
            .knowledge_dir
            .join("runtime-memory")
            .join(format!("{memory_id}.json"))
    }

    fn runtime_memory_proposal_artifact_path(&self, proposal_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join("memory-proposals")
            .join(format!("{proposal_id}.json"))
    }

    pub(super) fn runtime_mediation_checkpoint_path(
        &self,
        session_id: &str,
        run_id: &str,
        mediation_id: &str,
    ) -> PathBuf {
        self.state
            .paths
            .runtime_mediation_checkpoints_dir
            .join(session_id)
            .join(run_id)
            .join(format!("{mediation_id}.json"))
    }

    pub(super) fn runtime_mediation_checkpoint_ref(
        &self,
        session_id: &str,
        run_id: &str,
        mediation_id: &str,
    ) -> String {
        self.relative_storage_path(&self.runtime_mediation_checkpoint_path(
            session_id,
            run_id,
            mediation_id,
        ))
    }

    pub(super) fn persist_runtime_mediation_checkpoint(
        &self,
        session_id: &str,
        run_id: &str,
        mediation_id: &str,
        checkpoint: &RuntimeRunCheckpoint,
    ) -> Result<(String, String), AppError> {
        self.persist_runtime_artifact(
            self.runtime_mediation_checkpoint_path(session_id, run_id, mediation_id),
            checkpoint,
        )
    }

    fn relative_storage_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.state.paths.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn persist_runtime_artifact<T: Serialize>(
        &self,
        path: PathBuf,
        value: &T,
    ) -> Result<(String, String), AppError> {
        let payload = serde_json::to_vec_pretty(value)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &payload)?;
        Ok((self.relative_storage_path(&path), hash_bytes(&payload)))
    }

    pub(super) fn load_runtime_artifact<T: DeserializeOwned>(
        &self,
        storage_path: Option<&str>,
    ) -> Result<Option<T>, AppError> {
        let Some(storage_path) = storage_path.filter(|value| !value.trim().is_empty()) else {
            return Ok(None);
        };
        let path = self.state.paths.root.join(storage_path);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read(path)?;
        Ok(Some(serde_json::from_slice(&raw)?))
    }

    fn build_worker_dispatch_summary(
        &self,
        subruns: &[RuntimeSubrunSummary],
    ) -> RuntimeWorkerDispatchSummary {
        RuntimeWorkerDispatchSummary {
            total_subruns: subruns.len() as u64,
            active_subruns: subruns
                .iter()
                .filter(|subrun| subrun.status == "running")
                .count() as u64,
            completed_subruns: subruns
                .iter()
                .filter(|subrun| subrun.status == "completed")
                .count() as u64,
            failed_subruns: subruns
                .iter()
                .filter(|subrun| subrun.status == "failed")
                .count() as u64,
        }
    }

    fn hydrate_phase_four_runtime_projection(
        &self,
        connection: &Connection,
        detail: &mut RuntimeSessionDetail,
    ) -> Result<(), AppError> {
        let mut subrun_stmt = connection
            .prepare(
                "SELECT summary_json
                 FROM runtime_subrun_projections
                 WHERE session_id = ?1 AND parent_run_id = ?2
                 ORDER BY started_at ASC, run_id ASC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let subruns = subrun_stmt
            .query_map(params![detail.summary.id, detail.run.id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| AppError::database(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))?
            .into_iter()
            .map(|raw| serde_json::from_str::<RuntimeSubrunSummary>(&raw))
            .collect::<Result<Vec<_>, _>>()?;

        let mut handoff_stmt = connection
            .prepare(
                "SELECT summary_json
                 FROM runtime_handoff_projections
                 WHERE session_id = ?1 AND run_id = ?2
                 ORDER BY updated_at ASC, handoff_ref ASC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let handoffs = handoff_stmt
            .query_map(params![detail.summary.id, detail.run.id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| AppError::database(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))?
            .into_iter()
            .map(|raw| serde_json::from_str::<RuntimeHandoffSummary>(&raw))
            .collect::<Result<Vec<_>, _>>()?;

        let mailbox_projection: Option<(String, Option<String>)> = connection
            .query_row(
                "SELECT summary_json, body_storage_path
                 FROM runtime_mailbox_projections
                 WHERE session_id = ?1 AND run_id = ?2
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![detail.summary.id, detail.run.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?;
        let mailbox = if let Some((summary_json, body_storage_path)) = mailbox_projection {
            match serde_json::from_str::<RuntimeMailboxSummary>(&summary_json) {
                Ok(summary) => Some(summary),
                Err(_) => self
                    .load_runtime_artifact::<PersistedMailboxBody>(body_storage_path.as_deref())?
                    .map(|record| record.summary),
            }
        } else {
            None
        };

        let workflow_projection: Option<(String, String, Option<String>)> = connection
            .query_row(
                "SELECT summary_json, detail_json, detail_storage_path
                 FROM runtime_workflow_projections
                 WHERE session_id = ?1 AND run_id = ?2
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![detail.summary.id, detail.run.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?;
        let (workflow, workflow_detail) =
            if let Some((summary_json, detail_json, detail_storage_path)) = workflow_projection {
                let summary = serde_json::from_str::<RuntimeWorkflowSummary>(&summary_json).ok();
                let detail_value =
                    serde_json::from_str::<RuntimeWorkflowRunDetail>(&detail_json).ok();
                if let (Some(summary), Some(detail_value)) = (summary, detail_value) {
                    (Some(summary), Some(detail_value))
                } else if let Some(record) = self.load_runtime_artifact::<PersistedWorkflowState>(
                    detail_storage_path.as_deref(),
                )? {
                    (Some(record.summary), Some(record.detail))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

        let background_projection: Option<(String, Option<String>)> = connection
            .query_row(
                "SELECT summary_json, state_storage_path
                 FROM runtime_background_projections
                 WHERE session_id = ?1 AND run_id = ?2
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![detail.summary.id, detail.run.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?;
        let background = if let Some((summary_json, state_storage_path)) = background_projection {
            match serde_json::from_str::<RuntimeBackgroundRunSummary>(&summary_json) {
                Ok(summary) => Some(summary),
                Err(_) => self
                    .load_runtime_artifact::<PersistedBackgroundState>(
                        state_storage_path.as_deref(),
                    )?
                    .map(|record| record.summary),
            }
        } else {
            None
        };

        if !subruns.is_empty() {
            detail.subruns = subruns;
            detail.subrun_count = detail.subruns.len() as u64;
        }
        if !handoffs.is_empty() {
            detail.handoffs = handoffs;
        }
        if let Some(workflow) = workflow {
            detail.workflow = Some(workflow.clone());
            detail.run.workflow_run = Some(workflow.workflow_run_id.clone());
            detail.summary.workflow = Some(workflow);
        }
        if let Some(workflow_detail) = workflow_detail {
            detail.run.workflow_run_detail = Some(workflow_detail);
        }
        if let Some(mailbox) = mailbox {
            detail.run.mailbox_ref = Some(mailbox.mailbox_ref.clone());
            detail.pending_mailbox = Some(mailbox.clone());
            detail.summary.pending_mailbox = Some(mailbox);
        }
        if let Some(background) = background {
            detail.run.background_state = Some(background.status.clone());
            detail.background_run = Some(background.clone());
            detail.summary.background_run = Some(background);
        }
        if !detail.subruns.is_empty() {
            detail.run.handoff_ref = detail
                .handoffs
                .first()
                .map(|handoff| handoff.handoff_ref.clone());
            detail.run.worker_dispatch = Some(self.build_worker_dispatch_summary(&detail.subruns));
        }
        sync_runtime_session_detail(detail);
        Ok(())
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
            self.hydrate_phase_four_runtime_projection(&connection, &mut detail)?;
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

    pub(super) fn load_runtime_memory_records(
        &self,
        project_id: &str,
    ) -> Result<Vec<memory_runtime::PersistedRuntimeMemoryRecord>, AppError> {
        let connection = self.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT memory_id, project_id, owner_ref, source_run_id, kind, scope, title, summary,
                        freshness_state, last_validated_at, proposal_state, storage_path,
                        content_hash, updated_at
                 FROM runtime_memory_records
                 WHERE project_id = ?1 OR project_id IS NULL
                 ORDER BY updated_at DESC, memory_id ASC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map(params![project_id], |row| {
                Ok(memory_runtime::PersistedRuntimeMemoryRecord {
                    memory_id: row.get(0)?,
                    project_id: row.get(1)?,
                    owner_ref: row.get(2)?,
                    source_run_id: row.get(3)?,
                    kind: row.get(4)?,
                    scope: row.get(5)?,
                    title: row.get(6)?,
                    summary: row.get(7)?,
                    freshness_state: row.get(8)?,
                    last_validated_at: row.get(9)?,
                    proposal_state: row.get(10)?,
                    storage_path: row.get(11)?,
                    content_hash: row.get(12)?,
                    updated_at: row.get::<_, i64>(13)? as u64,
                })
            })
            .map_err(|error| AppError::database(error.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))
    }

    pub(super) fn persist_runtime_memory_record(
        &self,
        record: &memory_runtime::PersistedRuntimeMemoryRecord,
        body: &serde_json::Value,
    ) -> Result<(), AppError> {
        let connection = self.open_db()?;
        let (storage_path, content_hash) =
            self.persist_runtime_artifact(self.runtime_memory_body_path(&record.memory_id), body)?;
        connection
            .execute(
                "INSERT OR REPLACE INTO runtime_memory_records
                 (memory_id, workspace_id, project_id, owner_ref, source_run_id, kind, scope,
                  title, summary, freshness_state, last_validated_at, proposal_state, storage_path,
                  content_hash, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    record.memory_id,
                    self.state.workspace_id,
                    record.project_id,
                    record.owner_ref,
                    record.source_run_id,
                    record.kind,
                    record.scope,
                    record.title,
                    record.summary,
                    record.freshness_state,
                    record.last_validated_at.map(|value| value as i64),
                    record.proposal_state,
                    storage_path,
                    content_hash,
                    record.updated_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) fn persist_runtime_memory_proposal(
        &self,
        proposal: &RuntimeMemoryProposal,
        updated_at: u64,
    ) -> Result<(), AppError> {
        let connection = self.open_db()?;
        let artifact = memory_runtime::PersistedRuntimeMemoryProposalArtifact {
            proposal: proposal.clone(),
            updated_at,
        };
        let (artifact_storage_path, artifact_content_hash) = self.persist_runtime_artifact(
            self.runtime_memory_proposal_artifact_path(&proposal.proposal_id),
            &artifact,
        )?;
        connection
            .execute(
                "INSERT OR REPLACE INTO runtime_memory_proposals
                 (proposal_id, session_id, run_id, memory_id, kind, scope, title, summary,
                  proposal_state, proposal_reason, review_json, artifact_storage_path,
                  artifact_content_hash, updated_at, proposal_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    proposal.proposal_id,
                    proposal.session_id,
                    proposal.source_run_id,
                    proposal.memory_id,
                    proposal.kind,
                    proposal.scope,
                    proposal.title,
                    proposal.summary,
                    proposal.proposal_state,
                    proposal.proposal_reason,
                    proposal
                        .review
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?,
                    artifact_storage_path,
                    artifact_content_hash,
                    updated_at as i64,
                    serde_json::to_string(proposal)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
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
        let (
            pending_mediation_kind,
            pending_target_kind,
            pending_target_ref,
            pending_approval_layer,
            pending_provider_key,
            pending_checkpoint_ref,
        ) = pending_mediation_projection_fields(summary.pending_mediation.as_ref());
        let last_execution_outcome_json = summary
            .last_execution_outcome
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let last_mediation_outcome_json = run
            .last_mediation_outcome
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let (
            last_mediation_outcome,
            last_mediation_target_kind,
            last_mediation_target_ref,
            last_mediation_at,
        ) = last_mediation_projection_fields(run.last_mediation_outcome.as_ref());
        let session_auth_challenge_state = auth_challenge_state(run);
        let session_approval_lineage_json = approval_lineage_json(
            aggregate.detail.pending_approval.as_ref(),
            run.auth_target
                .as_ref()
                .or(run.checkpoint.pending_auth_challenge.as_ref()),
            run.last_mediation_outcome.as_ref(),
        )?;
        let denied_exposure_count =
            aggregate.detail.policy_decision_summary.denied_exposure_count as i64;
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
        let (
            run_pending_mediation_kind,
            run_pending_target_kind,
            run_pending_target_ref,
            run_pending_approval_layer,
            run_pending_provider_key,
            run_pending_checkpoint_ref,
        ) = pending_mediation_projection_fields(run.pending_mediation.as_ref());
        let run_last_execution_outcome_json = run
            .last_execution_outcome
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let run_last_mediation_outcome_json = run
            .last_mediation_outcome
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let (
            run_last_mediation_outcome,
            run_last_mediation_target_kind,
            run_last_mediation_target_ref,
            run_last_mediation_at,
        ) = last_mediation_projection_fields(run.last_mediation_outcome.as_ref());
        let run_auth_challenge_state = auth_challenge_state(run);
        let run_approval_lineage_json = approval_lineage_json(
            aggregate.detail.pending_approval.as_ref(),
            run.auth_target
                .as_ref()
                .or(run.checkpoint.pending_auth_challenge.as_ref()),
            run.last_mediation_outcome.as_ref(),
        )?;
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
        let run_denied_exposure_count = aggregate.detail.policy_decision_summary.denied_exposure_count as i64;
        let workflow_run_id = summary
            .workflow
            .as_ref()
            .map(|workflow| workflow.workflow_run_id.clone());
        let workflow_status = summary
            .workflow
            .as_ref()
            .map(|workflow| workflow.status.clone());
        let workflow_total_steps = summary
            .workflow
            .as_ref()
            .map_or(0_i64, |workflow| workflow.total_steps as i64);
        let workflow_completed_steps = summary
            .workflow
            .as_ref()
            .map_or(0_i64, |workflow| workflow.completed_steps as i64);
        let workflow_current_step_id = summary
            .workflow
            .as_ref()
            .and_then(|workflow| workflow.current_step_id.clone());
        let workflow_current_step_label = summary
            .workflow
            .as_ref()
            .and_then(|workflow| workflow.current_step_label.clone());
        let workflow_background_capable = bool_to_sql(
            summary
                .workflow
                .as_ref()
                .is_some_and(|workflow| workflow.background_capable),
        );
        let pending_mailbox_ref = summary
            .pending_mailbox
            .as_ref()
            .map(|mailbox| mailbox.mailbox_ref.clone());
        let pending_mailbox_count = summary
            .pending_mailbox
            .as_ref()
            .map_or(0_i64, |mailbox| mailbox.pending_count as i64);
        let handoff_count = aggregate.detail.handoffs.len() as i64;
        let background_run_id = summary
            .background_run
            .as_ref()
            .map(|background| background.run_id.clone());
        let background_workflow_run_id = summary
            .background_run
            .as_ref()
            .and_then(|background| background.workflow_run_id.clone());
        let background_status = summary
            .background_run
            .as_ref()
            .map(|background| background.status.clone());
        let run_workflow_run_id = run.workflow_run.clone();
        let run_workflow_step_id = run
            .workflow_run_detail
            .as_ref()
            .and_then(|workflow| workflow.current_step_id.clone());
        let run_workflow_status = run
            .workflow_run_detail
            .as_ref()
            .map(|workflow| workflow.status.clone());
        let run_mailbox_ref = run.mailbox_ref.clone();
        let run_handoff_ref = run.handoff_ref.clone();
        let run_background_state = run.background_state.clone();
        let worker_dispatch_json = run
            .worker_dispatch
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let worker_total_subruns = run
            .worker_dispatch
            .as_ref()
            .map_or(0_i64, |dispatch| dispatch.total_subruns as i64);
        let worker_active_subruns = run
            .worker_dispatch
            .as_ref()
            .map_or(0_i64, |dispatch| dispatch.active_subruns as i64);
        let worker_completed_subruns = run
            .worker_dispatch
            .as_ref()
            .map_or(0_i64, |dispatch| dispatch.completed_subruns as i64);
        let worker_failed_subruns = run
            .worker_dispatch
            .as_ref()
            .map_or(0_i64, |dispatch| dispatch.failed_subruns as i64);
        let workflow_run_detail_json = run
            .workflow_run_detail
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        connection
            .execute(
                "INSERT OR REPLACE INTO runtime_session_projections
                 (id, conversation_id, project_id, title, session_kind, status, updated_at, last_message_preview,
                  config_snapshot_id, effective_config_hash, started_from_scope_set, selected_actor_ref,
                  manifest_revision, active_run_id, subrun_count, workflow_run_id, workflow_status,
                 workflow_total_steps, workflow_completed_steps, workflow_current_step_id,
                  workflow_current_step_label, workflow_background_capable, pending_mailbox_ref,
                  pending_mailbox_count, handoff_count, background_run_id,
                  background_workflow_run_id, background_status, manifest_snapshot_ref,
                  session_policy_snapshot_ref, capability_plan_summary_json, provider_state_summary_json,
                  pending_mediation_json, pending_mediation_kind, pending_target_kind,
                  pending_target_ref, pending_approval_layer, pending_provider_key,
                  pending_checkpoint_ref, capability_state_ref, last_execution_outcome_json,
                  last_mediation_outcome_json, last_mediation_outcome, last_mediation_target_kind,
                  last_mediation_target_ref, last_mediation_at, auth_challenge_state,
                  approval_lineage_json, denied_exposure_count, granted_tool_count,
                  injected_skill_message_count, deferred_capability_count, hidden_capability_count,
                  degraded_provider_count, detail_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55)",
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
                    workflow_run_id,
                    workflow_status,
                    workflow_total_steps,
                    workflow_completed_steps,
                    workflow_current_step_id,
                    workflow_current_step_label,
                    workflow_background_capable,
                    pending_mailbox_ref,
                    pending_mailbox_count,
                    handoff_count,
                    background_run_id,
                    background_workflow_run_id,
                    background_status,
                    aggregate.metadata.manifest_snapshot_ref,
                    aggregate.metadata.session_policy_snapshot_ref,
                    capability_plan_summary_json,
                    provider_state_summary_json,
                    pending_mediation_json,
                    pending_mediation_kind,
                    pending_target_kind,
                    pending_target_ref,
                    pending_approval_layer,
                    pending_provider_key,
                    pending_checkpoint_ref,
                    summary.capability_state_ref,
                    last_execution_outcome_json,
                    last_mediation_outcome_json,
                    last_mediation_outcome,
                    last_mediation_target_kind,
                    last_mediation_target_ref,
                    last_mediation_at,
                    session_auth_challenge_state,
                    session_approval_lineage_json,
                    denied_exposure_count,
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
                 workflow_run_id, workflow_step_id, workflow_status, mailbox_ref, handoff_ref,
                  background_state, worker_total_subruns, worker_active_subruns,
                  worker_completed_subruns, worker_failed_subruns, worker_dispatch_json,
                  workflow_run_detail_json, approval_state, trace_id, turn_id, capability_plan_summary_json,
                  provider_state_summary_json, pending_mediation_json, pending_mediation_kind,
                  pending_target_kind, pending_target_ref, pending_approval_layer,
                  pending_provider_key, pending_checkpoint_ref, capability_state_ref,
                  last_execution_outcome_json, last_mediation_outcome_json, last_mediation_outcome,
                  last_mediation_target_kind, last_mediation_target_ref, last_mediation_at,
                  auth_challenge_state, approval_lineage_json, denied_exposure_count,
                  granted_tool_count, injected_skill_message_count, deferred_capability_count,
                  hidden_capability_count, degraded_provider_count, run_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56)",
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
                    run_workflow_run_id,
                    run_workflow_step_id,
                    run_workflow_status,
                    run_mailbox_ref,
                    run_handoff_ref,
                    run_background_state,
                    worker_total_subruns,
                    worker_active_subruns,
                    worker_completed_subruns,
                    worker_failed_subruns,
                    worker_dispatch_json,
                    workflow_run_detail_json,
                    run.approval_state,
                    run.trace_context.trace_id,
                    run.trace_context.turn_id,
                    run_capability_plan_summary_json,
                    run_provider_state_summary_json,
                    run_pending_mediation_json,
                    run_pending_mediation_kind,
                    run_pending_target_kind,
                    run_pending_target_ref,
                    run_pending_approval_layer,
                    run_pending_provider_key,
                    run_pending_checkpoint_ref,
                    run.capability_state_ref,
                    run_last_execution_outcome_json,
                    run_last_mediation_outcome_json,
                    run_last_mediation_outcome,
                    run_last_mediation_target_kind,
                    run_last_mediation_target_ref,
                    run_last_mediation_at,
                    run_auth_challenge_state,
                    run_approval_lineage_json,
                    run_denied_exposure_count,
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
                "DELETE FROM runtime_subrun_projections WHERE session_id = ?1 AND parent_run_id = ?2",
                params![summary.id, run.id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        for subrun in &aggregate.detail.subruns {
            connection
                .execute(
                    "INSERT OR REPLACE INTO runtime_subrun_projections
                     (run_id, session_id, conversation_id, parent_run_id, actor_ref, label, status,
                      run_kind, delegated_by_tool_call_id, workflow_run_id, mailbox_ref, handoff_ref,
                      started_at, updated_at, summary_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    params![
                        subrun.run_id,
                        summary.id,
                        summary.conversation_id,
                        subrun.parent_run_id,
                        subrun.actor_ref,
                        subrun.label,
                        subrun.status,
                        subrun.run_kind,
                        subrun.delegated_by_tool_call_id,
                        subrun.workflow_run_id,
                        subrun.mailbox_ref,
                        subrun.handoff_ref,
                        subrun.started_at as i64,
                        subrun.updated_at as i64,
                        serde_json::to_string(subrun)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        connection
            .execute(
                "DELETE FROM runtime_mailbox_projections WHERE session_id = ?1 AND run_id = ?2",
                params![summary.id, run.id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        if let Some(mailbox) = aggregate.detail.pending_mailbox.as_ref() {
            let mailbox_body = PersistedMailboxBody {
                session_id: summary.id.clone(),
                run_id: run.id.clone(),
                conversation_id: summary.conversation_id.clone(),
                summary: mailbox.clone(),
                handoff_refs: aggregate
                    .detail
                    .handoffs
                    .iter()
                    .map(|handoff| handoff.handoff_ref.clone())
                    .collect(),
            };
            let (body_storage_path, body_content_hash) = self.persist_runtime_artifact(
                self.runtime_mailbox_body_path(&mailbox.mailbox_ref),
                &mailbox_body,
            )?;
            connection
                .execute(
                    "INSERT OR REPLACE INTO runtime_mailbox_projections
                     (mailbox_ref, session_id, run_id, conversation_id, channel, status, pending_count,
                      total_messages, latest_handoff_ref, body_storage_path, body_content_hash,
                      updated_at, summary_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        mailbox.mailbox_ref,
                        summary.id,
                        run.id,
                        summary.conversation_id,
                        mailbox.channel,
                        mailbox.status,
                        mailbox.pending_count as i64,
                        mailbox.total_messages as i64,
                        aggregate.detail.handoffs.last().map(|handoff| handoff.handoff_ref.clone()),
                        body_storage_path,
                        body_content_hash,
                        mailbox.updated_at as i64,
                        serde_json::to_string(mailbox)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        connection
            .execute(
                "DELETE FROM runtime_handoff_projections WHERE session_id = ?1 AND run_id = ?2",
                params![summary.id, run.id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        for handoff in &aggregate.detail.handoffs {
            let envelope = PersistedHandoffEnvelope {
                session_id: summary.id.clone(),
                run_id: run.id.clone(),
                conversation_id: summary.conversation_id.clone(),
                parent_run_id: run.parent_run_id.clone(),
                delegated_by_tool_call_id: run.delegated_by_tool_call_id.clone(),
                sender_actor_ref: handoff.sender_actor_ref.clone(),
                receiver_actor_ref: handoff.receiver_actor_ref.clone(),
                mailbox_ref: handoff.mailbox_ref.clone(),
                artifact_refs: handoff.artifact_refs.clone(),
                handoff_state: handoff.state.clone(),
                updated_at: handoff.updated_at,
            };
            let (envelope_storage_path, envelope_content_hash) = self.persist_runtime_artifact(
                self.runtime_handoff_envelope_path(&handoff.handoff_ref),
                &envelope,
            )?;
            connection
                .execute(
                    "INSERT OR REPLACE INTO runtime_handoff_projections
                     (handoff_ref, session_id, run_id, conversation_id, parent_run_id,
                      delegated_by_tool_call_id, sender_actor_ref, receiver_actor_ref, mailbox_ref,
                      state, artifact_refs_json, envelope_storage_path, envelope_content_hash,
                      updated_at, summary_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    params![
                        handoff.handoff_ref,
                        summary.id,
                        run.id,
                        summary.conversation_id,
                        run.parent_run_id,
                        run.delegated_by_tool_call_id,
                        handoff.sender_actor_ref,
                        handoff.receiver_actor_ref,
                        handoff.mailbox_ref,
                        handoff.state,
                        serde_json::to_string(&handoff.artifact_refs)?,
                        envelope_storage_path,
                        envelope_content_hash,
                        handoff.updated_at as i64,
                        serde_json::to_string(handoff)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        connection
            .execute(
                "DELETE FROM runtime_workflow_projections WHERE session_id = ?1 AND run_id = ?2",
                params![summary.id, run.id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        if let (Some(workflow), Some(workflow_detail)) = (
            aggregate.detail.workflow.as_ref(),
            run.workflow_run_detail.as_ref(),
        ) {
            let workflow_state = PersistedWorkflowState {
                summary: workflow.clone(),
                detail: workflow_detail.clone(),
            };
            let (detail_storage_path, detail_content_hash) = self.persist_runtime_artifact(
                self.runtime_workflow_state_path(&workflow.workflow_run_id),
                &workflow_state,
            )?;
            connection
                .execute(
                    "INSERT OR REPLACE INTO runtime_workflow_projections
                     (workflow_run_id, session_id, run_id, conversation_id, label, status,
                      total_steps, completed_steps, current_step_id, current_step_label,
                      background_capable, detail_storage_path, detail_content_hash,
                      updated_at, summary_json, detail_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        workflow.workflow_run_id,
                        summary.id,
                        run.id,
                        summary.conversation_id,
                        workflow.label,
                        workflow.status,
                        workflow.total_steps as i64,
                        workflow.completed_steps as i64,
                        workflow.current_step_id,
                        workflow.current_step_label,
                        bool_to_sql(workflow.background_capable),
                        detail_storage_path,
                        detail_content_hash,
                        workflow.updated_at as i64,
                        serde_json::to_string(workflow)?,
                        serde_json::to_string(workflow_detail)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        connection
            .execute(
                "DELETE FROM runtime_background_projections WHERE session_id = ?1 AND run_id = ?2",
                params![summary.id, run.id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        if let Some(background) = aggregate.detail.background_run.as_ref() {
            let background_state = PersistedBackgroundState {
                summary: background.clone(),
            };
            let (state_storage_path, state_content_hash) = self.persist_runtime_artifact(
                self.runtime_background_state_path(&background.run_id),
                &background_state,
            )?;
            connection
                .execute(
                    "INSERT OR REPLACE INTO runtime_background_projections
                     (run_id, session_id, conversation_id, workflow_run_id, status,
                      background_capable, state_storage_path, state_content_hash, updated_at, summary_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        background.run_id,
                        summary.id,
                        summary.conversation_id,
                        background.workflow_run_id,
                        background.status,
                        bool_to_sql(background.background_capable),
                        state_storage_path,
                        state_content_hash,
                        background.updated_at as i64,
                        serde_json::to_string(background)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

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
                      risk_level, created_at, status, approval_layer, capability_id,
                      checkpoint_ref, provider_key, required_permission, requires_approval,
                      requires_auth, target_kind, target_ref, escalation_reason, approval_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
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
                        approval.approval_layer,
                        approval.capability_id,
                        approval.checkpoint_ref,
                        approval.provider_key,
                        approval.required_permission,
                        bool_to_sql(approval.requires_approval),
                        bool_to_sql(approval.requires_auth),
                        approval.target_kind,
                        approval.target_ref,
                        approval.escalation_reason,
                        serde_json::to_string(approval)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        connection
            .execute(
                "DELETE FROM runtime_auth_challenge_projections WHERE session_id = ?1",
                [summary.id.as_str()],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if let Some(challenge) = run
            .auth_target
            .as_ref()
            .or(run.checkpoint.pending_auth_challenge.as_ref())
        {
            connection
                .execute(
                    "INSERT OR REPLACE INTO runtime_auth_challenge_projections
                     (id, session_id, run_id, conversation_id, summary, detail, status,
                      resolution, created_at, updated_at, approval_layer, capability_id,
                      checkpoint_ref, provider_key, required_permission, requires_approval,
                      requires_auth, target_kind, target_ref, escalation_reason, challenge_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                    params![
                        challenge.id,
                        challenge.session_id,
                        challenge.run_id,
                        challenge.conversation_id,
                        challenge.summary,
                        challenge.detail,
                        challenge.status,
                        challenge.resolution,
                        challenge.created_at as i64,
                        run.updated_at as i64,
                        challenge.approval_layer,
                        challenge.capability_id,
                        challenge.checkpoint_ref,
                        challenge.provider_key,
                        challenge.required_permission,
                        bool_to_sql(challenge.requires_approval),
                        bool_to_sql(challenge.requires_auth),
                        challenge.target_kind,
                        challenge.target_ref,
                        challenge.escalation_reason,
                        serde_json::to_string(challenge)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }

        connection
            .execute(
                "DELETE FROM runtime_memory_proposals WHERE session_id = ?1",
                [summary.id.as_str()],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        if let Some(proposal) = run.pending_memory_proposal.as_ref() {
            self.persist_runtime_memory_proposal(proposal, run.updated_at)?;
        }

        Ok(())
    }
}
