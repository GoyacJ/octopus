<script setup lang="ts">
import { computed } from "vue";
import { RouterLink, RouterView } from "vue-router";

import { useHubStore } from "./stores/hub";

const hub = useHubStore();

const workspaceRoute = computed(() => {
  if (!hub.currentWorkspaceId || !hub.currentProjectId) {
    return null;
  }

  return `/workspaces/${hub.currentWorkspaceId}/projects/${hub.currentProjectId}`;
});

const runRoute = computed(() => {
  if (!hub.currentRunId) {
    return null;
  }

  return `/runs/${hub.currentRunId}`;
});

const automationRoute = computed(() => {
  if (!hub.currentWorkspaceId || !hub.currentProjectId || !hub.currentAutomationId) {
    return null;
  }

  return `/workspaces/${hub.currentWorkspaceId}/projects/${hub.currentProjectId}/automations/${hub.currentAutomationId}`;
});
</script>

<template>
  <div class="app-shell">
    <aside class="shell-rail">
      <p class="brand">Octopus GA Surface</p>
      <h1>{{ hub.workspaceName }}</h1>
      <p class="muted">{{ hub.projectName }}</p>

      <div class="status-card">
        <span class="status-label">Hub</span>
        <strong>
          {{ hub.connectionStatus?.mode ?? "local pending" }} /
          {{ hub.connectionStatus?.state ?? "connecting" }} /
          {{ hub.authState }}
        </strong>
        <span class="muted">
          Refreshed {{ hub.connectionStatus?.last_refreshed_at ?? "not yet loaded" }}
        </span>
      </div>

      <p v-if="hub.webhookSecretReveal" class="secret-banner">
        Webhook secret reveal: {{ hub.webhookSecretReveal }}
      </p>

      <nav class="nav-stack">
        <RouterLink v-if="workspaceRoute" :to="workspaceRoute">Workspace Shell</RouterLink>
        <RouterLink v-if="automationRoute" :to="automationRoute">Automation Detail</RouterLink>
        <RouterLink v-if="runRoute" :to="runRoute">Current Run</RouterLink>
      </nav>

      <p v-if="hub.surfaceError" class="error-banner">{{ hub.surfaceError }}</p>
    </aside>

    <main class="content-shell">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  display: grid;
  min-height: 100vh;
  grid-template-columns: minmax(260px, 320px) 1fr;
  background:
    radial-gradient(circle at top left, rgba(103, 232, 249, 0.16), transparent 25%),
    linear-gradient(180deg, #020617 0%, #0f172a 100%);
  color: #e2e8f0;
}

.shell-rail {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 2rem 1.4rem;
  border-right: 1px solid rgba(148, 163, 184, 0.16);
  background: rgba(2, 6, 23, 0.55);
  backdrop-filter: blur(18px);
}

.content-shell {
  padding: 2rem;
}

.brand,
.muted {
  margin: 0;
}

.brand {
  font-size: 0.78rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #67e8f9;
}

h1 {
  margin: 0;
  font-size: 1.6rem;
  line-height: 1.1;
}

.muted {
  color: #94a3b8;
}

.status-card {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 1rem;
  border: 1px solid rgba(148, 163, 184, 0.2);
  border-radius: 1rem;
  background: rgba(15, 23, 42, 0.72);
}

.status-label {
  font-size: 0.72rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #facc15;
}

.nav-stack {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.nav-stack a {
  border: 1px solid rgba(103, 232, 249, 0.2);
  border-radius: 999px;
  padding: 0.7rem 0.9rem;
  color: inherit;
  text-decoration: none;
  background: rgba(15, 23, 42, 0.6);
}

.nav-stack a.router-link-active {
  border-color: rgba(250, 204, 21, 0.45);
  background: rgba(250, 204, 21, 0.12);
}

.error-banner {
  margin: 0;
  padding: 0.9rem 1rem;
  border-radius: 1rem;
  color: #fecaca;
  background: rgba(127, 29, 29, 0.55);
}

.secret-banner {
  margin: 0;
  padding: 0.9rem 1rem;
  border-radius: 1rem;
  color: #fef3c7;
  background: rgba(133, 77, 14, 0.45);
  word-break: break-word;
}

@media (max-width: 900px) {
  .app-shell {
    grid-template-columns: 1fr;
  }

  .shell-rail {
    border-right: none;
    border-bottom: 1px solid rgba(148, 163, 184, 0.16);
  }
}
</style>
