<script setup lang="ts">
import { computed, onMounted, reactive, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import { useHubStore } from "../stores/hub";

const route = useRoute();
const router = useRouter();
const hub = useHubStore();
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

const visibleCapability = computed(() => hub.activeCapability);

async function loadWorkspaceSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);

  await hub.loadWorkspace(workspaceId, projectId);

  if (!taskDraft.capabilityId && visibleCapability.value) {
    taskDraft.capabilityId = visibleCapability.value.id;
  }
  if (!automationDraft.capabilityId && visibleCapability.value) {
    automationDraft.capabilityId = visibleCapability.value.id;
  }
}

async function createAndStart(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);
  const capabilityId = taskDraft.capabilityId || visibleCapability.value?.id;

  if (!capabilityId) {
    throw new Error("No visible capability is available for task creation.");
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

  if (!capabilityId) {
    throw new Error("No visible capability is available for automation creation.");
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
    void loadWorkspaceSurface();
  }
);

onMounted(() => {
  void loadWorkspaceSurface();
});
</script>

<template>
  <section class="surface-grid">
    <article class="surface-card">
      <p class="eyebrow">Workspace Context</p>
      <h1>{{ hub.workspaceName }}</h1>
      <p class="muted">{{ hub.projectName }}</p>
      <div class="meta-list">
        <span>Workspace: {{ hub.projectContext?.workspace.id ?? "loading" }}</span>
        <span>Project: {{ hub.projectContext?.project.id ?? "loading" }}</span>
      </div>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Task Create</p>
      <h2>Manual entry</h2>
      <label class="field">
        <span>Title</span>
        <input v-model="taskDraft.title" type="text" />
      </label>
      <label class="field">
        <span>Instruction</span>
        <textarea v-model="taskDraft.instruction" rows="3" />
      </label>
      <label class="field">
        <span>Action content</span>
        <textarea v-model="taskDraft.actionContent" rows="5" />
      </label>
      <label class="field">
        <span>Capability</span>
        <input v-model="taskDraft.capabilityId" type="text" />
      </label>
      <label class="field">
        <span>Estimated cost</span>
        <input v-model.number="taskDraft.estimatedCost" min="1" type="number" />
      </label>
      <button
        data-testid="create-start"
        :disabled="hub.taskSubmitting || hub.workspaceLoading || hub.readOnlyMode"
        @click="handleCreateAndStart"
      >
        {{
          hub.taskSubmitting
            ? "Starting..."
            : hub.readOnlyMode
              ? "Read-only"
              : "Create and Start"
        }}
      </button>
      <p v-if="hub.readOnlyMode" class="muted">
        Remote auth: {{ hub.authState }}
      </p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Automation Create</p>
      <h2>Minimum automation manager</h2>
      <label class="field">
        <span>Title</span>
        <input v-model="automationDraft.title" type="text" />
      </label>
      <label class="field">
        <span>Instruction</span>
        <textarea v-model="automationDraft.instruction" rows="3" />
      </label>
      <label class="field">
        <span>Action content</span>
        <textarea v-model="automationDraft.actionContent" rows="4" />
      </label>
      <label class="field">
        <span>Trigger type</span>
        <select v-model="automationDraft.triggerType">
          <option value="manual_event">manual_event</option>
          <option value="cron">cron</option>
          <option value="webhook">webhook</option>
          <option value="mcp_event">mcp_event</option>
        </select>
      </label>
      <template v-if="automationDraft.triggerType === 'cron'">
        <label class="field">
          <span>Schedule</span>
          <input v-model="automationDraft.cronSchedule" type="text" />
        </label>
        <label class="field">
          <span>Timezone</span>
          <input v-model="automationDraft.cronTimezone" type="text" />
        </label>
        <label class="field">
          <span>Next fire at</span>
          <input v-model="automationDraft.cronNextFireAt" type="text" />
        </label>
      </template>
      <template v-else-if="automationDraft.triggerType === 'webhook'">
        <label class="field">
          <span>Ingress mode</span>
          <input v-model="automationDraft.webhookIngressMode" type="text" />
        </label>
        <label class="field">
          <span>Secret header name</span>
          <input v-model="automationDraft.webhookSecretHeaderName" type="text" />
        </label>
        <label class="field">
          <span>Secret hint</span>
          <input v-model="automationDraft.webhookSecretHint" type="text" />
        </label>
        <label class="field">
          <span>Secret plaintext</span>
          <input v-model="automationDraft.webhookSecretPlaintext" type="text" />
        </label>
      </template>
      <template v-else-if="automationDraft.triggerType === 'mcp_event'">
        <label class="field">
          <span>Server id</span>
          <input v-model="automationDraft.mcpServerId" type="text" />
        </label>
        <label class="field">
          <span>Event name</span>
          <input v-model="automationDraft.mcpEventName" type="text" />
        </label>
        <label class="field">
          <span>Event pattern</span>
          <input v-model="automationDraft.mcpEventPattern" type="text" />
        </label>
      </template>
      <label class="field">
        <span>Capability</span>
        <input v-model="automationDraft.capabilityId" type="text" />
      </label>
      <label class="field">
        <span>Estimated cost</span>
        <input v-model.number="automationDraft.estimatedCost" min="1" type="number" />
      </label>
      <button
        data-testid="automation-create"
        :disabled="hub.automationSubmitting || hub.workspaceLoading || hub.readOnlyMode"
        @click="handleCreateAutomation"
      >
        {{
          hub.automationSubmitting
            ? "Creating..."
            : hub.readOnlyMode
              ? "Read-only"
              : "Create Automation"
        }}
      </button>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Hub Connections</p>
      <h2>
        {{ hub.connectionStatus?.mode ?? "unknown" }} /
        {{ hub.connectionStatus?.state ?? "unknown" }}
      </h2>
      <div class="meta-list">
        <span>Auth state: {{ hub.authState }}</span>
        <span>Active servers: {{ hub.connectionStatus?.active_server_count ?? 0 }}</span>
        <span>Healthy servers: {{ hub.connectionStatus?.healthy_server_count ?? 0 }}</span>
        <span>Last refresh: {{ hub.connectionStatus?.last_refreshed_at ?? "n/a" }}</span>
      </div>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Capability Visibility</p>
      <h2>{{ visibleCapability?.slug ?? "No capability grant" }}</h2>
      <p class="muted">
        {{ hub.capabilityVisibilities[0]?.explanation ?? "No capability explanation yet." }}
      </p>
      <div v-if="visibleCapability" class="meta-list">
        <span>Trust: {{ visibleCapability.trust_level }}</span>
        <span>Risk: {{ visibleCapability.risk_level }}</span>
        <span>Source: {{ visibleCapability.source }}</span>
      </div>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Automations</p>
      <h2>{{ hub.automations.length }} configured entries</h2>
      <ul v-if="hub.automations.length > 0" class="stack-list">
        <li v-for="automation in hub.automations" :key="automation.automation.id">
          <button class="automation-link" @click="openAutomation(automation.automation.id)">
            {{ automation.automation.title }}
          </button>
          <p class="muted">
            {{ automation.automation.status }} / {{ automation.trigger.trigger_type }}
          </p>
          <p>{{ automation.automation.instruction }}</p>
        </li>
      </ul>
      <p v-else class="muted">No automation has been configured for this project yet.</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Approval Inbox</p>
      <h2>{{ hub.inboxItems.length }} open items</h2>
      <ul v-if="hub.inboxItems.length > 0" class="stack-list">
        <li v-for="item in hub.inboxItems" :key="item.id">
          <strong>{{ item.title }}</strong>
          <p>{{ item.message }}</p>
        </li>
      </ul>
      <p v-else class="muted">No approval requests are waiting.</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Notifications</p>
      <h2>{{ hub.notifications.length }} pending signals</h2>
      <ul v-if="hub.notifications.length > 0" class="stack-list">
        <li v-for="notification in hub.notifications" :key="notification.id">
          <strong>{{ notification.title }}</strong>
          <p>{{ notification.message }}</p>
        </li>
      </ul>
      <p v-else class="muted">No notifications are pending.</p>
    </article>
  </section>
</template>

<style scoped>
.surface-grid {
  display: grid;
  gap: 1rem;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
}

.surface-card {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  padding: 1.2rem;
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 1rem;
  background: rgba(15, 23, 42, 0.45);
  box-shadow: 0 20px 40px rgba(15, 23, 42, 0.18);
}

.eyebrow {
  margin: 0;
  font-size: 0.72rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #67e8f9;
}

h1,
h2,
p {
  margin: 0;
}

.muted {
  color: #94a3b8;
}

.meta-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  font-size: 0.92rem;
  color: #cbd5e1;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  font-size: 0.92rem;
  color: #cbd5e1;
}

input,
textarea,
select {
  width: 100%;
  border: 1px solid rgba(125, 211, 252, 0.25);
  border-radius: 0.85rem;
  padding: 0.8rem 0.95rem;
  font: inherit;
  color: #e2e8f0;
  background: rgba(15, 23, 42, 0.72);
}

button {
  border: none;
  border-radius: 999px;
  padding: 0.9rem 1.1rem;
  font: inherit;
  font-weight: 600;
  color: #082f49;
  background: linear-gradient(135deg, #67e8f9, #facc15);
  cursor: pointer;
}

button:disabled {
  cursor: progress;
  opacity: 0.75;
}

.automation-link {
  padding: 0;
  color: #f8fafc;
  background: transparent;
  text-align: left;
}

.stack-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  margin: 0;
  padding-left: 1rem;
}
</style>
