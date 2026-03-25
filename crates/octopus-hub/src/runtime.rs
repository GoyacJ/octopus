use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::contracts::{
    ApprovalType, KnowledgeStatus, RunStatus, RunType, SandboxTier, TriggerSource, TrustLevel,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpBinding {
    pub server_name: String,
    pub event_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSubmissionRequest {
    pub workspace_id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub requested_by: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationCreateRequest {
    pub workspace_id: String,
    pub project_id: String,
    pub name: String,
    pub trigger_source: TriggerSource,
    pub requested_by: String,
    pub requires_approval: bool,
    pub mcp_binding: Option<McpBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationStateUpdateRequest {
    pub state: AutomationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDeliveryRequest {
    pub trigger_id: String,
    pub dedupe_key: String,
    pub requested_by: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEventDeliveryRequest {
    pub server_name: String,
    pub event_name: String,
    pub dedupe_key: String,
    pub requested_by: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResolutionRequest {
    pub decision: String,
    pub reviewed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSpaceCreateRequest {
    pub workspace_id: String,
    pub name: String,
    pub owner_refs: Vec<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCandidateCreateRequest {
    pub run_id: String,
    pub knowledge_space_id: String,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePromotionRequest {
    pub promoted_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxState {
    Open,
    Acknowledged,
    Resolved,
    Dismissed,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationState {
    Draft,
    Active,
    Paused,
    Suspended,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerDeliveryState {
    Pending,
    Claimed,
    Dispatched,
    Succeeded,
    Failed,
    Retried,
    DeadLetter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub project_id: String,
    pub run_type: RunType,
    pub status: RunStatus,
    pub idempotency_key: String,
    pub requested_by: String,
    pub title: String,
    pub checkpoint_token: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: String,
    pub project_id: String,
    pub run_id: String,
    pub version: u32,
    pub title: String,
    pub content_ref: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequestRecord {
    pub id: String,
    pub run_id: String,
    pub approval_type: ApprovalType,
    pub state: ApprovalState,
    pub target_ref: String,
    pub requested_at: String,
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItemRecord {
    pub id: String,
    pub workspace_id: String,
    pub owner_ref: String,
    pub state: InboxState,
    pub priority: String,
    pub target_ref: String,
    pub dedupe_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub name: String,
    pub trigger_ids: Vec<String>,
    pub state: AutomationState,
    pub requires_approval: bool,
    pub last_run_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRecord {
    pub id: String,
    pub automation_id: String,
    pub source_type: TriggerSource,
    pub dedupe_key: String,
    pub owner_ref: String,
    pub state: String,
    pub created_at: String,
    pub mcp_binding: Option<McpBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDeliveryRecord {
    pub id: String,
    pub trigger_id: String,
    pub source_type: TriggerSource,
    pub dedupe_key: String,
    pub state: TriggerDeliveryState,
    pub run_id: Option<String>,
    pub failure_reason: Option<String>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub name: String,
    pub message: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub action: String,
    pub actor: String,
    pub target_ref: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSpaceRecord {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub owner_refs: Vec<String>,
    pub scope: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCandidateRecord {
    pub id: String,
    pub knowledge_space_id: String,
    pub run_id: String,
    pub artifact_id: String,
    pub title: String,
    pub summary: String,
    pub status: KnowledgeStatus,
    pub trust_level: TrustLevel,
    pub source_ref: String,
    pub created_by: String,
    pub created_at: String,
    pub promoted_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeAssetRecord {
    pub id: String,
    pub knowledge_space_id: String,
    pub title: String,
    pub summary: String,
    pub layer: String,
    pub status: KnowledgeStatus,
    pub trust_level: TrustLevel,
    pub source_ref: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDetailResponse {
    pub run: RunRecord,
    pub artifact: Option<ArtifactRecord>,
    pub approval: Option<ApprovalRequestRecord>,
    pub inbox_item: Option<InboxItemRecord>,
    pub trace: Vec<TraceEvent>,
    pub audit: Vec<AuditEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationDetailResponse {
    pub automation: AutomationRecord,
    pub trigger: TriggerRecord,
    pub latest_delivery: Option<TriggerDeliveryRecord>,
    pub latest_run: Option<RunDetailResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationListResponse {
    pub items: Vec<AutomationDetailResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDeliveryResponse {
    pub delivery: TriggerDeliveryRecord,
    pub run: Option<RunDetailResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEventDeliveryResponse {
    pub items: Vec<TriggerDeliveryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSpaceDetailResponse {
    pub space: KnowledgeSpaceRecord,
    pub candidates: Vec<KnowledgeCandidateRecord>,
    pub assets: Vec<KnowledgeAssetRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSpaceListResponse {
    pub items: Vec<KnowledgeSpaceDetailResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeAssetListResponse {
    pub items: Vec<KnowledgeAssetRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCandidateResponse {
    pub candidate: KnowledgeCandidateRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgePromotionResponse {
    pub candidate: KnowledgeCandidateRecord,
    pub asset: KnowledgeAssetRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunListResponse {
    pub items: Vec<RunRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxListResponse {
    pub items: Vec<InboxItemRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEventEnvelope {
    pub sequence: u64,
    pub topic: String,
    pub occurred_at: String,
    pub run_id: Option<String>,
    pub workspace_id: Option<String>,
    pub automation_id: Option<String>,
    pub trigger_id: Option<String>,
    pub candidate_id: Option<String>,
    pub asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeLineageRecord {
    pub candidate_id: String,
    pub asset_id: String,
    pub run_id: String,
    pub artifact_id: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentLeaseRecord {
    pub id: String,
    pub run_id: String,
    pub sandbox_tier: SandboxTier,
    pub state: String,
    pub expiry_at: String,
    pub resume_token: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RunSource {
    Task,
    Trigger(TriggerSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    pub runs: HashMap<String, RunRecord>,
    pub run_workspaces: HashMap<String, String>,
    pub run_sources: HashMap<String, RunSource>,
    pub environment_leases: HashMap<String, EnvironmentLeaseRecord>,
    pub artifacts: HashMap<String, ArtifactRecord>,
    pub approvals: HashMap<String, ApprovalRequestRecord>,
    pub inbox_items: HashMap<String, InboxItemRecord>,
    pub traces: HashMap<String, Vec<TraceEvent>>,
    pub audits: HashMap<String, Vec<AuditEntry>>,
    pub automations: HashMap<String, AutomationRecord>,
    pub triggers: HashMap<String, TriggerRecord>,
    pub trigger_deliveries: HashMap<String, TriggerDeliveryRecord>,
    pub delivery_dedupe_index: HashMap<String, String>,
    pub latest_delivery_by_trigger: HashMap<String, String>,
    pub knowledge_spaces: HashMap<String, KnowledgeSpaceRecord>,
    pub knowledge_candidates: HashMap<String, KnowledgeCandidateRecord>,
    pub candidate_index_by_run_and_space: HashMap<String, String>,
    pub knowledge_assets: HashMap<String, KnowledgeAssetRecord>,
    pub knowledge_lineage: Vec<KnowledgeLineageRecord>,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        let default_space = default_knowledge_space();
        let mut knowledge_spaces = HashMap::new();
        knowledge_spaces.insert(default_space.id.clone(), default_space);

        Self {
            runs: HashMap::new(),
            run_workspaces: HashMap::new(),
            run_sources: HashMap::new(),
            environment_leases: HashMap::new(),
            artifacts: HashMap::new(),
            approvals: HashMap::new(),
            inbox_items: HashMap::new(),
            traces: HashMap::new(),
            audits: HashMap::new(),
            automations: HashMap::new(),
            triggers: HashMap::new(),
            trigger_deliveries: HashMap::new(),
            delivery_dedupe_index: HashMap::new(),
            latest_delivery_by_trigger: HashMap::new(),
            knowledge_spaces,
            knowledge_candidates: HashMap::new(),
            candidate_index_by_run_and_space: HashMap::new(),
            knowledge_assets: HashMap::new(),
            knowledge_lineage: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewRuntimeEvent {
    topic: String,
    occurred_at: String,
    run_id: Option<String>,
    workspace_id: Option<String>,
    automation_id: Option<String>,
    trigger_id: Option<String>,
    candidate_id: Option<String>,
    asset_id: Option<String>,
}

struct RunStartRequest {
    workspace_id: String,
    project_id: String,
    run_type: RunType,
    idempotency_key: String,
    requested_by: String,
    title: String,
    description: Option<String>,
    requires_approval: bool,
    initial_audit_action: &'static str,
    initial_audit_target: AuditTarget,
    run_source: RunSource,
}

enum AuditTarget {
    Run,
    Explicit(String),
}

struct DeliveryContext {
    automation_id: String,
    workspace_id: String,
    trigger_id: String,
    mutated: bool,
}

pub trait RuntimeRepository: Send + Sync {
    fn load_snapshot(&self) -> Result<RuntimeSnapshot, RuntimeError>;
    fn persist(
        &self,
        snapshot: &RuntimeSnapshot,
        events: &[NewRuntimeEvent],
    ) -> Result<Vec<RuntimeEventEnvelope>, RuntimeError>;
    fn list_events(&self, after_sequence: Option<u64>) -> Result<Vec<RuntimeEventEnvelope>, RuntimeError>;
}

#[derive(Default)]
pub struct InMemoryRuntimeRepository {
    snapshot: Mutex<RuntimeSnapshot>,
    events: Mutex<Vec<RuntimeEventEnvelope>>,
}

pub struct SqliteRuntimeRepository {
    database_path: PathBuf,
}

#[derive(Clone)]
pub struct RuntimeService {
    repository: Arc<dyn RuntimeRepository>,
    event_sender: broadcast::Sender<RuntimeEventEnvelope>,
}

pub type InMemoryRuntimeService = RuntimeService;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: String },
    #[error("{kind} {id} is in invalid state: {reason}")]
    InvalidState {
        kind: &'static str,
        id: String,
        reason: String,
    },
    #[error("invalid approval decision: {decision}")]
    InvalidDecision { decision: String },
    #[error("invalid request: {reason}")]
    InvalidRequest { reason: String },
    #[error("repository error: {reason}")]
    Repository { reason: String },
}

impl Default for RuntimeService {
    fn default() -> Self {
        Self::in_memory()
    }
}

impl RuntimeService {
    pub fn in_memory() -> Self {
        Self::from_repository(Arc::new(InMemoryRuntimeRepository::default()))
    }

    pub fn sqlite(path: impl AsRef<Path>) -> Result<Self, RuntimeError> {
        Ok(Self::from_repository(Arc::new(SqliteRuntimeRepository::new(path)?)))
    }

    pub fn sqlite_default_path() -> PathBuf {
        PathBuf::from("target/octopus-runtime.sqlite3")
    }

    pub fn sqlite_default() -> Result<Self, RuntimeError> {
        Self::sqlite(Self::sqlite_default_path())
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<RuntimeEventEnvelope> {
        self.event_sender.subscribe()
    }

    pub fn list_events(&self, after_sequence: Option<u64>) -> Result<Vec<RuntimeEventEnvelope>, RuntimeError> {
        self.repository.list_events(after_sequence)
    }

    pub fn submit_task(&self, request: TaskSubmissionRequest) -> RunDetailResponse {
        let mut snapshot = self.load_snapshot();
        let response = start_run(
            &mut snapshot,
            RunStartRequest {
                workspace_id: request.workspace_id.clone(),
                project_id: request.project_id.clone(),
                run_type: RunType::Task,
                idempotency_key: format!("task:{}", Uuid::new_v4()),
                requested_by: request.requested_by,
                title: request.title,
                description: request.description,
                requires_approval: request.requires_approval,
                initial_audit_action: "task.submitted",
                initial_audit_target: AuditTarget::Run,
                run_source: RunSource::Task,
            },
        );

        self.persist_snapshot(
            &snapshot,
            events_for_run_detail(
                &response,
                snapshot.run_workspaces.get(&response.run.id).cloned(),
            ),
        );
        response
    }

    pub fn create_automation(
        &self,
        request: AutomationCreateRequest,
    ) -> Result<AutomationDetailResponse, RuntimeError> {
        validate_mcp_binding(request.trigger_source, request.mcp_binding.as_ref())?;

        let mut snapshot = self.load_snapshot();
        let automation_id = Uuid::new_v4().to_string();
        let trigger_id = Uuid::new_v4().to_string();
        let now = now_iso();

        let automation = AutomationRecord {
            id: automation_id.clone(),
            workspace_id: request.workspace_id.clone(),
            project_id: request.project_id,
            name: request.name,
            trigger_ids: vec![trigger_id.clone()],
            state: AutomationState::Active,
            requires_approval: request.requires_approval,
            last_run_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let trigger = TriggerRecord {
            id: trigger_id.clone(),
            automation_id: automation_id.clone(),
            source_type: request.trigger_source,
            dedupe_key: format!("automation:{automation_id}"),
            owner_ref: format!("automation:{automation_id}"),
            state: "active".into(),
            created_at: now,
            mcp_binding: request.mcp_binding,
        };

        snapshot.automations.insert(automation_id.clone(), automation);
        snapshot.triggers.insert(trigger_id.clone(), trigger);

        let detail = hydrate_automation_detail(&snapshot, &automation_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "automation",
            id: automation_id.clone(),
        })?;
        self.persist_snapshot(
            &snapshot,
            vec![new_runtime_event(
                "automation.updated",
                None,
                Some(request.workspace_id),
                Some(automation_id),
                Some(trigger_id),
                None,
                None,
            )],
        );

        Ok(detail)
    }

    pub fn list_automations(&self) -> Vec<AutomationDetailResponse> {
        let snapshot = self.load_snapshot();
        let mut automation_ids = snapshot.automations.keys().cloned().collect::<Vec<_>>();
        automation_ids.sort();

        automation_ids
            .into_iter()
            .filter_map(|automation_id| hydrate_automation_detail(&snapshot, &automation_id))
            .collect()
    }

    pub fn update_automation_state(
        &self,
        automation_id: &str,
        request: AutomationStateUpdateRequest,
    ) -> Result<AutomationDetailResponse, RuntimeError> {
        let mut snapshot = self.load_snapshot();
        let workspace_id = {
            let automation = snapshot
                .automations
                .get_mut(automation_id)
                .ok_or_else(|| RuntimeError::NotFound {
                    kind: "automation",
                    id: automation_id.to_string(),
                })?;

            automation.state = request.state;
            automation.updated_at = now_iso();
            automation.workspace_id.clone()
        };

        sync_trigger_states(&mut snapshot, automation_id, request.state);
        let detail = hydrate_automation_detail(&snapshot, automation_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "automation",
            id: automation_id.to_string(),
        })?;

        self.persist_snapshot(
            &snapshot,
            vec![new_runtime_event(
                "automation.updated",
                None,
                Some(workspace_id),
                Some(automation_id.to_string()),
                Some(detail.trigger.id.clone()),
                None,
                None,
            )],
        );

        Ok(detail)
    }

    pub fn deliver_trigger(
        &self,
        request: TriggerDeliveryRequest,
    ) -> Result<TriggerDeliveryResponse, RuntimeError> {
        let mut snapshot = self.load_snapshot();
        let (response, context) = match deliver_trigger_inner(&mut snapshot, request) {
            Ok(value) => value,
            Err(error) => {
                self.persist_snapshot(&snapshot, Vec::new());
                return Err(error);
            }
        };

        if context.mutated {
            let mut events = vec![
                new_runtime_event(
                    "trigger.delivery_updated",
                    response.delivery.run_id.clone(),
                    Some(context.workspace_id.clone()),
                    Some(context.automation_id.clone()),
                    Some(context.trigger_id),
                    None,
                    None,
                ),
                new_runtime_event(
                    "automation.updated",
                    response.delivery.run_id.clone(),
                    Some(context.workspace_id.clone()),
                    Some(context.automation_id),
                    None,
                    None,
                    None,
                ),
            ];

            if let Some(run) = response.run.as_ref() {
                events.extend(events_for_run_detail(run, Some(context.workspace_id)));
            }
            self.persist_snapshot(&snapshot, events);
        }

        Ok(response)
    }

    pub fn deliver_mcp_event(
        &self,
        request: McpEventDeliveryRequest,
    ) -> Result<McpEventDeliveryResponse, RuntimeError> {
        let mut snapshot = self.load_snapshot();
        let mut trigger_ids = snapshot
            .triggers
            .values()
            .filter(|trigger| {
                trigger.source_type == TriggerSource::McpEvent
                    && trigger
                        .mcp_binding
                        .as_ref()
                        .map(|binding| {
                            binding.server_name == request.server_name
                                && binding.event_name == request.event_name
                        })
                        .unwrap_or(false)
            })
            .map(|trigger| trigger.id.clone())
            .collect::<Vec<_>>();
        trigger_ids.sort();

        if trigger_ids.is_empty() {
            return Err(RuntimeError::NotFound {
                kind: "mcp_binding",
                id: format!("{}:{}", request.server_name, request.event_name),
            });
        }

        let mut items = Vec::with_capacity(trigger_ids.len());
        let mut events = Vec::new();

        for trigger_id in trigger_ids {
            let (response, context) = match deliver_trigger_inner(
                &mut snapshot,
                TriggerDeliveryRequest {
                    trigger_id,
                    dedupe_key: request.dedupe_key.clone(),
                    requested_by: request.requested_by.clone(),
                    title: request.title.clone(),
                    description: request.description.clone(),
                },
            ) {
                Ok(value) => value,
                Err(error) => {
                    self.persist_snapshot(&snapshot, events);
                    return Err(error);
                }
            };

            if context.mutated {
                events.push(new_runtime_event(
                    "trigger.delivery_updated",
                    response.delivery.run_id.clone(),
                    Some(context.workspace_id.clone()),
                    Some(context.automation_id.clone()),
                    Some(context.trigger_id),
                    None,
                    None,
                ));
                events.push(new_runtime_event(
                    "automation.updated",
                    response.delivery.run_id.clone(),
                    Some(context.workspace_id.clone()),
                    Some(context.automation_id),
                    None,
                    None,
                    None,
                ));
                if let Some(run) = response.run.as_ref() {
                    events.extend(events_for_run_detail(run, Some(context.workspace_id)));
                }
            }

            items.push(response);
        }

        self.persist_snapshot(&snapshot, events);
        Ok(McpEventDeliveryResponse { items })
    }

    pub fn list_knowledge_spaces(&self) -> Vec<KnowledgeSpaceDetailResponse> {
        let snapshot = self.load_snapshot();
        let mut space_ids = snapshot.knowledge_spaces.keys().cloned().collect::<Vec<_>>();
        space_ids.sort();

        space_ids
            .into_iter()
            .filter_map(|space_id| hydrate_knowledge_space_detail(&snapshot, &space_id))
            .collect()
    }

    pub fn create_knowledge_space(
        &self,
        request: KnowledgeSpaceCreateRequest,
    ) -> Result<KnowledgeSpaceDetailResponse, RuntimeError> {
        if request.owner_refs.is_empty() {
            return Err(RuntimeError::InvalidRequest {
                reason: "knowledge spaces require at least one owner_ref".into(),
            });
        }
        if request.scope.trim().is_empty() {
            return Err(RuntimeError::InvalidRequest {
                reason: "knowledge spaces require a non-empty scope".into(),
            });
        }

        let mut snapshot = self.load_snapshot();
        let now = now_iso();
        let space = KnowledgeSpaceRecord {
            id: Uuid::new_v4().to_string(),
            workspace_id: request.workspace_id.clone(),
            name: request.name,
            owner_refs: request.owner_refs,
            scope: request.scope,
            state: "active".into(),
            created_at: now.clone(),
            updated_at: now,
        };
        let space_id = space.id.clone();
        snapshot.knowledge_spaces.insert(space_id.clone(), space);

        let detail = hydrate_knowledge_space_detail(&snapshot, &space_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "knowledge_space",
            id: space_id.clone(),
        })?;

        self.persist_snapshot(&snapshot, Vec::new());
        Ok(detail)
    }

    pub fn create_candidate_from_run(
        &self,
        request: KnowledgeCandidateCreateRequest,
    ) -> Result<KnowledgeCandidateRecord, RuntimeError> {
        let mut snapshot = self.load_snapshot();
        let workspace_id = snapshot
            .knowledge_spaces
            .get(&request.knowledge_space_id)
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "knowledge_space",
                id: request.knowledge_space_id.clone(),
            })?
            .workspace_id
            .clone();

        let (candidate, mutated) = create_candidate_from_run_inner(&mut snapshot, request)?;

        if mutated {
            self.persist_snapshot(
                &snapshot,
                vec![new_runtime_event(
                    "knowledge.candidate_updated",
                    Some(candidate.run_id.clone()),
                    Some(workspace_id),
                    None,
                    None,
                    Some(candidate.id.clone()),
                    None,
                )],
            );
        }

        Ok(candidate)
    }

    pub fn promote_candidate(
        &self,
        candidate_id: &str,
        request: KnowledgePromotionRequest,
    ) -> Result<KnowledgePromotionResponse, RuntimeError> {
        let mut snapshot = self.load_snapshot();
        let workspace_id = snapshot
            .knowledge_candidates
            .get(candidate_id)
            .and_then(|candidate| snapshot.knowledge_spaces.get(&candidate.knowledge_space_id))
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "knowledge_candidate",
                id: candidate_id.to_string(),
            })?
            .workspace_id
            .clone();

        let response = promote_candidate_inner(&mut snapshot, candidate_id, request)?;
        self.persist_snapshot(
            &snapshot,
            vec![
                new_runtime_event(
                    "knowledge.candidate_updated",
                    Some(response.candidate.run_id.clone()),
                    Some(workspace_id.clone()),
                    None,
                    None,
                    Some(response.candidate.id.clone()),
                    None,
                ),
                new_runtime_event(
                    "knowledge.asset_updated",
                    Some(response.candidate.run_id.clone()),
                    Some(workspace_id),
                    None,
                    None,
                    Some(response.candidate.id.clone()),
                    Some(response.asset.id.clone()),
                ),
            ],
        );

        Ok(response)
    }

    pub fn list_knowledge_assets(
        &self,
        knowledge_space_id: &str,
    ) -> Result<KnowledgeAssetListResponse, RuntimeError> {
        let snapshot = self.load_snapshot();
        if !snapshot.knowledge_spaces.contains_key(knowledge_space_id) {
            return Err(RuntimeError::NotFound {
                kind: "knowledge_space",
                id: knowledge_space_id.to_string(),
            });
        }

        let mut items = snapshot
            .knowledge_assets
            .values()
            .filter(|asset| asset.knowledge_space_id == knowledge_space_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.created_at.cmp(&right.created_at));

        Ok(KnowledgeAssetListResponse { items })
    }

    pub fn list_runs(&self, workspace_id: Option<&str>, project_id: Option<&str>) -> Vec<RunRecord> {
        let snapshot = self.load_snapshot();
        let mut items = snapshot
            .runs
            .values()
            .filter(|run| {
                let workspace_matches = workspace_id
                    .map(|expected| {
                        snapshot
                            .run_workspaces
                            .get(&run.id)
                            .map(|actual| actual == expected)
                            .unwrap_or(false)
                    })
                    .unwrap_or(true);
                let project_matches = project_id.map(|expected| run.project_id == expected).unwrap_or(true);

                workspace_matches && project_matches
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        items
    }

    pub fn list_inbox_items(&self, workspace_id: Option<&str>) -> Vec<InboxItemRecord> {
        let snapshot = self.load_snapshot();
        let mut items = snapshot
            .inbox_items
            .values()
            .filter(|item| workspace_id.map(|expected| item.workspace_id == expected).unwrap_or(true))
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.target_ref.cmp(&right.target_ref));
        items
    }

    pub fn get_run(&self, run_id: &str) -> Option<RunDetailResponse> {
        let snapshot = self.load_snapshot();
        hydrate_response(&snapshot, run_id)
    }

    pub fn resolve_approval(
        &self,
        approval_id: &str,
        request: ApprovalResolutionRequest,
    ) -> Result<RunDetailResponse, RuntimeError> {
        let decision = ApprovalDecision::try_from(request.decision.as_str())?;
        let mut snapshot = self.load_snapshot();
        let response = resolve_approval_inner(&mut snapshot, approval_id, request, decision)?;
        let workspace_id = snapshot.run_workspaces.get(&response.run.id).cloned();

        self.persist_snapshot(
            &snapshot,
            {
                let mut events = events_for_run_detail(&response, workspace_id);
                events.push(new_runtime_event(
                    "approval.updated",
                    Some(response.run.id.clone()),
                    snapshot.run_workspaces.get(&response.run.id).cloned(),
                    None,
                    None,
                    None,
                    None,
                ));
                events
            },
        );

        Ok(response)
    }

    pub fn resume_run(&self, run_id: &str) -> Result<RunDetailResponse, RuntimeError> {
        let mut snapshot = self.load_snapshot();
        let response = resume_run_inner(&mut snapshot, run_id)?;
        let workspace_id = snapshot.run_workspaces.get(run_id).cloned();

        self.persist_snapshot(&snapshot, events_for_run_detail(&response, workspace_id));
        Ok(response)
    }

    fn from_repository(repository: Arc<dyn RuntimeRepository>) -> Self {
        let (event_sender, _) = broadcast::channel(256);
        Self {
            repository,
            event_sender,
        }
    }

    fn load_snapshot(&self) -> RuntimeSnapshot {
        self.repository
            .load_snapshot()
            .expect("runtime repository should load snapshot")
    }

    fn persist_snapshot(&self, snapshot: &RuntimeSnapshot, events: Vec<NewRuntimeEvent>) {
        let persisted = self
            .repository
            .persist(snapshot, &events)
            .expect("runtime repository should persist snapshot");

        for event in persisted {
            let _ = self.event_sender.send(event);
        }
    }
}

impl RuntimeRepository for InMemoryRuntimeRepository {
    fn load_snapshot(&self) -> Result<RuntimeSnapshot, RuntimeError> {
        Ok(self
            .snapshot
            .lock()
            .expect("in-memory snapshot should lock")
            .clone())
    }

    fn persist(
        &self,
        snapshot: &RuntimeSnapshot,
        events: &[NewRuntimeEvent],
    ) -> Result<Vec<RuntimeEventEnvelope>, RuntimeError> {
        *self
            .snapshot
            .lock()
            .expect("in-memory snapshot should lock") = snapshot.clone();

        let mut stored = self.events.lock().expect("in-memory events should lock");
        let mut persisted = Vec::with_capacity(events.len());

        for event in events {
            let envelope = RuntimeEventEnvelope {
                sequence: stored.len() as u64 + 1,
                topic: event.topic.clone(),
                occurred_at: event.occurred_at.clone(),
                run_id: event.run_id.clone(),
                workspace_id: event.workspace_id.clone(),
                automation_id: event.automation_id.clone(),
                trigger_id: event.trigger_id.clone(),
                candidate_id: event.candidate_id.clone(),
                asset_id: event.asset_id.clone(),
            };
            stored.push(envelope.clone());
            persisted.push(envelope);
        }

        Ok(persisted)
    }

    fn list_events(&self, after_sequence: Option<u64>) -> Result<Vec<RuntimeEventEnvelope>, RuntimeError> {
        let stored = self.events.lock().expect("in-memory events should lock");
        Ok(stored
            .iter()
            .filter(|event| after_sequence.map(|after| event.sequence > after).unwrap_or(true))
            .cloned()
            .collect())
    }
}

impl SqliteRuntimeRepository {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, RuntimeError> {
        let database_path = path.as_ref().to_path_buf();
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(|error| RuntimeError::Repository {
                reason: error.to_string(),
            })?;
        }

        let repository = Self { database_path };
        repository.initialize()?;
        Ok(repository)
    }

    fn initialize(&self) -> Result<(), RuntimeError> {
        let connection = self.open_connection()?;
        connection
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS runtime_state (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    payload TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS runtime_events (
                    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                    topic TEXT NOT NULL,
                    occurred_at TEXT NOT NULL,
                    run_id TEXT,
                    workspace_id TEXT,
                    automation_id TEXT,
                    trigger_id TEXT,
                    candidate_id TEXT,
                    asset_id TEXT
                );
                ",
            )
            .map_err(|error| RuntimeError::Repository {
                reason: error.to_string(),
            })?;

        Ok(())
    }

    fn open_connection(&self) -> Result<Connection, RuntimeError> {
        Connection::open(&self.database_path).map_err(|error| RuntimeError::Repository {
            reason: error.to_string(),
        })
    }
}

impl RuntimeRepository for SqliteRuntimeRepository {
    fn load_snapshot(&self) -> Result<RuntimeSnapshot, RuntimeError> {
        let connection = self.open_connection()?;
        let payload = connection
            .query_row(
                "SELECT payload FROM runtime_state WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| RuntimeError::Repository {
                reason: error.to_string(),
            })?;

        match payload {
            Some(value) => serde_json::from_str(&value).map_err(|error| RuntimeError::Repository {
                reason: error.to_string(),
            }),
            None => Ok(RuntimeSnapshot::default()),
        }
    }

    fn persist(
        &self,
        snapshot: &RuntimeSnapshot,
        events: &[NewRuntimeEvent],
    ) -> Result<Vec<RuntimeEventEnvelope>, RuntimeError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction().map_err(|error| RuntimeError::Repository {
            reason: error.to_string(),
        })?;
        let payload = serde_json::to_string(snapshot).map_err(|error| RuntimeError::Repository {
            reason: error.to_string(),
        })?;
        let updated_at = now_iso();

        transaction
            .execute(
                "
                INSERT INTO runtime_state (id, payload, updated_at)
                VALUES (1, ?1, ?2)
                ON CONFLICT(id) DO UPDATE SET
                    payload = excluded.payload,
                    updated_at = excluded.updated_at
                ",
                params![payload, updated_at],
            )
            .map_err(|error| RuntimeError::Repository {
                reason: error.to_string(),
            })?;

        let mut persisted = Vec::with_capacity(events.len());
        for event in events {
            transaction
                .execute(
                    "
                    INSERT INTO runtime_events (
                        topic,
                        occurred_at,
                        run_id,
                        workspace_id,
                        automation_id,
                        trigger_id,
                        candidate_id,
                        asset_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ",
                    params![
                        event.topic,
                        event.occurred_at,
                        event.run_id,
                        event.workspace_id,
                        event.automation_id,
                        event.trigger_id,
                        event.candidate_id,
                        event.asset_id
                    ],
                )
                .map_err(|error| RuntimeError::Repository {
                    reason: error.to_string(),
                })?;

            persisted.push(RuntimeEventEnvelope {
                sequence: transaction.last_insert_rowid() as u64,
                topic: event.topic.clone(),
                occurred_at: event.occurred_at.clone(),
                run_id: event.run_id.clone(),
                workspace_id: event.workspace_id.clone(),
                automation_id: event.automation_id.clone(),
                trigger_id: event.trigger_id.clone(),
                candidate_id: event.candidate_id.clone(),
                asset_id: event.asset_id.clone(),
            });
        }

        transaction.commit().map_err(|error| RuntimeError::Repository {
            reason: error.to_string(),
        })?;

        Ok(persisted)
    }

    fn list_events(&self, after_sequence: Option<u64>) -> Result<Vec<RuntimeEventEnvelope>, RuntimeError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT sequence, topic, occurred_at, run_id, workspace_id, automation_id, trigger_id, candidate_id, asset_id
                FROM runtime_events
                WHERE (?1 IS NULL OR sequence > ?1)
                ORDER BY sequence ASC
                ",
            )
            .map_err(|error| RuntimeError::Repository {
                reason: error.to_string(),
            })?;

        let rows = statement
            .query_map([after_sequence.map(|value| value as i64)], |row| {
                Ok(RuntimeEventEnvelope {
                    sequence: row.get::<_, i64>(0)? as u64,
                    topic: row.get(1)?,
                    occurred_at: row.get(2)?,
                    run_id: row.get(3)?,
                    workspace_id: row.get(4)?,
                    automation_id: row.get(5)?,
                    trigger_id: row.get(6)?,
                    candidate_id: row.get(7)?,
                    asset_id: row.get(8)?,
                })
            })
            .map_err(|error| RuntimeError::Repository {
                reason: error.to_string(),
            })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|error| RuntimeError::Repository {
                reason: error.to_string(),
            })?);
        }

        Ok(items)
    }
}

fn default_knowledge_space() -> KnowledgeSpaceRecord {
    let now = now_iso();
    KnowledgeSpaceRecord {
        id: "knowledge-space-alpha".into(),
        workspace_id: "workspace-alpha".into(),
        name: "Workspace Alpha Shared Knowledge".into(),
        owner_refs: vec!["owner-1".into()],
        scope: "project:project-alpha".into(),
        state: "active".into(),
        created_at: now.clone(),
        updated_at: now,
    }
}

fn validate_mcp_binding(
    trigger_source: TriggerSource,
    mcp_binding: Option<&McpBinding>,
) -> Result<(), RuntimeError> {
    match (trigger_source, mcp_binding) {
        (TriggerSource::McpEvent, Some(binding))
            if !binding.server_name.trim().is_empty() && !binding.event_name.trim().is_empty() =>
        {
            Ok(())
        }
        (TriggerSource::McpEvent, _) => Err(RuntimeError::InvalidRequest {
            reason: "mcp_event automations require a non-empty mcp_binding".into(),
        }),
        (_, Some(_)) => Err(RuntimeError::InvalidRequest {
            reason: "mcp_binding is only allowed for mcp_event automations".into(),
        }),
        _ => Ok(()),
    }
}

fn deliver_trigger_inner(
    snapshot: &mut RuntimeSnapshot,
    request: TriggerDeliveryRequest,
) -> Result<(TriggerDeliveryResponse, DeliveryContext), RuntimeError> {
    let dedupe_key = format!("{}:{}", request.trigger_id, request.dedupe_key);

    if let Some(delivery_id) = snapshot.delivery_dedupe_index.get(&dedupe_key).cloned() {
        let delivery = snapshot
            .trigger_deliveries
            .get(&delivery_id)
            .cloned()
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "trigger_delivery",
                id: delivery_id,
            })?;
        let run = delivery
            .run_id
            .as_deref()
            .and_then(|run_id| hydrate_response(snapshot, run_id));
        let trigger = snapshot
            .triggers
            .get(&delivery.trigger_id)
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "trigger",
                id: delivery.trigger_id.clone(),
            })?;
        let automation = snapshot
            .automations
            .get(&trigger.automation_id)
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "automation",
                id: trigger.automation_id.clone(),
            })?;

        return Ok((
            TriggerDeliveryResponse { delivery, run },
            DeliveryContext {
                automation_id: automation.id.clone(),
                workspace_id: automation.workspace_id.clone(),
                trigger_id: trigger.id.clone(),
                mutated: false,
            },
        ));
    }

    let trigger = snapshot
        .triggers
        .get(&request.trigger_id)
        .cloned()
        .ok_or_else(|| RuntimeError::NotFound {
            kind: "trigger",
            id: request.trigger_id.clone(),
        })?;
    let automation = snapshot
        .automations
        .get(&trigger.automation_id)
        .cloned()
        .ok_or_else(|| RuntimeError::NotFound {
            kind: "automation",
            id: trigger.automation_id.clone(),
        })?;

    if automation.state != AutomationState::Active {
        let delivery = record_failed_delivery(
            snapshot,
            &trigger,
            &request.dedupe_key,
            format!(
                "automation {} is paused and cannot accept deliveries",
                automation.id
            ),
        );

        return Err(RuntimeError::InvalidState {
            kind: "automation",
            id: automation.id,
            reason: delivery
                .failure_reason
                .unwrap_or_else(|| "delivery failed".into()),
        });
    }

    let delivery_id = Uuid::new_v4().to_string();
    let run_title = request.title.clone().unwrap_or_else(|| automation.name.clone());
    let run_type = trigger_run_type(trigger.source_type);
    let run_response = start_run(
        snapshot,
        RunStartRequest {
            workspace_id: automation.workspace_id.clone(),
            project_id: automation.project_id.clone(),
            run_type,
            idempotency_key: format!("trigger:{}:{}", trigger.id, request.dedupe_key),
            requested_by: request.requested_by.clone(),
            title: run_title,
            description: request.description.clone(),
            requires_approval: automation.requires_approval,
            initial_audit_action: "trigger.delivered",
            initial_audit_target: AuditTarget::Explicit(delivery_id.clone()),
            run_source: RunSource::Trigger(trigger.source_type),
        },
    );

    push_trace(
        snapshot,
        &run_response.run.id,
        trace_event(
            "TriggerDelivered",
            format!("Trigger {} delivered {}", trigger.id, request.dedupe_key),
        ),
    );

    let delivery = TriggerDeliveryRecord {
        id: delivery_id.clone(),
        trigger_id: trigger.id.clone(),
        source_type: trigger.source_type,
        dedupe_key: request.dedupe_key.clone(),
        state: TriggerDeliveryState::Succeeded,
        run_id: Some(run_response.run.id.clone()),
        failure_reason: None,
        occurred_at: now_iso(),
    };

    snapshot
        .trigger_deliveries
        .insert(delivery_id.clone(), delivery.clone());
    snapshot
        .delivery_dedupe_index
        .insert(dedupe_key, delivery_id.clone());
    snapshot
        .latest_delivery_by_trigger
        .insert(trigger.id.clone(), delivery_id);

    if let Some(entry) = snapshot.automations.get_mut(&automation.id) {
        entry.last_run_id = Some(run_response.run.id.clone());
        entry.updated_at = now_iso();
    }

    let run = hydrate_response(snapshot, &run_response.run.id);

    Ok((
        TriggerDeliveryResponse { delivery, run },
        DeliveryContext {
            automation_id: automation.id,
            workspace_id: automation.workspace_id,
            trigger_id: trigger.id,
            mutated: true,
        },
    ))
}

fn start_run(snapshot: &mut RuntimeSnapshot, request: RunStartRequest) -> RunDetailResponse {
    let run_id = Uuid::new_v4().to_string();
    let now = now_iso();
    let initial_audit_target = match request.initial_audit_target {
        AuditTarget::Run => run_id.clone(),
        AuditTarget::Explicit(target_ref) => target_ref,
    };

    let mut run = RunRecord {
        id: run_id.clone(),
        project_id: request.project_id.clone(),
        run_type: request.run_type,
        status: RunStatus::Planning,
        idempotency_key: request.idempotency_key,
        requested_by: request.requested_by.clone(),
        title: request.title.clone(),
        checkpoint_token: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let mut artifact = None;
    let mut approval = None;
    let mut inbox_item = None;
    let mut trace = vec![trace_event(
        "RunStateChanged",
        format!("Run {} entered planning", run.id),
    )];
    let mut audit = vec![audit_entry(
        request.initial_audit_action,
        &request.requested_by,
        &initial_audit_target,
    )];

    if request.requires_approval {
        run.status = RunStatus::WaitingApproval;
        run.checkpoint_token = Some(format!("resume:{}", run.id));
        approval = Some(ApprovalRequestRecord {
            id: Uuid::new_v4().to_string(),
            run_id: run.id.clone(),
            approval_type: ApprovalType::Execution,
            state: ApprovalState::Pending,
            target_ref: run.id.clone(),
            requested_at: now_iso(),
            reviewed_by: None,
        });
        inbox_item = Some(InboxItemRecord {
            id: Uuid::new_v4().to_string(),
            workspace_id: request.workspace_id.clone(),
            owner_ref: "reviewer.execution".into(),
            state: InboxState::Open,
            priority: "high".into(),
            target_ref: run.id.clone(),
            dedupe_key: format!("approval:{}", run.id),
        });
        snapshot.environment_leases.insert(
            run.id.clone(),
            EnvironmentLeaseRecord {
                id: Uuid::new_v4().to_string(),
                run_id: run.id.clone(),
                sandbox_tier: SandboxTier::LocalTrusted,
                state: "active".into(),
                expiry_at: (Utc::now() + Duration::hours(1)).to_rfc3339(),
                resume_token: run
                    .checkpoint_token
                    .clone()
                    .expect("checkpoint token should exist for approval runs"),
            },
        );
        trace.push(trace_event(
            "ApprovalRequested",
            format!("Run {} requires execution approval", run.id),
        ));
        trace.push(trace_event(
            "RunStateChanged",
            format!("Run {} is waiting for approval", run.id),
        ));
    } else {
        run.status = RunStatus::Completed;
        let built_artifact = build_artifact(
            &run.id,
            &run.project_id,
            &run.title,
            request.description.as_deref(),
        );
        audit.push(audit_entry(
            "artifact.created",
            &request.requested_by,
            &built_artifact.id,
        ));
        artifact = Some(built_artifact);
        trace.push(trace_event(
            "RunStateChanged",
            format!("Run {} completed without approval", run.id),
        ));
    }

    let response = RunDetailResponse {
        run: run.clone(),
        artifact: artifact.clone(),
        approval: approval.clone(),
        inbox_item: inbox_item.clone(),
        trace: trace.clone(),
        audit: audit.clone(),
    };

    snapshot.runs.insert(run.id.clone(), run);
    snapshot
        .run_workspaces
        .insert(response.run.id.clone(), request.workspace_id);
    snapshot
        .run_sources
        .insert(response.run.id.clone(), request.run_source);
    if let Some(entry) = artifact {
        snapshot.artifacts.insert(entry.run_id.clone(), entry);
    }
    if let Some(entry) = approval {
        snapshot.approvals.insert(entry.id.clone(), entry);
    }
    if let Some(entry) = inbox_item {
        snapshot.inbox_items.insert(entry.target_ref.clone(), entry);
    }
    snapshot.traces.insert(response.run.id.clone(), trace);
    snapshot.audits.insert(response.run.id.clone(), audit);

    response
}

fn create_candidate_from_run_inner(
    snapshot: &mut RuntimeSnapshot,
    request: KnowledgeCandidateCreateRequest,
) -> Result<(KnowledgeCandidateRecord, bool), RuntimeError> {
    let run = snapshot
        .runs
        .get(&request.run_id)
        .cloned()
        .ok_or_else(|| RuntimeError::NotFound {
            kind: "run",
            id: request.run_id.clone(),
        })?;
    let artifact = snapshot
        .artifacts
        .get(&request.run_id)
        .cloned()
        .ok_or_else(|| RuntimeError::InvalidState {
            kind: "run",
            id: request.run_id.clone(),
            reason: "knowledge candidates require a completed artifact".into(),
        })?;
    let candidate_index = format!("{}:{}", request.knowledge_space_id, request.run_id);
    if let Some(candidate_id) = snapshot
        .candidate_index_by_run_and_space
        .get(&candidate_index)
    {
        return snapshot
            .knowledge_candidates
            .get(candidate_id)
            .cloned()
            .map(|candidate| (candidate, false))
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "knowledge_candidate",
                id: candidate_id.clone(),
            });
    }

    let trust_level = match snapshot.run_sources.get(&run.id).copied() {
        Some(RunSource::Trigger(TriggerSource::McpEvent)) => TrustLevel::Low,
        _ => TrustLevel::High,
    };
    let candidate = KnowledgeCandidateRecord {
        id: Uuid::new_v4().to_string(),
        knowledge_space_id: request.knowledge_space_id.clone(),
        run_id: run.id.clone(),
        artifact_id: artifact.id.clone(),
        title: artifact.title.clone(),
        summary: artifact.content_ref.clone(),
        status: KnowledgeStatus::Candidate,
        trust_level,
        source_ref: run.id.clone(),
        created_by: request.created_by.clone(),
        created_at: now_iso(),
        promoted_asset_id: None,
    };

    snapshot
        .candidate_index_by_run_and_space
        .insert(candidate_index, candidate.id.clone());
    snapshot
        .knowledge_candidates
        .insert(candidate.id.clone(), candidate.clone());

    push_trace(
        snapshot,
        &run.id,
        trace_event(
            "KnowledgeCandidateCreated",
            format!(
                "Candidate {} created in {}",
                candidate.id, candidate.knowledge_space_id
            ),
        ),
    );
    push_audit(
        snapshot,
        &run.id,
        audit_entry("knowledge.candidate.created", &request.created_by, &candidate.id),
    );

    Ok((candidate, true))
}

fn promote_candidate_inner(
    snapshot: &mut RuntimeSnapshot,
    candidate_id: &str,
    request: KnowledgePromotionRequest,
) -> Result<KnowledgePromotionResponse, RuntimeError> {
    let candidate = snapshot
        .knowledge_candidates
        .get(candidate_id)
        .cloned()
        .ok_or_else(|| RuntimeError::NotFound {
            kind: "knowledge_candidate",
            id: candidate_id.to_string(),
        })?;
    let space = snapshot
        .knowledge_spaces
        .get(&candidate.knowledge_space_id)
        .cloned()
        .ok_or_else(|| RuntimeError::NotFound {
            kind: "knowledge_space",
            id: candidate.knowledge_space_id.clone(),
        })?;

    if !space.owner_refs.iter().any(|owner| owner == &request.promoted_by) {
        return Err(RuntimeError::InvalidState {
            kind: "knowledge_candidate",
            id: candidate_id.to_string(),
            reason: format!(
                "{} is not an owner of knowledge space {}",
                request.promoted_by, space.id
            ),
        });
    }
    if candidate.status != KnowledgeStatus::Candidate {
        return Err(RuntimeError::InvalidState {
            kind: "knowledge_candidate",
            id: candidate_id.to_string(),
            reason: "candidate has already been promoted".into(),
        });
    }

    let asset = KnowledgeAssetRecord {
        id: Uuid::new_v4().to_string(),
        knowledge_space_id: candidate.knowledge_space_id.clone(),
        title: candidate.title.clone(),
        summary: candidate.summary.clone(),
        layer: "shared".into(),
        status: KnowledgeStatus::VerifiedShared,
        trust_level: candidate.trust_level,
        source_ref: candidate.id.clone(),
        created_at: now_iso(),
    };

    let updated_candidate = {
        let candidate_entry = snapshot
            .knowledge_candidates
            .get_mut(candidate_id)
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "knowledge_candidate",
                id: candidate_id.to_string(),
            })?;
        candidate_entry.status = KnowledgeStatus::VerifiedShared;
        candidate_entry.promoted_asset_id = Some(asset.id.clone());
        candidate_entry.clone()
    };

    snapshot
        .knowledge_assets
        .insert(asset.id.clone(), asset.clone());
    snapshot.knowledge_lineage.push(KnowledgeLineageRecord {
        candidate_id: updated_candidate.id.clone(),
        asset_id: asset.id.clone(),
        run_id: updated_candidate.run_id.clone(),
        artifact_id: updated_candidate.artifact_id.clone(),
        occurred_at: now_iso(),
    });

    push_trace(
        snapshot,
        &updated_candidate.run_id,
        trace_event(
            "KnowledgeCandidatePromoted",
            format!(
                "Candidate {} promoted to asset {}",
                updated_candidate.id, asset.id
            ),
        ),
    );
    push_audit(
        snapshot,
        &updated_candidate.run_id,
        audit_entry("knowledge.asset.promoted", &request.promoted_by, &asset.id),
    );

    Ok(KnowledgePromotionResponse {
        candidate: updated_candidate,
        asset,
    })
}

fn resolve_approval_inner(
    snapshot: &mut RuntimeSnapshot,
    approval_id: &str,
    request: ApprovalResolutionRequest,
    decision: ApprovalDecision,
) -> Result<RunDetailResponse, RuntimeError> {
    let (run_id, resolved_approval_id) = {
        let approval = snapshot
            .approvals
            .get(approval_id)
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "approval",
                id: approval_id.to_string(),
            })?;

        (approval.run_id.clone(), approval.id.clone())
    };

    {
        let run = snapshot.runs.get(&run_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "run",
            id: run_id.clone(),
        })?;

        if run.status != RunStatus::WaitingApproval {
            return Err(RuntimeError::InvalidState {
                kind: "approval",
                id: approval_id.to_string(),
                reason: "approval can only be resolved while waiting_approval".into(),
            });
        }
    }

    {
        let approval = snapshot
            .approvals
            .get_mut(approval_id)
            .ok_or_else(|| RuntimeError::NotFound {
                kind: "approval",
                id: approval_id.to_string(),
            })?;

        approval.reviewed_by = Some(request.reviewed_by.clone());
        approval.state = decision.into();
    }

    {
        let run = snapshot.runs.get_mut(&run_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "run",
            id: run_id.clone(),
        })?;

        run.status = decision.next_status();
        if decision == ApprovalDecision::Rejected {
            run.checkpoint_token = None;
        }
        run.updated_at = now_iso();
    }

    if let Some(inbox_item) = snapshot.inbox_items.get_mut(&run_id) {
        inbox_item.state = InboxState::Resolved;
    }

    if decision == ApprovalDecision::Approved {
        push_trace(
            snapshot,
            &run_id,
            trace_event(
                "ApprovalResolved",
                format!(
                    "Approval {} approved by {}",
                    resolved_approval_id, request.reviewed_by
                ),
            ),
        );
        push_trace(
            snapshot,
            &run_id,
            trace_event(
                "RunStateChanged",
                format!("Run {} paused and ready to resume", run_id),
            ),
        );
        push_audit(
            snapshot,
            &run_id,
            audit_entry("approval.approved", &request.reviewed_by, &resolved_approval_id),
        );
    } else {
        snapshot.environment_leases.remove(&run_id);
        push_trace(
            snapshot,
            &run_id,
            trace_event(
                "ApprovalResolved",
                format!(
                    "Approval {} rejected by {}",
                    resolved_approval_id, request.reviewed_by
                ),
            ),
        );
        push_trace(
            snapshot,
            &run_id,
            trace_event(
                "RunStateChanged",
                format!("Run {} terminated after rejection", run_id),
            ),
        );
        push_audit(
            snapshot,
            &run_id,
            audit_entry("approval.rejected", &request.reviewed_by, &resolved_approval_id),
        );
    }

    hydrate_response(snapshot, &run_id).ok_or_else(|| RuntimeError::NotFound {
        kind: "run",
        id: run_id,
    })
}

fn resume_run_inner(
    snapshot: &mut RuntimeSnapshot,
    run_id: &str,
) -> Result<RunDetailResponse, RuntimeError> {
    let (project_id, title, requested_by, checkpoint_token) = {
        let run = snapshot.runs.get(run_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "run",
            id: run_id.to_string(),
        })?;

        if run.status != RunStatus::Paused {
            return Err(RuntimeError::InvalidState {
                kind: "run",
                id: run.id.clone(),
                reason: "resume is only allowed after approval grants a checkpoint".into(),
            });
        }

        (
            run.project_id.clone(),
            run.title.clone(),
            run.requested_by.clone(),
            run.checkpoint_token.clone().ok_or_else(|| RuntimeError::InvalidState {
                kind: "run",
                id: run.id.clone(),
                reason: "paused runs require a checkpoint token".into(),
            })?,
        )
    };

    let lease = snapshot
        .environment_leases
        .get(run_id)
        .ok_or_else(|| RuntimeError::InvalidState {
            kind: "run",
            id: run_id.to_string(),
            reason: "resume requires an active environment lease".into(),
        })?;
    if lease.resume_token != checkpoint_token {
        return Err(RuntimeError::InvalidState {
            kind: "run",
            id: run_id.to_string(),
            reason: "resume token does not match the active environment lease".into(),
        });
    }

    {
        let run = snapshot.runs.get_mut(run_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "run",
            id: run_id.to_string(),
        })?;
        run.status = RunStatus::Running;
        run.checkpoint_token = None;
        run.updated_at = now_iso();
    }

    snapshot.environment_leases.remove(run_id);

    push_trace(
        snapshot,
        run_id,
        trace_event("RunStateChanged", format!("Run {} resumed execution", run_id)),
    );

    let artifact = build_artifact(
        run_id,
        &project_id,
        &title,
        Some("Generated after explicit resume"),
    );
    store_artifact_and_audit(snapshot, artifact, &requested_by, run_id);

    {
        let run = snapshot.runs.get_mut(run_id).ok_or_else(|| RuntimeError::NotFound {
            kind: "run",
            id: run_id.to_string(),
        })?;

        run.status = RunStatus::Completed;
        run.updated_at = now_iso();
    }

    push_trace(
        snapshot,
        run_id,
        trace_event("RunStateChanged", format!("Run {} completed after resume", run_id)),
    );
    push_audit(
        snapshot,
        run_id,
        audit_entry("run.resumed", &requested_by, run_id),
    );

    hydrate_response(snapshot, run_id).ok_or_else(|| RuntimeError::NotFound {
        kind: "run",
        id: run_id.to_string(),
    })
}

fn hydrate_response(snapshot: &RuntimeSnapshot, run_id: &str) -> Option<RunDetailResponse> {
    let run = snapshot.runs.get(run_id)?.clone();
    let artifact = snapshot.artifacts.get(run_id).cloned();
    let approval = snapshot
        .approvals
        .values()
        .find(|entry| entry.run_id == run_id)
        .cloned();
    let inbox_item = snapshot.inbox_items.get(run_id).cloned();
    let trace = snapshot.traces.get(run_id).cloned().unwrap_or_default();
    let audit = snapshot.audits.get(run_id).cloned().unwrap_or_default();

    Some(RunDetailResponse {
        run,
        artifact,
        approval,
        inbox_item,
        trace,
        audit,
    })
}

fn hydrate_automation_detail(
    snapshot: &RuntimeSnapshot,
    automation_id: &str,
) -> Option<AutomationDetailResponse> {
    let automation = snapshot.automations.get(automation_id)?.clone();
    let trigger_id = automation.trigger_ids.first()?.clone();
    let trigger = snapshot.triggers.get(&trigger_id)?.clone();
    let latest_delivery = snapshot
        .latest_delivery_by_trigger
        .get(&trigger_id)
        .and_then(|delivery_id| snapshot.trigger_deliveries.get(delivery_id))
        .cloned();
    let latest_run = automation
        .last_run_id
        .as_deref()
        .and_then(|run_id| hydrate_response(snapshot, run_id));

    Some(AutomationDetailResponse {
        automation,
        trigger,
        latest_delivery,
        latest_run,
    })
}

fn hydrate_knowledge_space_detail(
    snapshot: &RuntimeSnapshot,
    knowledge_space_id: &str,
) -> Option<KnowledgeSpaceDetailResponse> {
    let space = snapshot.knowledge_spaces.get(knowledge_space_id)?.clone();
    let mut candidates = snapshot
        .knowledge_candidates
        .values()
        .filter(|candidate| candidate.knowledge_space_id == knowledge_space_id)
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.created_at.cmp(&right.created_at));

    let mut assets = snapshot
        .knowledge_assets
        .values()
        .filter(|asset| asset.knowledge_space_id == knowledge_space_id)
        .cloned()
        .collect::<Vec<_>>();
    assets.sort_by(|left, right| left.created_at.cmp(&right.created_at));

    Some(KnowledgeSpaceDetailResponse {
        space,
        candidates,
        assets,
    })
}

fn record_failed_delivery(
    snapshot: &mut RuntimeSnapshot,
    trigger: &TriggerRecord,
    dedupe_key: &str,
    failure_reason: String,
) -> TriggerDeliveryRecord {
    let delivery_id = Uuid::new_v4().to_string();
    let composite_dedupe_key = format!("{}:{dedupe_key}", trigger.id);
    let delivery = TriggerDeliveryRecord {
        id: delivery_id.clone(),
        trigger_id: trigger.id.clone(),
        source_type: trigger.source_type,
        dedupe_key: dedupe_key.to_string(),
        state: TriggerDeliveryState::Failed,
        run_id: None,
        failure_reason: Some(failure_reason),
        occurred_at: now_iso(),
    };

    snapshot
        .trigger_deliveries
        .insert(delivery_id.clone(), delivery.clone());
    snapshot
        .delivery_dedupe_index
        .insert(composite_dedupe_key, delivery_id.clone());
    snapshot
        .latest_delivery_by_trigger
        .insert(trigger.id.clone(), delivery_id);

    delivery
}

fn sync_trigger_states(
    snapshot: &mut RuntimeSnapshot,
    automation_id: &str,
    automation_state: AutomationState,
) {
    let trigger_state = match automation_state {
        AutomationState::Active => "active",
        AutomationState::Draft => "draft",
        AutomationState::Paused => "paused",
        AutomationState::Suspended => "suspended",
        AutomationState::Archived => "archived",
    };

    for trigger in snapshot
        .triggers
        .values_mut()
        .filter(|entry| entry.automation_id == automation_id)
    {
        trigger.state = trigger_state.to_string();
    }
}

fn trigger_run_type(source_type: TriggerSource) -> RunType {
    match source_type {
        TriggerSource::Cron => RunType::Automation,
        TriggerSource::Webhook | TriggerSource::ManualEvent | TriggerSource::McpEvent => {
            RunType::Watch
        }
    }
}

fn build_artifact(
    run_id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
) -> ArtifactRecord {
    ArtifactRecord {
        id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        run_id: run_id.to_string(),
        version: 1,
        title: format!("Artifact for {}", title),
        content_ref: description
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "Generated by the minimal runtime skeleton".into()),
        state: "current".into(),
        created_at: now_iso(),
    }
}

fn store_artifact_and_audit(
    snapshot: &mut RuntimeSnapshot,
    artifact: ArtifactRecord,
    actor: &str,
    run_id: &str,
) {
    let artifact_id = artifact.id.clone();
    snapshot.artifacts.insert(run_id.to_string(), artifact);
    push_audit(snapshot, run_id, audit_entry("artifact.created", actor, &artifact_id));
}

fn trace_event(name: &str, message: String) -> TraceEvent {
    TraceEvent {
        name: name.to_string(),
        message,
        occurred_at: now_iso(),
    }
}

fn audit_entry(action: &str, actor: &str, target_ref: &str) -> AuditEntry {
    AuditEntry {
        action: action.to_string(),
        actor: actor.to_string(),
        target_ref: target_ref.to_string(),
        occurred_at: now_iso(),
    }
}

fn push_trace(snapshot: &mut RuntimeSnapshot, run_id: &str, entry: TraceEvent) {
    snapshot.traces.entry(run_id.to_string()).or_default().push(entry);
}

fn push_audit(snapshot: &mut RuntimeSnapshot, run_id: &str, entry: AuditEntry) {
    snapshot.audits.entry(run_id.to_string()).or_default().push(entry);
}

fn new_runtime_event(
    topic: &str,
    run_id: Option<String>,
    workspace_id: Option<String>,
    automation_id: Option<String>,
    trigger_id: Option<String>,
    candidate_id: Option<String>,
    asset_id: Option<String>,
) -> NewRuntimeEvent {
    NewRuntimeEvent {
        topic: topic.to_string(),
        occurred_at: now_iso(),
        run_id,
        workspace_id,
        automation_id,
        trigger_id,
        candidate_id,
        asset_id,
    }
}

fn events_for_run_detail(
    detail: &RunDetailResponse,
    workspace_id: Option<String>,
) -> Vec<NewRuntimeEvent> {
    let mut events = vec![new_runtime_event(
        "run.state_changed",
        Some(detail.run.id.clone()),
        workspace_id.clone(),
        None,
        None,
        None,
        None,
    )];

    if detail.approval.is_some() {
        events.push(new_runtime_event(
            "approval.updated",
            Some(detail.run.id.clone()),
            workspace_id,
            None,
            None,
            None,
            None,
        ));
    }

    events
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

impl ApprovalDecision {
    fn next_status(self) -> RunStatus {
        match self {
            Self::Approved => RunStatus::Paused,
            Self::Rejected => RunStatus::Terminated,
        }
    }
}

impl TryFrom<&str> for ApprovalDecision {
    type Error = RuntimeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            _ => Err(RuntimeError::InvalidDecision {
                decision: value.to_string(),
            }),
        }
    }
}

impl From<ApprovalDecision> for ApprovalState {
    fn from(value: ApprovalDecision) -> Self {
        match value {
            ApprovalDecision::Approved => Self::Approved,
            ApprovalDecision::Rejected => Self::Rejected,
        }
    }
}
