use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use api as _;
use async_trait::async_trait;
use octopus_core::{
    normalize_runtime_permission_mode_label, timestamp_now, AppError, ApprovalRequestRecord,
    AuditRecord, CostLedgerEntry, CreateRuntimeSessionInput, ProviderConfig,
    ResolveRuntimeApprovalInput, RuntimeBootstrap, RuntimeConfigPatch, RuntimeConfigSource,
    RuntimeConfigValidationResult, RuntimeConfigSnapshotSummary, RuntimeEffectiveConfig,
    RuntimeEventEnvelope, RuntimeMessage, RuntimeRunSnapshot, RuntimeSecretReferenceStatus,
    RuntimeSessionDetail, RuntimeSessionSummary, RuntimeTraceItem, SubmitRuntimeTurnInput,
    TraceEventRecord, RUNTIME_PERMISSION_WORKSPACE_WRITE,
};
use octopus_infra::WorkspacePaths;
use octopus_platform::{
    ObservationService, RuntimeConfigService, RuntimeExecutionService, RuntimeSessionService,
};
use plugins as _;
use rusqlite::{params, Connection};
use runtime::{apply_config_patch, ConfigDocument, ConfigLoader, ConfigSource, JsonValue};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::broadcast;
use tools as _;
use uuid::Uuid;

#[derive(Clone)]
pub struct RuntimeAdapter {
    state: Arc<RuntimeState>,
}

struct RuntimeState {
    workspace_id: String,
    paths: WorkspacePaths,
    observation: Arc<dyn ObservationService>,
    config_loader: ConfigLoader,
    sessions: Mutex<HashMap<String, RuntimeAggregate>>,
    order: Mutex<Vec<String>>,
    broadcasters: Mutex<HashMap<String, broadcast::Sender<RuntimeEventEnvelope>>>,
}

#[derive(Clone)]
struct RuntimeAggregate {
    detail: RuntimeSessionDetail,
    events: Vec<RuntimeEventEnvelope>,
}

fn optional_project_id(project_id: &str) -> Option<String> {
    if project_id.is_empty() {
        None
    } else {
        Some(project_id.to_string())
    }
}

impl RuntimeAdapter {
    pub fn new(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
    ) -> Self {
        let config_loader = ConfigLoader::new(&paths.root, paths.config_dir.join(".claw"));
        let adapter = Self {
            state: Arc::new(RuntimeState {
                workspace_id: workspace_id.into(),
                paths,
                observation,
                config_loader,
                sessions: Mutex::new(HashMap::new()),
                order: Mutex::new(Vec::new()),
                broadcasters: Mutex::new(HashMap::new()),
            }),
        };

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
            sessions.insert(detail.summary.id.clone(), RuntimeAggregate { detail, events });
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

    fn persist_session(&self, session_id: &str, aggregate: &RuntimeAggregate) -> Result<(), AppError> {
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
                 (id, conversation_id, project_id, title, status, updated_at, last_message_preview,
                  config_snapshot_id, effective_config_hash, started_from_scope_set, detail_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    summary.id,
                    summary.conversation_id,
                    summary.project_id,
                    summary.title,
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
                 (id, effective_config_hash, started_from_scope_set, source_paths, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    snapshot.id,
                    snapshot.effective_config_hash,
                    serde_json::to_string(&snapshot.started_from_scope_set)?,
                    serde_json::to_string(&snapshot.source_paths)?,
                    snapshot.created_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
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
            JsonValue::Array(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(Self::runtime_json_to_serde)
                    .collect(),
            ),
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

    fn scope_label(scope: ConfigSource) -> &'static str {
        match scope {
            ConfigSource::User => "user",
            ConfigSource::Project => "project",
            ConfigSource::Local => "local",
        }
    }

    fn parse_scope(scope: &str) -> Result<ConfigSource, AppError> {
        match scope {
            "user" => Ok(ConfigSource::User),
            "project" => Ok(ConfigSource::Project),
            "local" => Ok(ConfigSource::Local),
            other => Err(AppError::invalid_input(format!(
                "unsupported runtime config scope: {other}"
            ))),
        }
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
        documents: &[ConfigDocument],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        match self.state.config_loader.load_from_documents(documents) {
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

    fn build_effective_config(
        &self,
        documents: &[ConfigDocument],
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let runtime_config = self
            .state
            .config_loader
            .load_from_documents(documents)
            .map_err(|error| AppError::runtime(error.to_string()))?;
        let mut secret_references = Vec::new();
        let effective_value = Self::runtime_json_to_serde(&runtime_config.as_json());
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
                        Self::scope_label(document.source),
                        "",
                        value,
                        &mut secret_references,
                    )
                });
                let content_hash = document_value
                    .as_ref()
                    .map(Self::hash_value)
                    .transpose()?;

                Ok(RuntimeConfigSource {
                    scope: Self::scope_label(document.source).to_string(),
                    path: document.path.display().to_string(),
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

    fn patched_documents(
        &self,
        scope: &str,
        patch: &serde_json::Value,
    ) -> Result<Vec<ConfigDocument>, AppError> {
        let target_scope = Self::parse_scope(scope)?;
        let patch = Self::serde_to_runtime_json(patch)?;
        let patch_object = patch.as_object().ok_or_else(|| {
            AppError::invalid_input("runtime config patch must be a JSON object")
        })?;

        let target_path = self.state.config_loader.writable_path(target_scope);
        let mut documents = self
            .state
            .config_loader
            .load_documents()
            .map_err(|error| AppError::runtime(error.to_string()))?;

        let target_document = documents
            .iter_mut()
            .find(|document| document.path == target_path)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        let mut next = target_document.document.clone().unwrap_or_default();
        apply_config_patch(&mut next, patch_object);
        target_document.exists = true;
        target_document.loaded = true;
        target_document.document = Some(next);

        Ok(documents)
    }

    fn write_document(
        &self,
        scope: &str,
        document: &ConfigDocument,
    ) -> Result<(), AppError> {
        let path = self.state.config_loader.writable_path(Self::parse_scope(scope)?);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let document = document
            .document
            .as_ref()
            .cloned()
            .unwrap_or_default();
        let rendered = serde_json::to_vec_pretty(&Self::runtime_json_to_serde(&JsonValue::Object(
            document,
        )))?;
        fs::write(path, rendered)?;
        Ok(())
    }

    fn current_config_snapshot(&self) -> Result<RuntimeConfigSnapshotSummary, AppError> {
        let documents = self
            .state
            .config_loader
            .load_documents()
            .map_err(|error| AppError::runtime(error.to_string()))?;
        let effective = self.build_effective_config(&documents)?;
        let source_paths = documents
            .iter()
            .filter(|document| document.loaded)
            .map(|document| document.path.display().to_string())
            .collect::<Vec<_>>();
        let mut started_from_scope_set = Vec::new();
        for document in &documents {
            let scope = Self::scope_label(document.source).to_string();
            if document.loaded && !started_from_scope_set.iter().any(|existing| existing == &scope) {
                started_from_scope_set.push(scope);
            }
        }

        Ok(RuntimeConfigSnapshotSummary {
            id: format!("cfgsnap-{}", Uuid::new_v4()),
            effective_config_hash: effective.effective_config_hash,
            started_from_scope_set,
            source_paths,
            created_at: timestamp_now(),
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
}

#[async_trait]
impl RuntimeSessionService for RuntimeAdapter {
    async fn bootstrap(&self) -> Result<RuntimeBootstrap, AppError> {
        Ok(RuntimeBootstrap {
            provider: ProviderConfig {
                provider: "anthropic".into(),
                api_key: None,
                base_url: None,
                default_model: Some("claude-sonnet-4-5".into()),
            },
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
    ) -> Result<RuntimeSessionDetail, AppError> {
        let session_id = format!("rt-{}", Uuid::new_v4());
        let conversation_id = if input.conversation_id.is_empty() {
            format!("conv-{}", Uuid::new_v4())
        } else {
            input.conversation_id
        };
        let run_id = format!("run-{}", Uuid::new_v4());
        let now = timestamp_now();
        let snapshot = self.current_config_snapshot()?;
        self.persist_config_snapshot(&snapshot)?;

        let detail = RuntimeSessionDetail {
            summary: RuntimeSessionSummary {
                id: session_id.clone(),
                conversation_id: conversation_id.clone(),
                project_id: input.project_id,
                title: input.title,
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
                model_id: None,
                next_action: Some("submit_turn".into()),
                config_snapshot_id: snapshot.id.clone(),
                effective_config_hash: snapshot.effective_config_hash,
                started_from_scope_set: snapshot.started_from_scope_set,
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
}

#[async_trait]
impl RuntimeConfigService for RuntimeAdapter {
    async fn get_config(&self) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self
            .state
            .config_loader
            .load_documents()
            .map_err(|error| AppError::runtime(error.to_string()))?;
        let mut effective = self.build_effective_config(&documents)?;
        effective.validation = self.validate_documents(&documents)?;
        Ok(effective)
    }

    async fn validate_config(
        &self,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let documents = self.patched_documents(&patch.scope, &patch.patch)?;
        self.validate_documents(&documents)
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

        let documents = self.patched_documents(scope, &patch.patch)?;
        let validation = self.validate_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target_scope = Self::parse_scope(scope)?;
        let target_path = self.state.config_loader.writable_path(target_scope);
        let target = documents
            .iter()
            .find(|document| document.path == target_path)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        self.write_document(scope, target)?;

        let reloaded = self
            .state
            .config_loader
            .load_documents()
            .map_err(|error| AppError::runtime(error.to_string()))?;
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
        let normalized_permission_mode = normalize_runtime_permission_mode_label(&input.permission_mode)
            .ok_or_else(|| AppError::invalid_input(format!(
                "unsupported permission mode: {}",
                input.permission_mode
            )))?;
        let requires_approval = normalized_permission_mode == RUNTIME_PERMISSION_WORKSPACE_WRITE;

        let (message, trace, approval, run, conversation_id, project_id) = {
            let mut sessions = self
                .state
                .sessions
                .lock()
                .map_err(|_| AppError::runtime("runtime sessions mutex poisoned"))?;
            let aggregate = sessions
                .get_mut(session_id)
                .ok_or_else(|| AppError::not_found("runtime session"))?;

            let message = RuntimeMessage {
                id: format!("msg-{}", Uuid::new_v4()),
                session_id: session_id.into(),
                conversation_id: aggregate.detail.summary.conversation_id.clone(),
                sender_type: "user".into(),
                sender_label: "User".into(),
                content: input.content.clone(),
                timestamp: now,
                model_id: Some(input.model_id.clone()),
                status: if requires_approval {
                    "waiting_approval".into()
                } else {
                    "completed".into()
                },
            };
            aggregate.detail.messages.push(message.clone());

            let trace = RuntimeTraceItem {
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
                actor: "user".into(),
                related_message_id: Some(message.id.clone()),
                related_tool_name: None,
            };
            aggregate.detail.trace.push(trace.clone());

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

            aggregate.detail.summary.status = if requires_approval {
                "waiting_approval".into()
            } else {
                "completed".into()
            };
            aggregate.detail.summary.updated_at = now;
            aggregate.detail.summary.last_message_preview = Some(input.content.clone());
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
            aggregate.detail.run.model_id = Some(input.model_id);
            aggregate.detail.run.next_action = Some(if requires_approval {
                "approval".into()
            } else {
                "idle".into()
            });

            let run = aggregate.detail.run.clone();
            let conversation_id = aggregate.detail.summary.conversation_id.clone();
            let project_id = aggregate.detail.summary.project_id.clone();
            self.persist_session(session_id, aggregate)?;
            (message, trace, approval, run, conversation_id, project_id)
        };

        self.state
            .observation
            .append_trace(TraceEventRecord {
                id: trace.id.clone(),
                workspace_id: self.state.workspace_id.clone(),
                project_id: Some(project_id.clone()),
                run_id: Some(run.id.clone()),
                session_id: Some(session_id.into()),
                event_kind: "turn_submitted".into(),
                title: trace.title.clone(),
                detail: trace.detail.clone(),
                created_at: now,
            })
            .await?;
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
                metric: "turns".into(),
                amount: 1,
                unit: "count".into(),
                created_at: now,
            })
            .await?;

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
                    "message": message.clone(),
                })),
                run: None,
                message: Some(message),
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
                    "trace": trace.clone(),
                })),
                run: None,
                message: None,
                trace: Some(trace),
                approval: None,
                decision: None,
                summary: None,
                error: None,
            },
        ];

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
        let (approval, run, conversation_id, project_id) = {
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
            pending.status = match input.decision.as_str() {
                "approve" => "approved".into(),
                "reject" => "rejected".into(),
                _ => {
                    return Err(AppError::invalid_input(
                        "approval decision must be approve or reject",
                    ))
                }
            };

            aggregate.detail.run.status = if input.decision == "approve" {
                "completed".into()
            } else {
                "blocked".into()
            };
            aggregate.detail.run.current_step = if input.decision == "approve" {
                "completed".into()
            } else {
                "approval_rejected".into()
            };
            aggregate.detail.run.updated_at = now;
            aggregate.detail.run.next_action = None;
            aggregate.detail.summary.status = aggregate.detail.run.status.clone();
            aggregate.detail.summary.updated_at = now;

            let approval = pending.clone();
            aggregate.detail.pending_approval = None;
            let run = aggregate.detail.run.clone();
            let conversation_id = aggregate.detail.summary.conversation_id.clone();
            let project_id = aggregate.detail.summary.project_id.clone();
            self.persist_session(session_id, aggregate)?;
            (approval, run, conversation_id, project_id)
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

        for event in [
            RuntimeEventEnvelope {
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
            },
            RuntimeEventEnvelope {
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
            },
        ] {
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
