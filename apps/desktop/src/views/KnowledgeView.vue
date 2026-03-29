<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { RouterLink, useRoute } from "vue-router";

import { useHubStore } from "../stores/hub";

const route = useRoute();
const hub = useHubStore();

const projectKnowledgeIndex = computed(() => hub.projectKnowledgeIndex);
const candidateRunMap = computed(() => {
  const entries = projectKnowledgeIndex.value?.entries ?? [];
  return new Map(
    entries
      .filter((entry) => entry.kind === "candidate")
      .map((entry) => [entry.id, entry.source_run_id])
  );
});

const inboxRoute = computed(() => {
  const workspaceId = String(route.params.workspaceId);
  return `/workspaces/${workspaceId}/inbox`;
});

function sourceRunIdForEntry(
  entry: NonNullable<typeof projectKnowledgeIndex.value>["entries"][number]
): string | null {
  if (entry.kind === "candidate") {
    return entry.source_run_id;
  }

  return candidateRunMap.value.get(entry.source_candidate_id) ?? null;
}

async function loadKnowledgeSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  const projectId = String(route.params.projectId);

  await Promise.all([
    hub.loadProjectContext(workspaceId, projectId),
    hub.loadProjectKnowledge(workspaceId, projectId)
  ]);
}

watch(
  () => [route.params.workspaceId, route.params.projectId],
  () => {
    void loadKnowledgeSurface();
  }
);

onMounted(() => {
  void loadKnowledgeSurface();
});
</script>

<template>
  <section class="knowledge-layout">
    <article class="surface-card hero">
      <p class="eyebrow">Project Knowledge</p>
      <h1>
        {{
          projectKnowledgeIndex?.knowledge_space.display_name ??
          "Project knowledge loading"
        }}
      </h1>
      <p class="muted">
        Read-only project-scoped shared knowledge with source and governance traceability.
      </p>
      <div class="meta-list">
        <span>Entries: {{ projectKnowledgeIndex?.entries.length ?? 0 }}</span>
        <span>Workspace: {{ hub.workspaceName }}</span>
        <span>Project: {{ hub.projectName }}</span>
      </div>
    </article>

    <article class="surface-card">
      <div class="header-row">
        <div>
          <p class="eyebrow">Knowledge Space</p>
          <h2>{{ projectKnowledgeIndex?.knowledge_space.id ?? "loading" }}</h2>
        </div>
        <RouterLink
          :to="inboxRoute"
          data-testid="knowledge-open-inbox"
          class="ghost-link"
        >
          Open Inbox
        </RouterLink>
      </div>
      <p class="muted">
        Promotion requests stay authoritative in Run Detail, and approval resolution stays
        authoritative in Inbox.
      </p>
    </article>

    <article class="surface-card">
      <div class="header-row">
        <div>
          <p class="eyebrow">Visible Entries</p>
          <h2>{{ projectKnowledgeIndex?.entries.length ?? 0 }} mixed records</h2>
        </div>
      </div>

      <ul v-if="(projectKnowledgeIndex?.entries.length ?? 0) > 0" class="stack-list">
        <li
          v-for="entry in projectKnowledgeIndex?.entries ?? []"
          :key="`${entry.kind}:${entry.id}`"
          class="knowledge-card"
        >
          <div class="header-row">
            <div>
              <strong>{{ entry.id }}</strong>
              <div class="meta-list">
                <span>Kind: {{ entry.kind }}</span>
                <span>Status: {{ entry.status }}</span>
                <span>Trust: {{ entry.trust_level }}</span>
              </div>
            </div>

            <RouterLink
              v-if="sourceRunIdForEntry(entry)"
              :to="`/runs/${sourceRunIdForEntry(entry)}`"
              :data-testid="`knowledge-open-run-${sourceRunIdForEntry(entry)}`"
              class="ghost-link"
            >
              Open Run
            </RouterLink>
          </div>

          <div class="meta-list">
            <span>Capability: {{ entry.capability_id }}</span>
            <span>Created: {{ entry.created_at }}</span>
            <span>Knowledge space: {{ entry.knowledge_space_id }}</span>
          </div>

          <div class="meta-list">
            <span>
              Provenance:
              {{
                entry.kind === "candidate"
                  ? entry.provenance_source
                  : entry.provenance_source ?? "derived_from_candidate"
              }}
            </span>
            <span v-if="sourceRunIdForEntry(entry)">
              Source run: {{ sourceRunIdForEntry(entry) }}
            </span>
            <span v-if="entry.source_artifact_id">
              Source artifact: {{ entry.source_artifact_id }}
            </span>
            <span v-if="entry.source_candidate_id">
              Source candidate: {{ entry.source_candidate_id }}
            </span>
          </div>
        </li>
      </ul>

      <p v-else class="muted">
        No shared knowledge entries are visible for this project yet.
      </p>
    </article>
  </section>
</template>

<style scoped>
.knowledge-layout {
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
    radial-gradient(circle at top right, rgba(14, 165, 233, 0.18), transparent 32%),
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

.header-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.stack-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.knowledge-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 0.95rem;
  border-radius: 0.9rem;
  background: rgba(2, 6, 23, 0.6);
}

.meta-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  font-size: 0.92rem;
  color: #cbd5e1;
}

.ghost-link {
  display: inline-flex;
  align-items: center;
  border: 1px solid rgba(103, 232, 249, 0.24);
  border-radius: 999px;
  padding: 0.65rem 0.9rem;
  color: #e2e8f0;
  text-decoration: none;
  background: rgba(15, 23, 42, 0.6);
}
</style>
