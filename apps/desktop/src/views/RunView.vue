<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";

import { useHubStore } from "../stores/hub";

const route = useRoute();
const hub = useHubStore();

const runDetail = computed(() => hub.runDetail);
const artifacts = computed(() => hub.artifacts);
const knowledgeDetail = computed(() => hub.knowledgeDetail);

async function loadRunSurface(): Promise<void> {
  await hub.loadRun(String(route.params.runId));
}

watch(
  () => route.params.runId,
  () => {
    void loadRunSurface();
  }
);

onMounted(() => {
  void loadRunSurface();
});
</script>

<template>
  <section class="run-layout">
    <article class="surface-card hero">
      <p class="eyebrow">Run Detail</p>
      <h1>{{ runDetail?.task.title ?? "Run loading" }}</h1>
      <div class="meta-list">
        <span>Run: {{ runDetail?.run.id ?? "loading" }}</span>
        <span>Status: {{ runDetail?.run.status ?? "loading" }}</span>
        <span>Attempts: {{ runDetail?.run.attempt_count ?? 0 }}/{{ runDetail?.run.max_attempts ?? 0 }}</span>
      </div>
      <p class="muted">{{ runDetail?.task.instruction ?? "Loading task instruction..." }}</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Artifact View</p>
      <h2>{{ artifacts[0]?.artifact_type ?? "No artifact yet" }}</h2>
      <p class="artifact-content">{{ artifacts[0]?.content ?? "No artifact content recorded." }}</p>
      <div v-if="artifacts[0]" class="meta-list">
        <span>Trust: {{ artifacts[0].trust_level }}</span>
        <span>Knowledge gate: {{ artifacts[0].knowledge_gate_status }}</span>
        <span>Provenance: {{ artifacts[0].provenance_source }}</span>
      </div>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Shared Knowledge</p>
      <h2>{{ knowledgeDetail?.knowledge_space.display_name ?? "Knowledge loading" }}</h2>
      <div class="stack-list">
        <div v-for="candidate in knowledgeDetail?.candidates ?? []" :key="candidate.id">
          <strong>{{ candidate.id }}</strong>
          <p>{{ candidate.content }}</p>
        </div>
      </div>
      <p v-if="(knowledgeDetail?.candidates.length ?? 0) === 0" class="muted">
        No candidates have been captured for this run.
      </p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Trace Replay</p>
      <div v-if="(runDetail?.traces.length ?? 0) > 0" class="stack-list">
        <div v-for="trace in runDetail?.traces ?? []" :key="trace.id">
          <strong>{{ trace.stage }}</strong>
          <p>{{ trace.message }}</p>
        </div>
      </div>
      <p v-else class="muted">No trace replay is recorded for this run yet.</p>
    </article>

    <article class="surface-card">
      <p class="eyebrow">Approval / Notification State</p>
      <div class="meta-list">
        <span>Approvals: {{ runDetail?.approvals.length ?? 0 }}</span>
        <span>Inbox items: {{ runDetail?.inbox_items.length ?? 0 }}</span>
        <span>Notifications: {{ runDetail?.notifications.length ?? 0 }}</span>
      </div>
      <p class="muted">
        This minimum surface keeps the authoritative approval state visible from the run.
      </p>
    </article>
  </section>
</template>

<style scoped>
.run-layout {
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

.artifact-content {
  white-space: pre-wrap;
  font-size: 1.02rem;
  color: #e2e8f0;
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

.stack-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}
</style>
