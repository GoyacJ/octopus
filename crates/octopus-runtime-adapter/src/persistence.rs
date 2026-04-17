use super::*;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

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
    #[serde(default)]
    handoffs: Vec<PersistedMailboxHandoffRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedMailboxHandoffRecord {
    handoff_ref: String,
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
struct PersistedHandoffEnvelope {
    #[serde(default)]
    handoff_ref: String,
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

impl PersistedHandoffEnvelope {
    fn into_summary(self, fallback_handoff_ref: &str) -> RuntimeHandoffSummary {
        RuntimeHandoffSummary {
            handoff_ref: if self.handoff_ref.trim().is_empty() {
                fallback_handoff_ref.to_string()
            } else {
                self.handoff_ref
            },
            mailbox_ref: self.mailbox_ref,
            sender_actor_ref: self.sender_actor_ref,
            receiver_actor_ref: self.receiver_actor_ref,
            state: self.handoff_state,
            artifact_refs: self.artifact_refs,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedWorkflowState {
    session_id: String,
    run_id: String,
    conversation_id: String,
    parent_run_id: Option<String>,
    mailbox_ref: Option<String>,
    summary: RuntimeWorkflowSummary,
    detail: RuntimeWorkflowRunDetail,
    background: RuntimeBackgroundRunSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedBackgroundState {
    session_id: String,
    run_id: String,
    conversation_id: String,
    parent_run_id: Option<String>,
    workflow_run_id: Option<String>,
    summary: RuntimeBackgroundRunSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedRuntimeOutputArtifact {
    artifact_ref: String,
    session_id: String,
    conversation_id: String,
    run_id: String,
    parent_run_id: Option<String>,
    delegated_by_tool_call_id: Option<String>,
    actor_ref: String,
    workflow_run_id: Option<String>,
    checkpoint_artifact_ref: Option<String>,
    serialized_session: Value,
    usage_summary: RuntimeUsageSummary,
    updated_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedRuntimeCheckpointArtifact {
    #[serde(flatten)]
    pub(super) checkpoint: RuntimeRunCheckpoint,
    #[serde(default)]
    pub(super) serialized_session: Value,
    #[serde(default)]
    pub(super) compaction_metadata: Value,
}

impl PersistedRuntimeCheckpointArtifact {
    pub(super) fn from_public_checkpoint(
        checkpoint: RuntimeRunCheckpoint,
        serialized_session: Value,
        compaction_metadata: Value,
    ) -> Self {
        Self {
            checkpoint,
            serialized_session,
            compaction_metadata,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeArtifactProjectionRow {
    artifact_ref: String,
    session_id: String,
    conversation_id: String,
    run_id: String,
    parent_run_id: Option<String>,
    delegated_by_tool_call_id: Option<String>,
    actor_ref: String,
    workflow_run_id: Option<String>,
    storage_path: String,
    content_hash: String,
    byte_size: u64,
    content_type: String,
    updated_at: u64,
}

pub(super) fn runtime_output_artifact_ref(run_id: &str) -> String {
    format!("runtime-artifact-{run_id}")
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

fn deliverable_title_from_text(content: &str, fallback: &str) -> String {
    let candidate = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.trim_start_matches('#').trim())
        .filter(|line| !line.is_empty())
        .unwrap_or(fallback);
    let mut title = candidate.chars().take(80).collect::<String>();
    if title.is_empty() {
        title = fallback.to_string();
    }
    title
}

fn deliverable_summary_from_text(content: &str, fallback: &str) -> String {
    let collapsed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return fallback.to_string();
    }
    let mut summary = collapsed.chars().take(180).collect::<String>();
    if collapsed.chars().count() > 180 {
        summary.push_str("...");
    }
    summary
}

fn deliverable_content_bytes(content: &DeliverableVersionContent) -> Result<Vec<u8>, AppError> {
    if let Some(text_content) = content.text_content.as_ref() {
        return Ok(text_content.as_bytes().to_vec());
    }
    if let Some(data_base64) = content.data_base64.as_ref() {
        return BASE64_STANDARD
            .decode(data_base64.as_bytes())
            .map_err(|error: base64::DecodeError| AppError::invalid_input(error.to_string()));
    }
    Ok(Vec::new())
}

fn deliverable_file_extension(preview_kind: &str, content_type: Option<&str>) -> &'static str {
    match (preview_kind, content_type.unwrap_or_default()) {
        ("markdown", _) => "md",
        ("json", _) | (_, "application/json") => "json",
        (_, "text/plain") => "txt",
        (_, "text/html") => "html",
        (_, "image/png") => "png",
        (_, "image/jpeg") => "jpg",
        (_, "image/webp") => "webp",
        _ => "bin",
    }
}

fn infer_deliverable_content_type(
    preview_kind: &str,
    explicit_content_type: Option<&str>,
    text_content: Option<&str>,
    data_base64: Option<&str>,
) -> Option<String> {
    explicit_content_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            if text_content.is_some() {
                Some(match preview_kind {
                    "markdown" => "text/markdown",
                    "json" => "application/json",
                    _ => "text/plain",
                })
            } else if data_base64.is_some() {
                Some("application/octet-stream")
            } else {
                None
            }
            .map(str::to_string)
        })
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
) -> (Option<String>, Option<String>, Option<String>, Option<i64>) {
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
    fn purge_legacy_runtime_session_projection(
        &self,
        connection: &Connection,
        session_id: &str,
        error: &serde_json::Error,
    ) -> Result<(), AppError> {
        eprintln!("dropping legacy runtime projection {session_id}: {error}");

        for statement in [
            "DELETE FROM runtime_session_projections WHERE id = ?1",
            "DELETE FROM runtime_run_projections WHERE session_id = ?1",
            "DELETE FROM runtime_subrun_projections WHERE session_id = ?1",
            "DELETE FROM runtime_artifact_projections WHERE session_id = ?1",
            "DELETE FROM runtime_mailbox_projections WHERE session_id = ?1",
            "DELETE FROM runtime_handoff_projections WHERE session_id = ?1",
            "DELETE FROM runtime_workflow_projections WHERE session_id = ?1",
            "DELETE FROM runtime_background_projections WHERE session_id = ?1",
            "DELETE FROM runtime_approval_projections WHERE session_id = ?1",
            "DELETE FROM runtime_auth_challenge_projections WHERE session_id = ?1",
            "DELETE FROM runtime_memory_proposals WHERE session_id = ?1",
        ] {
            connection
                .execute(statement, [session_id])
                .map_err(|db_error| AppError::database(db_error.to_string()))?;
        }

        let _ = fs::remove_file(self.runtime_events_path(session_id));
        Ok(())
    }

    pub(super) fn runtime_events_path(&self, session_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_events_dir
            .join(format!("{session_id}.jsonl"))
    }

    fn runtime_mailbox_body_path(&self, mailbox_ref: &str) -> PathBuf {
        self.state
            .paths
            .runtime_state_dir
            .join("mailboxes")
            .join(format!("{mailbox_ref}.json"))
    }

    fn runtime_handoff_envelope_path(&self, handoff_ref: &str) -> PathBuf {
        self.state
            .paths
            .runtime_state_dir
            .join("handoffs")
            .join(format!("{handoff_ref}.json"))
    }

    pub(super) fn runtime_subrun_state_path(&self, run_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_state_dir
            .join("subruns")
            .join(format!("{run_id}.json"))
    }

    fn runtime_workflow_state_path(&self, workflow_run_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_state_dir
            .join("workflows")
            .join(format!("{workflow_run_id}.json"))
    }

    fn runtime_background_state_path(&self, run_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_state_dir
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
            .runtime_state_dir
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
        checkpoint: &PersistedRuntimeCheckpointArtifact,
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
        let (storage_path, content_hash, _) = self.persist_runtime_payload(path, &payload)?;
        Ok((storage_path, content_hash))
    }

    fn persist_runtime_payload(
        &self,
        path: PathBuf,
        payload: &[u8],
    ) -> Result<(String, String, u64), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, payload)?;
        Ok((
            self.relative_storage_path(&path),
            hash_bytes(payload),
            payload.len() as u64,
        ))
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

    fn load_runtime_output_artifact(
        &self,
        artifact_ref: Option<&str>,
    ) -> Result<Option<PersistedRuntimeOutputArtifact>, AppError> {
        let Some(artifact_ref) = artifact_ref.filter(|value| !value.trim().is_empty()) else {
            return Ok(None);
        };
        let path = self.runtime_output_artifact_path(artifact_ref);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read(path)?;
        Ok(Some(serde_json::from_slice(&raw)?))
    }

    fn load_primary_run_serialized_session(
        &self,
        detail: &RuntimeSessionDetail,
    ) -> Result<Value, AppError> {
        if let Some(checkpoint) = self.load_runtime_artifact::<PersistedRuntimeCheckpointArtifact>(
            detail.run.checkpoint.checkpoint_artifact_ref.as_deref(),
        )? {
            return Ok(checkpoint.serialized_session);
        }

        if let Some(output_artifact) =
            self.load_runtime_output_artifact(detail.run.artifact_refs.first().map(String::as_str))?
        {
            return Ok(output_artifact.serialized_session);
        }

        Ok(json!({}))
    }

    fn runtime_output_artifact_path(&self, artifact_ref: &str) -> PathBuf {
        self.state
            .paths
            .artifacts_dir
            .join("runtime")
            .join(format!("{artifact_ref}.json"))
    }

    fn deliverable_version_body_path(&self, artifact_id: &str, version: u32) -> PathBuf {
        self.state
            .paths
            .artifacts_dir
            .join("deliverables")
            .join(artifact_id)
            .join(format!("v{version}.json"))
    }

    fn deliverable_detail_from_connection(
        &self,
        connection: &Connection,
        artifact_id: &str,
    ) -> Result<Option<DeliverableDetail>, AppError> {
        connection
            .query_row(
                "SELECT id, workspace_id, project_id, conversation_id, session_id, run_id,
                        source_message_id, parent_artifact_id, title, status, preview_kind,
                        latest_version, promotion_state, promotion_knowledge_id, updated_at,
                        storage_path, content_hash, byte_size, content_type
                 FROM artifact_records
                 WHERE id = ?1",
                [artifact_id],
                |row| {
                    let id = row.get::<_, String>(0)?;
                    let title = row.get::<_, String>(8)?;
                    let preview_kind = row.get::<_, String>(10)?;
                    let latest_version = row.get::<_, i64>(11)?.max(0) as u32;
                    let updated_at = row.get::<_, i64>(14)?.max(0) as u64;
                    let content_type = row.get::<_, Option<String>>(18)?;
                    Ok(DeliverableDetail {
                        id: id.clone(),
                        workspace_id: row.get(1)?,
                        project_id: row.get(2)?,
                        conversation_id: row.get(3)?,
                        session_id: row.get(4)?,
                        run_id: row.get(5)?,
                        source_message_id: row.get(6)?,
                        parent_artifact_id: row.get(7)?,
                        title: title.clone(),
                        status: row.get(9)?,
                        preview_kind: preview_kind.clone(),
                        latest_version,
                        latest_version_ref: ArtifactVersionReference {
                            artifact_id: id,
                            version: latest_version,
                            title,
                            preview_kind,
                            updated_at,
                            content_type: content_type.clone(),
                        },
                        promotion_state: row.get(12)?,
                        promotion_knowledge_id: row.get(13)?,
                        updated_at,
                        storage_path: row.get(15)?,
                        content_hash: row.get(16)?,
                        byte_size: row
                            .get::<_, Option<i64>>(17)?
                            .map(|value| value.max(0) as u64),
                        content_type,
                    })
                },
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))
    }

    fn deliverable_version_exists(
        &self,
        connection: &Connection,
        artifact_id: &str,
        version: u32,
    ) -> Result<bool, AppError> {
        connection
            .query_row(
                "SELECT 1
                 FROM artifact_versions
                 WHERE artifact_id = ?1 AND version = ?2",
                params![artifact_id, i64::from(version)],
                |_row| Ok(()),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))
            .map(|value| value.is_some())
    }

    fn next_deliverable_version(
        &self,
        connection: &Connection,
        artifact_id: &str,
    ) -> Result<u32, AppError> {
        let latest = connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0)
                 FROM artifact_versions
                 WHERE artifact_id = ?1",
                [artifact_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(latest.max(0) as u32 + 1)
    }

    fn persist_deliverable_version_body(
        &self,
        content: &DeliverableVersionContent,
    ) -> Result<(String, String, u64), AppError> {
        let path = self.deliverable_version_body_path(&content.artifact_id, content.version);
        let payload = serde_json::to_vec_pretty(content)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, payload)?;

        let content_bytes = deliverable_content_bytes(content)?;
        let byte_size = content.byte_size.unwrap_or(content_bytes.len() as u64);
        let content_hash = if content_bytes.is_empty() {
            hash_bytes(b"")
        } else {
            hash_bytes(&content_bytes)
        };

        Ok((self.relative_storage_path(&path), content_hash, byte_size))
    }

    fn insert_deliverable_version_row(
        &self,
        connection: &Connection,
        detail: &DeliverableDetail,
        version: u32,
        title: &str,
        preview_kind: &str,
        updated_at: u64,
        storage_path: &str,
        content_hash: &str,
        byte_size: u64,
        content_type: Option<&str>,
        source_message_id: Option<&str>,
        parent_version: Option<u32>,
    ) -> Result<(), AppError> {
        connection
            .execute(
                "INSERT OR REPLACE INTO artifact_versions
                 (artifact_id, version, workspace_id, project_id, conversation_id, session_id,
                  run_id, source_message_id, parent_version, title, preview_kind, updated_at,
                  storage_path, content_hash, byte_size, content_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    &detail.id,
                    i64::from(version),
                    &detail.workspace_id,
                    &detail.project_id,
                    &detail.conversation_id,
                    &detail.session_id,
                    &detail.run_id,
                    source_message_id,
                    parent_version.map(i64::from),
                    title,
                    preview_kind,
                    updated_at as i64,
                    storage_path,
                    content_hash,
                    byte_size as i64,
                    content_type,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    fn upsert_deliverable_record(
        &self,
        connection: &Connection,
        detail: &DeliverableDetail,
    ) -> Result<(), AppError> {
        connection
            .execute(
                "INSERT OR REPLACE INTO artifact_records
                 (id, workspace_id, project_id, conversation_id, session_id, run_id,
                  source_message_id, parent_artifact_id, title, status, preview_kind,
                  latest_version, promotion_state, promotion_knowledge_id, updated_at,
                  storage_path, content_hash, byte_size, content_type)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    &detail.id,
                    &detail.workspace_id,
                    &detail.project_id,
                    &detail.conversation_id,
                    &detail.session_id,
                    &detail.run_id,
                    detail.source_message_id.as_deref(),
                    detail.parent_artifact_id.as_deref(),
                    &detail.title,
                    &detail.status,
                    &detail.preview_kind,
                    i64::from(detail.latest_version),
                    &detail.promotion_state,
                    detail.promotion_knowledge_id.as_deref(),
                    detail.updated_at as i64,
                    detail.storage_path.as_deref(),
                    detail.content_hash.as_deref(),
                    detail.byte_size.map(|value| value as i64),
                    detail.content_type.as_deref(),
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    fn persist_pending_runtime_deliverable(
        &self,
        connection: &Connection,
        deliverable: &PendingRuntimeDeliverable,
    ) -> Result<(), AppError> {
        let detail = &deliverable.detail;
        if self.deliverable_version_exists(connection, &detail.id, detail.latest_version)? {
            return Ok(());
        }

        let mut content = deliverable.content.clone();
        if content.file_name.is_none() {
            content.file_name = Some(format!(
                "{}-v{}.{}",
                detail.id,
                detail.latest_version,
                deliverable_file_extension(&content.preview_kind, content.content_type.as_deref(),)
            ));
        }

        let content_bytes = deliverable_content_bytes(&content)?;
        content.byte_size = Some(content_bytes.len() as u64);
        let (storage_path, content_hash, byte_size) =
            self.persist_deliverable_version_body(&content)?;

        let persisted_detail = DeliverableDetail {
            storage_path: Some(storage_path.clone()),
            content_hash: Some(content_hash.clone()),
            byte_size: Some(byte_size),
            content_type: content.content_type.clone(),
            latest_version_ref: ArtifactVersionReference {
                artifact_id: detail.id.clone(),
                version: detail.latest_version,
                title: detail.title.clone(),
                preview_kind: detail.preview_kind.clone(),
                updated_at: detail.updated_at,
                content_type: content.content_type.clone(),
            },
            ..detail.clone()
        };

        self.insert_deliverable_version_row(
            connection,
            &persisted_detail,
            persisted_detail.latest_version,
            &persisted_detail.title,
            &persisted_detail.preview_kind,
            persisted_detail.updated_at,
            &storage_path,
            &content_hash,
            byte_size,
            content.content_type.as_deref(),
            deliverable.source_message_id.as_deref(),
            deliverable.parent_version,
        )?;
        self.upsert_deliverable_record(connection, &persisted_detail)?;
        Ok(())
    }

    pub(crate) fn get_deliverable_detail(
        &self,
        artifact_id: &str,
    ) -> Result<Option<DeliverableDetail>, AppError> {
        let connection = self.open_db()?;
        self.deliverable_detail_from_connection(&connection, artifact_id)
    }

    pub(crate) fn list_deliverable_versions(
        &self,
        artifact_id: &str,
    ) -> Result<Vec<DeliverableVersionSummary>, AppError> {
        let connection = self.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT artifact_id, version, title, preview_kind, updated_at, session_id, run_id,
                        source_message_id, parent_version, byte_size, content_hash, content_type
                 FROM artifact_versions
                 WHERE artifact_id = ?1
                 ORDER BY version DESC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([artifact_id], |row| {
                Ok(DeliverableVersionSummary {
                    artifact_id: row.get(0)?,
                    version: row.get::<_, i64>(1)?.max(0) as u32,
                    title: row.get(2)?,
                    preview_kind: row.get(3)?,
                    updated_at: row.get::<_, i64>(4)?.max(0) as u64,
                    session_id: row.get(5)?,
                    run_id: row.get(6)?,
                    source_message_id: row.get(7)?,
                    parent_version: row
                        .get::<_, Option<i64>>(8)?
                        .map(|value| value.max(0) as u32),
                    byte_size: row
                        .get::<_, Option<i64>>(9)?
                        .map(|value| value.max(0) as u64),
                    content_hash: row.get(10)?,
                    content_type: row.get(11)?,
                })
            })
            .map_err(|error| AppError::database(error.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))
    }

    pub(crate) fn get_deliverable_version_content(
        &self,
        artifact_id: &str,
        version: u32,
    ) -> Result<Option<DeliverableVersionContent>, AppError> {
        let connection = self.open_db()?;
        let storage_path = connection
            .query_row(
                "SELECT storage_path
                 FROM artifact_versions
                 WHERE artifact_id = ?1 AND version = ?2",
                params![artifact_id, i64::from(version)],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?;
        let Some(storage_path) = storage_path else {
            return Ok(None);
        };
        self.load_runtime_artifact::<DeliverableVersionContent>(Some(&storage_path))
    }

    pub(crate) async fn create_deliverable_version(
        &self,
        artifact_id: &str,
        input: CreateDeliverableVersionInput,
    ) -> Result<DeliverableDetail, AppError> {
        let connection = self.open_db()?;
        let existing = self
            .deliverable_detail_from_connection(&connection, artifact_id)?
            .ok_or_else(|| AppError::not_found(format!("deliverable `{artifact_id}`")))?;
        let version = self.next_deliverable_version(&connection, artifact_id)?;
        let title = input
            .title
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .or_else(|| {
                input
                    .text_content
                    .as_deref()
                    .map(|content| deliverable_title_from_text(content, &existing.title))
            })
            .unwrap_or_else(|| existing.title.clone());
        let preview_kind = if input.preview_kind.trim().is_empty() {
            existing.preview_kind.clone()
        } else {
            input.preview_kind.clone()
        };
        let content_type = infer_deliverable_content_type(
            &preview_kind,
            input.content_type.as_deref(),
            input.text_content.as_deref(),
            input.data_base64.as_deref(),
        )
        .or(existing.content_type.clone());
        let mut content = DeliverableVersionContent {
            artifact_id: artifact_id.to_string(),
            version,
            preview_kind: preview_kind.clone(),
            editable: input.data_base64.is_none(),
            file_name: Some(format!(
                "{artifact_id}-v{version}.{}",
                deliverable_file_extension(&preview_kind, content_type.as_deref())
            )),
            content_type: content_type.clone(),
            text_content: input.text_content.clone(),
            data_base64: input.data_base64.clone(),
            byte_size: None,
        };
        let content_bytes = deliverable_content_bytes(&content)?;
        content.byte_size = Some(content_bytes.len() as u64);
        let (storage_path, content_hash, byte_size) =
            self.persist_deliverable_version_body(&content)?;
        let updated_at = timestamp_now();
        let parent_version = input.parent_version.or(Some(existing.latest_version));
        let updated_detail = DeliverableDetail {
            id: existing.id.clone(),
            workspace_id: existing.workspace_id.clone(),
            project_id: existing.project_id.clone(),
            conversation_id: existing.conversation_id.clone(),
            session_id: existing.session_id.clone(),
            run_id: existing.run_id.clone(),
            source_message_id: input.source_message_id.clone(),
            parent_artifact_id: existing.parent_artifact_id.clone(),
            title: title.clone(),
            status: existing.status.clone(),
            preview_kind: preview_kind.clone(),
            latest_version: version,
            latest_version_ref: ArtifactVersionReference {
                artifact_id: existing.id.clone(),
                version,
                title: title.clone(),
                preview_kind: preview_kind.clone(),
                updated_at,
                content_type: content_type.clone(),
            },
            promotion_state: existing.promotion_state.clone(),
            promotion_knowledge_id: existing.promotion_knowledge_id.clone(),
            updated_at,
            storage_path: Some(storage_path.clone()),
            content_hash: Some(content_hash.clone()),
            byte_size: Some(byte_size),
            content_type: content_type.clone(),
        };

        self.insert_deliverable_version_row(
            &connection,
            &updated_detail,
            version,
            &title,
            &preview_kind,
            updated_at,
            &storage_path,
            &content_hash,
            byte_size,
            content_type.as_deref(),
            input.source_message_id.as_deref(),
            parent_version,
        )?;
        self.upsert_deliverable_record(&connection, &updated_detail)?;
        Ok(updated_detail)
    }

    pub(crate) async fn promote_deliverable(
        &self,
        artifact_id: &str,
        input: PromoteDeliverableInput,
    ) -> Result<DeliverableDetail, AppError> {
        let connection = self.open_db()?;
        let existing = self
            .deliverable_detail_from_connection(&connection, artifact_id)?
            .ok_or_else(|| AppError::not_found(format!("deliverable `{artifact_id}`")))?;
        let latest_content =
            self.get_deliverable_version_content(artifact_id, existing.latest_version)?;
        let knowledge_title = input
            .title
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| existing.title.clone());
        let derived_summary = latest_content
            .as_ref()
            .and_then(|content| content.text_content.as_deref())
            .map(|content| deliverable_summary_from_text(content, &knowledge_title))
            .unwrap_or_else(|| deliverable_summary_from_text(&existing.title, &knowledge_title));
        let knowledge_summary = input
            .summary
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .unwrap_or(derived_summary);
        let knowledge_kind = input
            .kind
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("shared")
            .to_string();
        let knowledge_id = existing
            .promotion_knowledge_id
            .clone()
            .unwrap_or_else(|| format!("knowledge-{}", Uuid::new_v4()));
        let updated_at = timestamp_now();

        connection
            .execute(
                "INSERT OR REPLACE INTO knowledge_records
                 (id, workspace_id, project_id, title, summary, kind, status, source_type, source_ref, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    &knowledge_id,
                    &existing.workspace_id,
                    Some(existing.project_id.as_str()),
                    knowledge_title,
                    knowledge_summary,
                    knowledge_kind,
                    "active",
                    "artifact",
                    artifact_id,
                    updated_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        let promoted_detail = DeliverableDetail {
            promotion_state: "promoted".into(),
            promotion_knowledge_id: Some(knowledge_id.clone()),
            updated_at,
            latest_version_ref: existing.latest_version_ref.clone(),
            ..existing.clone()
        };
        self.upsert_deliverable_record(&connection, &promoted_detail)?;

        let trace_detail =
            format!("Promoted deliverable `{artifact_id}` to knowledge `{knowledge_id}`.");
        self.state
            .observation
            .append_trace(TraceEventRecord {
                id: format!("trace-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(promoted_detail.project_id.clone()),
                run_id: Some(promoted_detail.run_id.clone()),
                session_id: Some(promoted_detail.session_id.clone()),
                event_kind: "deliverable_promoted".into(),
                title: "Deliverable promoted".into(),
                detail: trace_detail.clone(),
                created_at: updated_at,
            })
            .await?;
        self.state
            .observation
            .append_audit(AuditRecord {
                id: format!("audit-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(promoted_detail.project_id.clone()),
                actor_type: "system".into(),
                actor_id: "runtime-adapter".into(),
                action: "deliverable.promote".into(),
                resource: artifact_id.into(),
                outcome: "success".into(),
                created_at: updated_at,
            })
            .await?;

        let event = RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "deliverable.promoted".into(),
            kind: Some("deliverable.promoted".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&promoted_detail.project_id),
            session_id: promoted_detail.session_id.clone(),
            conversation_id: promoted_detail.conversation_id.clone(),
            run_id: Some(promoted_detail.run_id.clone()),
            emitted_at: updated_at,
            sequence: 0,
            target_kind: Some("deliverable".into()),
            target_ref: Some(artifact_id.into()),
            outcome: Some("promoted".into()),
            ..Default::default()
        };
        self.emit_event(&promoted_detail.session_id, event).await?;

        Ok(promoted_detail)
    }

    fn persist_runtime_output_artifacts_for_run(
        &self,
        session_id: &str,
        conversation_id: &str,
        run: &RuntimeRunSnapshot,
        serialized_session: &Value,
    ) -> Result<BTreeMap<String, RuntimeArtifactProjectionRow>, AppError> {
        let mut rows = BTreeMap::new();
        for artifact_ref in &run.artifact_refs {
            let body = PersistedRuntimeOutputArtifact {
                artifact_ref: artifact_ref.clone(),
                session_id: session_id.to_string(),
                conversation_id: conversation_id.to_string(),
                run_id: run.id.clone(),
                parent_run_id: run.parent_run_id.clone(),
                delegated_by_tool_call_id: run.delegated_by_tool_call_id.clone(),
                actor_ref: run.actor_ref.clone(),
                workflow_run_id: run.workflow_run.clone(),
                checkpoint_artifact_ref: run.checkpoint.checkpoint_artifact_ref.clone(),
                serialized_session: serialized_session.clone(),
                usage_summary: run.usage_summary.clone(),
                updated_at: run.updated_at,
            };
            let payload = serde_json::to_vec_pretty(&body)?;
            let (storage_path, content_hash, byte_size) = self.persist_runtime_payload(
                self.runtime_output_artifact_path(artifact_ref),
                &payload,
            )?;
            rows.insert(
                artifact_ref.clone(),
                RuntimeArtifactProjectionRow {
                    artifact_ref: artifact_ref.clone(),
                    session_id: session_id.to_string(),
                    conversation_id: conversation_id.to_string(),
                    run_id: run.id.clone(),
                    parent_run_id: run.parent_run_id.clone(),
                    delegated_by_tool_call_id: run.delegated_by_tool_call_id.clone(),
                    actor_ref: run.actor_ref.clone(),
                    workflow_run_id: run.workflow_run.clone(),
                    storage_path,
                    content_hash,
                    byte_size,
                    content_type: "application/json".into(),
                    updated_at: run.updated_at,
                },
            );
        }
        Ok(rows)
    }

    fn load_subrun_state_artifacts(
        &self,
        session_id: &str,
        parent_run_id: &str,
        subruns: &[RuntimeSubrunSummary],
    ) -> Result<BTreeMap<String, team_runtime::PersistedSubrunState>, AppError> {
        let mut states = BTreeMap::new();
        for subrun in subruns {
            let path = self.runtime_subrun_state_path(&subrun.run_id);
            if !path.exists() {
                continue;
            }
            let raw = fs::read(path)?;
            let state = serde_json::from_slice::<team_runtime::PersistedSubrunState>(&raw)?;
            states.insert(subrun.run_id.clone(), state);
        }

        for subrun_dir in [self.state.paths.runtime_state_dir.join("subruns")] {
            if !subrun_dir.exists() {
                continue;
            }

            for entry in fs::read_dir(subrun_dir)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let raw = fs::read(&path)?;
                let state = serde_json::from_slice::<team_runtime::PersistedSubrunState>(&raw)?;
                if state.run.session_id != session_id
                    || state.run.parent_run_id.as_deref() != Some(parent_run_id)
                {
                    continue;
                }
                states.insert(state.run.id.clone(), state);
            }
        }

        Ok(states)
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
                "SELECT handoff_ref, summary_json, sender_actor_ref, receiver_actor_ref, mailbox_ref,
                        state, artifact_refs_json, updated_at, envelope_storage_path
                 FROM runtime_handoff_projections
                 WHERE session_id = ?1 AND run_id = ?2
                 ORDER BY updated_at ASC, handoff_ref ASC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let handoff_rows = handoff_stmt
            .query_map(params![detail.summary.id, detail.run.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            })
            .map_err(|error| AppError::database(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))?;
        let mut handoffs = Vec::with_capacity(handoff_rows.len());
        for (
            handoff_ref,
            summary_json,
            sender_actor_ref,
            receiver_actor_ref,
            mailbox_ref,
            state,
            artifact_refs_json,
            updated_at,
            envelope_storage_path,
        ) in handoff_rows
        {
            if let Ok(summary) = serde_json::from_str::<RuntimeHandoffSummary>(&summary_json) {
                handoffs.push(summary);
                continue;
            }
            if let Some(envelope) = self.load_runtime_artifact::<PersistedHandoffEnvelope>(
                envelope_storage_path.as_deref(),
            )? {
                handoffs.push(envelope.into_summary(&handoff_ref));
                continue;
            }
            let artifact_refs =
                serde_json::from_str::<Vec<String>>(&artifact_refs_json).unwrap_or_default();
            handoffs.push(RuntimeHandoffSummary {
                handoff_ref,
                mailbox_ref,
                sender_actor_ref,
                receiver_actor_ref,
                state,
                artifact_refs,
                updated_at: updated_at.max(0) as u64,
            });
        }

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
        let derived_workflow_artifact_path =
            detail.run.workflow_run.as_deref().map(|workflow_run_id| {
                self.relative_storage_path(&self.runtime_workflow_state_path(workflow_run_id))
            });
        let workflow_state_record = if let Some((_, _, detail_storage_path)) =
            workflow_projection.as_ref()
        {
            self.load_runtime_artifact::<PersistedWorkflowState>(detail_storage_path.as_deref())?
                .or_else(|| {
                    derived_workflow_artifact_path
                        .as_deref()
                        .and_then(|path| {
                            self.load_runtime_artifact::<PersistedWorkflowState>(Some(path))
                                .ok()
                        })
                        .flatten()
                })
        } else {
            derived_workflow_artifact_path
                .as_deref()
                .map(|path| self.load_runtime_artifact::<PersistedWorkflowState>(Some(path)))
                .transpose()?
                .flatten()
        };
        let (workflow, workflow_detail, workflow_background) =
            if let Some((summary_json, detail_json, _detail_storage_path)) = workflow_projection {
                let summary = serde_json::from_str::<RuntimeWorkflowSummary>(&summary_json).ok();
                let detail_value =
                    serde_json::from_str::<RuntimeWorkflowRunDetail>(&detail_json).ok();
                if let (Some(summary), Some(detail_value)) = (summary, detail_value) {
                    let needs_artifact = detail_value.steps.is_empty();
                    if needs_artifact {
                        if let Some(record) = workflow_state_record.clone() {
                            (
                                Some(record.summary),
                                Some(record.detail),
                                Some(record.background),
                            )
                        } else {
                            (Some(summary), Some(detail_value), None)
                        }
                    } else {
                        (
                            Some(summary),
                            Some(detail_value),
                            workflow_state_record
                                .clone()
                                .map(|record| record.background),
                        )
                    }
                } else if let Some(record) = workflow_state_record.clone() {
                    (
                        Some(record.summary),
                        Some(record.detail),
                        Some(record.background),
                    )
                } else {
                    (None, None, None)
                }
            } else if let Some(record) = workflow_state_record.clone() {
                (
                    Some(record.summary),
                    Some(record.detail),
                    Some(record.background),
                )
            } else {
                (None, None, None)
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
        let derived_background_artifact_path = {
            let run_id = detail
                .background_run
                .as_ref()
                .map(|background| background.run_id.clone())
                .unwrap_or_else(|| detail.run.id.clone());
            self.relative_storage_path(&self.runtime_background_state_path(&run_id))
        };
        let background_state_record = if let Some((_, state_storage_path)) =
            background_projection.as_ref()
        {
            self.load_runtime_artifact::<PersistedBackgroundState>(state_storage_path.as_deref())?
                .or_else(|| {
                    self.load_runtime_artifact::<PersistedBackgroundState>(Some(
                        &derived_background_artifact_path,
                    ))
                    .ok()
                    .flatten()
                })
        } else {
            self.load_runtime_artifact::<PersistedBackgroundState>(Some(
                &derived_background_artifact_path,
            ))?
        };
        let background = if let Some((summary_json, _state_storage_path)) = background_projection {
            match serde_json::from_str::<RuntimeBackgroundRunSummary>(&summary_json) {
                Ok(summary) if !summary.continuation_state.is_empty() => Some(summary),
                Ok(summary) => background_state_record
                    .clone()
                    .map(|record| record.summary)
                    .or(Some(summary)),
                Err(_) => background_state_record.clone().map(|record| record.summary),
            }
        } else {
            background_state_record
                .clone()
                .map(|record| record.summary)
                .or(workflow_background)
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
            detail.run.worker_dispatch =
                Some(team_runtime::build_worker_dispatch_summary(&detail.subruns));
        }
        sync_runtime_session_detail(detail);
        Ok(())
    }

    pub(super) fn load_persisted_sessions(&self) -> Result<(), AppError> {
        let connection = self.open_db()?;
        connection
            .execute(
                "UPDATE runtime_session_projections
                 SET manifest_snapshot_ref = id || '-manifest'
                 WHERE manifest_snapshot_ref IS NULL OR TRIM(manifest_snapshot_ref) = ''",
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "UPDATE runtime_session_projections
                 SET session_policy_snapshot_ref = id || '-policy'
                 WHERE session_policy_snapshot_ref IS NULL OR TRIM(session_policy_snapshot_ref) = ''",
                [],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = {
            let mut statement = connection
                .prepare(
                    "SELECT id, detail_json, manifest_snapshot_ref, session_policy_snapshot_ref
                     FROM runtime_session_projections
                     ORDER BY updated_at DESC, id DESC",
                )
                .map_err(|error| AppError::database(error.to_string()))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                })
                .map_err(|error| AppError::database(error.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| AppError::database(error.to_string()))?;
            rows
        };

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
            let (session_id, detail_json, manifest_snapshot_ref, session_policy_snapshot_ref) = row;
            let mut detail = match serde_json::from_str::<RuntimeSessionDetail>(&detail_json) {
                Ok(detail) => detail,
                Err(error) => {
                    self.purge_legacy_runtime_session_projection(&connection, &session_id, &error)?;
                    continue;
                }
            };
            sync_runtime_session_detail(&mut detail);
            self.hydrate_phase_four_runtime_projection(&connection, &mut detail)?;
            let subrun_states = self.load_subrun_state_artifacts(
                &detail.summary.id,
                &detail.run.id,
                &detail.subruns,
            )?;
            let primary_run_serialized_session =
                self.load_primary_run_serialized_session(&detail)?;
            team_runtime::apply_subrun_state_projection(&mut detail, &subrun_states);
            sync_runtime_session_detail(&mut detail);
            let events = self.load_event_log(&detail.summary.id)?;
            let manifest_snapshot_ref = manifest_snapshot_ref
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    AppError::runtime(format!(
                        "runtime session `{}` is missing manifest snapshot ref",
                        detail.summary.id
                    ))
                })?;
            let session_policy_snapshot_ref = session_policy_snapshot_ref
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    AppError::runtime(format!(
                        "runtime session `{}` is missing session policy snapshot ref",
                        detail.summary.id
                    ))
                })?;
            order.push(detail.summary.id.clone());
            sessions.insert(
                detail.summary.id.clone(),
                RuntimeAggregate {
                    detail,
                    events,
                    metadata: RuntimeAggregateMetadata {
                        manifest_snapshot_ref,
                        session_policy_snapshot_ref,
                        primary_run_serialized_session,
                        pending_deliverables: BTreeMap::new(),
                        subrun_states,
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
        let denied_exposure_count = aggregate
            .detail
            .policy_decision_summary
            .denied_exposure_count as i64;
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
        let run_denied_exposure_count = aggregate
            .detail
            .policy_decision_summary
            .denied_exposure_count as i64;
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
            if let Some(state) = aggregate.metadata.subrun_states.get(&subrun.run_id) {
                self.persist_runtime_artifact(
                    self.runtime_subrun_state_path(&subrun.run_id),
                    state,
                )?;
            }
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
                "DELETE FROM runtime_artifact_projections WHERE session_id = ?1",
                params![summary.id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let mut runtime_artifact_rows = self.persist_runtime_output_artifacts_for_run(
            &summary.id,
            &summary.conversation_id,
            run,
            &aggregate.metadata.primary_run_serialized_session,
        )?;
        for state in aggregate.metadata.subrun_states.values() {
            runtime_artifact_rows.extend(self.persist_runtime_output_artifacts_for_run(
                &summary.id,
                &summary.conversation_id,
                &state.run,
                &state.serialized_session,
            )?);
        }
        for artifact in runtime_artifact_rows.values() {
            connection
                .execute(
                    "INSERT OR REPLACE INTO runtime_artifact_projections
                     (artifact_ref, session_id, conversation_id, run_id, parent_run_id,
                      delegated_by_tool_call_id, actor_ref, workflow_run_id, storage_path,
                      content_hash, byte_size, content_type, updated_at, summary_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        artifact.artifact_ref,
                        artifact.session_id,
                        artifact.conversation_id,
                        artifact.run_id,
                        artifact.parent_run_id,
                        artifact.delegated_by_tool_call_id,
                        artifact.actor_ref,
                        artifact.workflow_run_id,
                        artifact.storage_path,
                        artifact.content_hash,
                        artifact.byte_size as i64,
                        artifact.content_type,
                        artifact.updated_at as i64,
                        serde_json::to_string(artifact)?,
                    ],
                )
                .map_err(|error| AppError::database(error.to_string()))?;
        }
        for deliverable in aggregate.metadata.pending_deliverables.values() {
            self.persist_pending_runtime_deliverable(&connection, deliverable)?;
        }

        connection
            .execute(
                "DELETE FROM runtime_mailbox_projections WHERE session_id = ?1 AND run_id = ?2",
                params![summary.id, run.id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let subruns_by_handoff_ref = aggregate
            .detail
            .subruns
            .iter()
            .filter_map(|subrun| {
                subrun
                    .handoff_ref
                    .as_ref()
                    .map(|handoff_ref| (handoff_ref.as_str(), subrun))
            })
            .collect::<BTreeMap<_, _>>();
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
                handoffs: aggregate
                    .detail
                    .handoffs
                    .iter()
                    .map(|handoff| {
                        let subrun_lineage = subruns_by_handoff_ref
                            .get(handoff.handoff_ref.as_str())
                            .copied();
                        PersistedMailboxHandoffRecord {
                            handoff_ref: handoff.handoff_ref.clone(),
                            parent_run_id: subrun_lineage
                                .and_then(|subrun| subrun.parent_run_id.clone())
                                .or_else(|| Some(run.id.clone())),
                            delegated_by_tool_call_id: subrun_lineage
                                .and_then(|subrun| subrun.delegated_by_tool_call_id.clone())
                                .or_else(|| run.delegated_by_tool_call_id.clone()),
                            sender_actor_ref: handoff.sender_actor_ref.clone(),
                            receiver_actor_ref: handoff.receiver_actor_ref.clone(),
                            mailbox_ref: handoff.mailbox_ref.clone(),
                            artifact_refs: handoff.artifact_refs.clone(),
                            handoff_state: handoff.state.clone(),
                            updated_at: handoff.updated_at,
                        }
                    })
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
            let subrun_lineage = subruns_by_handoff_ref
                .get(handoff.handoff_ref.as_str())
                .copied();
            let envelope = PersistedHandoffEnvelope {
                handoff_ref: handoff.handoff_ref.clone(),
                session_id: summary.id.clone(),
                run_id: run.id.clone(),
                conversation_id: summary.conversation_id.clone(),
                parent_run_id: subrun_lineage
                    .and_then(|subrun| subrun.parent_run_id.clone())
                    .or_else(|| Some(run.id.clone())),
                delegated_by_tool_call_id: subrun_lineage
                    .and_then(|subrun| subrun.delegated_by_tool_call_id.clone())
                    .or_else(|| run.delegated_by_tool_call_id.clone()),
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
                        envelope.parent_run_id,
                        envelope.delegated_by_tool_call_id,
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
                session_id: summary.id.clone(),
                run_id: run.id.clone(),
                conversation_id: summary.conversation_id.clone(),
                parent_run_id: run.parent_run_id.clone(),
                mailbox_ref: run.mailbox_ref.clone(),
                summary: workflow.clone(),
                detail: workflow_detail.clone(),
                background: aggregate.detail.background_run.clone().unwrap_or(
                    RuntimeBackgroundRunSummary {
                        run_id: run.id.clone(),
                        workflow_run_id: Some(workflow.workflow_run_id.clone()),
                        status: workflow.status.clone(),
                        background_capable: workflow.background_capable,
                        continuation_state: if workflow.background_capable {
                            "running".into()
                        } else {
                            "disabled".into()
                        },
                        blocking: workflow_detail.blocking.clone(),
                        updated_at: workflow.updated_at,
                    },
                ),
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
                session_id: summary.id.clone(),
                run_id: background.run_id.clone(),
                conversation_id: summary.conversation_id.clone(),
                parent_run_id: run.parent_run_id.clone(),
                workflow_run_id: background.workflow_run_id.clone(),
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
