<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { PlayCircle, ChevronRight, Clock, Hash, AlertTriangle } from "lucide-vue-next";

import { useHubStore } from "../stores/hub";

// UI Components
import OPill from "../components/ui/OPill.vue";
import OCard from "../components/ui/OCard.vue";
import OStatPill from "../components/ui/OStatPill.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";

const route = useRoute();
const router = useRouter();
const hub = useHubStore();

async function loadRunsSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);

  await Promise.all([
    hub.loadProjectContext(workspaceId, projectId),
    hub.loadRuns(workspaceId, projectId)
  ]);
}

async function openRun(runId: string): Promise<void> {
  await router.push(`/runs/${runId}`);
}

watch(
  () => [route.params.workspaceId, route.params.projectId],
  () => {
    void loadRunsSurface();
  }
);

onMounted(() => {
  void loadRunsSurface();
});
</script>

<template>
  <PageContainer>
    <PageHeader
      eyebrow="Execution History"
      :title="hub.projectName || 'All Runs'"
      subtitle="Project-scoped execution history ordered by latest activity."
    >
      <template #stats>
        <OStatPill label="Total Runs" :value="hub.runs.length" />
      </template>
    </PageHeader>

    <div class="runs-content">
      <div v-if="hub.runs.length > 0" class="runs-list">
        <OCard
          v-for="run in hub.runs"
          :key="run.id"
          hover
          @click="openRun(run.id)"
        >
          <div class="run-card-inner">
            <div class="run-main">
              <div class="run-header">
                <h2 class="run-title">{{ run.title }}</h2>
                <div class="run-pills">
                  <OPill :variant="run.status === 'completed' ? 'success' : run.status === 'running' ? 'info' : 'danger'">
                    {{ run.status }}
                  </OPill>
                  <OPill outline>{{ run.run_type }}</OPill>
                </div>
              </div>
              
              <div class="run-meta">
                <span class="meta-item">
                  <Hash :size="12" />
                  {{ run.id.slice(0, 8) }}
                </span>
                <span class="meta-item">
                  <Clock :size="12" />
                  {{ run.updated_at }}
                </span>
              </div>

              <div v-if="run.last_error" class="error-msg">
                <AlertTriangle :size="14" />
                <span>{{ run.last_error }}</span>
              </div>
            </div>
            <div class="run-chevron">
              <ChevronRight :size="20" />
            </div>
          </div>
        </OCard>
      </div>

      <div v-else class="empty-state">
        <div class="empty-icon"><PlayCircle :size="32" /></div>
        <h2 class="empty-title">No Runs Yet</h2>
        <p class="empty-text">When you start a task or automation, it will appear here.</p>
      </div>
    </div>
  </PageContainer>
</template>

<style scoped>
.runs-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.run-card-inner {
  display: flex;
  align-items: center;
  gap: 1.5rem;
  padding: 1.25rem 1.5rem;
  cursor: pointer;
}

.run-main {
  flex: 1;
}

.run-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.run-title {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.run-pills {
  display: flex;
  gap: 0.5rem;
}

.run-meta {
  display: flex;
  gap: 1.5rem;
  font-size: 0.8125rem;
  color: var(--text-subtle);
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

.error-msg {
  margin-top: 0.75rem;
  padding: 0.5rem 0.75rem;
  background-color: var(--color-danger-soft);
  color: var(--color-danger);
  font-size: 0.8125rem;
  border-radius: var(--radius-lg);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.run-chevron {
  color: var(--text-subtle);
  opacity: 0;
  transform: translateX(-10px);
  transition: var(--transition);
}

.o-card:hover .run-chevron {
  opacity: 1;
  color: var(--color-accent);
  transform: translateX(0);
}

.empty-state {
  text-align: center;
  padding: 4rem 2rem;
  background-color: var(--bg-surface);
  border: 2px dashed var(--color-border);
  border-radius: var(--radius-2xl);
}

.empty-icon {
  width: 3.5rem;
  height: 3.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 1.5rem;
  background-color: var(--bg-app);
  color: var(--text-subtle);
  border-radius: 50%;
}

.empty-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
}

.empty-text {
  color: var(--text-muted);
  font-size: 0.9375rem;
}
</style>
