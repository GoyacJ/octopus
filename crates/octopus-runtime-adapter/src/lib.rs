mod executor;
mod registry;

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use octopus_core::{
    normalize_runtime_permission_mode_label, timestamp_now, AppError, ApprovalRequestRecord,
    AuditRecord, ConfiguredModelRecord, CostLedgerEntry, CreateRuntimeSessionInput,
    ModelCatalogSnapshot, ProjectWorkspaceAssignments, ResolveRuntimeApprovalInput,
    ResolvedExecutionTarget, RuntimeBootstrap, RuntimeConfigPatch, RuntimeConfigSnapshotSummary,
    RuntimeConfigSource, RuntimeConfigValidationResult, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig, RuntimeEventEnvelope,
    RuntimeMessage, RuntimeRunSnapshot, RuntimeSecretReferenceStatus, RuntimeSessionDetail,
    RuntimeSessionSummary, RuntimeTraceItem, SubmitRuntimeTurnInput, TraceEventRecord,
    RUNTIME_PERMISSION_WORKSPACE_WRITE,
};
use octopus_infra::WorkspacePaths;
use octopus_platform::{
    ModelRegistryService, ObservationService, RuntimeConfigService, RuntimeExecutionService,
    RuntimeSessionService,
};
use plugins as _;
use runtime::{apply_config_patch, ConfigDocument, ConfigLoader, ConfigSource, JsonValue};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::broadcast;
use tools as _;
use uuid::Uuid;

use executor::ExecutionResponse;
pub use executor::{LiveRuntimeModelExecutor, MockRuntimeModelExecutor, RuntimeModelExecutor};
use registry::EffectiveModelRegistry;

#[derive(Clone)]
pub struct RuntimeAdapter {
    state: Arc<RuntimeState>,
}

struct RuntimeState {
    workspace_id: String,
    paths: WorkspacePaths,
    observation: Arc<dyn ObservationService>,
    config_loader: ConfigLoader,
    executor: Arc<dyn RuntimeModelExecutor>,
    sessions: Mutex<HashMap<String, RuntimeAggregate>>,
    config_snapshots: Mutex<HashMap<String, Value>>,
    order: Mutex<Vec<String>>,
    broadcasters: Mutex<HashMap<String, broadcast::Sender<RuntimeEventEnvelope>>>,
}

#[derive(Clone)]
struct RuntimeAggregate {
    detail: RuntimeSessionDetail,
    events: Vec<RuntimeEventEnvelope>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeConfigScopeKind {
    Workspace,
    Project,
    User,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeConfigDocumentRecord {
    scope: RuntimeConfigScopeKind,
    owner_id: Option<String>,
    source_key: String,
    display_path: String,
    storage_path: PathBuf,
    exists: bool,
    loaded: bool,
    document: Option<std::collections::BTreeMap<String, JsonValue>>,
}

fn optional_project_id(project_id: &str) -> Option<String> {
    if project_id.is_empty() {
        None
    } else {
        Some(project_id.to_string())
    }
}

fn merge_project_assignments(
    effective_config: &mut Value,
    assignments: Option<&ProjectWorkspaceAssignments>,
) {
    let Some(assignments) = assignments else {
        return;
    };
    let Some(root) = effective_config.as_object_mut() else {
        return;
    };

    let project_settings = root
        .entry("projectSettings".to_string())
        .or_insert_with(|| json!({}));
    let Some(project_settings_object) = project_settings.as_object_mut() else {
        return;
    };

    let project_assignments_value = serde_json::to_value(assignments).unwrap_or_else(|_| json!({}));
    project_settings_object.insert(
        "workspaceAssignments".to_string(),
        project_assignments_value,
    );
}

fn resolve_actor_label(
    paths: &WorkspacePaths,
    actor_kind: Option<&str>,
    actor_id: Option<&str>,
) -> Option<String> {
    let actor_id = actor_id?.trim();
    if actor_id.is_empty() {
        return None;
    }
    let connection = Connection::open(&paths.db_path).ok()?;
    match actor_kind.unwrap_or_default() {
        "team" => connection
            .query_row(
                "SELECT name FROM teams WHERE id = ?1",
                params![actor_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .map(|name| format!("{} · Team", name)),
        "agent" => connection
            .query_row(
                "SELECT name FROM agents WHERE id = ?1",
                params![actor_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .map(|name| format!("{} · Agent", name)),
        _ => Some(actor_id.to_string()),
    }
}

fn build_actor_system_prompt(sections: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    let sections = sections
        .into_iter()
        .flatten()
        .map(|section| section.trim().to_string())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>();
    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

fn resolve_actor_system_prompt(
    paths: &WorkspacePaths,
    actor_kind: Option<&str>,
    actor_id: Option<&str>,
) -> Option<String> {
    let actor_id = actor_id?.trim();
    if actor_id.is_empty() {
        return None;
    }
    let connection = Connection::open(&paths.db_path).ok()?;
    match actor_kind.unwrap_or_default() {
        "agent" => connection
            .query_row(
                "SELECT name, personality, prompt FROM agents WHERE id = ?1",
                params![actor_id],
                |row| {
                    let name: String = row.get(0)?;
                    let personality: String = row.get(1)?;
                    let prompt: String = row.get(2)?;
                    Ok(build_actor_system_prompt(vec![
                        Some(format!("You are the agent `{name}`.")),
                        (!personality.trim().is_empty())
                            .then(|| format!("Personality: {personality}")),
                        (!prompt.trim().is_empty()).then(|| format!("Instructions: {prompt}")),
                    ]))
                },
            )
            .optional()
            .ok()
            .flatten()
            .flatten(),
        "team" => connection
            .query_row(
                "SELECT name, personality, prompt, leader_agent_id, member_agent_ids FROM teams WHERE id = ?1",
                params![actor_id],
                |row| {
                    let name: String = row.get(0)?;
                    let personality: String = row.get(1)?;
                    let prompt: String = row.get(2)?;
                    let leader_agent_id: Option<String> = row.get(3)?;
                    let member_agent_ids_raw: String = row.get(4)?;
                    let member_agent_ids = serde_json::from_str::<Vec<String>>(&member_agent_ids_raw)
                        .unwrap_or_default();
                    Ok(build_actor_system_prompt(vec![
                        Some(format!("You are the team `{name}` operating as a single execution actor.")),
                        (!personality.trim().is_empty())
                            .then(|| format!("Team personality: {personality}")),
                        (!prompt.trim().is_empty())
                            .then(|| format!("Team instructions: {prompt}")),
                        leader_agent_id
                            .filter(|value| !value.trim().is_empty())
                            .map(|value| format!("Leader agent id: {value}")),
                        (!member_agent_ids.is_empty()).then(|| {
                            format!("Member agent ids: {}", member_agent_ids.join(", "))
                        }),
                    ]))
                },
            )
            .optional()
            .ok()
            .flatten()
            .flatten(),
        _ => None,
    }
}

impl RuntimeAdapter {
    pub fn new(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
    ) -> Self {
        Self::new_with_executor(
            workspace_id,
            paths,
            observation,
            Arc::new(LiveRuntimeModelExecutor::new()),
        )
    }

    pub fn new_with_executor(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
        executor: Arc<dyn RuntimeModelExecutor>,
    ) -> Self {
        let config_loader = ConfigLoader::new(&paths.root, paths.runtime_config_dir.clone());
        let adapter = Self {
            state: Arc::new(RuntimeState {
                workspace_id: workspace_id.into(),
                paths,
                observation,
                config_loader,
                executor,
                sessions: Mutex::new(HashMap::new()),
                config_snapshots: Mutex::new(HashMap::new()),
                order: Mutex::new(Vec::new()),
                broadcasters: Mutex::new(HashMap::new()),
            }),
        };

        if let Err(error) = adapter.load_persisted_config_snapshots() {
            eprintln!("failed to load runtime config snapshots: {error}");
        }
        if let Err(error) = adapter.load_persisted_sessions() {
            eprintln!("failed to load runtime projections: {error}");
        }

        adapter
    }

    fn session_sender(
        &self,
        session_id: &str,
    ) -> Result<broadcast::Sender<RuntimeEventEnvelope>, AppError> {
        let mut broadcasters = self
            .state
            .broadcasters
            .lock()
            .map_err(|_| AppError::runtime("broadcast mutex poisoned"))?;
        Ok(broadcasters
            .entry(session_id.to_string())
            .or_insert_with(|| broadcast::channel(128).0)
            .clone())
    }

    fn open_db(&self) -> Result<Connection, AppError> {
        Connection::open(&self.state.paths.db_path)
            .map_err(|error| AppError::database(error.to_string()))
    }

    fn load_project_assignments(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectWorkspaceAssignments>, AppError> {
        let connection = self.open_db()?;
        let assignments_json = connection
            .query_row(
                "SELECT assignments_json FROM projects WHERE id = ?1",
                [project_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?
            .flatten();
        assignments_json
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(serde_json::from_str::<ProjectWorkspaceAssignments>)
            .transpose()
            .map_err(|error| AppError::database(error.to_string()))
    }

    fn load_project_assignments_for_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<Option<ProjectWorkspaceAssignments>, AppError> {
        let project_id = documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::Project)
            .and_then(|document| document.owner_id.as_deref());
        match project_id {
            Some(project_id) => self.load_project_assignments(project_id),
            None => Ok(None),
        }
    }

    fn load_configured_model_usage_map(&self) -> Result<HashMap<String, u64>, AppError> {
        let connection = self.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT configured_model_id, used_tokens
                 FROM configured_model_usage_projections",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                let configured_model_id: String = row.get(0)?;
                let used_tokens: i64 = row.get(1)?;
                Ok((configured_model_id, used_tokens))
            })
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut usage = HashMap::new();
        for row in rows {
            let (configured_model_id, used_tokens) =
                row.map_err(|error| AppError::database(error.to_string()))?;
            usage.insert(configured_model_id, used_tokens.max(0) as u64);
        }
        Ok(usage)
    }

    fn configured_model_used_tokens(&self, configured_model_id: &str) -> Result<u64, AppError> {
        let connection = self.open_db()?;
        let used_tokens = connection
            .query_row(
                "SELECT used_tokens
                 FROM configured_model_usage_projections
                 WHERE configured_model_id = ?1",
                [configured_model_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?
            .unwrap_or(0);
        Ok(used_tokens.max(0) as u64)
    }

    fn increment_configured_model_usage(
        &self,
        configured_model_id: &str,
        consumed_tokens: u32,
        updated_at: u64,
    ) -> Result<u64, AppError> {
        let connection = self.open_db()?;
        connection
            .execute(
                "INSERT INTO configured_model_usage_projections
                 (configured_model_id, used_tokens, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(configured_model_id)
                 DO UPDATE SET
                   used_tokens = configured_model_usage_projections.used_tokens + excluded.used_tokens,
                   updated_at = excluded.updated_at",
                params![
                    configured_model_id,
                    i64::from(consumed_tokens),
                    updated_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.configured_model_used_tokens(configured_model_id)
    }

    fn ensure_configured_model_quota_available(
        &self,
        configured_model: &ConfiguredModelRecord,
    ) -> Result<(), AppError> {
        let Some(total_tokens) = configured_model
            .token_quota
            .as_ref()
            .and_then(|quota| quota.total_tokens)
        else {
            return Ok(());
        };
        let used_tokens =
            self.configured_model_used_tokens(&configured_model.configured_model_id)?;
        if used_tokens >= total_tokens {
            return Err(AppError::invalid_input(format!(
                "configured model `{}` has reached its total token limit",
                configured_model.configured_model_id
            )));
        }
        Ok(())
    }

    fn resolve_consumed_tokens(
        &self,
        configured_model: &ConfiguredModelRecord,
        response: &ExecutionResponse,
    ) -> Result<Option<u32>, AppError> {
        match response.total_tokens {
            Some(total_tokens) => Ok(Some(total_tokens)),
            None if configured_model
                .token_quota
                .as_ref()
                .and_then(|quota| quota.total_tokens)
                .is_some() =>
            {
                Err(AppError::runtime(format!(
                    "configured model `{}` requires provider token usage for quota enforcement",
                    configured_model.configured_model_id
                )))
            }
            None => Ok(None),
        }
    }

    fn runtime_events_path(&self, session_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_events_dir
            .join(format!("{session_id}.jsonl"))
    }

    fn runtime_debug_session_path(&self, session_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join(format!("{session_id}.json"))
    }

    fn runtime_debug_events_path(&self, session_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_sessions_dir
            .join(format!("{session_id}-events.json"))
    }

    fn load_persisted_sessions(&self) -> Result<(), AppError> {
        let connection = self.open_db()?;
        let mut statement = connection
            .prepare(
                "SELECT detail_json
                 FROM runtime_session_projections
                 ORDER BY updated_at DESC, id DESC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
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
            let detail_json = row.map_err(|error| AppError::database(error.to_string()))?;
            let detail = serde_json::from_str::<RuntimeSessionDetail>(&detail_json)?;
            let events = self.load_event_log(&detail.summary.id)?;
            order.push(detail.summary.id.clone());
            sessions.insert(
                detail.summary.id.clone(),
                RuntimeAggregate { detail, events },
            );
        }

        Ok(())
    }

    fn load_event_log(&self, session_id: &str) -> Result<Vec<RuntimeEventEnvelope>, AppError> {
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

    fn append_json_line(path: &Path, value: &impl Serialize) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, value)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    fn persist_session(
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

    fn persist_runtime_projections(&self, aggregate: &RuntimeAggregate) -> Result<(), AppError> {
        let connection = self.open_db()?;
        let summary = &aggregate.detail.summary;
        let run = &aggregate.detail.run;
        let started_from_scope_set = serde_json::to_string(&summary.started_from_scope_set)?;
        let detail_json = serde_json::to_string(&aggregate.detail)?;
        let run_json = serde_json::to_string(run)?;

        connection
            .execute(
                "INSERT OR REPLACE INTO runtime_session_projections
                 (id, conversation_id, project_id, title, session_kind, status, updated_at, last_message_preview,
                  config_snapshot_id, effective_config_hash, started_from_scope_set, detail_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
                    detail_json,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        connection
            .execute(
                "INSERT OR REPLACE INTO runtime_run_projections
                 (id, session_id, conversation_id, status, current_step, started_at, updated_at,
                  model_id, next_action, config_snapshot_id, effective_config_hash,
                  started_from_scope_set, run_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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

    fn persist_config_snapshot(
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

    fn load_persisted_config_snapshots(&self) -> Result<(), AppError> {
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

    fn hash_value(value: &serde_json::Value) -> Result<String, AppError> {
        let encoded = serde_json::to_vec(value)?;
        let digest = Sha256::digest(encoded);
        Ok(format!("{digest:x}"))
    }

    fn runtime_json_to_serde(value: &JsonValue) -> serde_json::Value {
        match value {
            JsonValue::Null => serde_json::Value::Null,
            JsonValue::Bool(value) => serde_json::Value::Bool(*value),
            JsonValue::Number(value) => serde_json::Value::Number((*value).into()),
            JsonValue::String(value) => serde_json::Value::String(value.clone()),
            JsonValue::Array(values) => {
                serde_json::Value::Array(values.iter().map(Self::runtime_json_to_serde).collect())
            }
            JsonValue::Object(entries) => serde_json::Value::Object(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::runtime_json_to_serde(value)))
                    .collect(),
            ),
        }
    }

    fn serde_to_runtime_json(value: &serde_json::Value) -> Result<JsonValue, AppError> {
        JsonValue::parse(&serde_json::to_string(value)?)
            .map_err(|error| AppError::invalid_input(error.to_string()))
    }

    fn public_scope_label(scope: RuntimeConfigScopeKind) -> &'static str {
        match scope {
            RuntimeConfigScopeKind::Workspace => "workspace",
            RuntimeConfigScopeKind::Project => "project",
            RuntimeConfigScopeKind::User => "user",
        }
    }

    fn parse_scope(scope: &str) -> Result<RuntimeConfigScopeKind, AppError> {
        match scope {
            "workspace" => Ok(RuntimeConfigScopeKind::Workspace),
            "project" => Ok(RuntimeConfigScopeKind::Project),
            "user" => Ok(RuntimeConfigScopeKind::User),
            other => Err(AppError::invalid_input(format!(
                "unsupported runtime config scope: {other}"
            ))),
        }
    }

    fn internal_scope(scope: RuntimeConfigScopeKind) -> ConfigSource {
        match scope {
            RuntimeConfigScopeKind::User => ConfigSource::User,
            RuntimeConfigScopeKind::Workspace => ConfigSource::Project,
            RuntimeConfigScopeKind::Project => ConfigSource::Local,
        }
    }

    fn scope_precedence(scope: RuntimeConfigScopeKind) -> u8 {
        match scope {
            RuntimeConfigScopeKind::User => 0,
            RuntimeConfigScopeKind::Workspace => 1,
            RuntimeConfigScopeKind::Project => 2,
        }
    }

    fn workspace_config_path(&self) -> PathBuf {
        self.state.paths.runtime_config_dir.join("workspace.json")
    }

    fn project_config_path(&self, project_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_project_config_dir
            .join(format!("{project_id}.json"))
    }

    fn user_config_path(&self, user_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_user_config_dir
            .join(format!("{user_id}.json"))
    }

    fn ensure_runtime_config_layout(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.state.paths.runtime_config_dir)?;
        fs::create_dir_all(&self.state.paths.runtime_project_config_dir)?;
        fs::create_dir_all(&self.state.paths.runtime_user_config_dir)?;
        Ok(())
    }

    fn read_optional_runtime_document(
        path: &Path,
    ) -> Result<Option<BTreeMap<String, JsonValue>>, AppError> {
        match fs::read_to_string(path) {
            Ok(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Ok(None);
                }
                let parsed = JsonValue::parse(trimmed)
                    .map_err(|error| AppError::runtime(error.to_string()))?;
                parsed.as_object().cloned().map(Some).ok_or_else(|| {
                    AppError::runtime("runtime config document must be a JSON object")
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn write_runtime_document(
        &self,
        path: &Path,
        document: &BTreeMap<String, JsonValue>,
    ) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let rendered = serde_json::to_vec_pretty(&Self::runtime_json_to_serde(
            &JsonValue::Object(document.clone()),
        ))?;
        fs::write(path, rendered)?;
        Ok(())
    }

    fn migrate_legacy_workspace_config_if_needed(&self) -> Result<(), AppError> {
        let workspace_path = self.workspace_config_path();
        if workspace_path.exists() {
            return Ok(());
        }

        self.ensure_runtime_config_layout()?;

        let mut merged = BTreeMap::new();
        let mut found = false;
        for legacy_path in [
            self.state
                .paths
                .config_dir
                .join(".claw")
                .join("settings.json"),
            self.state.paths.root.join(".claw.json"),
            self.state.paths.root.join(".claw").join("settings.json"),
        ] {
            if let Some(document) = Self::read_optional_runtime_document(&legacy_path)? {
                apply_config_patch(&mut merged, &document);
                found = true;
            }
        }

        if found {
            self.write_runtime_document(&workspace_path, &merged)?;
        }

        Ok(())
    }

    fn config_document_record(
        &self,
        scope: RuntimeConfigScopeKind,
        owner_id: Option<&str>,
        storage_path: PathBuf,
        display_path: String,
        source_key: String,
    ) -> Result<RuntimeConfigDocumentRecord, AppError> {
        let document = Self::read_optional_runtime_document(&storage_path)?;
        Ok(RuntimeConfigDocumentRecord {
            scope,
            owner_id: owner_id.map(ToOwned::to_owned),
            source_key,
            display_path,
            exists: storage_path.exists(),
            loaded: document.is_some(),
            storage_path,
            document,
        })
    }

    fn resolve_documents(
        &self,
        project_id: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<Vec<RuntimeConfigDocumentRecord>, AppError> {
        self.ensure_runtime_config_layout()?;
        self.migrate_legacy_workspace_config_if_needed()?;

        let mut documents = vec![self.config_document_record(
            RuntimeConfigScopeKind::Workspace,
            None,
            self.workspace_config_path(),
            "config/runtime/workspace.json".to_string(),
            "workspace".to_string(),
        )?];

        if let Some(project_id) = project_id.filter(|value| !value.is_empty()) {
            documents.push(self.config_document_record(
                RuntimeConfigScopeKind::Project,
                Some(project_id),
                self.project_config_path(project_id),
                format!("config/runtime/projects/{project_id}.json"),
                format!("project:{project_id}"),
            )?);
        }

        if let Some(user_id) = user_id.filter(|value| !value.is_empty()) {
            documents.push(self.config_document_record(
                RuntimeConfigScopeKind::User,
                Some(user_id),
                self.user_config_path(user_id),
                format!("config/runtime/users/{user_id}.json"),
                format!("user:{user_id}"),
            )?);
        }

        documents.sort_by_key(|document| Self::scope_precedence(document.scope));

        Ok(documents)
    }

    fn to_internal_documents(documents: &[RuntimeConfigDocumentRecord]) -> Vec<ConfigDocument> {
        documents
            .iter()
            .map(|document| ConfigDocument {
                source: Self::internal_scope(document.scope),
                path: document.storage_path.clone(),
                exists: document.exists,
                loaded: document.loaded,
                document: document.document.clone(),
            })
            .collect()
    }

    fn is_sensitive_key(key: &str) -> bool {
        let normalized = key
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        [
            "apikey",
            "token",
            "secret",
            "password",
            "authorization",
            "authtoken",
            "clientsecret",
            "accesskey",
        ]
        .iter()
        .any(|needle| normalized.contains(needle))
    }

    fn is_reference_value(value: &str) -> bool {
        value.starts_with("env:")
            || value.starts_with("keychain:")
            || value.starts_with("op://")
            || value.starts_with("vault:")
            || value.starts_with("secret-ref:")
    }

    fn record_secret_reference(
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
        scope: &str,
        path: &str,
        reference: Option<String>,
        status: &str,
    ) {
        if secret_references.iter().any(|existing| {
            existing.scope == scope
                && existing.path == path
                && existing.reference == reference
                && existing.status == status
        }) {
            return;
        }

        secret_references.push(RuntimeSecretReferenceStatus {
            scope: scope.to_string(),
            path: path.to_string(),
            reference,
            status: status.to_string(),
        });
    }

    fn redact_secret_value(
        scope: &str,
        path: &str,
        raw: &str,
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
    ) -> serde_json::Value {
        if Self::is_reference_value(raw) {
            let status = if let Some(env_key) = raw.strip_prefix("env:") {
                if std::env::var_os(env_key).is_some() {
                    "reference-present"
                } else {
                    "reference-missing"
                }
            } else {
                "reference-present"
            };
            Self::record_secret_reference(
                secret_references,
                scope,
                path,
                Some(raw.to_string()),
                status,
            );
            return serde_json::Value::String(raw.to_string());
        }

        Self::record_secret_reference(secret_references, scope, path, None, "inline-redacted");
        serde_json::Value::String("***".to_string())
    }

    fn redact_config_value(
        scope: &str,
        path: &str,
        value: &serde_json::Value,
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
    ) -> serde_json::Value {
        match value {
            serde_json::Value::Object(object) => serde_json::Value::Object(
                object
                    .iter()
                    .map(|(key, value)| {
                        let next_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{path}.{key}")
                        };
                        let next_value = match value {
                            serde_json::Value::String(raw) if Self::is_sensitive_key(key) => {
                                Self::redact_secret_value(scope, &next_path, raw, secret_references)
                            }
                            _ => Self::redact_config_value(
                                scope,
                                &next_path,
                                value,
                                secret_references,
                            ),
                        };
                        (key.clone(), next_value)
                    })
                    .collect(),
            ),
            serde_json::Value::Array(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|value| Self::redact_config_value(scope, path, value, secret_references))
                    .collect(),
            ),
            _ => value.clone(),
        }
    }

    fn validate_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let internal_documents = Self::to_internal_documents(documents);
        match self
            .state
            .config_loader
            .load_from_documents(&internal_documents)
        {
            Ok(_) => Ok(RuntimeConfigValidationResult {
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            }),
            Err(error) => Ok(RuntimeConfigValidationResult {
                valid: false,
                errors: vec![error.to_string()],
                warnings: Vec::new(),
            }),
        }
    }

    fn load_effective_config_json(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<Value, AppError> {
        let mut merged = BTreeMap::new();
        for document in documents {
            if let Some(record) = &document.document {
                apply_config_patch(&mut merged, record);
            }
        }
        let mut effective_config = Self::runtime_json_to_serde(&JsonValue::Object(merged));
        let project_assignments = self.load_project_assignments_for_documents(documents)?;
        merge_project_assignments(&mut effective_config, project_assignments.as_ref());
        Ok(effective_config)
    }

    fn effective_registry_from_json(
        &self,
        effective_config: &Value,
    ) -> Result<EffectiveModelRegistry, AppError> {
        EffectiveModelRegistry::from_effective_config(effective_config)
    }

    fn effective_registry(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<EffectiveModelRegistry, AppError> {
        let effective_config = self.load_effective_config_json(documents)?;
        self.effective_registry_from_json(&effective_config)
    }

    fn build_effective_config(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let mut secret_references = Vec::new();
        let effective_value = self.load_effective_config_json(documents)?;
        let effective_config =
            Self::redact_config_value("effective", "", &effective_value, &mut Vec::new());
        let effective_config_hash = Self::hash_value(&effective_value)?;

        let sources = documents
            .iter()
            .map(|document| {
                let document_value = document
                    .document
                    .as_ref()
                    .map(|value| Self::runtime_json_to_serde(&JsonValue::Object(value.clone())));
                let redacted_document = document_value.as_ref().map(|value| {
                    Self::redact_config_value(
                        Self::public_scope_label(document.scope),
                        "",
                        value,
                        &mut secret_references,
                    )
                });
                let content_hash = document_value.as_ref().map(Self::hash_value).transpose()?;

                Ok(RuntimeConfigSource {
                    scope: Self::public_scope_label(document.scope).to_string(),
                    owner_id: document.owner_id.clone(),
                    display_path: document.display_path.clone(),
                    source_key: document.source_key.clone(),
                    exists: document.exists,
                    loaded: document.loaded,
                    content_hash,
                    document: redacted_document,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(RuntimeEffectiveConfig {
            effective_config,
            effective_config_hash,
            sources,
            validation: RuntimeConfigValidationResult {
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            },
            secret_references,
        })
    }

    fn validate_registry_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let registry = self.effective_registry(documents)?;
        let mut validation = self.validate_documents(documents)?;
        validation
            .warnings
            .extend(registry.diagnostics().warnings.clone());
        validation
            .errors
            .extend(registry.diagnostics().errors.clone());
        validation.valid = validation.errors.is_empty();
        Ok(validation)
    }

    fn patched_documents(
        &self,
        scope: RuntimeConfigScopeKind,
        project_id: Option<&str>,
        user_id: Option<&str>,
        patch: &serde_json::Value,
    ) -> Result<Vec<RuntimeConfigDocumentRecord>, AppError> {
        let patch = Self::serde_to_runtime_json(patch)?;
        let patch_object = patch
            .as_object()
            .ok_or_else(|| AppError::invalid_input("runtime config patch must be a JSON object"))?;

        let mut documents = self.resolve_documents(project_id, user_id)?;

        let target_document = documents
            .iter_mut()
            .find(|document| document.scope == scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        let mut next = target_document.document.clone().unwrap_or_default();
        apply_config_patch(&mut next, patch_object);
        target_document.exists = true;
        target_document.loaded = true;
        target_document.document = Some(next);

        Ok(documents)
    }

    fn write_document(&self, document: &RuntimeConfigDocumentRecord) -> Result<(), AppError> {
        self.write_runtime_document(
            &document.storage_path,
            &document.document.clone().unwrap_or_default(),
        )
    }

    fn current_config_snapshot(
        &self,
        project_id: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<RuntimeConfigSnapshotSummary, AppError> {
        let documents = self.resolve_documents(project_id, user_id)?;
        let effective = self.build_effective_config(&documents)?;
        let effective_config = self.load_effective_config_json(&documents)?;
        let source_refs = documents
            .iter()
            .filter(|document| document.loaded)
            .map(|document| document.source_key.clone())
            .collect::<Vec<_>>();
        let mut started_from_scope_set = Vec::new();
        for document in &documents {
            let scope = Self::public_scope_label(document.scope).to_string();
            if document.loaded
                && !started_from_scope_set
                    .iter()
                    .any(|existing| existing == &scope)
            {
                started_from_scope_set.push(scope);
            }
        }

        Ok(RuntimeConfigSnapshotSummary {
            id: format!("cfgsnap-{}", Uuid::new_v4()),
            effective_config_hash: effective.effective_config_hash,
            started_from_scope_set,
            source_refs,
            created_at: timestamp_now(),
            effective_config: Some(effective_config),
        })
    }

    async fn emit_event(
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
        Self::append_json_line(&self.runtime_events_path(session_id), &event)?;
        drop(sessions);

        let sender = self.session_sender(session_id)?;
        let _ = sender.send(event);
        Ok(())
    }

    fn config_snapshot_value(&self, snapshot_id: &str) -> Result<Value, AppError> {
        self.state
            .config_snapshots
            .lock()
            .map_err(|_| AppError::runtime("runtime config snapshots mutex poisoned"))?
            .get(snapshot_id)
            .cloned()
            .ok_or_else(|| {
                AppError::runtime(format!(
                    "runtime config snapshot `{snapshot_id}` is unavailable"
                ))
            })
    }

    async fn probe_configured_model_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
        configured_model_id: &str,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError> {
        let validation = self.validate_registry_documents(documents)?;
        if !validation.valid {
            return Ok(RuntimeConfiguredModelProbeResult {
                valid: false,
                reachable: false,
                configured_model_id: configured_model_id.to_string(),
                configured_model_name: None,
                request_id: None,
                consumed_tokens: None,
                errors: validation.errors,
                warnings: validation.warnings,
            });
        }

        let effective_config = self.load_effective_config_json(documents)?;
        let registry = self.effective_registry_from_json(&effective_config)?;
        let resolved_target = match registry.resolve_target(configured_model_id, None) {
            Ok(target) => target,
            Err(error) => {
                return Ok(RuntimeConfiguredModelProbeResult {
                    valid: false,
                    reachable: false,
                    configured_model_id: configured_model_id.to_string(),
                    configured_model_name: None,
                    request_id: None,
                    consumed_tokens: None,
                    errors: vec![error.to_string()],
                    warnings: validation.warnings,
                });
            }
        };
        let configured_model = match registry.configured_model(configured_model_id).cloned() {
            Some(configured_model) => configured_model,
            None => {
                return Ok(RuntimeConfiguredModelProbeResult {
                    valid: false,
                    reachable: false,
                    configured_model_id: configured_model_id.to_string(),
                    configured_model_name: None,
                    request_id: None,
                    consumed_tokens: None,
                    errors: vec![format!(
                        "configured model `{configured_model_id}` is not registered"
                    )],
                    warnings: validation.warnings,
                });
            }
        };

        if let Err(error) = self.ensure_configured_model_quota_available(&configured_model) {
            return Ok(RuntimeConfiguredModelProbeResult {
                valid: true,
                reachable: false,
                configured_model_id: configured_model_id.to_string(),
                configured_model_name: Some(configured_model.name.clone()),
                request_id: None,
                consumed_tokens: None,
                errors: vec![error.to_string()],
                warnings: validation.warnings,
            });
        }

        let response = match self
            .execute_resolved_turn(&resolved_target, "Reply with exactly OK.", None, None)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return Ok(RuntimeConfiguredModelProbeResult {
                    valid: true,
                    reachable: false,
                    configured_model_id: configured_model_id.to_string(),
                    configured_model_name: Some(configured_model.name.clone()),
                    request_id: None,
                    consumed_tokens: None,
                    errors: vec![error.to_string()],
                    warnings: validation.warnings,
                });
            }
        };

        let consumed_tokens = match self.resolve_consumed_tokens(&configured_model, &response) {
            Ok(consumed_tokens) => consumed_tokens,
            Err(error) => {
                return Ok(RuntimeConfiguredModelProbeResult {
                    valid: true,
                    reachable: false,
                    configured_model_id: configured_model_id.to_string(),
                    configured_model_name: Some(configured_model.name.clone()),
                    request_id: response.request_id.clone(),
                    consumed_tokens: None,
                    errors: vec![error.to_string()],
                    warnings: validation.warnings,
                });
            }
        };

        let now = timestamp_now();
        self.state
            .observation
            .append_cost(CostLedgerEntry {
                id: format!("cost-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: None,
                run_id: None,
                configured_model_id: Some(resolved_target.configured_model_id.clone()),
                metric: response
                    .total_tokens
                    .map(|_| "tokens")
                    .unwrap_or("turns")
                    .into(),
                amount: response.total_tokens.map(i64::from).unwrap_or(1),
                unit: response
                    .total_tokens
                    .map(|_| "tokens")
                    .unwrap_or("count")
                    .into(),
                created_at: now,
            })
            .await?;
        if let Some(consumed_tokens) = consumed_tokens {
            self.increment_configured_model_usage(
                &resolved_target.configured_model_id,
                consumed_tokens,
                now,
            )?;
        }

        Ok(RuntimeConfiguredModelProbeResult {
            valid: true,
            reachable: true,
            configured_model_id: configured_model_id.to_string(),
            configured_model_name: Some(configured_model.name),
            request_id: response.request_id,
            consumed_tokens,
            errors: Vec::new(),
            warnings: validation.warnings,
        })
    }

    fn resolve_execution_target(
        &self,
        config_snapshot_id: &str,
        configured_model_id: &str,
    ) -> Result<(EffectiveModelRegistry, ResolvedExecutionTarget), AppError> {
        let effective_config = self.config_snapshot_value(config_snapshot_id)?;
        let registry = self.effective_registry_from_json(&effective_config)?;
        let target = registry.resolve_target(configured_model_id, None)?;
        Ok((registry, target))
    }

    fn resolve_execution_target_from_input(
        &self,
        config_snapshot_id: &str,
        input: &SubmitRuntimeTurnInput,
    ) -> Result<(EffectiveModelRegistry, String, ResolvedExecutionTarget), AppError> {
        let effective_config = self.config_snapshot_value(config_snapshot_id)?;
        let registry = self.effective_registry_from_json(&effective_config)?;
        let configured_model_id = input
            .configured_model_id
            .as_deref()
            .or(input.model_id.as_deref())
            .map(ToOwned::to_owned)
            .or_else(|| {
                registry
                    .default_configured_model_id("conversation")
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| {
                AppError::invalid_input(
                    "configuredModelId or modelId is required when no conversation default is configured",
                )
            })?;
        let target = registry.resolve_target(&configured_model_id, None)?;
        Ok((registry, configured_model_id, target))
    }

    async fn execute_resolved_turn(
        &self,
        target: &ResolvedExecutionTarget,
        content: &str,
        actor_kind: Option<&str>,
        actor_id: Option<&str>,
    ) -> Result<ExecutionResponse, AppError> {
        let system_prompt = resolve_actor_system_prompt(&self.state.paths, actor_kind, actor_id);
        self.state
            .executor
            .execute_turn(target, content, system_prompt.as_deref())
            .await
    }
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
                session_kind: input.session_kind.unwrap_or_else(|| "project".into()),
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

#[async_trait]
impl ModelRegistryService for RuntimeAdapter {
    async fn catalog_snapshot(&self) -> Result<ModelCatalogSnapshot, AppError> {
        let documents = self.resolve_documents(None, None)?;
        let registry = self.effective_registry(&documents)?;
        let usage = self.load_configured_model_usage_map()?;
        Ok(registry.snapshot_with_usage(&usage))
    }
}

#[async_trait]
impl RuntimeConfigService for RuntimeAdapter {
    async fn get_config(&self) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(None, None)?;
        let mut effective = self.build_effective_config(&documents)?;
        effective.validation = self.validate_registry_documents(&documents)?;
        Ok(effective)
    }

    async fn get_project_config(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(Some(project_id), Some(user_id))?;
        let mut effective = self.build_effective_config(&documents)?;
        effective.validation = self.validate_registry_documents(&documents)?;
        Ok(effective)
    }

    async fn get_user_config(&self, user_id: &str) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(None, Some(user_id))?;
        let mut effective = self.build_effective_config(&documents)?;
        effective.validation = self.validate_registry_documents(&documents)?;
        Ok(effective)
    }

    async fn validate_config(
        &self,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let documents =
            self.patched_documents(Self::parse_scope(&patch.scope)?, None, None, &patch.patch)?;
        self.validate_registry_documents(&documents)
    }

    async fn validate_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let documents = self.patched_documents(
            Self::parse_scope(&patch.scope)?,
            Some(project_id),
            Some(user_id),
            &patch.patch,
        )?;
        self.validate_registry_documents(&documents)
    }

    async fn validate_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let documents = self.patched_documents(
            Self::parse_scope(&patch.scope)?,
            None,
            Some(user_id),
            &patch.patch,
        )?;
        self.validate_registry_documents(&documents)
    }

    async fn probe_configured_model(
        &self,
        input: RuntimeConfiguredModelProbeInput,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError> {
        let scope = Self::parse_scope(&input.scope)?;
        if scope != RuntimeConfigScopeKind::Workspace {
            return Ok(RuntimeConfiguredModelProbeResult {
                valid: false,
                reachable: false,
                configured_model_id: input.configured_model_id,
                configured_model_name: None,
                request_id: None,
                consumed_tokens: None,
                errors: vec!["configured model probe only supports workspace scope".into()],
                warnings: Vec::new(),
            });
        }

        let documents = self.patched_documents(scope, None, None, &input.patch)?;
        self.probe_configured_model_documents(&documents, &input.configured_model_id)
            .await
    }

    async fn save_config(
        &self,
        scope: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        if patch.scope != scope {
            return Err(AppError::invalid_input(
                "runtime config patch scope must match route scope",
            ));
        }

        let target_scope = Self::parse_scope(scope)?;
        let documents = self.patched_documents(target_scope, None, None, &patch.patch)?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target = documents
            .iter()
            .find(|document| document.scope == target_scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        self.write_document(target)?;

        let reloaded = self.resolve_documents(None, None)?;
        let mut effective = self.build_effective_config(&reloaded)?;
        effective.validation = validation;
        Ok(effective)
    }

    async fn save_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        if patch.scope != "project" {
            return Err(AppError::invalid_input(
                "project runtime config patch scope must be project",
            ));
        }

        let documents = self.patched_documents(
            RuntimeConfigScopeKind::Project,
            Some(project_id),
            Some(user_id),
            &patch.patch,
        )?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target = documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::Project)
            .ok_or_else(|| AppError::not_found("project runtime config document"))?;
        self.write_document(target)?;

        let reloaded = self.resolve_documents(Some(project_id), Some(user_id))?;
        let mut effective = self.build_effective_config(&reloaded)?;
        effective.validation = validation;
        Ok(effective)
    }

    async fn save_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        if patch.scope != "user" {
            return Err(AppError::invalid_input(
                "user runtime config patch scope must be user",
            ));
        }

        let documents = self.patched_documents(
            RuntimeConfigScopeKind::User,
            None,
            Some(user_id),
            &patch.patch,
        )?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target = documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::User)
            .ok_or_else(|| AppError::not_found("user runtime config document"))?;
        self.write_document(target)?;

        let reloaded = self.resolve_documents(None, Some(user_id))?;
        let mut effective = self.build_effective_config(&reloaded)?;
        effective.validation = validation;
        Ok(effective)
    }
}

#[async_trait]
impl RuntimeExecutionService for RuntimeAdapter {
    async fn submit_turn(
        &self,
        session_id: &str,
        input: SubmitRuntimeTurnInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let normalized_permission_mode =
            normalize_runtime_permission_mode_label(&input.permission_mode).ok_or_else(|| {
                AppError::invalid_input(format!(
                    "unsupported permission mode: {}",
                    input.permission_mode
                ))
            })?;
        let requires_approval = normalized_permission_mode == RUNTIME_PERMISSION_WORKSPACE_WRITE;
        let config_snapshot_id = {
            let sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;
            aggregate.detail.run.config_snapshot_id.clone()
        };
        let (registry, configured_model_id, resolved_target) =
            self.resolve_execution_target_from_input(&config_snapshot_id, &input)?;
        let configured_model = registry
            .configured_model(&configured_model_id)
            .cloned()
            .ok_or_else(|| {
                AppError::invalid_input(format!(
                    "configured model `{configured_model_id}` is not registered"
                ))
            })?;
        self.ensure_configured_model_quota_available(&configured_model)?;
        let execution = if requires_approval {
            None
        } else {
            let response = self
                .execute_resolved_turn(
                    &resolved_target,
                    &input.content,
                    input.actor_kind.as_deref(),
                    input.actor_id.as_deref(),
                )
                .await?;
            let _ = self.resolve_consumed_tokens(&configured_model, &response)?;
            Some(response)
        };
        let consumed_tokens = execution
            .as_ref()
            .map(|response| self.resolve_consumed_tokens(&configured_model, response))
            .transpose()?
            .flatten();
        let requested_actor_kind = input.actor_kind.clone();
        let requested_actor_id = input.actor_id.clone();
        let resolved_actor_kind = input.actor_kind.clone();
        let resolved_actor_id = input.actor_id.clone();
        let resolved_actor_label = resolve_actor_label(
            &self.state.paths,
            resolved_actor_kind.as_deref(),
            resolved_actor_id.as_deref(),
        );

        let (
            user_message,
            submitted_trace,
            execution_trace,
            assistant_message,
            approval,
            run,
            conversation_id,
            project_id,
        ) = {
            let mut sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;

            let user_message = RuntimeMessage {
                id: format!("msg-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                sender_type: "user".into(),
                sender_label: "User".into(),
                content: input.content.clone(),
                timestamp: now,
                configured_model_id: Some(resolved_target.configured_model_id.clone()),
                configured_model_name: Some(resolved_target.configured_model_name.clone()),
                model_id: Some(resolved_target.registry_model_id.clone()),
                status: if requires_approval {
                    "waiting_approval".into()
                } else {
                    "completed".into()
                },
                requested_actor_kind: requested_actor_kind.clone(),
                requested_actor_id: requested_actor_id.clone(),
                resolved_actor_kind: resolved_actor_kind.clone(),
                resolved_actor_id: resolved_actor_id.clone(),
                resolved_actor_label: resolved_actor_label.clone(),
                used_default_actor: Some(resolved_actor_id.is_none()),
                resource_ids: Some(Vec::new()),
                attachments: Some(Vec::new()),
                artifacts: Some(Vec::new()),
                usage: None,
                tool_calls: None,
                process_entries: None,
            };
            aggregate.detail.messages.push(user_message.clone());

            let submitted_trace = RuntimeTraceItem {
                id: format!("trace-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                run_id: aggregate.detail.run.id.clone(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                kind: "step".into(),
                title: "Turn submitted".into(),
                detail: if requires_approval {
                    format!(
                        "Permission mode {} requires explicit approval before execution.",
                        normalized_permission_mode
                    )
                } else {
                    format!(
                        "Turn submitted and completed with permission mode {}.",
                        normalized_permission_mode
                    )
                },
                tone: if requires_approval {
                    "warning".into()
                } else {
                    "success".into()
                },
                timestamp: now,
                actor: resolved_actor_label
                    .clone()
                    .unwrap_or_else(|| "user".into()),
                actor_kind: resolved_actor_kind.clone(),
                actor_id: resolved_actor_id.clone(),
                related_message_id: Some(user_message.id.clone()),
                related_tool_name: None,
            };
            aggregate.detail.trace.push(submitted_trace.clone());

            let approval = requires_approval.then(|| ApprovalRequestRecord {
                id: format!("approval-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                run_id: aggregate.detail.run.id.clone(),
                tool_name: "runtime.turn".into(),
                summary: "Turn requires approval".into(),
                detail: format!(
                    "Permission mode {} requires explicit approval.",
                    normalized_permission_mode
                ),
                risk_level: "medium".into(),
                created_at: now,
                status: "pending".into(),
            });
            aggregate.detail.pending_approval = approval.clone();

            let assistant_message = execution.as_ref().map(|response| RuntimeMessage {
                id: format!("msg-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                sender_type: "assistant".into(),
                sender_label: resolved_actor_label
                    .clone()
                    .unwrap_or_else(|| resolved_target.provider_id.clone()),
                content: response.content.clone(),
                timestamp: now,
                configured_model_id: Some(resolved_target.configured_model_id.clone()),
                configured_model_name: Some(resolved_target.configured_model_name.clone()),
                model_id: Some(resolved_target.registry_model_id.clone()),
                status: "completed".into(),
                requested_actor_kind: requested_actor_kind.clone(),
                requested_actor_id: requested_actor_id.clone(),
                resolved_actor_kind: resolved_actor_kind.clone(),
                resolved_actor_id: resolved_actor_id.clone(),
                resolved_actor_label: resolved_actor_label.clone(),
                used_default_actor: Some(resolved_actor_id.is_none()),
                resource_ids: Some(Vec::new()),
                attachments: Some(Vec::new()),
                artifacts: Some(vec![format!("artifact-{}", aggregate.detail.run.id)]),
                usage: None,
                tool_calls: None,
                process_entries: None,
            });
            if let Some(message) = assistant_message.as_ref() {
                aggregate.detail.messages.push(message.clone());
            }

            let execution_trace = execution.as_ref().map(|response| RuntimeTraceItem {
                id: format!("trace-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                run_id: aggregate.detail.run.id.clone(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                kind: "step".into(),
                title: "Model execution completed".into(),
                detail: format!(
                    "Resolved {}:{} via {} and produced {} characters.",
                    resolved_target.provider_id,
                    resolved_target.configured_model_name,
                    resolved_target.protocol_family,
                    response.content.chars().count()
                ),
                tone: "success".into(),
                timestamp: now,
                actor: resolved_actor_label
                    .clone()
                    .unwrap_or_else(|| "assistant".into()),
                actor_kind: resolved_actor_kind.clone(),
                actor_id: resolved_actor_id.clone(),
                related_message_id: assistant_message.as_ref().map(|message| message.id.clone()),
                related_tool_name: None,
            });
            if let Some(trace) = execution_trace.as_ref() {
                aggregate.detail.trace.push(trace.clone());
            }

            aggregate.detail.summary.status = if requires_approval {
                "waiting_approval".into()
            } else {
                "completed".into()
            };
            aggregate.detail.summary.updated_at = now;
            aggregate.detail.summary.last_message_preview = Some(
                assistant_message
                    .as_ref()
                    .map(|message| message.content.clone())
                    .unwrap_or_else(|| input.content.clone()),
            );
            aggregate.detail.run.status = if requires_approval {
                "waiting_approval".into()
            } else {
                "completed".into()
            };
            aggregate.detail.run.current_step = if requires_approval {
                "awaiting_approval".into()
            } else {
                "completed".into()
            };
            aggregate.detail.run.updated_at = now;
            aggregate.detail.run.configured_model_id =
                Some(resolved_target.configured_model_id.clone());
            aggregate.detail.run.configured_model_name =
                Some(resolved_target.configured_model_name.clone());
            aggregate.detail.run.model_id = Some(resolved_target.registry_model_id.clone());
            aggregate.detail.run.consumed_tokens = consumed_tokens;
            aggregate.detail.run.next_action = Some(if requires_approval {
                "approval".into()
            } else {
                "idle".into()
            });
            aggregate.detail.run.resolved_target = Some(resolved_target.clone());
            aggregate.detail.run.requested_actor_kind = requested_actor_kind.clone();
            aggregate.detail.run.requested_actor_id = requested_actor_id.clone();
            aggregate.detail.run.resolved_actor_kind = resolved_actor_kind.clone();
            aggregate.detail.run.resolved_actor_id = resolved_actor_id.clone();
            aggregate.detail.run.resolved_actor_label = resolved_actor_label.clone();

            let run = aggregate.detail.run.clone();
            let conversation_id = aggregate.detail.summary.conversation_id.clone();
            let project_id = aggregate.detail.summary.project_id.clone();
            self.persist_session(session_id, aggregate)?;
            (
                user_message,
                submitted_trace,
                execution_trace,
                assistant_message,
                approval,
                run,
                conversation_id,
                project_id,
            )
        };

        self.state
            .observation
            .append_trace(TraceEventRecord {
                id: submitted_trace.id.clone(),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                run_id: Some(run.id.clone()),
                session_id: Some(session_id.into()),
                event_kind: "turn_submitted".into(),
                title: submitted_trace.title.clone(),
                detail: submitted_trace.detail.clone(),
                created_at: now,
            })
            .await?;
        if let Some(execution_trace) = execution_trace.as_ref() {
            self.state
                .observation
                .append_trace(TraceEventRecord {
                    id: execution_trace.id.clone(),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: Some(project_id.clone()),
                    run_id: Some(run.id.clone()),
                    session_id: Some(session_id.into()),
                    event_kind: "turn_executed".into(),
                    title: execution_trace.title.clone(),
                    detail: execution_trace.detail.clone(),
                    created_at: now,
                })
                .await?;
        }
        self.state
            .observation
            .append_audit(AuditRecord {
                id: format!("audit-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                actor_type: "session".into(),
                actor_id: session_id.into(),
                action: "runtime.submit_turn".into(),
                resource: run.id.clone(),
                outcome: run.status.clone(),
                created_at: now,
            })
            .await?;
        self.state
            .observation
            .append_cost(CostLedgerEntry {
                id: format!("cost-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                run_id: Some(run.id.clone()),
                configured_model_id: Some(resolved_target.configured_model_id.clone()),
                metric: execution
                    .as_ref()
                    .and_then(|response| response.total_tokens)
                    .map(|_| "tokens")
                    .unwrap_or("turns")
                    .into(),
                amount: execution
                    .as_ref()
                    .and_then(|response| response.total_tokens)
                    .map(i64::from)
                    .unwrap_or(1),
                unit: execution
                    .as_ref()
                    .and_then(|response| response.total_tokens)
                    .map(|_| "tokens")
                    .unwrap_or("count")
                    .into(),
                created_at: now,
            })
            .await?;
        if let Some(consumed_tokens) = consumed_tokens {
            self.increment_configured_model_usage(
                &resolved_target.configured_model_id,
                consumed_tokens,
                now,
            )?;
        }

        let mut events = vec![
            RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.message.created".into(),
                kind: Some("runtime.message.created".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "message": user_message.clone(),
                })),
                run: None,
                message: Some(user_message),
                trace: None,
                approval: None,
                decision: None,
                summary: None,
                error: None,
            },
            RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.trace.emitted".into(),
                kind: Some("runtime.trace.emitted".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "trace": submitted_trace.clone(),
                })),
                run: None,
                message: None,
                trace: Some(submitted_trace),
                approval: None,
                decision: None,
                summary: None,
                error: None,
            },
        ];

        if let Some(message) = assistant_message.clone() {
            events.push(RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.message.created".into(),
                kind: Some("runtime.message.created".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "message": message.clone(),
                })),
                run: None,
                message: Some(message),
                trace: None,
                approval: None,
                decision: None,
                summary: None,
                error: None,
            });
        }

        if let Some(trace) = execution_trace.clone() {
            events.push(RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.trace.emitted".into(),
                kind: Some("runtime.trace.emitted".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "trace": trace.clone(),
                })),
                run: None,
                message: None,
                trace: Some(trace),
                approval: None,
                decision: None,
                summary: None,
                error: None,
            });
        }

        if let Some(approval) = approval.clone() {
            events.push(RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.approval.requested".into(),
                kind: Some("runtime.approval.requested".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "approval": approval.clone(),
                    "run": run.clone(),
                })),
                run: Some(run.clone()),
                message: None,
                trace: None,
                approval: Some(approval),
                decision: None,
                summary: None,
                error: None,
            });
        }

        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.run.updated".into(),
            kind: Some("runtime.run.updated".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id,
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            trace: None,
            approval: None,
            decision: None,
            summary: None,
            error: None,
        });

        for event in events {
            self.emit_event(session_id, event).await?;
        }

        Ok(run)
    }

    async fn resolve_approval(
        &self,
        session_id: &str,
        approval_id: &str,
        input: ResolveRuntimeApprovalInput,
    ) -> Result<RuntimeRunSnapshot, AppError> {
        let now = timestamp_now();
        let decision_status = match input.decision.as_str() {
            "approve" => "approved",
            "reject" => "rejected",
            _ => {
                return Err(AppError::invalid_input(
                    "approval decision must be approve or reject",
                ))
            }
        };

        let (
            pending_input,
            pending_actor_kind,
            pending_actor_id,
            resolved_target,
            config_snapshot_id,
        ) = {
            let sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;
            let pending = aggregate
                .detail
                .pending_approval
                .as_ref()
                .ok_or_else(|| AppError::not_found("runtime approval"))?;
            if pending.id != approval_id {
                return Err(AppError::not_found("runtime approval"));
            }

            if decision_status == "approved" {
                let pending_input = aggregate
                    .detail
                    .messages
                    .iter()
                    .rev()
                    .find(|message| {
                        message.sender_type == "user" && message.status == "waiting_approval"
                    })
                    .map(|message| message.content.clone())
                    .ok_or_else(|| AppError::runtime("pending approval input is unavailable"))?;
                let resolved_target = aggregate
                    .detail
                    .run
                    .resolved_target
                    .clone()
                    .ok_or_else(|| AppError::runtime("resolved execution target is unavailable"))?;
                (
                    Some(pending_input),
                    aggregate.detail.run.resolved_actor_kind.clone(),
                    aggregate.detail.run.resolved_actor_id.clone(),
                    Some(resolved_target),
                    Some(aggregate.detail.run.config_snapshot_id.clone()),
                )
            } else {
                (None, None, None, None, None)
            }
        };

        let configured_model = match (resolved_target.as_ref(), config_snapshot_id.as_deref()) {
            (Some(target), Some(snapshot_id)) => {
                let (registry, _) =
                    self.resolve_execution_target(snapshot_id, &target.configured_model_id)?;
                let configured_model = registry
                    .configured_model(&target.configured_model_id)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::invalid_input(format!(
                            "configured model `{}` is not registered",
                            target.configured_model_id
                        ))
                    })?;
                self.ensure_configured_model_quota_available(&configured_model)?;
                Some(configured_model)
            }
            _ => None,
        };
        let execution = match (
            pending_input.as_deref(),
            resolved_target.as_ref(),
            configured_model.as_ref(),
        ) {
            (Some(content), Some(target), Some(configured_model)) => {
                let response = self
                    .execute_resolved_turn(
                        target,
                        content,
                        pending_actor_kind.as_deref(),
                        pending_actor_id.as_deref(),
                    )
                    .await?;
                let _ = self.resolve_consumed_tokens(configured_model, &response)?;
                Some(response)
            }
            _ => None,
        };
        let consumed_tokens = match (execution.as_ref(), configured_model.as_ref()) {
            (Some(response), Some(configured_model)) => {
                self.resolve_consumed_tokens(configured_model, response)?
            }
            _ => None,
        };

        let (approval, execution_trace, assistant_message, run, conversation_id, project_id) = {
            let mut sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;
            let pending = aggregate
                .detail
                .pending_approval
                .as_mut()
                .ok_or_else(|| AppError::not_found("runtime approval"))?;
            if pending.id != approval_id {
                return Err(AppError::not_found("runtime approval"));
            }

            pending.status = decision_status.into();
            let approval = pending.clone();

            if let Some(message) = aggregate.detail.messages.iter_mut().rev().find(|message| {
                message.sender_type == "user" && message.status == "waiting_approval"
            }) {
                message.status = if decision_status == "approved" {
                    "completed".into()
                } else {
                    "blocked".into()
                };
            }

            let assistant_message = execution.as_ref().map(|response| RuntimeMessage {
                id: format!("msg-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                sender_type: "assistant".into(),
                sender_label: aggregate
                    .detail
                    .run
                    .resolved_actor_label
                    .clone()
                    .or_else(|| {
                        aggregate
                            .detail
                            .run
                            .resolved_target
                            .as_ref()
                            .map(|target| target.provider_id.clone())
                    })
                    .unwrap_or_else(|| "assistant".into()),
                content: response.content.clone(),
                timestamp: now,
                configured_model_id: aggregate
                    .detail
                    .run
                    .resolved_target
                    .as_ref()
                    .map(|target| target.configured_model_id.clone()),
                configured_model_name: aggregate
                    .detail
                    .run
                    .resolved_target
                    .as_ref()
                    .map(|target| target.configured_model_name.clone()),
                model_id: aggregate
                    .detail
                    .run
                    .resolved_target
                    .as_ref()
                    .map(|target| target.registry_model_id.clone()),
                status: "completed".into(),
                requested_actor_kind: aggregate.detail.run.requested_actor_kind.clone(),
                requested_actor_id: aggregate.detail.run.requested_actor_id.clone(),
                resolved_actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
                resolved_actor_id: aggregate.detail.run.resolved_actor_id.clone(),
                resolved_actor_label: aggregate.detail.run.resolved_actor_label.clone(),
                used_default_actor: Some(aggregate.detail.run.resolved_actor_id.is_none()),
                resource_ids: Some(Vec::new()),
                attachments: Some(Vec::new()),
                artifacts: Some(vec![format!("artifact-{}", aggregate.detail.run.id)]),
                usage: None,
                tool_calls: None,
                process_entries: None,
            });
            if let Some(message) = assistant_message.as_ref() {
                aggregate.detail.messages.push(message.clone());
            }

            let execution_trace = execution.as_ref().map(|response| RuntimeTraceItem {
                id: format!("trace-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                run_id: aggregate.detail.run.id.clone(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                kind: "step".into(),
                title: "Model execution completed".into(),
                detail: format!(
                    "Approved turn executed and produced {} characters.",
                    response.content.chars().count()
                ),
                tone: "success".into(),
                timestamp: now,
                actor: aggregate
                    .detail
                    .run
                    .resolved_actor_label
                    .clone()
                    .unwrap_or_else(|| "assistant".into()),
                actor_kind: aggregate.detail.run.resolved_actor_kind.clone(),
                actor_id: aggregate.detail.run.resolved_actor_id.clone(),
                related_message_id: assistant_message.as_ref().map(|message| message.id.clone()),
                related_tool_name: None,
            });
            if let Some(trace) = execution_trace.as_ref() {
                aggregate.detail.trace.push(trace.clone());
            }

            aggregate.detail.run.status = if decision_status == "approved" {
                "completed".into()
            } else {
                "blocked".into()
            };
            aggregate.detail.run.current_step = if decision_status == "approved" {
                "completed".into()
            } else {
                "approval_rejected".into()
            };
            aggregate.detail.run.updated_at = now;
            aggregate.detail.run.consumed_tokens = consumed_tokens;
            aggregate.detail.run.next_action = Some(if decision_status == "approved" {
                "idle".into()
            } else {
                "blocked".into()
            });
            aggregate.detail.summary.status = aggregate.detail.run.status.clone();
            aggregate.detail.summary.updated_at = now;
            aggregate.detail.summary.last_message_preview = Some(
                assistant_message
                    .as_ref()
                    .map(|message| message.content.clone())
                    .or_else(|| {
                        aggregate
                            .detail
                            .messages
                            .iter()
                            .rev()
                            .find(|message| message.sender_type == "user")
                            .map(|message| message.content.clone())
                    })
                    .unwrap_or_default(),
            );

            aggregate.detail.pending_approval = None;
            let run = aggregate.detail.run.clone();
            let conversation_id = aggregate.detail.summary.conversation_id.clone();
            let project_id = aggregate.detail.summary.project_id.clone();
            self.persist_session(session_id, aggregate)?;
            (
                approval,
                execution_trace,
                assistant_message,
                run,
                conversation_id,
                project_id,
            )
        };

        self.state
            .observation
            .append_trace(TraceEventRecord {
                id: format!("trace-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                run_id: Some(run.id.clone()),
                session_id: Some(session_id.into()),
                event_kind: "approval_resolved".into(),
                title: "Approval resolved".into(),
                detail: input.decision.clone(),
                created_at: now,
            })
            .await?;
        if let Some(execution_trace) = execution_trace.as_ref() {
            self.state
                .observation
                .append_trace(TraceEventRecord {
                    id: execution_trace.id.clone(),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: Some(project_id.clone()),
                    run_id: Some(run.id.clone()),
                    session_id: Some(session_id.into()),
                    event_kind: "turn_executed".into(),
                    title: execution_trace.title.clone(),
                    detail: execution_trace.detail.clone(),
                    created_at: now,
                })
                .await?;
        }
        self.state
            .observation
            .append_audit(AuditRecord {
                id: format!("audit-{}", Uuid::new_v4()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                actor_type: "session".into(),
                actor_id: session_id.into(),
                action: "runtime.resolve_approval".into(),
                resource: approval.id.clone(),
                outcome: input.decision.clone(),
                created_at: now,
            })
            .await?;
        if let Some(response) = execution.as_ref() {
            self.state
                .observation
                .append_cost(CostLedgerEntry {
                    id: format!("cost-{}", Uuid::new_v4()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: Some(project_id.clone()),
                    run_id: Some(run.id.clone()),
                    configured_model_id: run
                        .resolved_target
                        .as_ref()
                        .map(|target| target.configured_model_id.clone()),
                    metric: response
                        .total_tokens
                        .map(|_| "tokens")
                        .unwrap_or("turns")
                        .into(),
                    amount: response.total_tokens.map(i64::from).unwrap_or(1),
                    unit: response
                        .total_tokens
                        .map(|_| "tokens")
                        .unwrap_or("count")
                        .into(),
                    created_at: now,
                })
                .await?;
        }
        if let (Some(consumed_tokens), Some(resolved_target)) =
            (consumed_tokens, run.resolved_target.as_ref())
        {
            self.increment_configured_model_usage(
                &resolved_target.configured_model_id,
                consumed_tokens,
                now,
            )?;
        }

        let mut events = vec![RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.approval.resolved".into(),
            kind: Some("runtime.approval.resolved".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id: conversation_id.clone(),
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "approval": approval.clone(),
                "decision": input.decision.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            trace: None,
            approval: Some(approval.clone()),
            decision: Some(input.decision.clone()),
            summary: None,
            error: None,
        }];

        if let Some(message) = assistant_message.clone() {
            events.push(RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.message.created".into(),
                kind: Some("runtime.message.created".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "message": message.clone(),
                })),
                run: None,
                message: Some(message),
                trace: None,
                approval: None,
                decision: None,
                summary: None,
                error: None,
            });
        }
        if let Some(trace) = execution_trace.clone() {
            events.push(RuntimeEventEnvelope {
                id: format!("evt-{}", Uuid::new_v4()),
                event_type: "runtime.trace.emitted".into(),
                kind: Some("runtime.trace.emitted".into()),
                workspace_id: self.state.workspace_id.clone(),
                project_id: optional_project_id(&project_id),
                session_id: session_id.into(),
                conversation_id: conversation_id.clone(),
                run_id: Some(run.id.clone()),
                emitted_at: now,
                sequence: 0,
                payload: Some(json!({
                    "trace": trace.clone(),
                })),
                run: None,
                message: None,
                trace: Some(trace),
                approval: None,
                decision: None,
                summary: None,
                error: None,
            });
        }
        events.push(RuntimeEventEnvelope {
            id: format!("evt-{}", Uuid::new_v4()),
            event_type: "runtime.run.updated".into(),
            kind: Some("runtime.run.updated".into()),
            workspace_id: self.state.workspace_id.clone(),
            project_id: optional_project_id(&project_id),
            session_id: session_id.into(),
            conversation_id,
            run_id: Some(run.id.clone()),
            emitted_at: now,
            sequence: 0,
            payload: Some(json!({
                "approval": approval.clone(),
                "decision": input.decision.clone(),
                "run": run.clone(),
            })),
            run: Some(run.clone()),
            message: None,
            trace: None,
            approval: Some(approval),
            decision: Some(input.decision),
            summary: None,
            error: None,
        });

        for event in events {
            self.emit_event(session_id, event).await?;
        }

        Ok(run)
    }

    async fn subscribe_events(
        &self,
        session_id: &str,
    ) -> Result<broadcast::Receiver<RuntimeEventEnvelope>, AppError> {
        Ok(self.session_sender(session_id)?.subscribe())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{fs, path::Path};

    use async_trait::async_trait;
    use octopus_core::CreateRuntimeSessionInput;
    use octopus_infra::build_infra_bundle;
    use octopus_platform::{
        ModelRegistryService, RuntimeConfigService, RuntimeExecutionService, RuntimeSessionService,
    };
    use rusqlite::params;
    use serde_json::json;

    fn test_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("octopus-runtime-adapter-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("test root");
        root
    }

    fn write_json(path: &Path, value: serde_json::Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("config dir");
        }
        fs::write(path, serde_json::to_vec_pretty(&value).expect("json")).expect("write config");
    }

    fn write_workspace_config(path: &Path, total_tokens: Option<u64>) {
        let configured_model = if let Some(total_tokens) = total_tokens {
            json!({
                "configuredModelId": "quota-model",
                "name": "Quota Model",
                "providerId": "anthropic",
                "modelId": "claude-sonnet-4-5",
                "credentialRef": "env:ANTHROPIC_API_KEY",
                "tokenQuota": {
                    "totalTokens": total_tokens
                },
                "enabled": true,
                "source": "workspace"
            })
        } else {
            json!({
                "configuredModelId": "quota-model",
                "name": "Quota Model",
                "providerId": "anthropic",
                "modelId": "claude-sonnet-4-5",
                "credentialRef": "env:ANTHROPIC_API_KEY",
                "enabled": true,
                "source": "workspace"
            })
        };

        write_json(
            path,
            json!({
                "configuredModels": {
                    "quota-model": configured_model
                }
            }),
        );
    }

    #[derive(Debug, Clone)]
    struct FixedTokenRuntimeModelExecutor {
        total_tokens: Option<u32>,
    }

    #[async_trait]
    impl RuntimeModelExecutor for FixedTokenRuntimeModelExecutor {
        async fn execute_turn(
            &self,
            _target: &ResolvedExecutionTarget,
            input: &str,
            system_prompt: Option<&str>,
        ) -> Result<ExecutionResponse, AppError> {
            let prompt_prefix = system_prompt
                .map(|value| format!(" [{value}]"))
                .unwrap_or_default();
            Ok(ExecutionResponse {
                content: format!("fixed token response{prompt_prefix} -> {input}"),
                request_id: Some("fixed-token-request".into()),
                total_tokens: self.total_tokens,
            })
        }
    }

    #[tokio::test]
    async fn runtime_config_resolution_respects_user_workspace_project_precedence() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(MockRuntimeModelExecutor),
        );

        let user_id = "user-owner";
        let project_id = "proj-redesign";

        write_json(
            &infra
                .paths
                .runtime_user_config_dir
                .join(format!("{user_id}.json")),
            json!({
                "model": "user-model",
                "provider": {
                    "defaultModel": "user-default"
                },
                "permissions": {
                    "defaultMode": "readonly"
                },
                "shared": {
                    "marker": "user",
                    "userOnly": true
                }
            }),
        );
        write_json(
            &infra.paths.runtime_config_dir.join("workspace.json"),
            json!({
                "model": "workspace-model",
                "permissions": {
                    "defaultMode": "plan"
                },
                "shared": {
                    "marker": "workspace",
                    "workspaceOnly": true
                }
            }),
        );
        write_json(
            &infra
                .paths
                .runtime_project_config_dir
                .join(format!("{project_id}.json")),
            json!({
                "model": "project-model",
                "shared": {
                    "marker": "project",
                    "projectOnly": true
                }
            }),
        );

        let workspace_config = adapter.get_config().await.expect("workspace config");
        assert_eq!(
            workspace_config
                .sources
                .iter()
                .map(|source| source.scope.as_str())
                .collect::<Vec<_>>(),
            vec!["workspace"]
        );
        assert_eq!(
            workspace_config.effective_config.get("model"),
            Some(&json!("workspace-model"))
        );
        assert_eq!(workspace_config.effective_config.get("provider"), None);

        let user_config = adapter.get_user_config(user_id).await.expect("user config");
        assert_eq!(
            user_config
                .sources
                .iter()
                .map(|source| source.source_key.clone())
                .collect::<Vec<_>>(),
            vec![format!("user:{user_id}"), "workspace".to_string()]
        );
        assert_eq!(
            user_config.effective_config.get("model"),
            Some(&json!("workspace-model"))
        );
        assert_eq!(
            user_config
                .effective_config
                .pointer("/permissions/defaultMode"),
            Some(&json!("plan"))
        );
        assert_eq!(
            user_config
                .effective_config
                .pointer("/provider/defaultModel"),
            Some(&json!("user-default"))
        );
        assert_eq!(
            user_config.effective_config.pointer("/shared/marker"),
            Some(&json!("workspace"))
        );
        assert_eq!(
            user_config.effective_config.pointer("/shared/userOnly"),
            Some(&json!(true))
        );
        assert_eq!(
            user_config
                .effective_config
                .pointer("/shared/workspaceOnly"),
            Some(&json!(true))
        );

        let project_config = adapter
            .get_project_config(project_id, user_id)
            .await
            .expect("project config");
        assert_eq!(
            project_config
                .sources
                .iter()
                .map(|source| source.source_key.clone())
                .collect::<Vec<_>>(),
            vec![
                format!("user:{user_id}"),
                "workspace".to_string(),
                format!("project:{project_id}"),
            ]
        );
        assert_eq!(
            project_config.effective_config.get("model"),
            Some(&json!("project-model"))
        );
        assert_eq!(
            project_config
                .effective_config
                .pointer("/permissions/defaultMode"),
            Some(&json!("plan"))
        );
        assert_eq!(
            project_config
                .effective_config
                .pointer("/provider/defaultModel"),
            Some(&json!("user-default"))
        );
        assert_eq!(
            project_config.effective_config.pointer("/shared/marker"),
            Some(&json!("project"))
        );
        assert_eq!(
            project_config.effective_config.pointer("/shared/userOnly"),
            Some(&json!(true))
        );
        assert_eq!(
            project_config
                .effective_config
                .pointer("/shared/workspaceOnly"),
            Some(&json!(true))
        );
        assert_eq!(
            project_config
                .effective_config
                .pointer("/shared/projectOnly"),
            Some(&json!(true))
        );

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn runtime_session_snapshot_uses_scope_order_from_user_to_project() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(MockRuntimeModelExecutor),
        );

        let user_id = "user-owner";
        let project_id = "proj-redesign";

        write_json(
            &infra
                .paths
                .runtime_user_config_dir
                .join(format!("{user_id}.json")),
            json!({ "model": "user-model" }),
        );
        write_json(
            &infra.paths.runtime_config_dir.join("workspace.json"),
            json!({ "model": "workspace-model" }),
        );
        write_json(
            &infra
                .paths
                .runtime_project_config_dir
                .join(format!("{project_id}.json")),
            json!({ "model": "project-model" }),
        );

        let detail = adapter
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: "conv-1".into(),
                    project_id: project_id.into(),
                    title: "Runtime precedence".into(),
                    session_kind: None,
                },
                user_id,
            )
            .await
            .expect("session");

        assert_eq!(
            detail.summary.started_from_scope_set,
            vec![
                "user".to_string(),
                "workspace".to_string(),
                "project".to_string()
            ]
        );

        let connection = Connection::open(&infra.paths.db_path).expect("db");
        let source_refs: String = connection
            .query_row(
                "SELECT source_refs FROM runtime_config_snapshots WHERE id = ?1",
                [&detail.summary.config_snapshot_id],
                |row| row.get(0),
            )
            .expect("source refs");
        assert_eq!(
            source_refs,
            json!([
                format!("user:{user_id}"),
                "workspace",
                format!("project:{project_id}"),
            ])
            .to_string()
        );

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn runtime_config_validation_rejects_non_positive_token_quota() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(MockRuntimeModelExecutor),
        );

        let validation = adapter
            .validate_config(RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "configuredModels": {
                        "quota-model": {
                            "configuredModelId": "quota-model",
                            "name": "Quota Model",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "tokenQuota": {
                                "totalTokens": 0
                            },
                            "enabled": true,
                            "source": "workspace"
                        }
                    }
                }),
            })
            .await
            .expect("validation result");

        assert!(!validation.valid);
        assert!(validation
            .errors
            .iter()
            .any(|error| error.contains("tokenQuota.totalTokens")));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn submit_turn_updates_configured_model_token_usage_and_catalog_snapshot() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        write_workspace_config(
            &infra.paths.runtime_config_dir.join("workspace.json"),
            Some(100),
        );

        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(FixedTokenRuntimeModelExecutor {
                total_tokens: Some(32),
            }),
        );

        let session = adapter
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: "conv-quota".into(),
                    project_id: "".into(),
                    title: "Quota Session".into(),
                    session_kind: None,
                },
                "user-owner",
            )
            .await
            .expect("session");

        let run = adapter
            .submit_turn(
                &session.summary.id,
                SubmitRuntimeTurnInput {
                    content: "Count tokens".into(),
                    model_id: None,
                    configured_model_id: Some("quota-model".into()),
                    permission_mode: "readonly".into(),
                    actor_kind: None,
                    actor_id: None,
                },
            )
            .await
            .expect("run");

        assert_eq!(run.consumed_tokens, Some(32));

        let catalog = adapter.catalog_snapshot().await.expect("catalog snapshot");
        let configured_model = catalog
            .configured_models
            .iter()
            .find(|model| model.configured_model_id == "quota-model")
            .expect("configured model");
        assert_eq!(
            configured_model
                .token_quota
                .as_ref()
                .and_then(|quota| quota.total_tokens),
            Some(100)
        );
        assert_eq!(configured_model.token_usage.used_tokens, 32);
        assert_eq!(configured_model.token_usage.remaining_tokens, Some(68));
        assert!(!configured_model.token_usage.exhausted);

        let connection = Connection::open(&infra.paths.db_path).expect("db");
        let used_tokens: i64 = connection
            .query_row(
                "SELECT used_tokens FROM configured_model_usage_projections WHERE configured_model_id = ?1",
                ["quota-model"],
                |row| row.get(0),
            )
            .expect("used tokens");
        assert_eq!(used_tokens, 32);
        let cost_configured_model_id: String = connection
            .query_row(
                "SELECT configured_model_id FROM cost_entries WHERE run_id = ?1 ORDER BY created_at DESC LIMIT 1",
                [&run.id],
                |row| row.get(0),
            )
            .expect("cost configured model id");
        assert_eq!(cost_configured_model_id, "quota-model");

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn configured_model_token_usage_survives_adapter_restart() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        write_workspace_config(
            &infra.paths.runtime_config_dir.join("workspace.json"),
            Some(100),
        );

        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(FixedTokenRuntimeModelExecutor {
                total_tokens: Some(24),
            }),
        );

        let session = adapter
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: "conv-restart".into(),
                    project_id: "".into(),
                    title: "Restart Session".into(),
                    session_kind: None,
                },
                "user-owner",
            )
            .await
            .expect("session");
        adapter
            .submit_turn(
                &session.summary.id,
                SubmitRuntimeTurnInput {
                    content: "Persist usage".into(),
                    model_id: None,
                    configured_model_id: Some("quota-model".into()),
                    permission_mode: "readonly".into(),
                    actor_kind: None,
                    actor_id: None,
                },
            )
            .await
            .expect("run");

        let reloaded = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(FixedTokenRuntimeModelExecutor {
                total_tokens: Some(24),
            }),
        );
        let catalog = reloaded.catalog_snapshot().await.expect("catalog snapshot");
        let configured_model = catalog
            .configured_models
            .iter()
            .find(|model| model.configured_model_id == "quota-model")
            .expect("configured model");
        assert_eq!(configured_model.token_usage.used_tokens, 24);
        assert_eq!(configured_model.token_usage.remaining_tokens, Some(76));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn submit_turn_blocks_when_configured_model_token_quota_is_exhausted() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        write_workspace_config(
            &infra.paths.runtime_config_dir.join("workspace.json"),
            Some(32),
        );

        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(FixedTokenRuntimeModelExecutor {
                total_tokens: Some(32),
            }),
        );

        let first_session = adapter
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: "conv-first".into(),
                    project_id: "".into(),
                    title: "First Session".into(),
                    session_kind: None,
                },
                "user-owner",
            )
            .await
            .expect("first session");
        let first_run = adapter
            .submit_turn(
                &first_session.summary.id,
                SubmitRuntimeTurnInput {
                    content: "Use the full quota".into(),
                    model_id: None,
                    configured_model_id: Some("quota-model".into()),
                    permission_mode: "readonly".into(),
                    actor_kind: None,
                    actor_id: None,
                },
            )
            .await
            .expect("first run");
        assert_eq!(first_run.consumed_tokens, Some(32));

        let second_session = adapter
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: "conv-second".into(),
                    project_id: "".into(),
                    title: "Second Session".into(),
                    session_kind: None,
                },
                "user-owner",
            )
            .await
            .expect("second session");
        let error = adapter
            .submit_turn(
                &second_session.summary.id,
                SubmitRuntimeTurnInput {
                    content: "This should be blocked".into(),
                    model_id: None,
                    configured_model_id: Some("quota-model".into()),
                    permission_mode: "readonly".into(),
                    actor_kind: None,
                    actor_id: None,
                },
            )
            .await
            .expect_err("quota exhaustion should block new requests");
        assert!(error
            .to_string()
            .contains("has reached its total token limit"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn submit_turn_injects_selected_agent_prompt_into_execution() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        write_workspace_config(
            &infra.paths.runtime_config_dir.join("workspace.json"),
            Some(100),
        );

        let connection = Connection::open(&infra.paths.db_path).expect("db");
        connection
            .execute(
                "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    "agent-project-delivery",
                    octopus_core::DEFAULT_WORKSPACE_ID,
                    octopus_core::DEFAULT_PROJECT_ID,
                    "project",
                    "Project Delivery Agent",
                    Option::<String>::None,
                    "Structured and pragmatic",
                    serde_json::to_string(&vec!["project", "delivery"]).expect("tags"),
                    "Always answer with an implementation plan first.",
                    serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                    serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                    serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                    "Tracks project work, runtime sessions, and follow-up actions.",
                    "active",
                    timestamp_now() as i64,
                ],
            )
            .expect("upsert agent prompt");
        drop(connection);

        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(MockRuntimeModelExecutor),
        );

        let session = adapter
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: "conv-agent-actor".into(),
                    project_id: octopus_core::DEFAULT_PROJECT_ID.into(),
                    title: "Agent Actor Session".into(),
                    session_kind: None,
                },
                "user-owner",
            )
            .await
            .expect("session");

        let run = adapter
            .submit_turn(
                &session.summary.id,
                SubmitRuntimeTurnInput {
                    content: "Design the rollout".into(),
                    model_id: None,
                    configured_model_id: Some("quota-model".into()),
                    permission_mode: "readonly".into(),
                    actor_kind: Some("agent".into()),
                    actor_id: Some("agent-project-delivery".into()),
                },
            )
            .await
            .expect("run");

        assert_eq!(run.resolved_actor_kind.as_deref(), Some("agent"));
        assert_eq!(
            run.resolved_actor_id.as_deref(),
            Some("agent-project-delivery")
        );

        let detail = adapter
            .get_session(&session.summary.id)
            .await
            .expect("session detail");
        let assistant_message = detail
            .messages
            .iter()
            .find(|message| message.sender_type == "assistant")
            .expect("assistant message");
        assert!(assistant_message.content.contains("You are the agent `"));
        assert!(assistant_message.content.contains("Project Delivery Agent"));
        assert!(assistant_message
            .content
            .contains("Personality: Structured and pragmatic"));
        assert!(assistant_message
            .content
            .contains("Instructions: Always answer with an implementation plan first."));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn resolve_approval_reuses_selected_team_prompt_for_execution() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        write_workspace_config(
            &infra.paths.runtime_config_dir.join("workspace.json"),
            Some(100),
        );

        let connection = Connection::open(&infra.paths.db_path).expect("db");
        connection
            .execute(
                "UPDATE teams SET personality = ?2, prompt = ?3, leader_agent_id = ?4, member_agent_ids = ?5 WHERE id = ?1",
                params![
                    "team-workspace-core",
                    "Cross-functional design review board",
                    "Debate options, then return a single aligned answer.",
                    "agent-orchestrator",
                    serde_json::to_string(&vec!["agent-orchestrator", "agent-project-delivery"]).expect("member ids"),
                ],
            )
            .expect("update team prompt");
        drop(connection);

        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(MockRuntimeModelExecutor),
        );

        let session = adapter
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: "conv-team-actor".into(),
                    project_id: octopus_core::DEFAULT_PROJECT_ID.into(),
                    title: "Team Actor Session".into(),
                    session_kind: None,
                },
                "user-owner",
            )
            .await
            .expect("session");

        let pending = adapter
            .submit_turn(
                &session.summary.id,
                SubmitRuntimeTurnInput {
                    content: "Review the proposal".into(),
                    model_id: None,
                    configured_model_id: Some("quota-model".into()),
                    permission_mode: "workspace-write".into(),
                    actor_kind: Some("team".into()),
                    actor_id: Some("team-workspace-core".into()),
                },
            )
            .await
            .expect("pending run");
        assert_eq!(pending.status, "waiting_approval");

        let approval_id = adapter
            .get_session(&session.summary.id)
            .await
            .expect("session detail")
            .pending_approval
            .as_ref()
            .map(|approval| approval.id.clone())
            .expect("approval id");

        let resolved = adapter
            .resolve_approval(
                &session.summary.id,
                &approval_id,
                ResolveRuntimeApprovalInput {
                    decision: "approve".into(),
                },
            )
            .await
            .expect("approved run");
        assert_eq!(resolved.status, "completed");

        let detail = adapter
            .get_session(&session.summary.id)
            .await
            .expect("session detail after approval");
        let assistant_message = detail
            .messages
            .iter()
            .rev()
            .find(|message| message.sender_type == "assistant")
            .expect("assistant message");
        assert!(assistant_message
            .content
            .contains("You are the team `Workspace Core` operating as a single execution actor."));
        assert!(assistant_message
            .content
            .contains("Team personality: Cross-functional design review board"));
        assert!(assistant_message
            .content
            .contains("Team instructions: Debate options, then return a single aligned answer."));
        assert!(assistant_message
            .content
            .contains("Leader agent id: agent-orchestrator"));
        assert!(assistant_message
            .content
            .contains("Member agent ids: agent-orchestrator, agent-project-delivery"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn quota_enabled_models_require_provider_token_usage_metadata() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        write_workspace_config(
            &infra.paths.runtime_config_dir.join("workspace.json"),
            Some(64),
        );

        let adapter = RuntimeAdapter::new_with_executor(
            octopus_core::DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            Arc::new(FixedTokenRuntimeModelExecutor { total_tokens: None }),
        );

        let session = adapter
            .create_session(
                CreateRuntimeSessionInput {
                    conversation_id: "conv-missing-usage".into(),
                    project_id: "".into(),
                    title: "Missing Usage".into(),
                    session_kind: None,
                },
                "user-owner",
            )
            .await
            .expect("session");
        let error = adapter
            .submit_turn(
                &session.summary.id,
                SubmitRuntimeTurnInput {
                    content: "This should fail".into(),
                    model_id: None,
                    configured_model_id: Some("quota-model".into()),
                    permission_mode: "readonly".into(),
                    actor_kind: None,
                    actor_id: None,
                },
            )
            .await
            .expect_err("missing token usage should fail");
        assert!(error
            .to_string()
            .contains("requires provider token usage for quota enforcement"));

        let connection = Connection::open(&infra.paths.db_path).expect("db");
        let usage_row: Option<i64> = connection
            .query_row(
                "SELECT used_tokens FROM configured_model_usage_projections WHERE configured_model_id = ?1",
                ["quota-model"],
                |row| row.get(0),
            )
            .optional()
            .expect("usage row");
        assert_eq!(usage_row, None);

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }
}
