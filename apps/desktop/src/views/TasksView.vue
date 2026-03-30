<script setup lang="ts">
import { computed, onMounted, reactive, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { 
  PlusCircle, 
  Settings, 
  ShieldCheck, 
  History, 
  ChevronRight,
  ExternalLink,
  Info
} from "lucide-vue-next";

import {
  buildWorkspaceProjectsRoute,
  useConnectionStore
} from "../stores/connection";
import { useHubStore } from "../stores/hub";

// UI Components
import OButton from "../components/ui/OButton.vue";
import OPill from "../components/ui/OPill.vue";
import OCard from "../components/ui/OCard.vue";
import OCardHeader from "../components/ui/OCardHeader.vue";
import OCardContent from "../components/ui/OCardContent.vue";
import OCardFooter from "../components/ui/OCardFooter.vue";
import OStatPill from "../components/ui/OStatPill.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";
import PageGrid from "../components/layout/PageGrid.vue";

const route = useRoute();
const router = useRouter();
const hub = useHubStore();
const connection = useConnectionStore();

type AutomationTriggerType = "manual_event" | "cron" | "webhook" | "mcp_event";
type CreateTriggerInput = Parameters<typeof hub.createAutomation>[0]["trigger"];

const taskDraft = reactive({
  title: "Write note",
  instruction: "Emit a deterministic artifact",
  actionContent: "hello",
  capabilityId: "",
  estimatedCost: 1
});

const automationDraft = reactive({
  title: "Manual automation",
  instruction: "Dispatch on demand",
  actionContent: "manual artifact",
  capabilityId: "",
  estimatedCost: 1,
  triggerType: "manual_event" as AutomationTriggerType,
  cronSchedule: "0 * * * * * *",
  cronTimezone: "UTC",
  cronNextFireAt: "2026-03-27T10:00:00Z",
  webhookIngressMode: "shared_secret_header",
  webhookSecretHeaderName: "X-Octopus-Trigger-Secret",
  webhookSecretHint: "hook",
  webhookSecretPlaintext: "",
  mcpServerId: "server-automation",
  mcpEventName: "connector.output.ready",
  mcpEventPattern: ""
});

const projectCapabilities = computed(() =>
  hub.taskCapabilityResolutions.map((resolution) => resolution.descriptor)
);

const taskCapabilityResolution = computed(
  () =>
    hub.taskCapabilityResolutions.find(
      (resolution) => resolution.descriptor.id === taskDraft.capabilityId
    ) ??
    hub.taskCapabilityResolutions[0] ??
    null
);

const automationCapabilityResolution = computed(
  () =>
    hub.automationCapabilityResolutions.find(
      (resolution) => resolution.descriptor.id === automationDraft.capabilityId
    ) ??
    hub.automationCapabilityResolutions[0] ??
    null
);

const visibleCapability = computed(
  () =>
    taskCapabilityResolution.value?.descriptor ??
    automationCapabilityResolution.value?.descriptor ??
    hub.activeCapability
);
const localMode = computed(() => hub.connectionStatus?.mode === "local");
const unsupportedLocalTrigger = computed(
  () =>
    localMode.value &&
    (automationDraft.triggerType === "webhook" ||
      automationDraft.triggerType === "mcp_event")
);
const automationTriggerOptions = computed(() => [
  {
    value: "manual_event" as const,
    disabled: false
  },
  {
    value: "cron" as const,
    disabled: false
  },
  {
    value: "webhook" as const,
    disabled: localMode.value
  },
  {
    value: "mcp_event" as const,
    disabled: localMode.value
  }
]);

async function loadTaskSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);

  try {
    await hub.loadTaskSurface(workspaceId, projectId);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    const rememberedProjectId = connection.profile.projectId ?? "";
    if (
      connection.remoteMode &&
      rememberedProjectId === projectId &&
      message.includes(`project \`${projectId}\` not found`)
    ) {
      connection.clearRememberedProject();
      await router.replace(buildWorkspaceProjectsRoute(workspaceId));
      return;
    }

    return;
  }

  if (connection.remoteMode) {
    connection.rememberProject(projectId);
  }

  if (!taskDraft.capabilityId && taskCapabilityResolution.value) {
    taskDraft.capabilityId = taskCapabilityResolution.value.descriptor.id;
  }
  if (!automationDraft.capabilityId && automationCapabilityResolution.value) {
    automationDraft.capabilityId = automationCapabilityResolution.value.descriptor.id;
  }
}

async function createAndStart(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);
  const capabilityId = taskDraft.capabilityId || visibleCapability.value?.id;

  if (!capabilityId) {
    throw new Error("No governed capability is available for task creation.");
  }

  const runDetail = await hub.createAndStartTask({
    workspace_id: workspaceId,
    project_id: projectId,
    title: taskDraft.title,
    instruction: taskDraft.instruction,
    action: {
      kind: "emit_text",
      content: taskDraft.actionContent
    },
    capability_id: capabilityId,
    estimated_cost: taskDraft.estimatedCost,
    idempotency_key: `${workspaceId}:${projectId}:${taskDraft.title}`
  });

  await router.push(`/runs/${runDetail.run.id}`);
}

async function handleCreateAndStart(): Promise<void> {
  try {
    await createAndStart();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function openAutomation(automationId: string): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);
  await router.push(
    `/workspaces/${workspaceId}/projects/${projectId}/automations/${automationId}`
  );
}

function buildCreateTriggerInput(): CreateTriggerInput {
  switch (automationDraft.triggerType) {
    case "cron":
      return {
        trigger_type: "cron",
        config: {
          schedule: automationDraft.cronSchedule,
          timezone: automationDraft.cronTimezone,
          next_fire_at: automationDraft.cronNextFireAt
        }
      };
    case "webhook":
      return {
        trigger_type: "webhook",
        config: {
          ingress_mode: automationDraft.webhookIngressMode,
          secret_header_name: automationDraft.webhookSecretHeaderName,
          secret_hint: automationDraft.webhookSecretHint || null,
          secret_plaintext: automationDraft.webhookSecretPlaintext || null
        }
      };
    case "mcp_event":
      return {
        trigger_type: "mcp_event",
        config: {
          server_id: automationDraft.mcpServerId,
          event_name: automationDraft.mcpEventName || null,
          event_pattern: automationDraft.mcpEventPattern || null
        }
      };
    case "manual_event":
    default:
      return {
        trigger_type: "manual_event",
        config: {}
      };
  }
}

async function createAutomation(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);
  const capabilityId = automationDraft.capabilityId || visibleCapability.value?.id;

  if (unsupportedLocalTrigger.value) {
    throw new Error("Local host only supports manual_event and cron in this slice.");
  }

  if (!capabilityId) {
    throw new Error("No governed capability is available for automation creation.");
  }

  const created = await hub.createAutomation({
    workspace_id: workspaceId,
    project_id: projectId,
    title: automationDraft.title,
    instruction: automationDraft.instruction,
    action: {
      kind: "emit_text",
      content: automationDraft.actionContent
    },
    capability_id: capabilityId,
    estimated_cost: automationDraft.estimatedCost,
    trigger: buildCreateTriggerInput()
  });

  await openAutomation(created.automation.id);
}

async function handleCreateAutomation(): Promise<void> {
  try {
    await createAutomation();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

watch(
  () => [route.params.workspaceId, route.params.projectId],
  () => {
    void loadTaskSurface();
  }
);

watch(
  () => taskDraft.estimatedCost,
  (estimatedCost) => {
    const workspaceId = String(route.params.workspaceId);
    const projectId = String(route.params.projectId);
    if (!workspaceId || !projectId) {
      return;
    }

    void hub.loadTaskCapabilityResolutions(workspaceId, projectId, estimatedCost);
  }
);

watch(
  () => automationDraft.estimatedCost,
  (estimatedCost) => {
    const workspaceId = String(route.params.workspaceId);
    const projectId = String(route.params.projectId);
    if (!workspaceId || !projectId) {
      return;
    }

    void hub.loadAutomationCapabilityResolutions(workspaceId, projectId, estimatedCost);
  }
);

watch(
  () => localMode.value,
  (isLocalMode) => {
    if (
      isLocalMode &&
      (automationDraft.triggerType === "webhook" ||
        automationDraft.triggerType === "mcp_event")
    ) {
      automationDraft.triggerType = "manual_event";
    }
  }
);

onMounted(() => {
  void loadTaskSurface();
});
</script>

<template>
  <PageContainer>
    <PageHeader
      eyebrow="Task Workbench"
      :title="hub.workspaceName"
      :subtitle="hub.projectName"
    >
      <template #stats>
        <OStatPill label="Workspace" :value="hub.projectContext?.workspace.id ?? '...'" />
        <OStatPill label="Project" :value="hub.projectContext?.project.id ?? '...'" />
        <OStatPill label="Auth" :value="hub.authState" highlight />
      </template>
    </PageHeader>

    <PageGrid cols="1-side">
      <template #main>
        <OCard>
          <OCardHeader 
            title="New Task" 
            subtitle="Manual task entry for direct execution"
          >
            <template #icon><PlusCircle :size="20" /></template>
          </OCardHeader>
          
          <OCardContent>
            <div class="form-grid">
              <div class="field">
                <label>Title</label>
                <input v-model="taskDraft.title" type="text" placeholder="Enter task title..." />
              </div>
              <div class="field">
                <label>Instruction</label>
                <textarea v-model="taskDraft.instruction" rows="2" placeholder="What should the agent do?" />
              </div>
              <div class="field">
                <label>Action Content</label>
                <textarea v-model="taskDraft.actionContent" rows="4" placeholder="Payload or detailed content..." />
              </div>
              <div class="field-row">
                <div class="field">
                  <label>Capability</label>
                  <select v-model="taskDraft.capabilityId">
                    <option v-for="cap in projectCapabilities" :key="cap.id" :value="cap.id">
                      {{ cap.slug }}
                    </option>
                  </select>
                </div>
                <div class="field">
                  <label>Est. Cost</label>
                  <input v-model.number="taskDraft.estimatedCost" min="1" type="number" />
                </div>
              </div>
            </div>
          </OCardContent>
          
          <OCardFooter border>
            <OButton
              data-testid="create-start"
              :disabled="hub.taskSubmitting || hub.workspaceLoading || hub.readOnlyMode"
              :loading="hub.taskSubmitting"
              @click="handleCreateAndStart"
            >
              {{ hub.readOnlyMode ? "Read-only" : "Create & Start" }}
            </OButton>
          </OCardFooter>
        </OCard>

        <OCard>
          <OCardHeader 
            title="New Automation" 
            subtitle="Configure recurring or event-driven runs"
          >
            <template #icon><Settings :size="20" /></template>
          </OCardHeader>

          <OCardContent>
            <div class="form-grid">
              <div class="field">
                <label>Title</label>
                <input v-model="automationDraft.title" type="text" />
              </div>
              <div class="field">
                <label>Instruction</label>
                <textarea v-model="automationDraft.instruction" rows="2" />
              </div>
              <div class="field-row">
                <div class="field">
                  <label>Trigger Type</label>
                  <select v-model="automationDraft.triggerType">
                    <option
                      v-for="option in automationTriggerOptions"
                      :key="option.value"
                      :value="option.value"
                      :disabled="option.disabled"
                    >
                      {{ option.value }}
                    </option>
                  </select>
                </div>
                <div class="field">
                  <label>Capability</label>
                  <select v-model="automationDraft.capabilityId">
                    <option v-for="cap in projectCapabilities" :key="cap.id" :value="cap.id">
                      {{ cap.slug }}
                    </option>
                  </select>
                </div>
              </div>

              <div v-if="localMode && (automationDraft.triggerType === 'webhook' || automationDraft.triggerType === 'mcp_event')" class="alert warning">
                <Info :size="16" />
                <span>Local host only supports manual and cron. Webhook/MCP require remote ingress.</span>
              </div>

              <div v-if="automationDraft.triggerType === 'cron'" class="form-section">
                <div class="field">
                  <label>Schedule (Cron)</label>
                  <input v-model="automationDraft.cronSchedule" type="text" placeholder="* * * * * *" />
                </div>
                <div class="field-row">
                  <div class="field">
                    <label>Timezone</label>
                    <input v-model="automationDraft.cronTimezone" type="text" />
                  </div>
                  <div class="field">
                    <label>Next Fire</label>
                    <input v-model="automationDraft.cronNextFireAt" type="text" />
                  </div>
                </div>
              </div>

              <div v-else-if="automationDraft.triggerType === 'webhook'" class="form-section">
                <div class="field">
                  <label>Secret Header Name</label>
                  <input v-model="automationDraft.webhookSecretHeaderName" type="text" />
                </div>
                <div class="field">
                  <label>Secret Hint</label>
                  <input v-model="automationDraft.webhookSecretHint" type="text" />
                </div>
              </div>
            </div>
          </OCardContent>

          <OCardFooter border>
            <OButton
              data-testid="automation-create"
              :disabled="hub.automationSubmitting || hub.workspaceLoading || hub.readOnlyMode || unsupportedLocalTrigger"
              :loading="hub.automationSubmitting"
              @click="handleCreateAutomation"
            >
              Create Automation
            </OButton>
          </OCardFooter>
        </OCard>
      </template>

      <template #side>
        <OCard padding>
          <h3 class="side-title">Project Capabilities</h3>
          <ul v-if="projectCapabilities.length > 0" class="cap-list">
            <li v-for="capability in projectCapabilities" :key="capability.id" class="cap-item">
              <div class="cap-info">
                <span class="cap-name">{{ capability.slug }}</span>
                <div class="cap-pills">
                  <OPill size="sm" :variant="capability.trust_level === 'trusted' ? 'success' : 'default'">
                    {{ capability.trust_level }}
                  </OPill>
                  <OPill size="sm">{{ capability.source }}</OPill>
                </div>
              </div>
            </li>
          </ul>
          <p v-else class="empty-msg">No capabilities bound.</p>
        </OCard>

        <OCard padding variant="highlight">
          <h3 class="side-title"><ShieldCheck :size="16" class="inline-icon" /> Governance Status</h3>
          <div v-if="taskCapabilityResolution" class="gov-detail">
            <div class="gov-row">
              <span class="gov-label">Task Cap</span>
              <span class="gov-val">{{ taskCapabilityResolution.descriptor.slug }}</span>
            </div>
            <div class="gov-row">
              <span class="gov-label">Execution</span>
              <OPill 
                :variant="taskCapabilityResolution.execution_state === 'allowed' ? 'success' : 'danger'"
              >
                {{ taskCapabilityResolution.execution_state }}
              </OPill>
            </div>
            <p class="gov-expl">{{ taskCapabilityResolution.explanation }}</p>
          </div>
          <div class="divider"></div>
          <div v-if="automationCapabilityResolution" class="gov-detail">
            <div class="gov-row">
              <span class="gov-label">Auto Cap</span>
              <span class="gov-val">{{ automationCapabilityResolution.descriptor.slug }}</span>
            </div>
            <div class="gov-row">
              <span class="gov-label">Execution</span>
              <OPill 
                :variant="automationCapabilityResolution.execution_state === 'allowed' ? 'success' : 'danger'"
              >
                {{ automationCapabilityResolution.execution_state }}
              </OPill>
            </div>
          </div>
        </OCard>

        <OCard padding>
          <h3 class="side-title"><History :size="16" class="inline-icon" /> Recent Automations</h3>
          <ul v-if="hub.automations.length > 0" class="mini-list">
            <li v-for="automation in hub.automations" :key="automation.automation.id">
              <button class="mini-item" @click="openAutomation(automation.automation.id)">
                <span class="mini-icon">⟳</span>
                <span class="mini-text">{{ automation.automation.title }}</span>
                <span class="mini-tag">{{ automation.automation.status }}</span>
                <ChevronRight :size="14" class="mini-arrow" />
              </button>
            </li>
          </ul>
          <p v-else class="empty-msg">No automations yet.</p>
        </OCard>
      </template>
    </PageGrid>
  </PageContainer>
</template>

<style scoped>
.form-grid {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.field-row {
  display: grid;
  grid-template-columns: 1fr 120px;
  gap: 1rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field label {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text-muted);
}

input, textarea, select {
  padding: 0.75rem 0.875rem;
  background-color: var(--bg-app);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  font-size: 0.9375rem;
  transition: var(--transition);
  width: 100%;
}

input:focus, textarea:focus, select:focus {
  border-color: var(--color-accent);
  background-color: white;
  box-shadow: 0 0 0 3px var(--color-accent-soft);
  outline: none;
}

.form-section {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding: 1rem;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
}

.side-title {
  font-size: 0.875rem;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--text-subtle);
  margin-bottom: 1rem;
  letter-spacing: 0.02em;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.cap-list, .mini-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.cap-item {
  padding: 0.75rem 0;
  border-bottom: 1px solid var(--color-border);
}

.cap-item:last-child { border: none; }

.cap-name {
  display: block;
  font-weight: 600;
  margin-bottom: 0.375rem;
}

.cap-pills {
  display: flex;
  gap: 0.5rem;
}

.gov-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.gov-label { font-size: 0.75rem; color: var(--text-muted); }
.gov-val { font-size: 0.8125rem; font-weight: 600; }
.gov-expl { font-size: 0.75rem; color: var(--text-muted); line-height: 1.4; margin-top: 0.5rem; }

.divider { height: 1px; background-color: var(--color-border); margin: 1rem 0; }

.mini-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.625rem;
  border-radius: var(--radius-lg);
  text-align: left;
  transition: var(--transition);
}

.mini-item:hover { background-color: var(--bg-app); }

.mini-icon { color: var(--color-accent); font-weight: 700; }
.mini-text { flex: 1; font-size: 0.875rem; font-weight: 500; }
.mini-tag { font-size: 0.625rem; background: var(--color-border); padding: 0.125rem 0.375rem; border-radius: 4px; }
.mini-arrow { opacity: 0; transform: translateX(-5px); transition: var(--transition); }
.mini-item:hover .mini-arrow { opacity: 1; transform: translateX(0); }

.alert {
  padding: 0.75rem;
  border-radius: var(--radius-lg);
  font-size: 0.8125rem;
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
}

.alert.warning {
  background-color: var(--color-warning-soft);
  color: #92400e;
  border: 1px solid rgba(245, 158, 11, 0.2);
}

.empty-msg { font-size: 0.8125rem; color: var(--text-subtle); font-style: italic; }

.inline-icon { vertical-align: middle; margin-top: -2px; }
</style>
