<script setup lang="ts">
import { computed } from "vue";
import { RouterView, useRoute, useRouter } from "vue-router";

import {
  buildProjectConversationRoute,
  buildProjectDashboardRoute,
  buildProjectTasksRoute,
  buildWorkspaceInboxRoute,
  buildWorkspaceProjectsRoute,
  useConnectionStore
} from "./stores/connection";
import { useHubStore } from "./stores/hub";
import { usePreferencesStore } from "./stores/preferences";

const hub = useHubStore();
const connection = useConnectionStore();
const preferences = usePreferencesStore();
const route = useRoute();
const router = useRouter();

preferences.initialize();

function coerceRouteParam(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

const activeWorkspaceId = computed(
  () =>
    coerceRouteParam(route.params.workspaceId) ??
    hub.currentWorkspaceId ??
    connection.profile.workspaceId ??
    (connection.profile.mode === "local" ? "demo" : null)
);

const activeProjectId = computed(
  () =>
    coerceRouteParam(route.params.projectId) ??
    hub.currentProjectId ??
    connection.profile.projectId ??
    (connection.profile.mode === "local" ? "demo" : null)
);

const dashboardRoute = computed(() => {
  if (!activeWorkspaceId.value || !activeProjectId.value) {
    return null;
  }

  return buildProjectDashboardRoute(activeWorkspaceId.value, activeProjectId.value);
});

const conversationRoute = computed(() => {
  if (!activeWorkspaceId.value || !activeProjectId.value) {
    return null;
  }

  return buildProjectConversationRoute(activeWorkspaceId.value, activeProjectId.value);
});

const tasksRoute = computed(() => {
  if (!activeWorkspaceId.value || !activeProjectId.value) {
    return null;
  }

  return buildProjectTasksRoute(activeWorkspaceId.value, activeProjectId.value);
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

const projectsRoute = computed(() => {
  if (!activeWorkspaceId.value) {
    return null;
  }

  return buildWorkspaceProjectsRoute(activeWorkspaceId.value);
});

const inboxRoute = computed(() => {
  if (!activeWorkspaceId.value) {
    return null;
  }

  return buildWorkspaceInboxRoute(activeWorkspaceId.value);
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
  () =>
    hub.projectContext?.workspace.display_name ??
    activeWorkspaceId.value ??
    preferences.t("shell.workspaceFallback")
);

const projectTitle = computed(
  () =>
    hub.projectContext?.project.display_name ??
    activeProjectId.value ??
    preferences.t("shell.projectFallback")
);

const connectionBanner = computed(() => connection.connectionBanner);
const reminderSignals = computed(() => {
  const notificationReminders = hub.notifications.map((notification) => ({
    id: notification.id,
    title: notification.title,
    message: notification.message,
    tone: "info"
  }));

  if (connectionBanner.value) {
    return [
      {
        id: `banner:${connectionBanner.value.kind}`,
        title: connectionBanner.value.title,
        message: connectionBanner.value.message,
        tone: connectionBanner.value.tone
      },
      ...notificationReminders
    ];
  }

  return notificationReminders;
});

const contextItems = computed(() => [
  {
    key: "workspace",
    label: preferences.t("shell.context"),
    value: workspaceTitle.value
  },
  {
    key: "mode",
    label: preferences.t("shell.mode"),
    value: hub.connectionStatus?.mode ?? connection.profile.mode
  },
  {
    key: "connection",
    label: preferences.t("shell.connection"),
    value: hub.connectionStatus?.state ?? "connecting"
  },
  {
    key: "auth",
    label: preferences.t("shell.auth"),
    value: hub.authState
  },
  {
    key: "locale",
    label: preferences.t("shell.locale"),
    value: preferences.localeLabel
  },
  {
    key: "theme",
    label: preferences.t("shell.theme"),
    value: preferences.themeLabel
  }
]);

const primaryNav = computed(() => [
  {
    key: "dashboard",
    label: preferences.t("nav.dashboard"),
    route: dashboardRoute.value
  },
  {
    key: "conversation",
    label: preferences.t("nav.conversation"),
    route: conversationRoute.value,
    testId: "shell-open-conversation"
  },
  {
    key: "runs",
    label: preferences.t("nav.runs"),
    route: runsRoute.value
  },
  {
    key: "inbox",
    label: preferences.t("nav.inbox"),
    route: inboxRoute.value
  },
  {
    key: "knowledge",
    label: preferences.t("nav.knowledge"),
    route: knowledgeRoute.value
  }
]);

const toolNav = computed(() => [
  {
    key: "projects",
    label: preferences.t("nav.projects"),
    route: projectsRoute.value
  },
  {
    key: "connections",
    label: preferences.t("nav.connections"),
    route: "/connections"
  },
  {
    key: "models",
    label: preferences.t("nav.models"),
    route: modelsRoute.value
  },
  {
    key: "preferences",
    label: preferences.t("nav.preferences"),
    route: "/preferences",
    testId: "shell-open-preferences"
  },
  {
    key: "tasks",
    label: preferences.t("nav.tasks"),
    route: tasksRoute.value
  }
]);

function navigate(path: string | null): void {
  if (!path) {
    return;
  }

  void router.push(path);
}

function isActive(path: string | null): boolean {
  return Boolean(path) && (route.path === path || route.path.startsWith(`${path}/`));
}
</script>

<template>
  <div class="app-shell">
    <aside class="shell-aside">
      <div class="shell-header">
        <p class="shell-brand">{{ preferences.t("shell.brand") }}</p>
        <h1 class="shell-title">{{ workspaceTitle }}</h1>
        <p class="shell-subtitle">{{ projectTitle }}</p>
      </div>

      <section class="shell-section">
        <p class="shell-section-label">{{ preferences.t("shell.context") }}</p>
        <ul class="shell-context-list">
          <li v-for="item in contextItems" :key="item.key" class="shell-context-card">
            <span class="shell-context-label">{{ item.label }}</span>
            <strong>{{ item.value }}</strong>
          </li>
        </ul>
      </section>

      <section
        v-if="connectionBanner"
        :data-kind="connectionBanner.kind"
        :class="['shell-reminder', `shell-reminder--${connectionBanner.tone}`]"
        data-testid="connection-banner"
      >
        <strong>{{ connectionBanner.title }}</strong>
        <p>{{ connectionBanner.message }}</p>
      </section>

      <section class="shell-section">
        <p class="shell-section-label">{{ preferences.t("shell.reminders") }}</p>
        <div v-if="reminderSignals.length > 0" class="shell-reminder-list">
          <button
            v-for="reminder in reminderSignals.slice(0, 3)"
            :key="reminder.id"
            :class="['shell-reminder', `shell-reminder--${reminder.tone}`]"
            type="button"
            @click="navigate(notificationsRoute)"
          >
            <strong>{{ reminder.title }}</strong>
            <p>{{ reminder.message }}</p>
          </button>
        </div>
        <p v-else class="shell-muted">{{ preferences.t("shell.noReminders") }}</p>
      </section>

      <section class="shell-section">
        <p class="shell-section-label">{{ preferences.t("shell.primary") }}</p>
        <nav class="shell-nav">
          <button
            v-for="item in primaryNav"
            :key="item.key"
            :data-testid="item.testId"
            :class="[
              'shell-nav-item',
              {
                'is-active': isActive(item.route),
                'is-disabled': !item.route
              }
            ]"
            :disabled="!item.route"
            type="button"
            @click="navigate(item.route)"
          >
            {{ item.label }}
          </button>
        </nav>
      </section>

      <section class="shell-section">
        <p class="shell-section-label">{{ preferences.t("shell.tools") }}</p>
        <nav class="shell-nav">
          <button
            v-for="item in toolNav"
            :key="item.key"
            :data-testid="item.testId"
            :class="[
              'shell-nav-item',
              {
                'is-active': isActive(item.route),
                'is-disabled': !item.route
              }
            ]"
            :disabled="!item.route"
            type="button"
            @click="navigate(item.route)"
          >
            {{ item.label }}
          </button>
        </nav>
      </section>

      <section v-if="runRoute || automationRoute || hub.webhookSecretReveal" class="shell-section">
        <p class="shell-section-label">{{ preferences.t("shell.runtime") }}</p>
        <div class="shell-nav">
          <button
            v-if="runRoute"
            class="shell-nav-item"
            type="button"
            @click="navigate(runRoute)"
          >
            Current Run
          </button>
          <button
            v-if="automationRoute"
            class="shell-nav-item"
            type="button"
            @click="navigate(automationRoute)"
          >
            Automation Detail
          </button>
          <p v-if="hub.webhookSecretReveal" class="shell-secret">
            Webhook secret reveal: {{ hub.webhookSecretReveal }}
          </p>
        </div>
      </section>

      <p v-if="hub.surfaceError" class="shell-error">{{ hub.surfaceError }}</p>
    </aside>

    <main class="shell-content">
      <RouterView />
    </main>
  </div>
</template>
