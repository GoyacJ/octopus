<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import { useHubStore } from "../stores/hub";

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
  <section class="runs-layout">
    <article class="surface-card hero">
      <p class="eyebrow">Recent Runs</p>
      <h1>{{ hub.projectName }}</h1>
      <p class="muted">Project-scoped execution history ordered by latest activity.</p>
    </article>

    <article class="surface-card">
      <div class="header-row">
        <div>
          <p class="eyebrow">Runs</p>
          <h2>{{ hub.runs.length }} recent records</h2>
        </div>
      </div>

      <ul v-if="hub.runs.length > 0" class="stack-list">
        <li v-for="run in hub.runs" :key="run.id" class="run-card">
          <button class="run-link" @click="openRun(run.id)">{{ run.title }}</button>
          <div class="meta-list">
            <span>Status: {{ run.status }}</span>
            <span>Type: {{ run.run_type }}</span>
            <span>Updated: {{ run.updated_at }}</span>
          </div>
          <p class="muted">Run ID: {{ run.id }}</p>
          <p v-if="run.last_error" class="error-copy">{{ run.last_error }}</p>
        </li>
      </ul>

      <p v-else class="muted">No runs have been recorded for this project yet.</p>
    </article>
  </section>
</template>

<style scoped>
.runs-layout {
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
    radial-gradient(circle at top right, rgba(34, 197, 94, 0.18), transparent 32%),
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
p {
  margin: 0;
}

.muted {
  color: #94a3b8;
}

.stack-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.run-card {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  padding: 0.95rem;
  border-radius: 0.9rem;
  background: rgba(2, 6, 23, 0.6);
}

.run-link {
  align-self: flex-start;
  border: none;
  border-radius: 999px;
  padding: 0.7rem 0.95rem;
  font: inherit;
  font-weight: 600;
  color: #082f49;
  background: linear-gradient(135deg, #67e8f9, #facc15);
  cursor: pointer;
}

.meta-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  font-size: 0.92rem;
  color: #cbd5e1;
}

.error-copy {
  color: #fecaca;
}
</style>
