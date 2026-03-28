import type { HubClient } from "@octopus/hub-client";
import { defineStore } from "pinia";
import { computed, ref } from "vue";

type ProjectContext = Awaited<ReturnType<HubClient["getProjectContext"]>>;
type CapabilityResolution = Awaited<
  ReturnType<HubClient["listCapabilityResolutions"]>
>[number];
type HubConnectionStatus = Awaited<
  ReturnType<HubClient["getHubConnectionStatus"]>
>;
type ApprovalRequest = Awaited<ReturnType<HubClient["getApprovalRequest"]>>;
type InboxItem = Awaited<ReturnType<HubClient["listInboxItems"]>>[number];
type Notification = Awaited<ReturnType<HubClient["listNotifications"]>>[number];
type AutomationSummary = Awaited<ReturnType<HubClient["listAutomations"]>>[number];
type AutomationDetail = Awaited<ReturnType<HubClient["getAutomationDetail"]>>;
type CreateAutomationCommand = Parameters<HubClient["createAutomation"]>[0];
type CreateAutomationResponse = Awaited<
  ReturnType<HubClient["createAutomation"]>
>;
type ManualDispatchCommand = Parameters<HubClient["manualDispatch"]>[0];
type RunDetail = Awaited<ReturnType<HubClient["getRunDetail"]>>;
type RunSummary = Awaited<ReturnType<HubClient["listRuns"]>>[number];
type Artifact = Awaited<ReturnType<HubClient["listArtifacts"]>>[number];
type KnowledgeDetail = Awaited<ReturnType<HubClient["getKnowledgeDetail"]>>;
type TaskCreateCommand = Parameters<HubClient["createTask"]>[0];
type ApprovalDecision = Parameters<HubClient["resolveApproval"]>[0]["decision"];

const DESKTOP_ACTOR_REF = "workspace_admin:desktop_operator";

let hubClient: HubClient | null = null;

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function requireHubClient(): HubClient {
  if (!hubClient) {
    throw new Error("HubClient has not been configured for the desktop shell.");
  }

  return hubClient;
}

export function configureHubClient(client: HubClient): void {
  hubClient = client;
}

export const useHubStore = defineStore("hub", () => {
  const currentWorkspaceId = ref<string | null>(null);
  const currentProjectId = ref<string | null>(null);
  const currentAutomationId = ref<string | null>(null);
  const currentRunId = ref<string | null>(null);

  const projectContext = ref<ProjectContext | null>(null);
  const taskCapabilityResolutions = ref<CapabilityResolution[]>([]);
  const automationCapabilityResolutions = ref<CapabilityResolution[]>([]);
  const taskCapabilityEstimatedCost = ref(1);
  const automationCapabilityEstimatedCost = ref(1);
  const connectionStatus = ref<HubConnectionStatus | null>(null);
  const runs = ref<RunSummary[]>([]);
  const inboxItems = ref<InboxItem[]>([]);
  const notifications = ref<Notification[]>([]);
  const approvalDetails = ref<Record<string, ApprovalRequest>>({});
  const automations = ref<AutomationSummary[]>([]);
  const automationDetail = ref<AutomationDetail | null>(null);
  const webhookSecretReveal = ref<string | null>(null);
  const runDetail = ref<RunDetail | null>(null);
  const artifacts = ref<Artifact[]>([]);
  const knowledgeDetail = ref<KnowledgeDetail | null>(null);

  const workspaceLoading = ref(false);
  const runsLoading = ref(false);
  const inboxLoading = ref(false);
  const notificationsLoading = ref(false);
  const connectionLoading = ref(false);
  const automationLoading = ref(false);
  const automationSubmitting = ref(false);
  const automationActionLoading = ref(false);
  const taskSubmitting = ref(false);
  const runLoading = ref(false);
  const governanceActionLoading = ref(false);
  const governanceActionTarget = ref<string | null>(null);
  const surfaceError = ref<string | null>(null);

  const workspaceName = computed(
    () => projectContext.value?.workspace.display_name ?? "Workspace"
  );
  const projectName = computed(
    () => projectContext.value?.project.display_name ?? "Project"
  );
  const activeCapability = computed(
    () =>
      taskCapabilityResolutions.value[0]?.descriptor ??
      automationCapabilityResolutions.value[0]?.descriptor ??
      null
  );
  const authState = computed(
    () => connectionStatus.value?.auth_state ?? "auth_required"
  );
  const readOnlyMode = computed(() => authState.value !== "authenticated");

  function setProjectScope(workspaceId: string, projectId: string): void {
    currentWorkspaceId.value = workspaceId;
    currentProjectId.value = projectId;
  }

  function setWorkspaceScope(workspaceId: string): void {
    currentWorkspaceId.value = workspaceId;
  }

  function upsertAutomation(nextAutomation: AutomationSummary): void {
    const nextAutomations = [...automations.value];
    const index = nextAutomations.findIndex(
      (automation) => automation.automation.id === nextAutomation.automation.id
    );

    if (index >= 0) {
      nextAutomations.splice(index, 1, nextAutomation);
    } else {
      nextAutomations.unshift(nextAutomation);
    }

    automations.value = nextAutomations;
  }

  function setAutomationDetail(nextAutomation: AutomationDetail): void {
    setProjectScope(
      nextAutomation.automation.workspace_id,
      nextAutomation.automation.project_id
    );
    currentAutomationId.value = nextAutomation.automation.id;
    automationDetail.value = nextAutomation;
    upsertAutomation(nextAutomation);
  }

  function rememberApproval(approval: ApprovalRequest): void {
    approvalDetails.value = {
      ...approvalDetails.value,
      [approval.id]: approval
    };
  }

  function rememberApprovals(approvals: ApprovalRequest[]): void {
    if (approvals.length === 0) {
      return;
    }

    const nextApprovals = { ...approvalDetails.value };
    for (const approval of approvals) {
      nextApprovals[approval.id] = approval;
    }
    approvalDetails.value = nextApprovals;
  }

  async function hydrateApprovalDetails(approvalIds: string[]): Promise<void> {
    const uniqueApprovalIds = [...new Set(approvalIds)].filter(Boolean);
    if (uniqueApprovalIds.length === 0) {
      return;
    }

    const client = requireHubClient();
    const approvals = await Promise.all(
      uniqueApprovalIds.map((approvalId) => client.getApprovalRequest(approvalId))
    );
    rememberApprovals(approvals);
  }

  async function loadProjectContext(
    workspaceId: string,
    projectId: string
  ): Promise<void> {
    surfaceError.value = null;
    setProjectScope(workspaceId, projectId);

    try {
      const client = requireHubClient();
      projectContext.value = await client.getProjectContext(workspaceId, projectId);
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    }
  }

  async function loadConnectionStatus(): Promise<void> {
    connectionLoading.value = true;
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      connectionStatus.value = await client.getHubConnectionStatus();
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      connectionLoading.value = false;
    }
  }

  async function loadRuns(
    workspaceId: string,
    projectId: string
  ): Promise<void> {
    runsLoading.value = true;
    surfaceError.value = null;
    setProjectScope(workspaceId, projectId);

    try {
      const client = requireHubClient();
      runs.value = await client.listRuns(workspaceId, projectId);
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      runsLoading.value = false;
    }
  }

  async function loadInboxItems(workspaceId: string): Promise<void> {
    inboxLoading.value = true;
    surfaceError.value = null;
    setWorkspaceScope(workspaceId);

    try {
      const client = requireHubClient();
      const nextInboxItems = await client.listInboxItems(workspaceId);
      inboxItems.value = nextInboxItems;
      await hydrateApprovalDetails(
        nextInboxItems.map((item) => item.approval_request_id)
      );
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      inboxLoading.value = false;
    }
  }

  async function loadNotifications(workspaceId: string): Promise<void> {
    notificationsLoading.value = true;
    surfaceError.value = null;
    setWorkspaceScope(workspaceId);

    try {
      const client = requireHubClient();
      notifications.value = await client.listNotifications(workspaceId);
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      notificationsLoading.value = false;
    }
  }

  async function loadAutomations(
    workspaceId: string,
    projectId: string
  ): Promise<void> {
    surfaceError.value = null;
    setProjectScope(workspaceId, projectId);

    try {
      const client = requireHubClient();
      automations.value = await client.listAutomations(workspaceId, projectId);
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    }
  }

  async function loadTaskSurface(
    workspaceId: string,
    projectId: string,
    taskEstimatedCost = taskCapabilityEstimatedCost.value,
    automationEstimatedCost = automationCapabilityEstimatedCost.value
  ): Promise<void> {
    workspaceLoading.value = true;
    surfaceError.value = null;
    setProjectScope(workspaceId, projectId);
    taskCapabilityEstimatedCost.value = taskEstimatedCost;
    automationCapabilityEstimatedCost.value = automationEstimatedCost;

    try {
      const client = requireHubClient();
      const [
        nextProjectContext,
        nextTaskCapabilityResolutions,
        nextAutomationCapabilityResolutions,
        nextConnectionStatus,
        nextAutomations
      ] = await Promise.all([
        client.getProjectContext(workspaceId, projectId),
        client.listCapabilityResolutions(workspaceId, projectId, taskEstimatedCost),
        client.listCapabilityResolutions(
          workspaceId,
          projectId,
          automationEstimatedCost
        ),
        client.getHubConnectionStatus(),
        client.listAutomations(workspaceId, projectId)
      ]);

      projectContext.value = nextProjectContext;
      taskCapabilityResolutions.value = nextTaskCapabilityResolutions;
      automationCapabilityResolutions.value = nextAutomationCapabilityResolutions;
      connectionStatus.value = nextConnectionStatus;
      automations.value = nextAutomations;
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      workspaceLoading.value = false;
    }
  }

  async function loadTaskCapabilityResolutions(
    workspaceId: string,
    projectId: string,
    estimatedCost: number
  ): Promise<void> {
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      taskCapabilityEstimatedCost.value = estimatedCost;
      taskCapabilityResolutions.value = await client.listCapabilityResolutions(
        workspaceId,
        projectId,
        estimatedCost
      );
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    }
  }

  async function loadAutomationCapabilityResolutions(
    workspaceId: string,
    projectId: string,
    estimatedCost: number
  ): Promise<void> {
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      automationCapabilityEstimatedCost.value = estimatedCost;
      automationCapabilityResolutions.value = await client.listCapabilityResolutions(
        workspaceId,
        projectId,
        estimatedCost
      );
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    }
  }

  async function createAutomation(
    command: CreateAutomationCommand
  ): Promise<CreateAutomationResponse> {
    automationSubmitting.value = true;
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      const response = await client.createAutomation(command);
      currentAutomationId.value = response.automation.id;
      webhookSecretReveal.value = response.webhook_secret;
      await loadAutomations(command.workspace_id, command.project_id);
      return response;
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      automationSubmitting.value = false;
    }
  }

  async function loadAutomation(automationId: string): Promise<void> {
    automationLoading.value = true;
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      const nextAutomationDetail = await client.getAutomationDetail(automationId);
      setAutomationDetail(nextAutomationDetail);
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      automationLoading.value = false;
    }
  }

  async function mutateAutomation(
    load: () => Promise<AutomationDetail>
  ): Promise<AutomationDetail> {
    automationActionLoading.value = true;
    surfaceError.value = null;

    try {
      const nextAutomationDetail = await load();
      setAutomationDetail(nextAutomationDetail);
      return nextAutomationDetail;
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      automationActionLoading.value = false;
    }
  }

  async function activateAutomation(automationId: string): Promise<AutomationDetail> {
    const client = requireHubClient();
    return mutateAutomation(() => client.activateAutomation(automationId));
  }

  async function pauseAutomation(automationId: string): Promise<AutomationDetail> {
    const client = requireHubClient();
    return mutateAutomation(() => client.pauseAutomation(automationId));
  }

  async function archiveAutomation(automationId: string): Promise<AutomationDetail> {
    const client = requireHubClient();
    return mutateAutomation(() => client.archiveAutomation(automationId));
  }

  async function manualDispatch(
    command: ManualDispatchCommand
  ): Promise<AutomationDetail> {
    const client = requireHubClient();
    return mutateAutomation(() => client.manualDispatch(command));
  }

  async function retryAutomationDelivery(
    deliveryId: string
  ): Promise<AutomationDetail> {
    const client = requireHubClient();
    return mutateAutomation(() =>
      client.retryTriggerDelivery({
        delivery_id: deliveryId
      })
    );
  }

  async function createAndStartTask(command: TaskCreateCommand): Promise<RunDetail> {
    taskSubmitting.value = true;
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      const task = await client.createTask(command);
      const nextRunDetail = await client.startTask(task.id);
      const [nextArtifacts, nextKnowledgeDetail] = await Promise.all([
        client.listArtifacts(nextRunDetail.run.id),
        client.getKnowledgeDetail(nextRunDetail.run.id)
      ]);

      currentRunId.value = nextRunDetail.run.id;
      setProjectScope(command.workspace_id, command.project_id);
      runDetail.value = nextRunDetail;
      artifacts.value = nextArtifacts;
      knowledgeDetail.value = nextKnowledgeDetail;
      rememberApprovals(nextRunDetail.approvals);
      await loadRuns(command.workspace_id, command.project_id);

      return nextRunDetail;
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      taskSubmitting.value = false;
    }
  }

  async function loadRun(runId: string): Promise<void> {
    runLoading.value = true;
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      const [nextRunDetail, nextArtifacts, nextKnowledgeDetail, nextConnectionStatus] =
        await Promise.all([
          client.getRunDetail(runId),
          client.listArtifacts(runId),
          client.getKnowledgeDetail(runId),
          client.getHubConnectionStatus()
        ]);

      currentRunId.value = runId;
      setProjectScope(nextRunDetail.run.workspace_id, nextRunDetail.run.project_id);
      runDetail.value = nextRunDetail;
      artifacts.value = nextArtifacts;
      knowledgeDetail.value = nextKnowledgeDetail;
      connectionStatus.value = nextConnectionStatus;
      rememberApprovals(nextRunDetail.approvals);
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      runLoading.value = false;
    }
  }

  async function resolveGovernanceApproval(
    approvalId: string,
    decision: ApprovalDecision,
    note = decision
  ): Promise<RunDetail> {
    governanceActionLoading.value = true;
    governanceActionTarget.value = approvalId;
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      const nextRunDetail = await client.resolveApproval({
        approval_id: approvalId,
        decision,
        actor_ref: DESKTOP_ACTOR_REF,
        note
      });
      rememberApprovals(nextRunDetail.approvals);

      const refreshes: Promise<void>[] = [];

      if (currentWorkspaceId.value) {
        refreshes.push(loadInboxItems(currentWorkspaceId.value));
        refreshes.push(loadNotifications(currentWorkspaceId.value));
      }

      if (currentWorkspaceId.value && currentProjectId.value) {
        refreshes.push(loadRuns(currentWorkspaceId.value, currentProjectId.value));
      }

      if (currentRunId.value === nextRunDetail.run.id) {
        refreshes.push(loadRun(nextRunDetail.run.id));
      }

      await Promise.all(refreshes);

      return nextRunDetail;
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      governanceActionLoading.value = false;
      governanceActionTarget.value = null;
    }
  }

  async function requestKnowledgePromotion(
    candidateId: string,
    note = "request knowledge promotion"
  ): Promise<ApprovalRequest> {
    governanceActionLoading.value = true;
    governanceActionTarget.value = candidateId;
    surfaceError.value = null;

    try {
      const client = requireHubClient();
      const approval = await client.requestKnowledgePromotion({
        candidate_id: candidateId,
        actor_ref: DESKTOP_ACTOR_REF,
        note
      });
      rememberApproval(approval);

      const refreshes: Promise<void>[] = [];
      if (currentWorkspaceId.value) {
        refreshes.push(loadInboxItems(currentWorkspaceId.value));
        refreshes.push(loadNotifications(currentWorkspaceId.value));
      }
      if (currentRunId.value) {
        refreshes.push(loadRun(currentRunId.value));
      }
      await Promise.all(refreshes);

      return approval;
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      governanceActionLoading.value = false;
      governanceActionTarget.value = null;
    }
  }

  return {
    currentWorkspaceId,
    currentProjectId,
    currentAutomationId,
    currentRunId,
    projectContext,
    taskCapabilityResolutions,
    automationCapabilityResolutions,
    taskCapabilityEstimatedCost,
    automationCapabilityEstimatedCost,
    connectionStatus,
    runs,
    inboxItems,
    notifications,
    approvalDetails,
    automations,
    automationDetail,
    webhookSecretReveal,
    runDetail,
    artifacts,
    knowledgeDetail,
    workspaceLoading,
    runsLoading,
    inboxLoading,
    notificationsLoading,
    connectionLoading,
    automationLoading,
    automationSubmitting,
    automationActionLoading,
    taskSubmitting,
    runLoading,
    governanceActionLoading,
    governanceActionTarget,
    surfaceError,
    workspaceName,
    projectName,
    activeCapability,
    authState,
    readOnlyMode,
    loadProjectContext,
    loadConnectionStatus,
    loadRuns,
    loadInboxItems,
    loadNotifications,
    loadAutomations,
    loadTaskSurface,
    loadTaskCapabilityResolutions,
    loadAutomationCapabilityResolutions,
    createAutomation,
    loadAutomation,
    activateAutomation,
    pauseAutomation,
    archiveAutomation,
    manualDispatch,
    retryAutomationDelivery,
    createAndStartTask,
    loadRun,
    resolveGovernanceApproval,
    requestKnowledgePromotion
  };
});
