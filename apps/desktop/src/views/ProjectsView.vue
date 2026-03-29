<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";

import {
  buildProjectTasksRoute,
  useConnectionStore
} from "../stores/connection";
import { useHubStore } from "../stores/hub";

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
  <section class="projects-layout">
    <article class="surface-card hero">
      <p class="eyebrow">Projects</p>
      <h1>{{ hub.currentWorkspaceId ?? route.params.workspaceId }}</h1>
      <p class="muted">
        Select an existing project in the current workspace to enter the tracked task
        workbench.
      </p>
    </article>

    <article class="surface-card">
      <div class="header-row">
        <div>
          <p class="eyebrow">Workspace Projects</p>
          <h2>{{ hub.projects.length }} available</h2>
        </div>
      </div>

      <ul v-if="hub.projects.length > 0" class="stack-list">
        <li v-for="project in hub.projects" :key="project.id" class="project-card">
          <div class="project-copy">
            <strong>{{ project.display_name }}</strong>
            <p class="muted">Project ID: {{ project.id }}</p>
            <p class="muted">Updated: {{ project.updated_at }}</p>
          </div>
          <button
            :data-testid="`project-open-${project.id}`"
            class="project-link"
            @click="openProject(project.id)"
          >
            Open Tasks
          </button>
        </li>
      </ul>

      <p v-else class="muted">
        No tracked projects are available in this workspace yet.
      </p>
    </article>
  </section>
</template>

<style scoped>
.projects-layout {
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
    radial-gradient(circle at top right, rgba(250, 204, 21, 0.18), transparent 32%),
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

.project-card {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: center;
  padding: 0.95rem;
  border-radius: 0.9rem;
  background: rgba(2, 6, 23, 0.6);
}

.project-copy {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.project-link {
  flex-shrink: 0;
  border: none;
  border-radius: 999px;
  padding: 0.7rem 0.95rem;
  font: inherit;
  font-weight: 600;
  color: #082f49;
  background: linear-gradient(135deg, #67e8f9, #facc15);
  cursor: pointer;
}

@media (max-width: 720px) {
  .project-card {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
