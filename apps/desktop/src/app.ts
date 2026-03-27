import type { HubClient, LocalHubTransport } from "@octopus/hub-client";
import { createLocalHubClient } from "@octopus/hub-client";
import { createPinia } from "pinia";
import { createApp } from "vue";
import {
  createMemoryHistory,
  createRouter,
  createWebHistory,
  type Router
} from "vue-router";

import AppShell from "./App.vue";
import { configureHubClient } from "./stores/hub";
import AutomationDetailView from "./views/AutomationDetailView.vue";
import RunView from "./views/RunView.vue";
import WorkspaceView from "./views/WorkspaceView.vue";

export interface DesktopPlugins {
  pinia: ReturnType<typeof createPinia>;
  router: Router;
}

function createRoutes() {
  return [
    {
      path: "/",
      redirect: "/workspaces/demo/projects/demo"
    },
    {
      path: "/workspaces/:workspaceId/projects/:projectId",
      component: WorkspaceView
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
