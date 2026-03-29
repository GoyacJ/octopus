export type RunStatus =
  | "created"
  | "running"
  | "waiting_approval"
  | "blocked"
  | "completed"
  | "failed"
  | "cancelled"
  | "terminated"
  | "resuming";

export type ApprovalRequestStatus =
  | "pending"
  | "approved"
  | "rejected"
  | "expired"
  | "cancelled";

export type InboxItemStatus = "open" | "in_progress" | "resolved" | "dismissed";
export type NotificationStatus =
  | "pending"
  | "delivered"
  | "read"
  | "failed"
  | "dismissed";
export type KnowledgeCandidateStatus =
  | "candidate"
  | "verified_shared"
  | "promoted_org"
  | "revoked_or_tombstoned";
export type KnowledgeAssetStatus = "verified_shared" | "deprecated";
export type HubAuthState = "authenticated" | "auth_required" | "token_expired";
export type AutomationStatus = "active" | "paused" | "archived";

export interface Workspace {
  id: string;
  slug: string;
  display_name: string;
  created_at: string;
  updated_at: string;
}

export interface Project {
  id: string;
  workspace_id: string;
  slug: string;
  display_name: string;
  created_at: string;
  updated_at: string;
}

export interface ProjectContext {
  workspace: Workspace;
  project: Project;
}

export interface KnowledgeSpace {
  id: string;
  workspace_id: string;
  project_id: string;
  owner_ref: string;
  display_name: string;
  created_at: string;
  updated_at: string;
}

export interface EmitTextAction {
  kind: "emit_text";
  content: string;
}

export interface ConnectorCallAction {
  kind: "connector_call";
  tool_name: string;
  arguments: Record<string, unknown>;
}

export interface FailOnceThenEmitTextAction {
  kind: "fail_once_then_emit_text";
  failure_message: string;
  content: string;
}

export interface AlwaysFailAction {
  kind: "always_fail";
  message: string;
}

export type TaskAction =
  | EmitTextAction
  | ConnectorCallAction
  | FailOnceThenEmitTextAction
  | AlwaysFailAction;

export interface ManualEventCreateTriggerInput {
  trigger_type: "manual_event";
  config: Record<string, never>;
}

export interface CronCreateTriggerInput {
  trigger_type: "cron";
  config: {
    schedule: string;
    timezone: string;
    next_fire_at: string;
  };
}

export interface WebhookCreateTriggerInput {
  trigger_type: "webhook";
  config: {
    ingress_mode: string;
    secret_header_name: string;
    secret_hint?: string | null;
    secret_plaintext?: string | null;
  };
}

export interface McpEventCreateTriggerInput {
  trigger_type: "mcp_event";
  config: {
    server_id: string;
    event_name: string | null;
    event_pattern: string | null;
  };
}

export type CreateTriggerInput =
  | ManualEventCreateTriggerInput
  | CronCreateTriggerInput
  | WebhookCreateTriggerInput
  | McpEventCreateTriggerInput;

export interface CreateAutomationCommand {
  workspace_id: string;
  project_id: string;
  title: string;
  instruction: string;
  action: TaskAction;
  capability_id: string;
  estimated_cost: number;
  trigger: CreateTriggerInput;
}

export interface Automation {
  id: string;
  workspace_id: string;
  project_id: string;
  trigger_id: string;
  status: AutomationStatus;
  title: string;
  instruction: string;
  action: TaskAction;
  capability_id: string;
  estimated_cost: number;
  created_at: string;
  updated_at: string;
}

export interface ManualEventTrigger {
  id: string;
  automation_id: string;
  trigger_type: "manual_event";
  config: Record<string, never>;
  created_at: string;
  updated_at: string;
}

export interface CronTrigger {
  id: string;
  automation_id: string;
  trigger_type: "cron";
  config: {
    schedule: string;
    timezone: string;
    next_fire_at: string;
  };
  created_at: string;
  updated_at: string;
}

export interface WebhookTrigger {
  id: string;
  automation_id: string;
  trigger_type: "webhook";
  config: {
    ingress_mode: string;
    secret_header_name: string;
    secret_hint: string | null;
    secret_present: boolean;
  };
  created_at: string;
  updated_at: string;
}

export interface McpEventTrigger {
  id: string;
  automation_id: string;
  trigger_type: "mcp_event";
  config: {
    server_id: string;
    event_name: string | null;
    event_pattern: string | null;
  };
  created_at: string;
  updated_at: string;
}

export type Trigger =
  | ManualEventTrigger
  | CronTrigger
  | WebhookTrigger
  | McpEventTrigger;

export interface CreateAutomationResponse {
  automation: Automation;
  trigger: Trigger;
  webhook_secret: string | null;
}

export interface TriggerDelivery {
  id: string;
  trigger_id: string;
  run_id: string | null;
  status:
    | "pending"
    | "delivering"
    | "retry_scheduled"
    | "succeeded"
    | "failed";
  dedupe_key: string;
  payload: unknown;
  attempt_count: number;
  last_error: string | null;
  created_at: string;
  updated_at: string;
}

export interface AutomationSummary {
  automation: Automation;
  trigger: Trigger;
  recent_deliveries: TriggerDelivery[];
  last_run_summary: RunSummary | null;
}

export type AutomationDetail = AutomationSummary;

export interface AutomationLifecycleCommand {
  automation_id: string;
  action: "activate" | "pause" | "archive";
}

export interface ManualDispatchCommand {
  trigger_id: string;
  dedupe_key: string;
  payload: unknown;
}

export interface TriggerDeliveryRetryCommand {
  delivery_id: string;
}

export interface RunRetryCommand {
  run_id: string;
}

export interface RunTerminateCommand {
  run_id: string;
  reason: string;
}

export interface TaskCreateCommand {
  workspace_id: string;
  project_id: string;
  title: string;
  instruction: string;
  action: TaskAction;
  capability_id: string;
  estimated_cost: number;
  idempotency_key: string;
}

export interface Task {
  id: string;
  workspace_id: string;
  project_id: string;
  source_kind: "manual" | "automation";
  automation_id: string | null;
  title: string;
  instruction: string;
  action: TaskAction;
  capability_id: string;
  estimated_cost: number;
  idempotency_key: string;
  created_at: string;
  updated_at: string;
}

export interface Run {
  id: string;
  task_id: string;
  workspace_id: string;
  project_id: string;
  automation_id: string | null;
  trigger_delivery_id: string | null;
  run_type: "task" | "automation";
  status: RunStatus;
  approval_request_id: string | null;
  idempotency_key: string;
  attempt_count: number;
  max_attempts: number;
  checkpoint_seq: number;
  resume_token: string | null;
  last_error: string | null;
  created_at: string;
  updated_at: string;
  started_at: string | null;
  completed_at: string | null;
  terminated_at: string | null;
}

export interface RunSummary {
  id: string;
  task_id: string;
  workspace_id: string;
  project_id: string;
  title: string;
  run_type: "task" | "automation";
  status: RunStatus;
  approval_request_id: string | null;
  attempt_count: number;
  max_attempts: number;
  last_error: string | null;
  created_at: string;
  updated_at: string;
  started_at: string | null;
  completed_at: string | null;
  terminated_at: string | null;
}

export interface CapabilityDescriptor {
  id: string;
  slug: string;
  kind: string;
  source: string;
  platform: string;
  risk_level: string;
  requires_approval: boolean;
  input_schema_uri: string | null;
  output_schema_uri: string | null;
  fallback_capability_id: string | null;
  trust_level: "trusted" | "external_untrusted";
  created_at: string;
  updated_at: string;
}

export interface ApprovalRequest {
  id: string;
  workspace_id: string;
  project_id: string;
  run_id: string;
  task_id: string;
  approval_type: "execution" | "knowledge_promotion";
  target_ref: string;
  status: ApprovalRequestStatus;
  reason: string;
  dedupe_key: string;
  decided_by: string | null;
  decision_note: string | null;
  decided_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ApprovalResolveCommand {
  approval_id: string;
  decision: "approve" | "reject" | "expire" | "cancel";
  actor_ref: string;
  note: string;
}

export type CapabilityExecutionState =
  | "executable"
  | "approval_required"
  | "denied";

export type CapabilityResolutionReasonCode =
  | "capability_not_registered"
  | "capability_not_bound"
  | "capability_not_granted"
  | "budget_policy_missing"
  | "budget_hard_limit_exceeded"
  | "risk_level_high"
  | "budget_soft_limit_exceeded"
  | "within_budget";

export interface CapabilityResolution {
  descriptor: CapabilityDescriptor;
  scope_ref: string;
  execution_state: CapabilityExecutionState;
  reason_code: CapabilityResolutionReasonCode;
  explanation: string;
}

export type CapabilityVisibility = CapabilityResolution;

export interface Artifact {
  id: string;
  workspace_id: string;
  project_id: string;
  run_id: string;
  task_id: string;
  artifact_type: "execution_output";
  content: string;
  provenance_source: "builtin" | "mcp_connector";
  source_descriptor_id: string;
  source_invocation_id: string | null;
  trust_level: "trusted" | "external_untrusted";
  knowledge_gate_status: "eligible" | "blocked_low_trust";
  created_at: string;
  updated_at: string;
}

export interface ArtifactSummary {
  id: string;
  run_id: string;
  task_id: string;
  artifact_type: "execution_output";
  provenance_source: "builtin" | "mcp_connector";
  trust_level: "trusted" | "external_untrusted";
  knowledge_gate_status: "eligible" | "blocked_low_trust";
  created_at: string;
}

export interface AuditRecord {
  id: string;
  workspace_id: string;
  project_id: string;
  run_id: string;
  task_id: string;
  event_type: string;
  message: string;
  created_at: string;
}

export interface TraceRecord {
  id: string;
  workspace_id: string;
  project_id: string;
  run_id: string;
  task_id: string;
  stage: string;
  attempt: number;
  message: string;
  created_at: string;
}

export interface InboxItem {
  id: string;
  workspace_id: string;
  project_id: string;
  run_id: string;
  approval_request_id: string;
  item_type: "approval_request";
  target_ref: string;
  status: InboxItemStatus;
  dedupe_key: string;
  title: string;
  message: string;
  created_at: string;
  updated_at: string;
  resolved_at: string | null;
}

export interface Notification {
  id: string;
  workspace_id: string;
  project_id: string;
  run_id: string;
  approval_request_id: string;
  target_ref: string;
  status: NotificationStatus;
  dedupe_key: string;
  title: string;
  message: string;
  created_at: string;
  updated_at: string;
}

export interface PolicyDecisionLog {
  id: string;
  workspace_id: string;
  project_id: string;
  run_id: string;
  task_id: string;
  capability_id: string;
  decision: "allow" | "require_approval" | "deny";
  reason: string;
  estimated_cost: number;
  approval_request_id: string | null;
  created_at: string;
}

export interface KnowledgeCandidate {
  id: string;
  knowledge_space_id: string;
  source_run_id: string;
  source_task_id: string;
  source_artifact_id: string;
  capability_id: string;
  status: KnowledgeCandidateStatus;
  content: string;
  provenance_source: "builtin" | "mcp_connector";
  source_trust_level: "trusted" | "external_untrusted";
  dedupe_key: string;
  created_at: string;
  updated_at: string;
}

export interface KnowledgeAsset {
  id: string;
  knowledge_space_id: string;
  source_candidate_id: string;
  capability_id: string;
  status: KnowledgeAssetStatus;
  content: string;
  trust_level: "verified" | "unverified";
  created_at: string;
  updated_at: string;
}

export interface KnowledgeLineageRecord {
  id: string;
  workspace_id: string;
  project_id: string;
  run_id: string;
  task_id: string;
  source_ref: string;
  target_ref: string;
  relation_type: "derived_from" | "promoted_from" | "recalled_by";
  created_at: string;
}

export interface CandidateKnowledgeSummary {
  kind: "candidate";
  id: string;
  knowledge_space_id: string;
  capability_id: string;
  status: KnowledgeCandidateStatus;
  source_run_id: string;
  source_artifact_id: string;
  source_candidate_id: null;
  provenance_source: "builtin" | "mcp_connector";
  trust_level: "trusted" | "external_untrusted";
  created_at: string;
}

export interface AssetKnowledgeSummary {
  kind: "asset";
  id: string;
  knowledge_space_id: string;
  capability_id: string;
  status: KnowledgeAssetStatus;
  source_run_id: null;
  source_artifact_id: null;
  source_candidate_id: string;
  provenance_source: null;
  trust_level: "verified";
  created_at: string;
}

export type KnowledgeSummary = CandidateKnowledgeSummary | AssetKnowledgeSummary;

export interface KnowledgeDetail {
  knowledge_space: KnowledgeSpace;
  candidates: KnowledgeCandidate[];
  assets: KnowledgeAsset[];
  lineage: KnowledgeLineageRecord[];
}

export interface ProjectKnowledgeIndex {
  knowledge_space: KnowledgeSpace;
  entries: KnowledgeSummary[];
}

export interface KnowledgePromoteCommand {
  candidate_id: string;
  actor_ref: string;
  note: string;
}

export interface RequestKnowledgePromotionCommand {
  candidate_id: string;
  actor_ref: string;
  note: string;
}

export interface HubConnectionServerSummary {
  id: string;
  capability_id: string;
  namespace: string;
  platform: string;
  trust_level: "trusted" | "external_untrusted";
  health_status: "healthy" | "degraded" | "unreachable";
  lease_ttl_seconds: number;
  last_checked_at: string;
}

export interface HubConnectionStatus {
  mode: "local" | "remote";
  state: "connected" | "degraded" | "disconnected";
  auth_state: HubAuthState;
  active_server_count: number;
  healthy_server_count: number;
  servers: HubConnectionServerSummary[];
  last_refreshed_at: string;
}

export interface HubLoginCommand {
  workspace_id: string;
  email: string;
  password: string;
}

export interface HubSession {
  session_id: string;
  user_id: string;
  email: string;
  workspace_id: string;
  actor_ref: string;
  issued_at: string;
  expires_at: string;
}

export interface HubLoginResponse {
  access_token: string;
  session: HubSession;
}

export interface HubAuthError {
  error: string;
  error_code: "auth_required" | "token_expired" | "workspace_forbidden";
  auth_state: HubAuthState;
}

export interface LocalHubTransportCommands {
  list_projects: string;
  get_project_context: string;
  get_project_knowledge: string;
  list_automations: string;
  create_automation: string;
  get_automation_detail: string;
  activate_automation: string;
  pause_automation: string;
  archive_automation: string;
  manual_dispatch: string;
  retry_trigger_delivery: string;
  create_task: string;
  start_task: string;
  list_runs: string;
  get_run_detail: string;
  retry_run: string;
  terminate_run: string;
  get_approval_request: string;
  resolve_approval: string;
  list_inbox_items: string;
  list_notifications: string;
  list_artifacts: string;
  get_knowledge_detail: string;
  request_knowledge_promotion: string;
  promote_knowledge: string;
  list_capability_visibility: string;
  get_connection_status: string;
}

export interface LocalHubTransportContract {
  event_channel: string;
  commands: LocalHubTransportCommands;
}

export interface RunUpdatedEvent {
  event_type: "run.updated";
  sequence: number;
  occurred_at: string;
  payload: RunSummary;
}

export interface InboxUpdatedEvent {
  event_type: "inbox.updated";
  sequence: number;
  occurred_at: string;
  payload: InboxItem[];
}

export interface NotificationUpdatedEvent {
  event_type: "notification.updated";
  sequence: number;
  occurred_at: string;
  payload: Notification[];
}

export interface HubConnectionUpdatedEvent {
  event_type: "hub.connection.updated";
  sequence: number;
  occurred_at: string;
  payload: HubConnectionStatus;
}

export type HubEvent =
  | RunUpdatedEvent
  | InboxUpdatedEvent
  | NotificationUpdatedEvent
  | HubConnectionUpdatedEvent;

export interface RunDetail {
  run: Run;
  task: Task;
  artifacts: Artifact[];
  audits: AuditRecord[];
  traces: TraceRecord[];
  approvals: ApprovalRequest[];
  inbox_items: InboxItem[];
  notifications: Notification[];
  policy_decisions: PolicyDecisionLog[];
  knowledge_candidates: KnowledgeCandidate[];
  knowledge_assets: KnowledgeAsset[];
  knowledge_lineage: KnowledgeLineageRecord[];
}
