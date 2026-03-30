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
  parseHubRefreshCommand,
  parseHubRefreshResponse,
  parseHubSession,
  parseModelCatalogItems,
  parseModelProfiles,
  parseModelProviders,
  parseTenantModelPolicy,
  parseKnowledgePromoteCommand,
  parseProjectKnowledgeIndex,
  parseRequestKnowledgePromotionCommand,
  parseManualDispatchCommand,
  parseNotifications,
  parseProjects,
  parseProjectContext,
  parseRunDetail,
  parseRunRetryCommand,
  parseRunSummaries,
  parseRunTerminateCommand,
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
  type HubRefreshResponse,
  type HubSession,
  type InboxItem,
  type KnowledgeDetail,
  type KnowledgePromoteCommand,
  type ProjectKnowledgeIndex,
  type RequestKnowledgePromotionCommand,
  type LocalHubTransportContract,
  type ManualDispatchCommand,
  type ModelCatalogItem,
  type ModelProfile,
  type ModelProvider,
  type Notification,
  type Project,
  type ProjectContext,
  type RunDetail,
  type RunRetryCommand,
  type RunSummary,
  type RunTerminateCommand,
  type TenantModelPolicy,
  type Task,
  type TaskCreateCommand,
  type TriggerDeliveryRetryCommand
} from "@octopus/schema-ts";

function normalizeLocalCommandName(command: string): string {
  return command;
}

export const HUB_EVENT_CHANNEL = LOCAL_HUB_TRANSPORT.event_channel;

export const LOCAL_HUB_COMMANDS = {
  listProjects: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_projects
  ),
  getProjectContext: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_project_context
  ),
  getProjectKnowledge: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_project_knowledge
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
  listModelProviders: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_model_providers
  ),
  listModelCatalogItems: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_model_catalog_items
  ),
  listModelProfiles: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.list_model_profiles
  ),
  getWorkspaceModelPolicy: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.get_workspace_model_policy
  ),
  retryRun: normalizeLocalCommandName(LOCAL_HUB_TRANSPORT.commands.retry_run),
  terminateRun: normalizeLocalCommandName(
    LOCAL_HUB_TRANSPORT.commands.terminate_run
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
  listProjects(workspaceId: string): Promise<Project[]>;
  getProjectContext(workspaceId: string, projectId: string): Promise<ProjectContext>;
  getProjectKnowledge(
    workspaceId: string,
    projectId: string
  ): Promise<ProjectKnowledgeIndex>;
  listModelProviders(workspaceId: string): Promise<ModelProvider[]>;
  listModelCatalogItems(workspaceId: string): Promise<ModelCatalogItem[]>;
  listModelProfiles(workspaceId: string): Promise<ModelProfile[]>;
  getWorkspaceModelPolicy(workspaceId: string): Promise<TenantModelPolicy | null>;
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
  retryRun(command: RunRetryCommand): Promise<RunDetail>;
  terminateRun(command: RunTerminateCommand): Promise<RunDetail>;
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
  getRefreshToken?: () => string | null | undefined | Promise<string | null | undefined>;
  onRefreshTokens?: (response: HubRefreshResponse) => void | Promise<void>;
  clearSessionTokens?: () => void | Promise<void>;
}

export interface RemoteHubAuthClient {
  login(command: HubLoginCommand): Promise<HubLoginResponse>;
  refreshSession(): Promise<HubRefreshResponse>;
  getCurrentSession(): Promise<HubSession>;
  logout(): Promise<void>;
}

export type {
  HubAuthError,
  HubConnectionStatus,
  HubLoginCommand,
  HubLoginResponse,
  HubRefreshResponse,
  HubSession
};

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

function createAuthRequiredError(message = "authentication required"): HubClientAuthError {
  return new HubClientAuthError(401, {
    error: message,
    error_code: "auth_required",
    auth_state: "auth_required"
  });
}

function isTokenExpiredAuthError(error: unknown): error is HubClientAuthError {
  return error instanceof HubClientAuthError && error.kind === "token_expired";
}

function authErrorRequiresReauthentication(
  error: unknown
): error is HubClientAuthError {
  return (
    error instanceof HubClientAuthError &&
    (error.authState === "auth_required" || error.authState === "token_expired")
  );
}

async function performRefreshSession(
  fetchImpl: typeof globalThis.fetch,
  options: RemoteHubClientOptions
): Promise<HubRefreshResponse> {
  const refreshToken = await options.getRefreshToken?.();
  if (!refreshToken) {
    throw createAuthRequiredError();
  }

  return parseHubRefreshResponse(
    await readRemoteJson(fetchImpl, remotePath(options.baseUrl, "/api/auth/refresh"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(
        parseHubRefreshCommand({
          refresh_token: refreshToken
        })
      )
    })
  );
}

function createRefreshCoordinator(
  fetchImpl: typeof globalThis.fetch,
  options: RemoteHubClientOptions
): () => Promise<HubRefreshResponse> {
  let refreshInFlight: Promise<HubRefreshResponse> | null = null;

  return async () => {
    if (refreshInFlight) {
      return refreshInFlight;
    }

    refreshInFlight = (async () => {
      try {
        const response = await performRefreshSession(fetchImpl, options);
        await options.onRefreshTokens?.(response);
        return response;
      } catch (error) {
        if (authErrorRequiresReauthentication(error)) {
          await options.clearSessionTokens?.();
        }
        throw error;
      } finally {
        refreshInFlight = null;
      }
    })();

    return refreshInFlight;
  };
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

function resolveRemoteFetch(options: RemoteHubClientOptions): typeof globalThis.fetch {
  const fetchImpl = options.fetch ?? globalThis.fetch;
  if (!fetchImpl) {
    throw new Error("fetch is not available in this environment.");
  }

  return fetchImpl;
}

export function createLocalHubClient(transport: LocalHubTransport): HubClient {
  async function getWorkspaceModelPolicy(
    workspaceId: string
  ): Promise<TenantModelPolicy | null> {
    const result = await transport.invoke(
      LOCAL_HUB_COMMANDS.getWorkspaceModelPolicy,
      {
        workspaceId
      }
    );

    return result === null ? null : parseTenantModelPolicy(result);
  }

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
    async listProjects(workspaceId) {
      return parseProjects(
        await transport.invoke(LOCAL_HUB_COMMANDS.listProjects, {
          workspaceId
        })
      );
    },
    async getProjectContext(workspaceId, projectId) {
      return parseProjectContext(
        await transport.invoke(LOCAL_HUB_COMMANDS.getProjectContext, {
          workspaceId,
          projectId
        })
      );
    },
    async getProjectKnowledge(workspaceId, projectId) {
      return parseProjectKnowledgeIndex(
        await transport.invoke(LOCAL_HUB_COMMANDS.getProjectKnowledge, {
          workspaceId,
          projectId
        })
      );
    },
    async listModelProviders(workspaceId) {
      return parseModelProviders(
        await transport.invoke(LOCAL_HUB_COMMANDS.listModelProviders, {
          workspaceId
        })
      );
    },
    async listModelCatalogItems(workspaceId) {
      return parseModelCatalogItems(
        await transport.invoke(LOCAL_HUB_COMMANDS.listModelCatalogItems, {
          workspaceId
        })
      );
    },
    async listModelProfiles(workspaceId) {
      return parseModelProfiles(
        await transport.invoke(LOCAL_HUB_COMMANDS.listModelProfiles, {
          workspaceId
        })
      );
    },
    async getWorkspaceModelPolicy(workspaceId) {
      return getWorkspaceModelPolicy(workspaceId);
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
    async retryRun(command) {
      return parseRunDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.retryRun,
          parseRunRetryCommand(command)
        )
      );
    },
    async terminateRun(command) {
      return parseRunDetail(
        await transport.invoke(
          LOCAL_HUB_COMMANDS.terminateRun,
          parseRunTerminateCommand(command)
        )
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

export function createRemoteHubAuthClient(
  options: RemoteHubClientOptions
): RemoteHubAuthClient {
  const fetchImpl = resolveRemoteFetch(options);

  return {
    async login(command) {
      const parsed = parseHubLoginCommand(command);
      return parseHubLoginResponse(
        await readRemoteJson(fetchImpl, remotePath(options.baseUrl, "/api/auth/login"), {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(parsed)
        })
      );
    },
    async refreshSession() {
      return performRefreshSession(fetchImpl, options);
    },
    async getCurrentSession() {
      return parseHubSession(
        await readRemoteJson(
          fetchImpl,
          remotePath(options.baseUrl, "/api/auth/session"),
          { method: "GET" },
          options.getAccessToken
        )
      );
    },
    async logout() {
      await readRemoteJson(
        fetchImpl,
        remotePath(options.baseUrl, "/api/auth/logout"),
        { method: "POST" },
        options.getAccessToken
      );
    }
  };
}

export function createRemoteHubClient(options: RemoteHubClientOptions): HubClient {
  const fetchImpl = resolveRemoteFetch(options);
  let currentAccessToken: string | null | undefined;
  let currentRefreshToken: string | null | undefined;
  const getAccessToken = async () => currentAccessToken ?? options.getAccessToken?.();
  const getRefreshToken = async () => currentRefreshToken ?? options.getRefreshToken?.();
  const clearSessionTokens = async () => {
    currentAccessToken = null;
    currentRefreshToken = null;
    await options.clearSessionTokens?.();
  };
  const refreshSession = createRefreshCoordinator(fetchImpl, {
    ...options,
    getRefreshToken,
    clearSessionTokens,
    onRefreshTokens: async (response) => {
      currentAccessToken = response.access_token;
      currentRefreshToken = response.refresh_token;
      await options.onRefreshTokens?.(response);
    }
  });
  const createEventSource =
    options.createEventSource ??
    ((url: string) => {
      const EventSourceCtor = globalThis.EventSource;
      if (!EventSourceCtor) {
        throw new Error("EventSource is not available in this environment.");
      }
      return new EventSourceCtor(url) as unknown as EventSourceLike;
    });

  async function readAuthenticatedJson(
    url: string,
    init?: RequestInit,
    allowRefresh = true
  ): Promise<unknown> {
    try {
      return await readRemoteJson(fetchImpl, url, init, getAccessToken);
    } catch (error) {
      if (!allowRefresh || !isTokenExpiredAuthError(error) || !options.getRefreshToken) {
        throw error;
      }

      await refreshSession();

      try {
        return await readRemoteJson(fetchImpl, url, init, getAccessToken);
      } catch (replayError) {
        if (authErrorRequiresReauthentication(replayError)) {
          await clearSessionTokens();
        }
        throw replayError;
      }
    }
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

    return parseCapabilityResolutions(await readAuthenticatedJson(capabilitiesUrl.toString()));
  }

  return {
    async listProjects(workspaceId) {
      return parseProjects(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/projects`
          )
        )
      );
    },
    async getProjectContext(workspaceId, projectId) {
      return parseProjectContext(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/context`
          )
        )
      );
    },
    async getProjectKnowledge(workspaceId, projectId) {
      return parseProjectKnowledgeIndex(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/knowledge`
          )
        )
      );
    },
    async listModelProviders(workspaceId) {
      return parseModelProviders(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/models/providers`
          )
        )
      );
    },
    async listModelCatalogItems(workspaceId) {
      return parseModelCatalogItems(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/models/catalog`
          )
        )
      );
    },
    async listModelProfiles(workspaceId) {
      return parseModelProfiles(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/models/profiles`
          )
        )
      );
    },
    async getWorkspaceModelPolicy(workspaceId) {
      const result = await readAuthenticatedJson(
        remotePath(
          options.baseUrl,
          `/api/workspaces/${encodePathSegment(workspaceId)}/models/policy`
        )
      );
      return result === null ? null : parseTenantModelPolicy(result);
    },
    async listAutomations(workspaceId, projectId) {
      return parseAutomationSummaries(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/automations`
          )
        )
      );
    },
    async createAutomation(command) {
      const parsed = parseCreateAutomationCommand(command);
      return parseCreateAutomationResponse(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(parsed.workspace_id)}/projects/${encodePathSegment(parsed.project_id)}/automations`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          }
        )
      );
    },
    async listRuns(workspaceId, projectId) {
      return parseRunSummaries(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/projects/${encodePathSegment(projectId)}/runs`
          )
        )
      );
    },
    async getAutomationDetail(automationId) {
      return parseAutomationDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/automations/${encodePathSegment(automationId)}`
          )
        )
      );
    },
    async activateAutomation(automationId) {
      return parseAutomationDetail(
        await readAuthenticatedJson(
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
          }
        )
      );
    },
    async pauseAutomation(automationId) {
      return parseAutomationDetail(
        await readAuthenticatedJson(
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
          }
        )
      );
    },
    async archiveAutomation(automationId) {
      return parseAutomationDetail(
        await readAuthenticatedJson(
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
          }
        )
      );
    },
    async manualDispatch(command) {
      const parsed = parseManualDispatchCommand(command);
      return parseAutomationDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/triggers/${encodePathSegment(parsed.trigger_id)}/manual-dispatch`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          }
        )
      );
    },
    async retryTriggerDelivery(command) {
      const parsed = parseTriggerDeliveryRetryCommand(command);
      return parseAutomationDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/trigger-deliveries/${encodePathSegment(parsed.delivery_id)}/retry`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          }
        )
      );
    },
    async createTask(command) {
      return parseTask(
        await readAuthenticatedJson(remotePath(options.baseUrl, "/api/tasks"), {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(parseTaskCreateCommand(command))
        })
      );
    },
    async startTask(taskId) {
      return parseRunDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/tasks/${encodePathSegment(taskId)}/start`
          ),
          { method: "POST" }
        )
      );
    },
    async getRunDetail(runId) {
      return parseRunDetail(
        await readAuthenticatedJson(
          remotePath(options.baseUrl, `/api/runs/${encodePathSegment(runId)}`)
        )
      );
    },
    async retryRun(command) {
      const parsed = parseRunRetryCommand(command);
      return parseRunDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/runs/${encodePathSegment(parsed.run_id)}/retry`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          }
        )
      );
    },
    async terminateRun(command) {
      const parsed = parseRunTerminateCommand(command);
      return parseRunDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/runs/${encodePathSegment(parsed.run_id)}/terminate`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          }
        )
      );
    },
    async getApprovalRequest(approvalId) {
      return parseApprovalRequest(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/approvals/${encodePathSegment(approvalId)}`
          )
        )
      );
    },
    async resolveApproval(command) {
      const parsed = parseApprovalResolveCommand(command);
      return parseRunDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/approvals/${encodePathSegment(parsed.approval_id)}/resolve`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          }
        )
      );
    },
    async listInboxItems(workspaceId) {
      return parseInboxItems(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/inbox`
          )
        )
      );
    },
    async listNotifications(workspaceId) {
      return parseNotifications(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/workspaces/${encodePathSegment(workspaceId)}/notifications`
          )
        )
      );
    },
    async listArtifacts(runId) {
      return parseArtifacts(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/runs/${encodePathSegment(runId)}/artifacts`
          )
        )
      );
    },
    async getKnowledgeDetail(runId) {
      return parseKnowledgeDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/runs/${encodePathSegment(runId)}/knowledge`
          )
        )
      );
    },
    async requestKnowledgePromotion(command) {
      const parsed = parseRequestKnowledgePromotionCommand(command);
      return parseApprovalRequest(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/knowledge/candidates/${encodePathSegment(parsed.candidate_id)}/request-promotion`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          }
        )
      );
    },
    async promoteKnowledge(command) {
      const parsed = parseKnowledgePromoteCommand(command);
      return parseKnowledgeDetail(
        await readAuthenticatedJson(
          remotePath(
            options.baseUrl,
            `/api/knowledge/candidates/${encodePathSegment(parsed.candidate_id)}/promote`
          ),
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(parsed)
          }
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
        await readAuthenticatedJson(remotePath(options.baseUrl, "/api/hub/connection"))
      );
    },
    async subscribe(listener, onError) {
      let activeEventSource: EventSourceLike | null = null;
      let closed = false;
      let reconnectedAfterRefresh = false;

      const connect = async () => {
        const eventsUrl = new URL(remotePath(options.baseUrl, "/api/events"));
        const accessToken = await getAccessToken();
        if (accessToken) {
          eventsUrl.searchParams.set("access_token", accessToken);
        }

        const eventSource = createEventSource(eventsUrl.toString());
        activeEventSource = eventSource;
        eventSource.onmessage = (event) => {
          try {
            listener(parseHubEvent(JSON.parse(event.data)));
          } catch (error) {
            onError?.(toError(error));
          }
        };
        eventSource.onerror = (error) => {
          void handleError(eventSource, error);
        };
      };

      const handleError = async (
        eventSource: EventSourceLike,
        error: unknown
      ): Promise<void> => {
        if (closed || activeEventSource !== eventSource) {
          return;
        }

        if (!reconnectedAfterRefresh && isTokenExpiredAuthError(error)) {
          reconnectedAfterRefresh = true;
          eventSource.close();

          try {
            await refreshSession();
            if (!closed) {
              await connect();
            }
            return;
          } catch (refreshError) {
            onError?.(toError(refreshError));
            return;
          }
        }

        if (authErrorRequiresReauthentication(error)) {
          await clearSessionTokens();
        }
        onError?.(toError(error));
      };

      await connect();

      return () => {
        closed = true;
        activeEventSource?.close();
      };
    }
  };
}
