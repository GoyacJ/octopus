<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { RouterLink, useRoute } from "vue-router";
import { Mail, BookOpen, ExternalLink, Search } from "lucide-vue-next";

import { useHubStore } from "../stores/hub";

// UI Components
import OButton from "../components/ui/OButton.vue";
import OPill from "../components/ui/OPill.vue";
import OCard from "../components/ui/OCard.vue";
import OStatPill from "../components/ui/OStatPill.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";

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
  <PageContainer>
    <PageHeader
      eyebrow="Project Knowledge"
      :title="projectKnowledgeIndex?.knowledge_space.display_name ?? 'Knowledge Space'"
      subtitle="Shared knowledge with source and governance traceability."
    >
      <template #stats>
        <OStatPill label="Entries" :value="projectKnowledgeIndex?.entries.length ?? 0" />
        <OStatPill label="Space ID" :value="projectKnowledgeIndex?.knowledge_space.id ?? '...'" />
      </template>
      <template #actions>
        <OButton variant="secondary" :to="inboxRoute">
          <template #icon-left><Mail :size="16" /></template>
          Open Inbox
        </OButton>
      </template>
    </PageHeader>

    <div class="knowledge-content">
      <div v-if="(projectKnowledgeIndex?.entries.length ?? 0) > 0" class="knowledge-list">
        <OCard
          v-for="entry in projectKnowledgeIndex?.entries ?? []"
          :key="`${entry.kind}:${entry.id}`"
          hover
        >
          <div class="card-inner">
            <div class="card-header-row">
              <div class="item-id-group">
                <h2 class="item-id">{{ entry.id }}</h2>
                <div class="item-pills">
                  <OPill :variant="entry.kind === 'candidate' ? 'warning' : 'success'">
                    {{ entry.kind }}
                  </OPill>
                  <OPill>{{ entry.status }}</OPill>
                  <OPill variant="info" outline>Trust: {{ entry.trust_level }}</OPill>
                </div>
              </div>

              <OButton
                v-if="sourceRunIdForEntry(entry)"
                size="sm"
                variant="secondary"
                :to="`/runs/${sourceRunIdForEntry(entry)}`"
              >
                <template #icon-left><ExternalLink :size="14" /></template>
                View Source
              </OButton>
            </div>

            <div class="card-details">
              <div class="detail-group">
                <span class="detail-label">Capability</span>
                <span class="detail-value">{{ entry.capability_id }}</span>
              </div>
              <div class="detail-group">
                <span class="detail-label">Provenance</span>
                <span class="detail-value">
                  {{ entry.kind === "candidate" ? entry.provenance_source : entry.provenance_source ?? "derived" }}
                </span>
              </div>
              <div class="detail-group">
                <span class="detail-label">Created</span>
                <span class="detail-value">{{ entry.created_at }}</span>
              </div>
            </div>

            <div v-if="entry.source_artifact_id || entry.source_candidate_id" class="card-footer-meta">
              <span v-if="entry.source_artifact_id">Artifact: {{ entry.source_artifact_id }}</span>
              <span v-if="entry.source_candidate_id">Candidate: {{ entry.source_candidate_id }}</span>
            </div>
          </div>
        </OCard>
      </div>

      <div v-else class="empty-state">
        <div class="empty-icon"><BookOpen :size="32" /></div>
        <h2 class="empty-title">No Knowledge Found</h2>
        <p class="empty-text">Shared knowledge entries for this project will appear here.</p>
      </div>
    </div>
  </PageContainer>
</template>

<style scoped>
.knowledge-list {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.card-inner {
  padding: 1.5rem;
}

.card-header-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1.5rem;
}

.item-id {
  font-size: 1.125rem;
  font-weight: 700;
  font-family: monospace;
  margin: 0 0 0.5rem;
  color: var(--text-primary);
}

.item-pills {
  display: flex;
  gap: 0.5rem;
}

.card-details {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1.25rem;
  padding: 1rem;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
}

.detail-group {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.detail-label {
  font-size: 0.625rem;
  font-weight: 700;
  color: var(--text-subtle);
  text-transform: uppercase;
}

.detail-value {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--text-primary);
  word-break: break-all;
}

.card-footer-meta {
  margin-top: 1rem;
  display: flex;
  gap: 1.5rem;
  font-size: 0.75rem;
  color: var(--text-subtle);
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
