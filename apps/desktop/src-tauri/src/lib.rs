use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};

use chrono::Utc;
use octopus_execution::ExecutionAction;
use octopus_runtime::{
    ApprovalDecision, ApprovalRequestRecord, ArtifactRecord, AuditRecord, AutomationDetailRecord,
    AutomationRecord, BudgetPolicyRecord, CapabilityBindingRecord, CapabilityDescriptorRecord,
    CapabilityGrantRecord, CreateAutomationInput, CreateTaskInput, CreateTriggerInput,
    DispatchManualEventInput, InboxItemRecord, KnowledgeAssetRecord, KnowledgeCandidateRecord,
    KnowledgeLineageRecord, KnowledgeSpaceRecord, NotificationRecord, PolicyDecisionLogRecord,
    ProjectKnowledgeIndexRecord, RunExecutionReport, RunRecord, RunSummaryRecord, Slice2Runtime,
    TaskRecord, TraceRecord, TriggerRecord,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{Emitter, Manager, Runtime, State};
use thiserror::Error;

const DEFAULT_WORKSPACE_ID: &str = "demo";
const DEFAULT_WORKSPACE_SLUG: &str = "demo";
const DEFAULT_WORKSPACE_NAME: &str = "Demo Workspace";
const DEFAULT_PROJECT_ID: &str = "demo";
const DEFAULT_PROJECT_SLUG: &str = "demo";
const DEFAULT_PROJECT_NAME: &str = "Demo Project";
const DEFAULT_KNOWLEDGE_SPACE_NAME: &str = "Demo Project Knowledge";
const DEFAULT_KNOWLEDGE_OWNER_REF: &str = "workspace_admin:desktop_operator";
const DEFAULT_CAPABILITY_ID: &str = "capability-local-demo";
const DEFAULT_CAPABILITY_RISK_LEVEL: &str = "low";
const DEFAULT_BUDGET_SOFT_LIMIT: i64 = 5;
const DEFAULT_BUDGET_HARD_LIMIT: i64 = 10;
const LOCAL_MODE_UNSUPPORTED_TRIGGER_MESSAGE: &str =
    "Local host only supports manual_event and cron";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalHubTransportCommands {
    pub list_projects: String,
    pub get_project_context: String,
    pub get_project_knowledge: String,
    pub list_automations: String,
    pub create_automation: String,
    pub get_automation_detail: String,
    pub activate_automation: String,
    pub pause_automation: String,
    pub archive_automation: String,
    pub manual_dispatch: String,
    pub retry_trigger_delivery: String,
    pub create_task: String,
    pub start_task: String,
    pub list_runs: String,
    pub get_run_detail: String,
    pub retry_run: String,
    pub terminate_run: String,
    pub get_approval_request: String,
    pub resolve_approval: String,
    pub list_inbox_items: String,
    pub list_notifications: String,
    pub list_artifacts: String,
    pub get_knowledge_detail: String,
    pub request_knowledge_promotion: String,
    pub promote_knowledge: String,
    pub list_capability_visibility: String,
    pub get_connection_status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalHubTransportContract {
    pub event_channel: String,
    pub commands: LocalHubTransportCommands,
}

pub fn local_hub_transport_contract() -> &'static LocalHubTransportContract {
    static CONTRACT: OnceLock<LocalHubTransportContract> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../schemas/interop/local-hub-transport.json"))
            .expect("valid local hub transport contract")
    })
}

pub fn normalize_tauri_invoke_command(command: &str) -> String {
    command
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct LocalHostConfig {
    db_path: PathBuf,
}

impl LocalHostConfig {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    fn db_path(&self) -> &PathBuf {
        &self.db_path
    }
}

#[derive(Debug, Clone)]
pub struct EmittedEvent {
    pub channel: String,
    pub payload: Value,
}

pub trait LocalHubEventEmitter: Send + Sync {
    fn emit(&self, channel: &str, payload: &Value) -> Result<(), LocalHostError>;
}

#[derive(Debug, Default)]
pub struct CollectingEventEmitter {
    events: Mutex<Vec<EmittedEvent>>,
}

impl CollectingEventEmitter {
    pub fn events_snapshot(&self) -> Vec<EmittedEvent> {
        self.events.lock().expect("events mutex poisoned").clone()
    }
}

impl LocalHubEventEmitter for CollectingEventEmitter {
    fn emit(&self, channel: &str, payload: &Value) -> Result<(), LocalHostError> {
        self.events
            .lock()
            .expect("events mutex poisoned")
            .push(EmittedEvent {
                channel: channel.to_string(),
                payload: payload.clone(),
            });
        Ok(())
    }
}

#[derive(Clone)]
pub struct DesktopLocalHost {
    inner: Arc<DesktopLocalHostInner>,
}

struct DesktopLocalHostInner {
    runtime: Slice2Runtime,
    emitter: Arc<dyn LocalHubEventEmitter>,
    sequence: AtomicU64,
}

#[derive(Debug, Serialize)]
struct RunDetailResponse {
    run: RunRecord,
    task: TaskRecord,
    artifacts: Vec<ArtifactRecord>,
    audits: Vec<AuditRecord>,
    traces: Vec<TraceRecord>,
    approvals: Vec<ApprovalRequestRecord>,
    inbox_items: Vec<InboxItemRecord>,
    notifications: Vec<NotificationRecord>,
    policy_decisions: Vec<PolicyDecisionLogRecord>,
    knowledge_candidates: Vec<KnowledgeCandidateRecord>,
    knowledge_assets: Vec<KnowledgeAssetRecord>,
    knowledge_lineage: Vec<KnowledgeLineageRecord>,
}

#[derive(Debug, Serialize)]
struct KnowledgeDetailResponse {
    knowledge_space: KnowledgeSpaceRecord,
    candidates: Vec<KnowledgeCandidateRecord>,
    assets: Vec<KnowledgeAssetRecord>,
    lineage: Vec<KnowledgeLineageRecord>,
}

type ProjectKnowledgeIndexResponse = ProjectKnowledgeIndexRecord;

#[derive(Debug, Serialize)]
struct HubConnectionServerSummary {
    id: String,
    capability_id: String,
    namespace: String,
    platform: String,
    trust_level: String,
    health_status: String,
    lease_ttl_seconds: i64,
    last_checked_at: String,
}

#[derive(Debug, Serialize)]
struct HubConnectionStatusResponse {
    mode: String,
    state: String,
    auth_state: String,
    active_server_count: usize,
    healthy_server_count: usize,
    servers: Vec<HubConnectionServerSummary>,
    last_refreshed_at: String,
}

#[derive(Debug, Serialize)]
struct SurfaceCreateAutomationResponse {
    automation: AutomationRecord,
    trigger: TriggerRecord,
    webhook_secret: Option<String>,
}

#[derive(Debug, Error)]
pub enum LocalHostError {
    #[error("unsupported local transport command `{0}`")]
    UnsupportedCommand(String),
    #[error("invalid payload for `{command}`: {source}")]
    InvalidPayload {
        command: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Runtime(#[from] octopus_runtime::RuntimeError),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("failed to emit Tauri event `{channel}`: {message}")]
    TauriEmit { channel: String, message: String },
}

#[derive(Debug, Deserialize)]
struct ProjectScopedCommand {
    #[serde(rename = "workspaceId", alias = "workspace_id")]
    workspace_id: String,
    #[serde(rename = "projectId", alias = "project_id")]
    project_id: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceScopedCommand {
    #[serde(rename = "workspaceId", alias = "workspace_id")]
    workspace_id: String,
}

#[derive(Debug, Deserialize)]
struct AutomationIdCommand {
    #[serde(rename = "automationId", alias = "automation_id")]
    automation_id: String,
}

#[derive(Debug, Deserialize)]
struct RunIdCommand {
    #[serde(rename = "runId", alias = "run_id")]
    run_id: String,
}

#[derive(Debug, Deserialize)]
struct ApprovalIdCommand {
    #[serde(rename = "approvalId", alias = "approval_id")]
    approval_id: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceIdCommand {
    #[serde(rename = "workspaceId", alias = "workspace_id")]
    workspace_id: String,
}

#[derive(Debug, Deserialize)]
struct CapabilityVisibilityCommand {
    #[serde(rename = "workspaceId", alias = "workspace_id")]
    workspace_id: String,
    #[serde(rename = "projectId", alias = "project_id")]
    project_id: String,
    #[serde(rename = "estimatedCost", alias = "estimated_cost")]
    estimated_cost: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SurfaceTaskCreateCommand {
    workspace_id: String,
    project_id: String,
    title: String,
    instruction: String,
    action: ExecutionAction,
    capability_id: String,
    estimated_cost: i64,
    idempotency_key: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceAutomationLifecycleCommand {
    automation_id: String,
    action: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceApprovalResolveCommand {
    approval_id: String,
    decision: String,
    actor_ref: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceRequestKnowledgePromotionCommand {
    candidate_id: String,
    actor_ref: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceKnowledgePromoteCommand {
    candidate_id: String,
    actor_ref: String,
    note: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceTriggerDeliveryRetryCommand {
    delivery_id: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceRunRetryCommand {
    run_id: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceRunTerminateCommand {
    run_id: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceCreateAutomationCommand {
    workspace_id: String,
    project_id: String,
    title: String,
    instruction: String,
    action: ExecutionAction,
    capability_id: String,
    estimated_cost: i64,
    trigger: SurfaceCreateTriggerInput,
}

#[derive(Debug, Deserialize)]
struct SurfaceManualEventTriggerConfig {}

#[derive(Debug, Deserialize)]
struct SurfaceCronTriggerConfig {
    schedule: String,
    timezone: String,
    next_fire_at: String,
}

#[derive(Debug, Deserialize)]
struct SurfaceWebhookTriggerConfig {
    ingress_mode: String,
    secret_header_name: String,
    secret_hint: Option<String>,
    secret_plaintext: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SurfaceMcpEventTriggerConfig {
    server_id: String,
    event_name: Option<String>,
    event_pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "trigger_type", rename_all = "snake_case")]
enum SurfaceCreateTriggerInput {
    ManualEvent { config: SurfaceManualEventTriggerConfig },
    Cron { config: SurfaceCronTriggerConfig },
    Webhook { config: SurfaceWebhookTriggerConfig },
    McpEvent { config: SurfaceMcpEventTriggerConfig },
}

impl SurfaceCreateTriggerInput {
    fn into_runtime(self) -> Result<CreateTriggerInput, LocalHostError> {
        match self {
            Self::ManualEvent { config } => {
                let _ = config;
                Ok(CreateTriggerInput::ManualEvent)
            }
            Self::Cron { config } => Ok(CreateTriggerInput::Cron {
                schedule: config.schedule,
                timezone: config.timezone,
                next_fire_at: config.next_fire_at,
            }),
            Self::Webhook { config } => {
                let _ = (
                    config.ingress_mode,
                    config.secret_header_name,
                    config.secret_hint,
                    config.secret_plaintext,
                );
                Err(LocalHostError::BadRequest(
                    LOCAL_MODE_UNSUPPORTED_TRIGGER_MESSAGE.to_string(),
                ))
            }
            Self::McpEvent { config } => {
                let _ = (config.server_id, config.event_name, config.event_pattern);
                Err(LocalHostError::BadRequest(
                    LOCAL_MODE_UNSUPPORTED_TRIGGER_MESSAGE.to_string(),
                ))
            }
        }
    }
}

impl DesktopLocalHost {
    pub async fn open(
        config: LocalHostConfig,
        emitter: Arc<dyn LocalHubEventEmitter>,
    ) -> Result<Self, LocalHostError> {
        if let Some(parent) = config.db_path().parent() {
            fs::create_dir_all(parent)?;
        }

        let runtime = Slice2Runtime::open_at(config.db_path()).await?;
        let host = Self {
            inner: Arc::new(DesktopLocalHostInner {
                runtime,
                emitter,
                sequence: AtomicU64::new(1),
            }),
        };
        host.seed_demo_context().await?;
        host.emit_connection_updated().await?;
        Ok(host)
    }

    pub async fn invoke_transport_command(
        &self,
        command: &str,
        payload: Value,
    ) -> Result<Value, LocalHostError> {
        let normalized_command = normalize_tauri_invoke_command(command);
        let contract = local_hub_transport_contract();
        let commands = &contract.commands;

        if normalized_command
            == normalize_tauri_invoke_command(commands.list_projects.as_str())
        {
            let command = self
                .parse_payload::<WorkspaceScopedCommand>(commands.list_projects.as_str(), payload)?;
            return Ok(json!(
                self.inner.runtime.list_projects(&command.workspace_id).await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.get_project_context.as_str())
        {
            let command = self.parse_payload::<ProjectScopedCommand>(
                commands.get_project_context.as_str(),
                payload,
            )?;
            return Ok(json!(
                self.inner
                    .runtime
                    .fetch_project_context(&command.workspace_id, &command.project_id)
                    .await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.get_project_knowledge.as_str())
        {
            let command = self.parse_payload::<ProjectScopedCommand>(
                commands.get_project_knowledge.as_str(),
                payload,
            )?;
            return Ok(json!(
                self.build_project_knowledge_index_response(
                    &command.workspace_id,
                    &command.project_id,
                )
                .await?
            ));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.list_automations.as_str()) {
            let command =
                self.parse_payload::<ProjectScopedCommand>(commands.list_automations.as_str(), payload)?;
            return Ok(json!(
                self.inner
                    .runtime
                    .list_automations(&command.workspace_id, &command.project_id)
                    .await?
            ));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.create_automation.as_str())
        {
            let command = self.parse_payload::<SurfaceCreateAutomationCommand>(
                commands.create_automation.as_str(),
                payload,
            )?;
            let report = self
                .inner
                .runtime
                .create_automation_with_trigger(
                    CreateAutomationInput {
                        workspace_id: command.workspace_id,
                        project_id: command.project_id,
                        title: command.title,
                        instruction: command.instruction,
                        action: command.action,
                        capability_id: command.capability_id,
                        estimated_cost: command.estimated_cost,
                    },
                    command.trigger.into_runtime()?,
                )
                .await?;
            return Ok(json!(SurfaceCreateAutomationResponse {
                automation: report.automation,
                trigger: report.trigger,
                webhook_secret: report.webhook_secret,
            }));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.get_automation_detail.as_str())
        {
            let command = self.parse_payload::<AutomationIdCommand>(
                commands.get_automation_detail.as_str(),
                payload,
            )?;
            return Ok(json!(
                self.inner
                    .runtime
                    .load_automation_detail(&command.automation_id)
                    .await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.activate_automation.as_str())
        {
            let detail = self
                .transition_automation(
                    commands.activate_automation.as_str(),
                    payload,
                    "activate",
                )
                .await?;
            return Ok(json!(detail));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.pause_automation.as_str()) {
            let detail = self
                .transition_automation(commands.pause_automation.as_str(), payload, "pause")
                .await?;
            return Ok(json!(detail));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.archive_automation.as_str())
        {
            let detail = self
                .transition_automation(commands.archive_automation.as_str(), payload, "archive")
                .await?;
            return Ok(json!(detail));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.manual_dispatch.as_str()) {
            let command = self.parse_payload::<DispatchManualEventInput>(
                commands.manual_dispatch.as_str(),
                payload,
            )?;
            let report = self.inner.runtime.dispatch_manual_event(command).await?;
            self.emit_run_updated(&report.run_report.run, &report.task).await?;
            self.emit_workspace_updates(&report.run_report.run.workspace_id)
                .await?;
            return Ok(json!(
                self.inner
                    .runtime
                    .load_automation_detail(&report.automation.id)
                    .await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.retry_trigger_delivery.as_str())
        {
            let command = self.parse_payload::<SurfaceTriggerDeliveryRetryCommand>(
                commands.retry_trigger_delivery.as_str(),
                payload,
            )?;
            let report = self
                .inner
                .runtime
                .retry_trigger_delivery(&command.delivery_id)
                .await?;
            self.emit_run_updated(&report.run_report.run, &report.task).await?;
            self.emit_workspace_updates(&report.run_report.run.workspace_id)
                .await?;
            return Ok(json!(
                self.inner
                    .runtime
                    .load_automation_detail(&report.automation.id)
                    .await?
            ));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.create_task.as_str()) {
            let command =
                self.parse_payload::<SurfaceTaskCreateCommand>(commands.create_task.as_str(), payload)?;
            let task = self
                .inner
                .runtime
                .create_task(CreateTaskInput {
                    workspace_id: command.workspace_id,
                    project_id: command.project_id,
                    source_kind: "manual".to_string(),
                    automation_id: None,
                    title: command.title,
                    instruction: command.instruction,
                    action: command.action,
                    capability_id: command.capability_id,
                    estimated_cost: command.estimated_cost,
                    idempotency_key: command.idempotency_key,
                })
                .await?;
            return Ok(json!(task));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.start_task.as_str()) {
            let task_id = self.parse_payload::<TaskIdCommand>(commands.start_task.as_str(), payload)?;
            let report = self.inner.runtime.start_task(&task_id.task_id).await?;
            let response = self.build_run_detail_response(report).await?;
            self.emit_run_updated(&response.run, &response.task).await?;
            self.emit_workspace_updates(&response.run.workspace_id).await?;
            return Ok(json!(response));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.list_runs.as_str()) {
            let command =
                self.parse_payload::<ProjectScopedCommand>(commands.list_runs.as_str(), payload)?;
            return Ok(json!(
                self.inner
                    .runtime
                    .list_runs(&command.workspace_id, &command.project_id)
                    .await?
            ));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.get_run_detail.as_str()) {
            let command =
                self.parse_payload::<RunIdCommand>(commands.get_run_detail.as_str(), payload)?;
            let response = self
                .build_run_detail_response(self.inner.runtime.load_run_report(&command.run_id).await?)
                .await?;
            return Ok(json!(response));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.retry_run.as_str()) {
            let command =
                self.parse_payload::<SurfaceRunRetryCommand>(commands.retry_run.as_str(), payload)?;
            let response = self
                .build_run_detail_response(self.inner.runtime.retry_run(&command.run_id).await?)
                .await?;
            self.emit_run_updated(&response.run, &response.task).await?;
            self.emit_workspace_updates(&response.run.workspace_id).await?;
            return Ok(json!(response));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.terminate_run.as_str()) {
            let command = self.parse_payload::<SurfaceRunTerminateCommand>(
                commands.terminate_run.as_str(),
                payload,
            )?;
            let response = self
                .build_run_detail_response(
                    self.inner
                        .runtime
                        .terminate_run(&command.run_id, &command.reason)
                        .await?,
                )
                .await?;
            self.emit_run_updated(&response.run, &response.task).await?;
            self.emit_workspace_updates(&response.run.workspace_id).await?;
            return Ok(json!(response));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.get_approval_request.as_str())
        {
            let command = self.parse_payload::<ApprovalIdCommand>(
                commands.get_approval_request.as_str(),
                payload,
            )?;
            let approval = self
                .inner
                .runtime
                .fetch_approval_request(&command.approval_id)
                .await?
                .ok_or_else(|| {
                    LocalHostError::NotFound(format!(
                        "approval request `{}` not found",
                        command.approval_id
                    ))
                })?;
            return Ok(json!(approval));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.resolve_approval.as_str()) {
            let command = self.parse_payload::<SurfaceApprovalResolveCommand>(
                commands.resolve_approval.as_str(),
                payload,
            )?;
            let decision = parse_approval_decision(command.decision.as_str())?;
            let report = self
                .inner
                .runtime
                .resolve_approval(
                    &command.approval_id,
                    decision,
                    &command.actor_ref,
                    &command.note,
                )
                .await?;
            let response = self.build_run_detail_response(report).await?;
            self.emit_run_updated(&response.run, &response.task).await?;
            self.emit_workspace_updates(&response.run.workspace_id).await?;
            return Ok(json!(response));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.list_inbox_items.as_str()) {
            let command = self.parse_payload::<WorkspaceIdCommand>(
                commands.list_inbox_items.as_str(),
                payload,
            )?;
            return Ok(json!(
                self.inner
                    .runtime
                    .list_inbox_items_by_workspace(&command.workspace_id)
                    .await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.list_notifications.as_str())
        {
            let command = self.parse_payload::<WorkspaceIdCommand>(
                commands.list_notifications.as_str(),
                payload,
            )?;
            let notifications = self
                .normalize_notifications(
                    self.inner
                        .runtime
                        .list_notifications_by_workspace(&command.workspace_id)
                        .await?,
                )
                .await?;
            return Ok(json!(notifications));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.list_artifacts.as_str()) {
            let command =
                self.parse_payload::<RunIdCommand>(commands.list_artifacts.as_str(), payload)?;
            return Ok(json!(
                self.inner
                    .runtime
                    .list_artifacts_by_run(&command.run_id)
                    .await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.get_knowledge_detail.as_str())
        {
            let command = self.parse_payload::<RunIdCommand>(
                commands.get_knowledge_detail.as_str(),
                payload,
            )?;
            return Ok(json!(
                self.build_knowledge_detail_response(&command.run_id).await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.request_knowledge_promotion.as_str())
        {
            let command = self.parse_payload::<SurfaceRequestKnowledgePromotionCommand>(
                commands.request_knowledge_promotion.as_str(),
                payload,
            )?;
            let approval = self
                .inner
                .runtime
                .request_knowledge_promotion(
                    &command.candidate_id,
                    &command.actor_ref,
                    &command.note,
                )
                .await?;
            self.emit_workspace_updates(&approval.workspace_id).await?;
            return Ok(json!(approval));
        }

        if normalized_command == normalize_tauri_invoke_command(commands.promote_knowledge.as_str())
        {
            let command = self.parse_payload::<SurfaceKnowledgePromoteCommand>(
                commands.promote_knowledge.as_str(),
                payload,
            )?;
            let report = self
                .inner
                .runtime
                .promote_knowledge_candidate(
                    &command.candidate_id,
                    &command.actor_ref,
                    &command.note,
                )
                .await?;
            return Ok(json!(
                self.build_knowledge_detail_response(&report.candidate.source_run_id)
                    .await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.list_capability_visibility.as_str())
        {
            let command = self.parse_payload::<CapabilityVisibilityCommand>(
                commands.list_capability_visibility.as_str(),
                payload,
            )?;
            let estimated_cost = command.estimated_cost.unwrap_or(1);
            if estimated_cost < 0 {
                return Err(LocalHostError::BadRequest(
                    "estimated_cost must be greater than or equal to 0".to_string(),
                ));
            }
            return Ok(json!(
                self.inner
                    .runtime
                    .list_capability_resolutions(
                        &command.workspace_id,
                        &command.project_id,
                        estimated_cost,
                    )
                    .await?
            ));
        }

        if normalized_command
            == normalize_tauri_invoke_command(commands.get_connection_status.as_str())
        {
            return Ok(json!(self.build_connection_status().await?));
        }

        Err(LocalHostError::UnsupportedCommand(command.to_string()))
    }

    pub async fn tick_due_triggers(&self, now: &str) -> Result<Vec<Value>, LocalHostError> {
        let reports = self.inner.runtime.tick_due_triggers(now).await?;
        let mut payloads = Vec::with_capacity(reports.len());
        for report in reports {
            let automation_id = report.automation.id.clone();
            let detail = self
                .inner
                .runtime
                .load_automation_detail(&automation_id)
                .await?;
            self.emit_run_updated(&report.run_report.run, &report.task).await?;
            self.emit_workspace_updates(&report.run_report.run.workspace_id)
                .await?;
            payloads.push(serde_json::to_value(detail)?);
        }
        Ok(payloads)
    }

    async fn seed_demo_context(&self) -> Result<(), LocalHostError> {
        self.inner
            .runtime
            .ensure_project_context(
                DEFAULT_WORKSPACE_ID,
                DEFAULT_WORKSPACE_SLUG,
                DEFAULT_WORKSPACE_NAME,
                DEFAULT_PROJECT_ID,
                DEFAULT_PROJECT_SLUG,
                DEFAULT_PROJECT_NAME,
            )
            .await?;
        self.inner
            .runtime
            .ensure_project_knowledge_space(
                DEFAULT_WORKSPACE_ID,
                DEFAULT_PROJECT_ID,
                DEFAULT_KNOWLEDGE_SPACE_NAME,
                DEFAULT_KNOWLEDGE_OWNER_REF,
            )
            .await?;

        let mut descriptor = CapabilityDescriptorRecord::new(
            DEFAULT_CAPABILITY_ID,
            DEFAULT_CAPABILITY_ID,
            DEFAULT_CAPABILITY_RISK_LEVEL,
            false,
        );
        descriptor.source = "octopus-runtime".to_string();
        descriptor.platform = "local".to_string();

        self.inner
            .runtime
            .upsert_capability_descriptor(descriptor)
            .await?;
        self.inner
            .runtime
            .upsert_capability_binding(CapabilityBindingRecord::project_scope(
                format!("binding-{DEFAULT_CAPABILITY_ID}"),
                DEFAULT_CAPABILITY_ID,
                DEFAULT_WORKSPACE_ID,
                DEFAULT_PROJECT_ID,
            ))
            .await?;
        self.inner
            .runtime
            .upsert_capability_grant(CapabilityGrantRecord::project_scope(
                format!("grant-{DEFAULT_CAPABILITY_ID}"),
                DEFAULT_CAPABILITY_ID,
                DEFAULT_WORKSPACE_ID,
                DEFAULT_PROJECT_ID,
            ))
            .await?;
        self.inner
            .runtime
            .upsert_budget_policy(BudgetPolicyRecord::project_scope(
                format!("budget-{DEFAULT_PROJECT_ID}"),
                DEFAULT_WORKSPACE_ID,
                DEFAULT_PROJECT_ID,
                DEFAULT_BUDGET_SOFT_LIMIT,
                DEFAULT_BUDGET_HARD_LIMIT,
            ))
            .await?;
        Ok(())
    }

    async fn transition_automation(
        &self,
        command_name: &str,
        payload: Value,
        expected_action: &str,
    ) -> Result<AutomationDetailRecord, LocalHostError> {
        let command =
            self.parse_payload::<SurfaceAutomationLifecycleCommand>(command_name, payload)?;
        if command.action != expected_action {
            return Err(LocalHostError::BadRequest(format!(
                "action/body mismatch: expected `{expected_action}`"
            )));
        }

        match expected_action {
            "activate" => {
                self.inner
                    .runtime
                    .activate_automation(&command.automation_id)
                    .await?;
            }
            "pause" => {
                self.inner
                    .runtime
                    .pause_automation(&command.automation_id)
                    .await?;
            }
            "archive" => {
                self.inner
                    .runtime
                    .archive_automation(&command.automation_id)
                    .await?;
            }
            _ => {
                return Err(LocalHostError::BadRequest(format!(
                    "unsupported lifecycle action `{expected_action}`"
                )))
            }
        }

        Ok(self
            .inner
            .runtime
            .load_automation_detail(&command.automation_id)
            .await?)
    }

    async fn build_run_detail_response(
        &self,
        report: RunExecutionReport,
    ) -> Result<RunDetailResponse, LocalHostError> {
        let task = self.inner.runtime.fetch_task(&report.run.task_id).await?;
        let knowledge_assets = self
            .inner
            .runtime
            .list_knowledge_assets_by_run(&report.run.id)
            .await?;
        let knowledge_lineage = self
            .inner
            .runtime
            .list_knowledge_lineage_by_run(&report.run.id)
            .await?;

        Ok(RunDetailResponse {
            run: report.run,
            task,
            artifacts: report.artifacts,
            audits: report.audits,
            traces: report.traces,
            approvals: report.approvals,
            inbox_items: report.inbox_items,
            notifications: self.normalize_notifications(report.notifications).await?,
            policy_decisions: report.policy_decisions,
            knowledge_candidates: report.knowledge_candidates,
            knowledge_assets,
            knowledge_lineage,
        })
    }

    async fn build_knowledge_detail_response(
        &self,
        run_id: &str,
    ) -> Result<KnowledgeDetailResponse, LocalHostError> {
        let report = self.inner.runtime.load_run_report(run_id).await?;
        let knowledge_space = self
            .inner
            .runtime
            .fetch_project_knowledge_space(&report.run.workspace_id, &report.run.project_id)
            .await?
            .ok_or_else(|| {
                LocalHostError::NotFound("project knowledge space not found".to_string())
            })?;

        Ok(KnowledgeDetailResponse {
            knowledge_space,
            candidates: self
                .inner
                .runtime
                .list_knowledge_candidates_by_run(run_id)
                .await?,
            assets: self
                .inner
                .runtime
                .list_knowledge_assets_by_run(run_id)
                .await?,
            lineage: self
                .inner
                .runtime
                .list_knowledge_lineage_by_run(run_id)
            .await?,
        })
    }

    async fn build_project_knowledge_index_response(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<ProjectKnowledgeIndexResponse, LocalHostError> {
        Ok(self
            .inner
            .runtime
            .get_project_knowledge_index(workspace_id, project_id)
            .await?)
    }

    async fn build_connection_status(&self) -> Result<HubConnectionStatusResponse, LocalHostError> {
        let servers = self.inner.runtime.list_mcp_servers().await?;
        let healthy_server_count = servers
            .iter()
            .filter(|server| server.health_status == "healthy")
            .count();
        let last_refreshed_at = servers
            .iter()
            .map(|server| server.last_checked_at.as_str())
            .max()
            .map(str::to_owned)
            .unwrap_or_else(current_timestamp);

        Ok(HubConnectionStatusResponse {
            mode: "local".to_string(),
            state: "connected".to_string(),
            auth_state: "authenticated".to_string(),
            active_server_count: servers.len(),
            healthy_server_count,
            servers: servers
                .into_iter()
                .map(|server| HubConnectionServerSummary {
                    id: server.id,
                    capability_id: server.capability_id,
                    namespace: server.namespace,
                    platform: server.platform,
                    trust_level: server.trust_level,
                    health_status: server.health_status,
                    lease_ttl_seconds: server.lease_ttl_seconds,
                    last_checked_at: server.last_checked_at,
                })
                .collect(),
            last_refreshed_at,
        })
    }

    async fn emit_connection_updated(&self) -> Result<(), LocalHostError> {
        let payload = event_json(
            "hub.connection.updated",
            self.next_sequence(),
            json!(self.build_connection_status().await?),
        );
        self.emit_event(payload)
    }

    async fn emit_run_updated(
        &self,
        run: &RunRecord,
        task: &TaskRecord,
    ) -> Result<(), LocalHostError> {
        let payload = event_json(
            "run.updated",
            self.next_sequence(),
            json!(RunSummaryRecord::new(run, task)),
        );
        self.emit_event(payload)
    }

    async fn emit_workspace_updates(&self, workspace_id: &str) -> Result<(), LocalHostError> {
        let inbox_items = self
            .inner
            .runtime
            .list_inbox_items_by_workspace(workspace_id)
            .await?;
        self.emit_event(event_json(
            "inbox.updated",
            self.next_sequence(),
            json!(inbox_items),
        ))?;

        let notifications = self
            .normalize_notifications(
                self.inner
                    .runtime
                    .list_notifications_by_workspace(workspace_id)
                    .await?,
            )
            .await?;
        self.emit_event(event_json(
            "notification.updated",
            self.next_sequence(),
            json!(notifications),
        ))?;

        Ok(())
    }

    fn emit_event(&self, payload: Value) -> Result<(), LocalHostError> {
        self.inner
            .emitter
            .emit(local_hub_transport_contract().event_channel.as_str(), &payload)
    }

    fn next_sequence(&self) -> u64 {
        self.inner.sequence.fetch_add(1, Ordering::SeqCst)
    }

    fn parse_payload<T: DeserializeOwned>(
        &self,
        command: &str,
        payload: Value,
    ) -> Result<T, LocalHostError> {
        let normalized_payload = match payload {
            Value::Null => Value::Object(Default::default()),
            value => value,
        };
        serde_json::from_value(normalized_payload).map_err(|source| {
            LocalHostError::InvalidPayload {
                command: command.to_string(),
                source,
            }
        })
    }

    async fn normalize_notifications(
        &self,
        notifications: Vec<NotificationRecord>,
    ) -> Result<Vec<NotificationRecord>, LocalHostError> {
        let mut normalized = Vec::with_capacity(notifications.len());
        for mut notification in notifications {
            if notification.status == "delivered" {
                if let Some(approval) = self
                    .inner
                    .runtime
                    .fetch_approval_request(&notification.approval_request_id)
                    .await?
                {
                    if approval.status == "pending" {
                        notification.status = "pending".to_string();
                    }
                }
            }
            normalized.push(notification);
        }
        Ok(normalized)
    }
}

#[derive(Debug, Deserialize)]
struct TaskIdCommand {
    #[serde(rename = "taskId", alias = "task_id")]
    task_id: String,
}

fn parse_approval_decision(decision: &str) -> Result<ApprovalDecision, LocalHostError> {
    match decision {
        "approve" => Ok(ApprovalDecision::Approve),
        "reject" => Ok(ApprovalDecision::Reject),
        "expire" => Ok(ApprovalDecision::Expire),
        "cancel" => Ok(ApprovalDecision::Cancel),
        other => Err(LocalHostError::BadRequest(format!(
            "unsupported approval decision `{other}`"
        ))),
    }
}

fn event_json(event_type: &str, sequence: u64, payload: Value) -> Value {
    json!({
        "event_type": event_type,
        "sequence": sequence,
        "occurred_at": current_timestamp(),
        "payload": payload
    })
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

struct TauriEventEmitterHandle<R: Runtime> {
    handle: tauri::AppHandle<R>,
}

impl<R: Runtime> TauriEventEmitterHandle<R> {
    fn new(handle: tauri::AppHandle<R>) -> Self {
        Self { handle }
    }
}

impl<R: Runtime> LocalHubEventEmitter for TauriEventEmitterHandle<R> {
    fn emit(&self, channel: &str, payload: &Value) -> Result<(), LocalHostError> {
        self.handle
            .emit(channel, payload.clone())
            .map_err(|error| LocalHostError::TauriEmit {
                channel: channel.to_string(),
                message: error.to_string(),
            })
    }
}

struct DesktopLocalHostState {
    host: DesktopLocalHost,
}

async fn invoke_from_state(
    state: &DesktopLocalHostState,
    command: &str,
    payload: Value,
) -> Result<Value, String> {
    state
        .host
        .invoke_transport_command(command, payload)
        .await
        .map_err(|error| error.to_string())
}

fn require_string(
    preferred: Option<String>,
    fallback: Option<String>,
    field: &str,
) -> Result<String, String> {
    preferred
        .or(fallback)
        .ok_or_else(|| format!("missing required argument `{field}`"))
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_list_projects(
    state: State<'_, DesktopLocalHostState>,
    workspaceId: Option<String>,
    workspace_id: Option<String>,
) -> Result<Value, String> {
    let workspace_id = require_string(workspaceId, workspace_id, "workspaceId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.list_projects.as_str(),
        json!({
            "workspaceId": workspace_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_get_project_context(
    state: State<'_, DesktopLocalHostState>,
    workspaceId: Option<String>,
    workspace_id: Option<String>,
    projectId: Option<String>,
    project_id: Option<String>,
) -> Result<Value, String> {
    let workspace_id = require_string(workspaceId, workspace_id, "workspaceId")?;
    let project_id = require_string(projectId, project_id, "projectId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .get_project_context
            .as_str(),
        json!({
            "workspaceId": workspace_id,
            "projectId": project_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_get_project_knowledge(
    state: State<'_, DesktopLocalHostState>,
    workspaceId: Option<String>,
    workspace_id: Option<String>,
    projectId: Option<String>,
    project_id: Option<String>,
) -> Result<Value, String> {
    let workspace_id = require_string(workspaceId, workspace_id, "workspaceId")?;
    let project_id = require_string(projectId, project_id, "projectId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .get_project_knowledge
            .as_str(),
        json!({
            "workspaceId": workspace_id,
            "projectId": project_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_list_automations(
    state: State<'_, DesktopLocalHostState>,
    workspaceId: Option<String>,
    workspace_id: Option<String>,
    projectId: Option<String>,
    project_id: Option<String>,
) -> Result<Value, String> {
    let workspace_id = require_string(workspaceId, workspace_id, "workspaceId")?;
    let project_id = require_string(projectId, project_id, "projectId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .list_automations
            .as_str(),
        json!({
            "workspaceId": workspace_id,
            "projectId": project_id,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_create_automation(
    state: State<'_, DesktopLocalHostState>,
    workspace_id: String,
    project_id: String,
    title: String,
    instruction: String,
    action: Value,
    capability_id: String,
    estimated_cost: i64,
    trigger: Value,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .create_automation
            .as_str(),
        json!({
            "workspace_id": workspace_id,
            "project_id": project_id,
            "title": title,
            "instruction": instruction,
            "action": action,
            "capability_id": capability_id,
            "estimated_cost": estimated_cost,
            "trigger": trigger,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_get_automation_detail(
    state: State<'_, DesktopLocalHostState>,
    automationId: Option<String>,
    automation_id: Option<String>,
) -> Result<Value, String> {
    let automation_id = require_string(automationId, automation_id, "automationId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .get_automation_detail
            .as_str(),
        json!({
            "automationId": automation_id,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_activate_automation(
    state: State<'_, DesktopLocalHostState>,
    automation_id: String,
    action: String,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .activate_automation
            .as_str(),
        json!({
            "automation_id": automation_id,
            "action": action,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_pause_automation(
    state: State<'_, DesktopLocalHostState>,
    automation_id: String,
    action: String,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .pause_automation
            .as_str(),
        json!({
            "automation_id": automation_id,
            "action": action,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_archive_automation(
    state: State<'_, DesktopLocalHostState>,
    automation_id: String,
    action: String,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .archive_automation
            .as_str(),
        json!({
            "automation_id": automation_id,
            "action": action,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_manual_dispatch(
    state: State<'_, DesktopLocalHostState>,
    trigger_id: String,
    dedupe_key: String,
    payload: Value,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.manual_dispatch.as_str(),
        json!({
            "trigger_id": trigger_id,
            "dedupe_key": dedupe_key,
            "payload": payload,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_retry_trigger_delivery(
    state: State<'_, DesktopLocalHostState>,
    delivery_id: String,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .retry_trigger_delivery
            .as_str(),
        json!({
            "delivery_id": delivery_id,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_create_task(
    state: State<'_, DesktopLocalHostState>,
    workspace_id: String,
    project_id: String,
    title: String,
    instruction: String,
    action: Value,
    capability_id: String,
    estimated_cost: i64,
    idempotency_key: String,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.create_task.as_str(),
        json!({
            "workspace_id": workspace_id,
            "project_id": project_id,
            "title": title,
            "instruction": instruction,
            "action": action,
            "capability_id": capability_id,
            "estimated_cost": estimated_cost,
            "idempotency_key": idempotency_key,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_start_task(
    state: State<'_, DesktopLocalHostState>,
    taskId: Option<String>,
    task_id: Option<String>,
) -> Result<Value, String> {
    let task_id = require_string(taskId, task_id, "taskId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.start_task.as_str(),
        json!({
            "taskId": task_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_list_runs(
    state: State<'_, DesktopLocalHostState>,
    workspaceId: Option<String>,
    workspace_id: Option<String>,
    projectId: Option<String>,
    project_id: Option<String>,
) -> Result<Value, String> {
    let workspace_id = require_string(workspaceId, workspace_id, "workspaceId")?;
    let project_id = require_string(projectId, project_id, "projectId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.list_runs.as_str(),
        json!({
            "workspaceId": workspace_id,
            "projectId": project_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_get_run_detail(
    state: State<'_, DesktopLocalHostState>,
    runId: Option<String>,
    run_id: Option<String>,
) -> Result<Value, String> {
    let run_id = require_string(runId, run_id, "runId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.get_run_detail.as_str(),
        json!({
            "runId": run_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_retry_run(
    state: State<'_, DesktopLocalHostState>,
    runId: Option<String>,
    run_id: Option<String>,
) -> Result<Value, String> {
    let run_id = require_string(runId, run_id, "runId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.retry_run.as_str(),
        json!({
            "run_id": run_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_terminate_run(
    state: State<'_, DesktopLocalHostState>,
    runId: Option<String>,
    run_id: Option<String>,
    reason: String,
) -> Result<Value, String> {
    let run_id = require_string(runId, run_id, "runId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.terminate_run.as_str(),
        json!({
            "run_id": run_id,
            "reason": reason,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_get_approval_request(
    state: State<'_, DesktopLocalHostState>,
    approvalId: Option<String>,
    approval_id: Option<String>,
) -> Result<Value, String> {
    let approval_id = require_string(approvalId, approval_id, "approvalId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .get_approval_request
            .as_str(),
        json!({
            "approvalId": approval_id,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_resolve_approval(
    state: State<'_, DesktopLocalHostState>,
    approval_id: String,
    decision: String,
    actor_ref: String,
    note: String,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .resolve_approval
            .as_str(),
        json!({
            "approval_id": approval_id,
            "decision": decision,
            "actor_ref": actor_ref,
            "note": note,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_list_inbox_items(
    state: State<'_, DesktopLocalHostState>,
    workspaceId: Option<String>,
    workspace_id: Option<String>,
) -> Result<Value, String> {
    let workspace_id = require_string(workspaceId, workspace_id, "workspaceId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .list_inbox_items
            .as_str(),
        json!({
            "workspaceId": workspace_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_list_notifications(
    state: State<'_, DesktopLocalHostState>,
    workspaceId: Option<String>,
    workspace_id: Option<String>,
) -> Result<Value, String> {
    let workspace_id = require_string(workspaceId, workspace_id, "workspaceId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .list_notifications
            .as_str(),
        json!({
            "workspaceId": workspace_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_list_artifacts(
    state: State<'_, DesktopLocalHostState>,
    runId: Option<String>,
    run_id: Option<String>,
) -> Result<Value, String> {
    let run_id = require_string(runId, run_id, "runId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract().commands.list_artifacts.as_str(),
        json!({
            "runId": run_id,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_get_knowledge_detail(
    state: State<'_, DesktopLocalHostState>,
    runId: Option<String>,
    run_id: Option<String>,
) -> Result<Value, String> {
    let run_id = require_string(runId, run_id, "runId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .get_knowledge_detail
            .as_str(),
        json!({
            "runId": run_id,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_request_knowledge_promotion(
    state: State<'_, DesktopLocalHostState>,
    candidate_id: String,
    actor_ref: String,
    note: String,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .request_knowledge_promotion
            .as_str(),
        json!({
            "candidate_id": candidate_id,
            "actor_ref": actor_ref,
            "note": note,
        }),
    )
    .await
}

#[tauri::command]
async fn hub_promote_knowledge(
    state: State<'_, DesktopLocalHostState>,
    candidate_id: String,
    actor_ref: String,
    note: String,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .promote_knowledge
            .as_str(),
        json!({
            "candidate_id": candidate_id,
            "actor_ref": actor_ref,
            "note": note,
        }),
    )
    .await
}

#[allow(non_snake_case)]
#[tauri::command]
async fn hub_list_capability_visibility(
    state: State<'_, DesktopLocalHostState>,
    workspaceId: Option<String>,
    workspace_id: Option<String>,
    projectId: Option<String>,
    project_id: Option<String>,
    estimatedCost: Option<i64>,
    estimated_cost: Option<i64>,
) -> Result<Value, String> {
    let workspace_id = require_string(workspaceId, workspace_id, "workspaceId")?;
    let project_id = require_string(projectId, project_id, "projectId")?;
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .list_capability_visibility
            .as_str(),
        json!({
            "workspaceId": workspace_id,
            "projectId": project_id,
            "estimatedCost": estimatedCost.or(estimated_cost),
        }),
    )
    .await
}

#[tauri::command]
async fn hub_get_connection_status(
    state: State<'_, DesktopLocalHostState>,
) -> Result<Value, String> {
    invoke_from_state(
        &state,
        local_hub_transport_contract()
            .commands
            .get_connection_status
            .as_str(),
        json!({}),
    )
    .await
}

async fn cron_ticker_loop(host: DesktopLocalHost) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Err(error) = host.tick_due_triggers(&current_timestamp()).await {
            eprintln!("desktop local cron tick failed: {error}");
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let mut db_path = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            db_path.push("desktop-local-host.sqlite");

            let emitter = Arc::new(TauriEventEmitterHandle::new(app.handle().clone()));
            let host = tauri::async_runtime::block_on(DesktopLocalHost::open(
                LocalHostConfig::new(db_path),
                emitter,
            ))
            .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;

            let ticker_host = host.clone();
            tauri::async_runtime::spawn(async move {
                cron_ticker_loop(ticker_host).await;
            });

            app.manage(DesktopLocalHostState { host });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hub_list_projects,
            hub_get_project_context,
            hub_get_project_knowledge,
            hub_list_automations,
            hub_create_automation,
            hub_get_automation_detail,
            hub_activate_automation,
            hub_pause_automation,
            hub_archive_automation,
            hub_manual_dispatch,
            hub_retry_trigger_delivery,
            hub_create_task,
            hub_start_task,
            hub_list_runs,
            hub_get_run_detail,
            hub_retry_run,
            hub_terminate_run,
            hub_get_approval_request,
            hub_resolve_approval,
            hub_list_inbox_items,
            hub_list_notifications,
            hub_list_artifacts,
            hub_get_knowledge_detail,
            hub_request_knowledge_promotion,
            hub_promote_knowledge,
            hub_list_capability_visibility,
            hub_get_connection_status
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Octopus desktop local host");
}
