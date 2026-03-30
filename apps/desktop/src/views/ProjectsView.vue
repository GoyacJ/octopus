<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Layers, ChevronRight, LayoutGrid } from "lucide-vue-next";

import {
  buildProjectTasksRoute,
  useConnectionStore
} from "../stores/connection";
import { useHubStore } from "../stores/hub";

// UI Components
import OCard from "../components/ui/OCard.vue";
import OStatPill from "../components/ui/OStatPill.vue";
import PageHeader from "../components/layout/PageHeader.vue";
import PageContainer from "../components/layout/PageContainer.vue";

const route = useRoute();
const router = useRouter();
const hub = useHubStore();
const connection = useConnectionStore();

async function loadProjectsSurface(): Promise<void> {
  const workspaceId = String(route.params.workspaceId);

  try {
    await hub.loadProjects(workspaceId);
  } catch {
    // The shell already surfaces the shared error state.
  }
}

async function openProject(projectId: string): Promise<void> {
  const workspaceId = String(route.params.workspaceId);
  connection.rememberProject(projectId);
  await router.push(buildProjectTasksRoute(workspaceId, projectId));
}

watch(
  () => route.params.workspaceId,
  () => {
    void loadProjectsSurface();
  }
);

onMounted(() => {
  void loadProjectsSurface();
});
</script>

<template>
  <PageContainer narrow>
    <PageHeader
      eyebrow="Workspace"
      :title="hub.currentWorkspaceId ?? route.params.workspaceId"
      subtitle="Select a project to enter the workbench."
    >
      <template #stats>
        <OStatPill label="Projects" :value="hub.projects.length" highlight />
      </template>
    </PageHeader>

    <div class="projects-content">
      <div v-if="hub.projects.length > 0" class="projects-grid">
        <OCard
          v-for="project in hub.projects"
          :key="project.id"
          hover
          @click="openProject(project.id)"
        >
          <div class="project-card-inner">
            <div class="project-icon-box">
              <LayoutGrid :size="20" />
            </div>
            <div class="project-info">
              <h2 class="project-name">{{ project.display_name }}</h2>
              <div class="project-meta">
                <span class="meta-item">ID: {{ project.id }}</span>
                <span class="meta-divider"></span>
                <span class="meta-item">Updated: {{ project.updated_at }}</span>
              </div>
            </div>
            <div class="project-action">
              <span class="action-text">Enter</span>
              <ChevronRight :size="18" />
            </div>
          </div>
        </OCard>
      </div>

      <div v-else class="empty-state">
        <div class="empty-icon"><Layers :size="32" /></div>
        <h2 class="empty-title">No Projects Yet</h2>
        <p class="empty-text">There are no tracked projects available in this workspace.</p>
      </div>
    </div>
  </PageContainer>
</template>

<style scoped>
.projects-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}

.project-card-inner {
  display: flex;
  align-items: center;
  gap: 1.25rem;
  padding: 1.25rem 1.5rem;
  cursor: pointer;
}

.project-icon-box {
  width: 2.5rem;
  height: 2.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
  color: var(--color-accent);
  transition: var(--transition);
}

.o-card:hover .project-icon-box {
  background-color: var(--color-accent-soft);
}

.project-info {
  flex: 1;
}

.project-name {
  font-size: 1.125rem;
  font-weight: 700;
  margin: 0 0 0.25rem;
  color: var(--text-primary);
}

.project-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8125rem;
  color: var(--text-subtle);
}

.meta-divider { width: 3px; height: 3px; border-radius: 50%; background: var(--color-border-hover); }

.project-action {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.875rem;
  font-weight: 700;
  color: var(--color-accent);
  opacity: 0;
  transform: translateX(-10px);
  transition: var(--transition);
}

.o-card:hover .project-action {
  opacity: 1;
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
