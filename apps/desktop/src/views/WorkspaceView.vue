<script setup lang="ts">
import { computed, onMounted, reactive, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import { useHubStore } from "../stores/hub";

const route = useRoute();
const router = useRouter();
const hub = useHubStore();

const taskDraft = reactive({
  title: "Write note",
  instruction: "Emit a deterministic artifact",
  actionContent: "hello",
  capabilityId: "",
  estimatedCost: 1
});

const visibleCapability = computed(() => hub.activeCapability);

async function loadWorkspaceSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);

  await hub.loadWorkspace(workspaceId, projectId);

  if (!taskDraft.capabilityId && visibleCapability.value) {
    taskDraft.capabilityId = visibleCapability.value.id;
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
        @click="createAndStart"
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
textarea {
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

.stack-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  margin: 0;
  padding-left: 1rem;
}
</style>
