import type { HubClient } from "@octopus/hub-client";
import { defineStore } from "pinia";
import { computed, ref } from "vue";

type ProjectContext = Awaited<ReturnType<HubClient["getProjectContext"]>>;
type CapabilityVisibility = Awaited<
  ReturnType<HubClient["listCapabilityVisibility"]>
>[number];
type HubConnectionStatus = Awaited<
  ReturnType<HubClient["getHubConnectionStatus"]>
>;
type InboxItem = Awaited<ReturnType<HubClient["listInboxItems"]>>[number];
type Notification = Awaited<ReturnType<HubClient["listNotifications"]>>[number];
type RunDetail = Awaited<ReturnType<HubClient["getRunDetail"]>>;
type Artifact = Awaited<ReturnType<HubClient["listArtifacts"]>>[number];
type KnowledgeDetail = Awaited<ReturnType<HubClient["getKnowledgeDetail"]>>;
type TaskCreateCommand = Parameters<HubClient["createTask"]>[0];

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
  const currentRunId = ref<string | null>(null);

  const projectContext = ref<ProjectContext | null>(null);
  const capabilityVisibilities = ref<CapabilityVisibility[]>([]);
  const connectionStatus = ref<HubConnectionStatus | null>(null);
  const inboxItems = ref<InboxItem[]>([]);
  const notifications = ref<Notification[]>([]);
  const runDetail = ref<RunDetail | null>(null);
  const artifacts = ref<Artifact[]>([]);
  const knowledgeDetail = ref<KnowledgeDetail | null>(null);

  const workspaceLoading = ref(false);
  const taskSubmitting = ref(false);
  const runLoading = ref(false);
  const surfaceError = ref<string | null>(null);

  const workspaceName = computed(
    () => projectContext.value?.workspace.display_name ?? "Workspace"
  );
  const projectName = computed(
    () => projectContext.value?.project.display_name ?? "Project"
  );
  const activeCapability = computed(
    () => capabilityVisibilities.value[0]?.descriptor ?? null
  );

  async function loadWorkspace(
    workspaceId: string,
    projectId: string
  ): Promise<void> {
    workspaceLoading.value = true;
    surfaceError.value = null;
    currentWorkspaceId.value = workspaceId;
    currentProjectId.value = projectId;

    try {
      const client = requireHubClient();
      const [
        nextProjectContext,
        nextCapabilityVisibilities,
        nextConnectionStatus,
        nextInboxItems,
        nextNotifications
      ] = await Promise.all([
        client.getProjectContext(workspaceId, projectId),
        client.listCapabilityVisibility(workspaceId, projectId),
        client.getHubConnectionStatus(),
        client.listInboxItems(workspaceId),
        client.listNotifications(workspaceId)
      ]);

      projectContext.value = nextProjectContext;
      capabilityVisibilities.value = nextCapabilityVisibilities;
      connectionStatus.value = nextConnectionStatus;
      inboxItems.value = nextInboxItems;
      notifications.value = nextNotifications;
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      workspaceLoading.value = false;
    }
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
      runDetail.value = nextRunDetail;
      artifacts.value = nextArtifacts;
      knowledgeDetail.value = nextKnowledgeDetail;

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
      const [nextRunDetail, nextArtifacts, nextKnowledgeDetail] =
        await Promise.all([
          client.getRunDetail(runId),
          client.listArtifacts(runId),
          client.getKnowledgeDetail(runId)
        ]);

      currentRunId.value = runId;
      runDetail.value = nextRunDetail;
      artifacts.value = nextArtifacts;
      knowledgeDetail.value = nextKnowledgeDetail;
    } catch (error) {
      surfaceError.value = toErrorMessage(error);
      throw error;
    } finally {
      runLoading.value = false;
    }
  }

  return {
    currentWorkspaceId,
    currentProjectId,
    currentRunId,
    projectContext,
    capabilityVisibilities,
    connectionStatus,
    inboxItems,
    notifications,
    runDetail,
    artifacts,
    knowledgeDetail,
    workspaceLoading,
    taskSubmitting,
    runLoading,
    surfaceError,
    workspaceName,
    projectName,
    activeCapability,
    loadWorkspace,
    createAndStartTask,
    loadRun
  };
});
