<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";

import { useHubStore } from "../stores/hub";

const route = useRoute();
const hub = useHubStore();

const automationDetail = computed(() => hub.automationDetail);
const triggerConfig = computed(() =>
  JSON.stringify(automationDetail.value?.trigger.config ?? {}, null, 2)
);
const manualDispatchAvailable = computed(
  () => automationDetail.value?.trigger.trigger_type === "manual_event"
);

async function loadAutomationSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);
  const automationId = String(route.params.automationId);

  await hub.loadWorkspace(workspaceId, projectId);
  await hub.loadAutomation(automationId);
}

async function activateAutomation(): Promise<void> {
  await hub.activateAutomation(String(route.params.automationId));
}

async function pauseAutomation(): Promise<void> {
  await hub.pauseAutomation(String(route.params.automationId));
}

async function archiveAutomation(): Promise<void> {
  await hub.archiveAutomation(String(route.params.automationId));
}

async function manualDispatch(): Promise<void> {
  const detail = automationDetail.value;
  if (!detail) {
    return;
  }

  await hub.manualDispatch({
    trigger_id: detail.trigger.id,
    dedupe_key: `desktop:${detail.automation.id}:${Date.now()}`,
    payload: {
      source: "desktop"
    }
  });
}

async function retryDelivery(deliveryId: string): Promise<void> {
  await hub.retryAutomationDelivery(deliveryId);
}

async function handleActivateAutomation(): Promise<void> {
  try {
    await activateAutomation();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handlePauseAutomation(): Promise<void> {
  try {
    await pauseAutomation();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handleArchiveAutomation(): Promise<void> {
  try {
    await archiveAutomation();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handleManualDispatch(): Promise<void> {
  try {
    await manualDispatch();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handleRetryDelivery(deliveryId: string): Promise<void> {
  try {
    await retryDelivery(deliveryId);
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

watch(
  () => [route.params.workspaceId, route.params.projectId, route.params.automationId],
  () => {
    void loadAutomationSurface();
  }
);

onMounted(() => {
  void loadAutomationSurface();
});
</script>

<template>
  <section class="detail-grid">
    <article class="surface-card hero">
      <p class="eyebrow">Automation Detail</p>
      <h1>{{ automationDetail?.automation.title ?? "Automation loading" }}</h1>
      <div class="meta-list">
        <span>ID: {{ automationDetail?.automation.id ?? "loading" }}</span>
        <span>Status: {{ automationDetail?.automation.status ?? "loading" }}</span>
        <span>Trigger: {{ automationDetail?.trigger.trigger_type ?? "loading" }}</span>
      </div>
      <p class="muted">
        {{ automationDetail?.automation.instruction ?? "Loading automation instruction..." }}
      </p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Lifecycle</p>
      <h2>Minimum state machine</h2>
      <div class="button-row">
        <button
          data-testid="automation-activate"
          :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
          @click="handleActivateAutomation"
        >
          Activate
        </button>
        <button
          data-testid="automation-pause"
          :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
          @click="handlePauseAutomation"
        >
          Pause
        </button>
        <button
          data-testid="automation-archive"
          :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
          @click="handleArchiveAutomation"
        >
          Archive
        </button>
      </div>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Trigger Config</p>
      <h2>{{ automationDetail?.trigger.trigger_type ?? "trigger loading" }}</h2>
      <pre class="code-block">{{ triggerConfig }}</pre>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Manual Dispatch</p>
      <h2>{{ manualDispatchAvailable ? "manual_event" : "non-manual trigger" }}</h2>
      <p class="muted">
        {{
          manualDispatchAvailable
            ? "Use the runtime-backed dispatch flow for on-demand execution."
            : "Only manual_event triggers expose dispatch in this minimum surface."
        }}
      </p>
      <button
        v-if="manualDispatchAvailable"
        data-testid="manual-dispatch"
        :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
        @click="handleManualDispatch"
      >
        Dispatch Now
      </button>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Recent Deliveries</p>
      <h2>{{ automationDetail?.recent_deliveries.length ?? 0 }} recent records</h2>
      <div v-if="(automationDetail?.recent_deliveries.length ?? 0) > 0" class="stack-list">
        <div
          v-for="delivery in automationDetail?.recent_deliveries ?? []"
          :key="delivery.id"
          class="delivery-card"
        >
          <strong>{{ delivery.id }}</strong>
          <div class="meta-list">
            <span>Status: {{ delivery.status }}</span>
            <span>Attempts: {{ delivery.attempt_count }}</span>
          </div>
          <p v-if="delivery.last_error" class="muted">{{ delivery.last_error }}</p>
          <button
            :data-testid="`retry-delivery-${delivery.id}`"
            :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
            @click="handleRetryDelivery(delivery.id)"
          >
            Retry Delivery
          </button>
        </div>
      </div>
      <p v-else class="muted">No delivery has been recorded for this automation yet.</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Last Run</p>
      <h2>{{ automationDetail?.last_run_summary?.status ?? "No run yet" }}</h2>
      <div v-if="automationDetail?.last_run_summary" class="meta-list">
        <span>Run: {{ automationDetail.last_run_summary.id }}</span>
        <span>Type: {{ automationDetail.last_run_summary.run_type }}</span>
        <span>Title: {{ automationDetail.last_run_summary.title }}</span>
      </div>
      <p v-else class="muted">The automation has not produced a run summary yet.</p>
    </article>
  </section>
</template>

<style scoped>
.detail-grid {
  display: grid;
  gap: 1rem;
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

.hero {
  background:
    radial-gradient(circle at top right, rgba(250, 204, 21, 0.16), transparent 32%),
    rgba(15, 23, 42, 0.56);
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
p,
pre {
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

.button-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
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
}

.delivery-card {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  padding: 0.9rem;
  border-radius: 0.9rem;
  background: rgba(15, 23, 42, 0.55);
}

.code-block {
  overflow: auto;
  border-radius: 0.9rem;
  padding: 0.9rem;
  color: #e2e8f0;
  background: rgba(2, 6, 23, 0.8);
}
</style>
