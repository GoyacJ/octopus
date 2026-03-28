import {
  LOCAL_HUB_TRANSPORT,
  parseApprovalRequest,
  parseApprovalResolveCommand,
  parseAutomationDetail,
  parseAutomationLifecycleCommand,
  parseAutomationSummaries,
  parseArtifacts,
  parseCapabilityResolutions,
  parseCreateAutomationCommand,
  parseCreateAutomationResponse,
  parseHubAuthError,
  parseHubConnectionStatus,
  parseHubEvent,
  parseInboxItems,
  parseKnowledgeDetail,
  parseHubLoginCommand,
  parseHubLoginResponse,
  parseHubSession,
  parseKnowledgePromoteCommand,
  parseRequestKnowledgePromotionCommand,
  parseManualDispatchCommand,
  parseNotifications,
  parseProjectContext,
  parseRunDetail,
  parseRunSummaries,
  parseTask,
  parseTaskCreateCommand,
  parseTriggerDeliveryRetryCommand,
  type ApprovalResolveCommand,
  type ApprovalRequest,
  type AutomationDetail,
  type AutomationSummary,
  type Artifact,
  type CapabilityResolution,
  type CreateAutomationCommand,
  type CreateAutomationResponse,
  type HubAuthError,
  type HubConnectionStatus,
  type HubEvent,
  type HubLoginCommand,
  type HubLoginResponse,
  type HubSession,
  type InboxItem,
  type KnowledgeDetail,
  type KnowledgePromoteCommand,
  type RequestKnowledgePromotionCommand,
  type LocalHubTransportContract,
  type ManualDispatchCommand,
  type Notification,
  type ProjectContext,
  type RunDetail,
  type RunSummary,
  type Task,
  type TaskCreateCommand,
  type TriggerDeliveryRetryCommand
} from "@octopus/schema-ts";

function normalizeLocalCommandName(command: string): string {
  return command;
}

export const HUB_EVENT_CHANNEL = LOCAL_HUB_TRANSPORT.event_channel;

export const LOCAL_HUB_COMMANDS = {
  getProjectContext: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_project_context
  ),
  listAutomations: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_automations
  ),
  createAutomation: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.create_automation
  ),
  getAutomationDetail: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_automation_detail
  ),
  activateAutomation: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.activate_automation
  ),
  pauseAutomation: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.pause_automation
  ),
  archiveAutomation: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.archive_automation
  ),
  manualDispatch: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.manual_dispatch
  ),
  retryTriggerDelivery: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.retry_trigger_delivery
  ),
  createTask: normalizeLocalCommandName(LOCAL_HUB_TRANSPORT.commands.create_task),
  startTask: normalizeLocalCommandName(LOCAL_HUB_TRANSPORT.commands.start_task),
  listRuns: normalizeLocalCommandName(LOCAL_HUB_TRANSPORT.commands.list_runs),
  getRunDetail: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_run_detail
  ),
  getApprovalRequest: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_approval_request
  ),
  resolveApproval: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.resolve_approval
  ),
  listInboxItems: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_inbox_items
  ),
  listNotifications: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_notifications
  ),
  listArtifacts: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_artifacts
  ),
  getKnowledgeDetail: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_knowledge_detail
  ),
  requestKnowledgePromotion: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.request_knowledge_promotion
  ),
  promoteKnowledge: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.promote_knowledge
  ),
  listCapabilityVisibility: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_capability_visibility
  ),
  getConnectionStatus: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_connection_status
  )
} as const;

export type Unsubscribe = () => void | Promise<void>;

export interface HubClient {
  getProjectContext(workspaceId: string, projectId: string): Promise<ProjectContext>;
  listAutomations(
    workspaceId: string,
    projectId: string
  ): Promise<AutomationSummary[]>;
  createAutomation(
    command: CreateAutomationCommand
  ): Promise<CreateAutomationResponse>;
  getAutomationDetail(automationId: string): Promise<AutomationDetail>;
  activateAutomation(automationId: string): Promise<AutomationDetail>;
  pauseAutomation(automationId: string): Promise<AutomationDetail>;
  archiveAutomation(automationId: string): Promise<AutomationDetail>;
  manualDispatch(command: ManualDispatchCommand): Promise<AutomationDetail>;
  retryTriggerDelivery(
    command: TriggerDeliveryRetryCommand
  ): Promise<AutomationDetail>;
  createTask(command: TaskCreateCommand): Promise<Task>;
  startTask(taskId: string): Promise<RunDetail>;
  listRuns(workspaceId: string, projectId: string): Promise<RunSummary[]>;
  getRunDetail(runId: string): Promise<RunDetail>;
  getApprovalRequest(approvalId: string): Promise<ApprovalRequest>;
  resolveApproval(command: ApprovalResolveCommand): Promise<RunDetail>;
  listInboxItems(workspaceId: string): Promise<InboxItem[]>;
  listNotifications(workspaceId: string): Promise<Notification[]>;
  listArtifacts(runId: string): Promise<Artifact[]>;
  getKnowledgeDetail(runId: string): Promise<KnowledgeDetail>;
  requestKnowledgePromotion(
    command: RequestKnowledgePromotionCommand
  ): Promise<ApprovalRequest>;
  promoteKnowledge(command: KnowledgePromoteCommand): Promise<KnowledgeDetail>;
  listCapabilityResolutions(
    workspaceId: string,
    projectId: string,
    estimatedCost: number
  ): Promise<CapabilityResolution[]>;
  listCapabilityVisibility(
    workspaceId: string,
    projectId: string,
    estimatedCost?: number
  ): Promise<CapabilityResolution[]>;
  getHubConnectionStatus(): Promise<HubConnectionStatus>;
  subscribe(
    listener: (event: HubEvent) => void,
    onError?: (error: unknown) => void
  ): Promise<Unsubscribe>;
}

export interface LocalHubTransport {
  invoke(command: string, payload?: unknown): Promise<unknown>;
  listen(
    channel: string,
    handler: (payload: unknown) => void
  ): Unsubscribe | Promise<Unsubscribe>;
}

export interface EventSourceMessage {
  data: string;
}

export interface EventSourceLike {
  close(): void;
  onmessage: ((event: EventSourceMessage) => void) | null;
  onerror: ((error: unknown) => void) | null;
}

export interface RemoteHubClientOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
  createEventSource?: (url: string) => EventSourceLike;
  getAccessToken?: () => string | null | undefined | Promise<string | null | undefined>;
}

export class HubClientTransportError extends Error {
  readonly status: number | null;
  readonly details: unknown;

  constructor(message: string, status: number | null = null, details: unknown = null) {
    super(message);
    this.name = "HubClientTransportError";
    this.status = status;
    this.details = details;
  }
}

export class HubClientAuthError extends HubClientTransportError {
  readonly kind: HubAuthError["error_code"];
  readonly authState: HubAuthError["auth_state"];

  constructor(status: number, details: HubAuthError) {
    super(details.error, status, details);
    this.name = "HubClientAuthError";
    this.kind = details.error_code;
    this.authState = details.auth_state;
  }
}

function normalizeBaseUrl(baseUrl: string): string {
  return baseUrl.endsWith("/") ? baseUrl.slice(0, -1) : baseUrl;
}

function toError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

async function maybeReadJson(response: Response): Promise<unknown> {
  const contentType = response.headers.get("content-type") ?? "";
  if (contentType.includes("application/json")) {
    return response.json();
  }

  const text = await response.text();
  return text.length > 0 ? text : null;
}

async function readRemoteJson(
  fetchImpl: typeof globalThis.fetch,
  url: string,
  init?: RequestInit,
  getAccessToken?: RemoteHubClientOptions["getAccessToken"]
): Promise<unknown> {
  const requestInit = await withAuthorization(init, getAccessToken);
  const response = await fetchImpl(url, requestInit);
  const body = await maybeReadJson(response);

  if (!response.ok) {
    const authError = normalizeAuthError(response.status, body);
    if (authError) {
      throw new HubClientAuthError(response.status, authError);
    }

    throw new HubClientTransportError(
      `Hub request failed: ${response.status} ${response.statusText}`,
      response.status,
      body
    );
  }

  return body;
}

function encodePathSegment(value: string): string {
  return encodeURIComponent(value);
}

function remotePath(baseUrl: string, path: string): string {
  return `${normalizeBaseUrl(baseUrl)}${path}`;
}

async function withAuthorization(
  init: RequestInit | undefined,
  getAccessToken: RemoteHubClientOptions["getAccessToken"]
): Promise<RequestInit | undefined> {
  const accessToken = await getAccessToken?.();
  if (!accessToken) {
    return init;
  }

  const headers = new Headers(init?.headers);
  headers.set("authorization", `Bearer ${accessToken}`);

  return {
    ...init,
    headers
  };
}

function normalizeAuthError(status: number, body: unknown): HubAuthError | null {
  if (status !== 401 && status !== 403) {
    return null;
  }

  try {
    return parseHubAuthError(body);
  } catch {
    return null;
  }
}

export function createLocalHubClient(transport: LocalHubTransport): HubClient {
  async function listCapabilityResolutions(
    workspaceId: string,
    projectId: string,
    estimatedCost: number
  ): Promise<CapabilityResolution[]> {
    return parseCapabilityResolutions(
      await transport.invoke(LOCAL_HUB_COMMANDS.listCapabilityVisibility, {
        workspaceId,
        projectId,
        estimatedCost
      })
    );
  }

  return {
    async getProjectContext(workspaceId, projectId) {
      return parseProjectContext(
        await transport.invoke(LOCAL_HUB_COMMANDS.getProjectContext, {
          workspaceId,
          projectId
        })
      );
    },
    async listAutomations(workspaceId, projectId) {
      return parseAutomationSummaries(
        await transport.invoke(LOCAL_HUB_COMMANDS.listAutomations, {
          workspaceId,
          projectId
        })
      );
    },
    async createAutomation(command) {
      return parseCreateAutomationResponse(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.createAutomation,
          parseCreateAutomationCommand(command)
        )
      );
    },
    async getAutomationDetail(automationId) {
      return parseAutomationDetail(
        await transport.invoke(LOCAL_HUB_COMMANDS.getAutomationDetail, {
          automationId
        })
      );
    },
    async activateAutomation(automationId) {
      return parseAutomationDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.activateAutomation,
          parseAutomationLifecycleCommand({
            automation_id: automationId,
            action: "activate"
          })
        )
      );
    },
    async pauseAutomation(automationId) {
      return parseAutomationDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.pauseAutomation,
          parseAutomationLifecycleCommand({
            automation_id: automationId,
            action: "pause"
          })
        )
      );
    },
    async archiveAutomation(automationId) {
      return parseAutomationDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.archiveAutomation,
          parseAutomationLifecycleCommand({
            automation_id: automationId,
            action: "archive"
          })
        )
      );
    },
    async manualDispatch(command) {
      return parseAutomationDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.manualDispatch,
          parseManualDispatchCommand(command)
        )
      );
    },
    async retryTriggerDelivery(command) {
      return parseAutomationDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.retryTriggerDelivery,
          parseTriggerDeliveryRetryCommand(command)
        )
      );
    },
    async createTask(command) {
      return parseTask(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.createTask,
          parseTaskCreateCommand(command)
        )
      );
    },
    async startTask(taskId) {
      return parseRunDetail(
        await transport.invoke(LOCAL_HUB_COMMANDS.startTask, { taskId })
      );
    },
    async listRuns(workspaceId, projectId) {
      return parseRunSummaries(
        await transport.invoke(LOCAL_HUB_COMMANDS.listRuns, {
          workspaceId,
          projectId
        })
      );
    },
    async getRunDetail(runId) {
      return parseRunDetail(
        await transport.invoke(LOCAL_HUB_COMMANDS.getRunDetail, { runId })
      );
    },
    async getApprovalRequest(approvalId) {
      return parseApprovalRequest(
        await transport.invoke(LOCAL_HUB_COMMANDS.getApprovalRequest, {
          approvalId
        })
      );
    },
    async resolveApproval(command) {
      return parseRunDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.resolveApproval,
          parseApprovalResolveCommand(command)
        )
      );
    },
    async listInboxItems(workspaceId) {
      return parseInboxItems(
        await transport.invoke(LOCAL_HUB_COMMANDS.listInboxItems, { workspaceId })
      );
    },
    async listNotifications(workspaceId) {
      return parseNotifications(
        await transport.invoke(LOCAL_HUB_COMMANDS.listNotifications, { workspaceId })
      );
    },
    async listArtifacts(runId) {
      return parseArtifacts(
        await transport.invoke(LOCAL_HUB_COMMANDS.listArtifacts, { runId })
      );
    },
    async getKnowledgeDetail(runId) {
      return parseKnowledgeDetail(
        await transport.invoke(LOCAL_HUB_COMMANDS.getKnowledgeDetail, { runId })
      );
    },
    async requestKnowledgePromotion(command) {
      return parseApprovalRequest(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.requestKnowledgePromotion,
          parseRequestKnowledgePromotionCommand(command)
        )
      );
    },
    async promoteKnowledge(command) {
      return parseKnowledgeDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.promoteKnowledge,
          parseKnowledgePromoteCommand(command)
        )
      );
    },
    async listCapabilityResolutions(workspaceId, projectId, estimatedCost) {
      return listCapabilityResolutions(workspaceId, projectId, estimatedCost);
    },
    async listCapabilityVisibility(workspaceId, projectId, estimatedCost = 1) {
      return listCapabilityResolutions(workspaceId, projectId, estimatedCost);
    },
    async getHubConnectionStatus() {
      return parseHubConnectionStatus(
        await transport.invoke(LOCAL_HUB_COMMANDS.getConnectionStatus)
      );
    },
    async subscribe(listener, onError) {
      const unsubscribe = await transport.listen(HUB_EVENT_CHANNEL, (payload) => {
        try {
          listener(parseHubEvent(payload));
        } catch (error) {
          onError?.(toError(error));
        }
      });

      return async () => {
        await unsubscribe();
      };
    }
  };
}

export function createRemoteHubClient(options: RemoteHubClientOptions): HubClient {
  const fetchImpl = options.fetch ?? globalThis.fetch;
  const createEventSource =
    options.createEventSource ??
    ((url: string) => {
      const EventSourceCtor = globalThis.EventSource;
      if (!EventSourceCtor) {
        throw new Error("EventSource is not available in this environment.");
      }
      return new EventSourceCtor(url) as unknown as EventSourceLike;
    });

  if (!fetchImpl) {
    throw new Error("fetch is not available in this environment.");
  }

  async function listCapabilityResolutions(
    workspaceId: string,
    projectId: string,
    estimatedCost: number
  ): Promise<CapabilityResolution[]> {
    const capabilitiesUrl = new URL(
      remotePath(
        options.baseUrl,
        `/api/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/capabilities`
      )
    );
    capabilitiesUrl.searchParams.set("estimated_cost", String(estimatedCost));

    return parseCapabilityResolutions(
      await readRemoteJson(
        fetchImpl,
        capabilitiesUrl.toString(),
        undefined,
        options.getAccessToken
      )
    );
  }

  return {
    async getProjectContext(workspaceId, projectId) {
      return parseProjectContext(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/context`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async listAutomations(workspaceId, projectId) {
      return parseAutomationSummaries(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/automations`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async createAutomation(command) {
      const parsed = parseCreateAutomationCommand(command);
      return parseCreateAutomationResponse(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(parsed.workspace_id)}/projects/${encodePathSegment(parsed.project_id)}/automations`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          },
          options.getAccessToken
        )
      );
    },
    async listRuns(workspaceId, projectId) {
      return parseRunSummaries(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/runs`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async getAutomationDetail(automationId) {
      return parseAutomationDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/automations/${encodePathSegment(automationId)}`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async activateAutomation(automationId) {
      return parseAutomationDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/automations/${encodePathSegment(automationId)}/activate`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(
              parseAutomationLifecycleCommand({
                automation_id: automationId,
                action: "activate"
              })
            )
          },
          options.getAccessToken
        )
      );
    },
    async pauseAutomation(automationId) {
      return parseAutomationDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/automations/${encodePathSegment(automationId)}/pause`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(
              parseAutomationLifecycleCommand({
                automation_id: automationId,
                action: "pause"
              })
            )
          },
          options.getAccessToken
        )
      );
    },
    async archiveAutomation(automationId) {
      return parseAutomationDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/automations/${encodePathSegment(automationId)}/archive`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(
              parseAutomationLifecycleCommand({
                automation_id: automationId,
                action: "archive"
              })
            )
          },
          options.getAccessToken
        )
      );
    },
    async manualDispatch(command) {
      const parsed = parseManualDispatchCommand(command);
      return parseAutomationDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/triggers/${encodePathSegment(parsed.trigger_id)}/manual-dispatch`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          },
          options.getAccessToken
        )
      );
    },
    async retryTriggerDelivery(command) {
      const parsed = parseTriggerDeliveryRetryCommand(command);
      return parseAutomationDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/trigger-deliveries/${encodePathSegment(parsed.delivery_id)}/retry`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          },
          options.getAccessToken
        )
      );
    },
    async createTask(command) {
      return parseTask(
        await readRemoteJson(fetchImpl, remotePath(options.baseUrl, "/api/tasks"), {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(parseTaskCreateCommand(command))
        }, options.getAccessToken)
      );
    },
    async startTask(taskId) {
      return parseRunDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/tasks/${encodePathSegment(taskId)}/start`
          ),
          { method: "POST" },
          options.getAccessToken
        )
      );
    },
    async getRunDetail(runId) {
      return parseRunDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(options.baseUrl, `/api/runs/${encodePathSegment(runId)}`),
          undefined,
          options.getAccessToken
        )
      );
    },
    async getApprovalRequest(approvalId) {
      return parseApprovalRequest(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/approvals/${encodePathSegment(approvalId)}`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async resolveApproval(command) {
      const parsed = parseApprovalResolveCommand(command);
      return parseRunDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/approvals/${encodePathSegment(parsed.approval_id)}/resolve`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          },
          options.getAccessToken
        )
      );
    },
    async listInboxItems(workspaceId) {
      return parseInboxItems(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/inbox`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async listNotifications(workspaceId) {
      return parseNotifications(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/notifications`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async listArtifacts(runId) {
      return parseArtifacts(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/runs/${encodePathSegment(runId)}/artifacts`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async getKnowledgeDetail(runId) {
      return parseKnowledgeDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/runs/${encodePathSegment(runId)}/knowledge`
          ),
          undefined,
          options.getAccessToken
        )
      );
    },
    async requestKnowledgePromotion(command) {
      const parsed = parseRequestKnowledgePromotionCommand(command);
      return parseApprovalRequest(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/knowledge/candidates/${encodePathSegment(parsed.candidate_id)}/request-promotion`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          },
          options.getAccessToken
        )
      );
    },
    async promoteKnowledge(command) {
      const parsed = parseKnowledgePromoteCommand(command);
      return parseKnowledgeDetail(
        await readRemoteJson(
          fetchImpl,
          remotePath(
            options.baseUrl,
            `/api/knowledge/candidates/${encodePathSegment(parsed.candidate_id)}/promote`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          },
          options.getAccessToken
        )
      );
    },
    async listCapabilityResolutions(workspaceId, projectId, estimatedCost) {
      return listCapabilityResolutions(workspaceId, projectId, estimatedCost);
    },
    async listCapabilityVisibility(workspaceId, projectId, estimatedCost = 1) {
      return listCapabilityResolutions(workspaceId, projectId, estimatedCost);
    },
    async getHubConnectionStatus() {
      return parseHubConnectionStatus(
        await readRemoteJson(
          fetchImpl,
          remotePath(options.baseUrl, "/api/hub/connection"),
          undefined,
          options.getAccessToken
        )
      );
    },
    async subscribe(listener, onError) {
      const eventsUrl = new URL(remotePath(options.baseUrl, "/api/events"));
      const accessToken = await options.getAccessToken?.();
      if (accessToken) {
        eventsUrl.searchParams.set("access_token", accessToken);
      }

      const eventSource = createEventSource(eventsUrl.toString());
      eventSource.onmessage = (event) => {
        try {
          listener(parseHubEvent(JSON.parse(event.data)));
        } catch (error) {
          onError?.(toError(error));
        }
      };
      eventSource.onerror = (error) => {
        onError?.(toError(error));
      };

      return () => {
        eventSource.close();
      };
    }
  };
}
