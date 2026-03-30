<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { 
  Settings, 
  Play, 
  Pause, 
  Archive, 
  Zap, 
  Clock, 
  Activity, 
  RefreshCw,
  Hash
} from "lucide-vue-next";

import { useHubStore } from "../stores/hub";

// UI Components
import OButton from "../components/ui/OButton.vue";
import OPill from "../components/ui/OPill.vue";
import OCard from "../components/ui/OCard.vue";
import OCardHeader from "../components/ui/OCardHeader.vue";
import OCardContent from "../components/ui/OCardContent.vue";
import OStatPill from "../components/ui/OStatPill.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";
import PageGrid from "../components/layout/PageGrid.vue";

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

  await Promise.all([
    hub.loadProjectContext(workspaceId, projectId),
    hub.loadAutomation(automationId)
  ]);
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
    await loadAutomationSurface();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handlePauseAutomation(): Promise<void> {
  try {
    await pauseAutomation();
    await loadAutomationSurface();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handleArchiveAutomation(): Promise<void> {
  try {
    await archiveAutomation();
    await loadAutomationSurface();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handleManualDispatch(): Promise<void> {
  try {
    await manualDispatch();
    await loadAutomationSurface();
  } catch {
    // The store already exposes the error banner for the shell.
  }
}

async function handleRetryDelivery(deliveryId: string): Promise<void> {
  try {
    await retryDelivery(deliveryId);
    await loadAutomationSurface();
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
  <PageContainer>
    <PageHeader
      eyebrow="Automation Detail"
      :title="automationDetail?.automation.title ?? 'Automation'"
      :subtitle="automationDetail?.automation.instruction"
    >
      <template #stats>
        <OStatPill label="Status" highlight>
          <span class="status-dot" :class="automationDetail?.automation.status"></span>
          {{ automationDetail?.automation.status ?? "..." }}
        </OStatPill>
        <OStatPill label="Trigger" :value="automationDetail?.trigger.trigger_type ?? '...'" />
        <OStatPill label="ID" :value="automationDetail?.automation.id.slice(0, 8) ?? '...'" mono />
      </template>
      <template #actions>
        <OButton
          v-if="manualDispatchAvailable"
          data-testid="manual-dispatch"
          variant="primary"
          :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
          @click="handleManualDispatch"
        >
          <template #icon-left><Zap :size="16" /></template>
          Dispatch Now
        </OButton>
      </template>
    </PageHeader>

    <PageGrid cols="1-side">
      <template #main>
        <OCard>
          <OCardHeader title="Lifecycle & Actions">
            <template #icon><Settings :size="20" /></template>
          </OCardHeader>
          <OCardContent>
            <div class="lifecycle-actions">
              <button
                data-testid="automation-activate"
                class="btn-action"
                :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
                @click="handleActivateAutomation"
              >
                <Play :size="20" />
                <span>Activate</span>
              </button>
              <button
                data-testid="automation-pause"
                class="btn-action"
                :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
                @click="handlePauseAutomation"
              >
                <Pause :size="20" />
                <span>Pause</span>
              </button>
              <button
                data-testid="automation-archive"
                class="btn-action danger"
                :disabled="hub.readOnlyMode || hub.automationActionLoading || hub.automationLoading"
                @click="handleArchiveAutomation"
              >
                <Archive :size="20" />
                <span>Archive</span>
              </button>
            </div>
          </OCardContent>
        </OCard>

        <OCard>
          <OCardHeader title="Trigger Configuration">
            <template #icon><Clock :size="20" /></template>
          </OCardHeader>
          <OCardContent>
            <pre class="code-block">{{ triggerConfig }}</pre>
          </OCardContent>
        </OCard>

        <OCard>
          <OCardHeader title="Recent Deliveries">
            <template #icon><Activity :size="20" /></template>
          </OCardHeader>
          <OCardContent>
            <div v-if="automationDetail?.recent_deliveries.length" class="delivery-list">
              <div v-for="del in automationDetail.recent_deliveries" :key="del.id" class="delivery-item">
                <div class="delivery-main">
                  <span class="delivery-id"><Hash :size="12" /> {{ del.id }}</span>
                  <OPill size="sm" :variant="del.status === 'succeeded' ? 'success' : 'danger'">
                    {{ del.status }}
                  </OPill>
                </div>
                <div class="delivery-meta">
                  <span>Attempts: {{ del.attempt_count }}</span>
                  <span v-if="del.last_error" class="error-text">• {{ del.last_error }}</span>
                </div>
                <div class="delivery-actions">
                  <OButton
                    :data-testid="`retry-delivery-${del.id}`"
                    size="sm"
                    variant="secondary"
                    :disabled="hub.readOnlyMode || hub.automationActionLoading"
                    @click="handleRetryDelivery(del.id)"
                  >
                    <template #icon-left><RefreshCw :size="12" /></template>
                    Retry
                  </OButton>
                </div>
              </div>
            </div>
            <p v-else class="empty-msg">No deliveries recorded yet.</p>
          </OCardContent>
        </OCard>
      </template>

      <template #side>
        <OCard padding variant="highlight">
          <h3 class="side-title"><Zap :size="16" class="inline-icon" /> Last Execution</h3>
          <div v-if="automationDetail?.last_run_summary" class="run-summary">
            <div class="summary-row">
              <span class="label">Status</span>
              <OPill :variant="automationDetail.last_run_summary.status === 'completed' ? 'success' : 'info'">
                {{ automationDetail.last_run_summary.status }}
              </OPill>
            </div>
            <div class="summary-row">
              <span class="label">Run ID</span>
              <span class="val mono">{{ automationDetail.last_run_summary.id.slice(0, 8) }}</span>
            </div>
            <div class="summary-row">
              <span class="label">Type</span>
              <span class="val">{{ automationDetail.last_run_summary.run_type }}</span>
            </div>
            <div class="summary-title">{{ automationDetail.last_run_summary.title }}</div>
          </div>
          <p v-else class="empty-msg">No run summary available.</p>
        </OCard>
      </template>
    </PageGrid>
  </PageContainer>
</template>

<style scoped>
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--text-subtle);
}
.status-dot.active { background-color: var(--color-success); }
.status-dot.paused { background-color: var(--color-warning); }

.lifecycle-actions {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 0.75rem;
}

.btn-action {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-primary);
  transition: var(--transition);
  border: 1px solid transparent;
}

.btn-action:hover:not(:disabled) {
  background-color: var(--color-accent-soft);
  color: var(--color-accent);
  border-color: var(--color-accent-soft);
}

.btn-action.danger:hover:not(:disabled) {
  background-color: var(--color-danger-soft);
  color: var(--color-danger);
  border-color: var(--color-danger-soft);
}

.btn-action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.code-block {
  background-color: #1e293b;
  color: #e2e8f0;
  padding: 1rem;
  border-radius: var(--radius-lg);
  font-family: monospace;
  font-size: 0.8125rem;
  overflow-x: auto;
  margin: 0;
}

.delivery-list { display: flex; flex-direction: column; gap: 0.75rem; }

.delivery-item {
  padding: 1rem;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
}

.delivery-main { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
.delivery-id { font-family: monospace; font-size: 0.8125rem; font-weight: 600; display: flex; align-items: center; gap: 0.375rem; color: var(--text-primary); }

.delivery-meta { font-size: 0.75rem; color: var(--text-subtle); margin-bottom: 0.75rem; }
.error-text { color: var(--color-danger); }

.summary-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem; }
.summary-row .label { font-size: 0.75rem; color: var(--text-muted); }
.summary-row .val { font-size: 0.8125rem; font-weight: 600; }
.summary-title { font-size: 0.8125rem; color: var(--text-primary); margin-top: 0.5rem; line-height: 1.4; font-weight: 600; }

.side-title {
  font-size: 0.875rem;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--text-subtle);
  margin-bottom: 1rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.empty-msg { font-size: 0.8125rem; color: var(--text-subtle); font-style: italic; }

.inline-icon { vertical-align: middle; margin-top: -2px; }
</style>
