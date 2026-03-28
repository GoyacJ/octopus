import type { HubClient, LocalHubTransport } from "@octopus/hub-client";
import { createLocalHubClient } from "@octopus/hub-client";
import { createPinia } from "pinia";
import { createApp } from "vue";
import {
  createMemoryHistory,
  createRouter,
  createWebHistory,
  type RouteLocationGeneric,
  type RouteRecordRaw,
  type Router
} from "vue-router";

import AppShell from "./App.vue";
import { configureHubClient } from "./stores/hub";
import AutomationDetailView from "./views/AutomationDetailView.vue";
import ConnectionsView from "./views/ConnectionsView.vue";
import InboxView from "./views/InboxView.vue";
import NotificationsView from "./views/NotificationsView.vue";
import RunView from "./views/RunView.vue";
import RunsView from "./views/RunsView.vue";
import TasksView from "./views/TasksView.vue";

export interface DesktopPlugins {
  pinia: ReturnType<typeof createPinia>;
  router: Router;
}

function routeParam(to: RouteLocationGeneric, key: string): string {
  const value = to.params[key];

  if (Array.isArray(value)) {
    return value[0] ?? "";
  }

  return value ?? "";
}

function createRoutes(): RouteRecordRaw[] {
  return [
    {
      path: "/",
      redirect: "/workspaces/demo/projects/demo/tasks"
    },
    {
      path: "/workspaces/:workspaceId/projects/:projectId",
      redirect: (to) =>
        `/workspaces/${routeParam(to, "workspaceId")}/projects/${routeParam(to, "projectId")}/tasks`
    },
    {
      path: "/workspaces/:workspaceId/projects/:projectId/tasks",
      component: TasksView
    },
    {
      path: "/workspaces/:workspaceId/projects/:projectId/runs",
      component: RunsView
    },
    {
      path: "/workspaces/:workspaceId/inbox",
      component: InboxView
    },
    {
      path: "/workspaces/:workspaceId/projects/:projectId/inbox",
      redirect: (to) => `/workspaces/${routeParam(to, "workspaceId")}/inbox`
    },
    {
      path: "/workspaces/:workspaceId/notifications",
      component: NotificationsView
    },
    {
      path: "/workspaces/:workspaceId/projects/:projectId/notifications",
      redirect: (to) => `/workspaces/${routeParam(to, "workspaceId")}/notifications`
    },
    {
      path: "/connections",
      component: ConnectionsView
    },
    {
      path: "/workspaces/:workspaceId/projects/:projectId/automations/:automationId",
      component: AutomationDetailView
    },
    {
      path: "/runs/:runId",
      component: RunView
    }
  ];
}

export function createDesktopPlugins(
  client: HubClient,
  useMemoryHistory = false
): DesktopPlugins {
  configureHubClient(client);

  const pinia = createPinia();
  const router = createRouter({
    history: useMemoryHistory ? createMemoryHistory() : createWebHistory(),
    routes: createRoutes()
  });

  return { pinia, router };
}

export function createDesktopApp(
  client: HubClient,
  useMemoryHistory = false
): DesktopPlugins & { app: ReturnType<typeof createApp> } {
  const app = createApp(AppShell);
  const { pinia, router } = createDesktopPlugins(client, useMemoryHistory);

  app.use(pinia);
  app.use(router);

  return { app, pinia, router };
}

export function createWindowLocalHubClient(): HubClient {
  const transport = window.__OCTOPUS_LOCAL_HUB__;

  if (!transport) {
    throw new Error(
      "No local Hub transport bridge is registered on window.__OCTOPUS_LOCAL_HUB__."
    );
  }

  return createLocalHubClient(transport);
}

export type { HubClient, LocalHubTransport };
