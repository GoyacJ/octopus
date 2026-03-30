<script setup lang="ts">
import { computed } from "vue";
import { RouterLink, RouterView, useRoute } from "vue-router";
import { 
  Layers, 
  Edit3, 
  Play, 
  Database, 
  Inbox, 
  Bell, 
  Link2, 
  Settings, 
  Activity,
  Box,
  AlertCircle
} from "lucide-vue-next";

import { useConnectionStore } from "./stores/connection";
import { useHubStore } from "./stores/hub";

const hub = useHubStore();
const connection = useConnectionStore();
const route = useRoute();

function coerceRouteParam(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

const activeWorkspaceId = computed(
  () => coerceRouteParam(route.params.workspaceId) ?? hub.currentWorkspaceId
);

const activeProjectId = computed(
  () => coerceRouteParam(route.params.projectId) ?? hub.currentProjectId
);

const projectsRoute = computed(() => {
  if (!activeWorkspaceId.value) {
    return null;
  }

  return `/workspaces/${activeWorkspaceId.value}/projects`;
});

const tasksRoute = computed(() => {
  if (!activeWorkspaceId.value || !activeProjectId.value) {
    return null;
  }

  return `/workspaces/${activeWorkspaceId.value}/projects/${activeProjectId.value}/tasks`;
});

const runsRoute = computed(() => {
  if (!activeWorkspaceId.value || !activeProjectId.value) {
    return null;
  }

  return `/workspaces/${activeWorkspaceId.value}/projects/${activeProjectId.value}/runs`;
});

const knowledgeRoute = computed(() => {
  if (!activeWorkspaceId.value || !activeProjectId.value) {
    return null;
  }

  return `/workspaces/${activeWorkspaceId.value}/projects/${activeProjectId.value}/knowledge`;
});

const inboxRoute = computed(() => {
  if (!activeWorkspaceId.value) {
    return null;
  }

  return `/workspaces/${activeWorkspaceId.value}/inbox`;
});

const modelsRoute = computed(() => {
  if (!activeWorkspaceId.value) {
    return null;
  }

  return `/workspaces/${activeWorkspaceId.value}/models`;
});

const notificationsRoute = computed(() => {
  if (!activeWorkspaceId.value) {
    return null;
  }

  return `/workspaces/${activeWorkspaceId.value}/notifications`;
});

const runRoute = computed(() => {
  if (!hub.currentRunId) {
    return null;
  }

  return `/runs/${hub.currentRunId}`;
});

const automationRoute = computed(() => {
  if (!activeWorkspaceId.value || !activeProjectId.value || !hub.currentAutomationId) {
    return null;
  }

  return `/workspaces/${activeWorkspaceId.value}/projects/${activeProjectId.value}/automations/${hub.currentAutomationId}`;
});

const workspaceTitle = computed(
  () => hub.projectContext?.workspace.display_name ?? activeWorkspaceId.value ?? "Workspace"
);

const projectTitle = computed(
  () =>
    hub.projectContext?.project.display_name ??
    activeProjectId.value ??
    "Project Selection"
);

const connectionBanner = computed(() => connection.connectionBanner);
</script>

<template>
  <div class="app-shell">
    <aside class="shell-sidebar">
      <div class="sidebar-header">
        <p class="brand">Octopus</p>
        <h1 class="workspace-title">{{ workspaceTitle }}</h1>
        <p class="project-title">{{ projectTitle }}</p>
      </div>

      <div class="status-card">
        <div class="status-header">
          <span class="status-indicator" :class="hub.connectionStatus?.state"></span>
          <span class="status-label">Hub: {{ hub.connectionStatus?.mode ?? "local" }}</span>
        </div>
        <div class="status-detail">
          <span class="status-state">{{ hub.connectionStatus?.state ?? "connecting" }}</span>
          <span class="status-auth">{{ hub.authState }}</span>
        </div>
      </div>

      <section
        v-if="connectionBanner"
        :class="['banner', `banner--${connectionBanner.tone}`]"
        data-testid="connection-banner"
      >
        <div class="banner-icon"><AlertCircle :size="14" /></div>
        <div class="banner-content">
          <strong>{{ connectionBanner.title }}</strong>
          <p>{{ connectionBanner.message }}</p>
        </div>
      </section>

      <nav class="nav-stack">
        <RouterLink v-if="projectsRoute" :to="projectsRoute" class="nav-item">
          <span class="nav-icon"><Layers :size="18" /></span>
          <span class="nav-text">Projects</span>
        </RouterLink>
        <RouterLink v-if="tasksRoute" :to="tasksRoute" class="nav-item">
          <span class="nav-icon"><Edit3 :size="18" /></span>
          <span class="nav-text">Tasks</span>
        </RouterLink>
        <RouterLink v-if="runsRoute" :to="runsRoute" class="nav-item">
          <span class="nav-icon"><Activity :size="18" /></span>
          <span class="nav-text">Runs</span>
        </RouterLink>
        <RouterLink v-if="knowledgeRoute" :to="knowledgeRoute" class="nav-item">
          <span class="nav-icon"><Database :size="18" /></span>
          <span class="nav-text">Knowledge</span>
        </RouterLink>
        <RouterLink v-if="modelsRoute" :to="modelsRoute" class="nav-item">
          <span class="nav-icon"><Box :size="18" /></span>
          <span class="nav-text">Models</span>
        </RouterLink>
        <RouterLink v-if="inboxRoute" :to="inboxRoute" class="nav-item">
          <span class="nav-icon"><Inbox :size="18" /></span>
          <span class="nav-text">Inbox</span>
        </RouterLink>
        <RouterLink v-if="notificationsRoute" :to="notificationsRoute" class="nav-item">
          <span class="nav-icon"><Bell :size="18" /></span>
          <span class="nav-text">Notifications</span>
        </RouterLink>
        
        <div class="nav-divider"></div>
        
        <RouterLink to="/connections" class="nav-item">
          <span class="nav-icon"><Link2 :size="18" /></span>
          <span class="nav-text">Connections</span>
        </RouterLink>
        <RouterLink v-if="automationRoute" :to="automationRoute" class="nav-item">
          <span class="nav-icon"><Settings :size="18" /></span>
          <span class="nav-text">Automation</span>
        </RouterLink>
        <RouterLink v-if="runRoute" :to="runRoute" class="nav-item highlight">
          <span class="nav-icon"><Play :size="18" /></span>
          <span class="nav-text">Current Run</span>
        </RouterLink>
      </nav>

      <div class="sidebar-footer">
        <p v-if="hub.surfaceError" class="error-msg">{{ hub.surfaceError }}</p>
        <p v-if="hub.webhookSecretReveal" class="secret-info">
          Secret: {{ hub.webhookSecretReveal }}
        </p>
      </div>
    </aside>

    <main class="content-view">
      <div class="view-header">
        <!-- View specific header can be injected here -->
      </div>
      <div class="view-body">
        <RouterView />
      </div>
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  display: grid;
  grid-template-columns: 280px 1fr;
  min-height: 100vh;
  background-color: var(--bg-app);
}

.shell-sidebar {
  display: flex;
  flex-direction: column;
  padding: 1.5rem 1rem;
  background-color: var(--bg-surface);
  border-right: 1px solid var(--color-border);
  position: sticky;
  top: 0;
  height: 100vh;
  overflow-y: auto;
  z-index: 10;
}

.sidebar-header {
  padding: 0.5rem 0.75rem 1.5rem;
}

.brand {
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--color-accent);
  margin-bottom: 0.5rem;
}

.workspace-title {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 0.25rem;
  line-height: 1.2;
}

.project-title {
  font-size: 0.875rem;
  color: var(--text-muted);
  margin: 0;
}

.status-card {
  margin: 0 0.5rem 1.5rem;
  padding: 0.875rem;
  background-color: var(--bg-app);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
}

.status-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.status-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--text-subtle);
}

.status-indicator.connected {
  background-color: var(--color-success);
  box-shadow: 0 0 0 2px var(--color-success-soft);
}

.status-indicator.connecting {
  background-color: var(--color-warning);
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0% { opacity: 1; }
  50% { opacity: 0.5; }
  100% { opacity: 1; }
}

.status-label {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.status-detail {
  display: flex;
  justify-content: space-between;
  font-size: 0.8125rem;
  font-weight: 500;
}

.status-state {
  color: var(--text-primary);
  text-transform: capitalize;
}

.status-auth {
  color: var(--color-accent);
}

.banner {
  margin: 0 0.5rem 1.5rem;
  padding: 0.75rem 0.875rem;
  border-radius: var(--radius-lg);
  display: flex;
  gap: 0.75rem;
  font-size: 0.8125rem;
}

.banner--warning {
  background-color: var(--color-warning-soft);
  border: 1px solid rgba(245, 158, 11, 0.2);
  color: #92400e;
}

.banner--danger {
  background-color: var(--color-danger-soft);
  border: 1px solid rgba(244, 63, 94, 0.2);
  color: #9f1239;
}

.banner-icon {
  flex-shrink: 0;
  width: 1.25rem;
  height: 1.25rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.05);
  font-weight: 700;
}

.banner-content strong {
  display: block;
  margin-bottom: 0.125rem;
}

.banner-content p {
  margin: 0;
  line-height: 1.4;
  opacity: 0.9;
}

.nav-stack {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  flex: 1;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.625rem 0.75rem;
  border-radius: var(--radius-lg);
  color: var(--text-muted);
  font-weight: 500;
  font-size: 0.9375rem;
  transition: var(--transition);
}

.nav-item:hover {
  background-color: var(--bg-app);
  color: var(--text-primary);
}

.nav-item.router-link-active {
  background-color: var(--color-accent-soft);
  color: var(--color-accent);
}

.nav-item.highlight {
  margin-top: 0.5rem;
  background-color: var(--text-primary);
  color: white;
}

.nav-item.highlight:hover {
  background-color: #1e293b;
}

.nav-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  opacity: 0.8;
}

.nav-divider {
  height: 1px;
  background-color: var(--color-border);
  margin: 0.75rem 0.75rem;
}

.sidebar-footer {
  margin-top: auto;
  padding: 1rem 0.5rem 0;
}

.error-msg {
  font-size: 0.75rem;
  color: var(--color-danger);
  background-color: var(--color-danger-soft);
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-lg);
  margin: 0;
}

.secret-info {
  font-size: 0.7rem;
  color: var(--text-subtle);
  word-break: break-all;
  margin: 0.5rem 0 0;
}

.content-view {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.view-body {
  flex: 1;
  overflow-y: auto;
  padding: 2rem;
}

@media (max-width: 1024px) {
  .app-shell {
    grid-template-columns: 240px 1fr;
  }
}

@media (max-width: 768px) {
  .app-shell {
    grid-template-columns: 1fr;
  }
  
  .shell-sidebar {
    height: auto;
    position: relative;
    border-right: none;
    border-bottom: 1px solid var(--color-border);
  }
}
</style>
