import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  createLocalHubClient,
  type HubClient,
  type LocalHubTransport,
  type RemoteHubAuthClient
} from "@octopus/hub-client";

import AppShell from "../src/App.vue";
import { createDesktopPlugins } from "../src/app";
import {
  configureDesktopConnectionRuntime,
  loadDesktopConnectionProfile,
  persistDesktopConnectionProfile,
  resetDesktopConnectionRuntime,
  resolveDesktopEntryRoute
} from "../src/stores/connection";

const remoteProfile = {
  mode: "remote",
  baseUrl: "http://127.0.0.1:4000",
  workspaceId: "workspace-alpha",
  email: "admin@octopus.local"
} as const;

const remoteSessionFixture = {
  session_id: "session-1",
  user_id: "user-1",
  email: "admin@octopus.local",
  workspace_id: "workspace-alpha",
  actor_ref: "workspace_admin:bootstrap_admin",
  issued_at: "2026-03-29T10:00:00Z",
  expires_at: "2026-03-29T12:00:00Z"
} as const;

const localProjectContextFixture = {
  workspace: {
    id: "demo",
    slug: "demo",
    display_name: "Demo Workspace",
    created_at: "2026-03-28T10:00:00Z",
    updated_at: "2026-03-28T10:00:00Z"
  },
  project: {
    id: "demo",
    workspace_id: "demo",
    slug: "demo",
    display_name: "Demo Project",
    created_at: "2026-03-28T10:00:00Z",
    updated_at: "2026-03-28T10:00:00Z"
  }
} as const;

const remoteProjectsFixture = [
  {
    id: "project-ops",
    workspace_id: "workspace-alpha",
    slug: "project-ops",
    display_name: "Ops Project",
    created_at: "2026-03-29T10:00:00Z",
    updated_at: "2026-03-29T10:00:02Z"
  },
  {
    id: "project-auth",
    workspace_id: "workspace-alpha",
    slug: "project-auth",
    display_name: "Auth Project",
    created_at: "2026-03-29T10:00:00Z",
    updated_at: "2026-03-29T10:00:01Z"
  }
] as const;

function remoteProjectContextFixture(projectId: string) {
  const project = remoteProjectsFixture.find((item) => item.id === projectId);
  if (!project) {
    throw new Error(`unknown project fixture: ${projectId}`);
  }

  return {
    workspace: {
      id: "workspace-alpha",
      slug: "workspace-alpha",
      display_name: "Workspace Alpha",
      created_at: "2026-03-29T10:00:00Z",
      updated_at: "2026-03-29T10:00:00Z"
    },
    project
  } as const;
}

const localCapabilityResolutionFixture = [
  {
    descriptor: {
      id: "capability-local-demo",
      slug: "capability-local-demo",
      kind: "core",
      source: "octopus-runtime",
      platform: "local",
      risk_level: "low",
      requires_approval: false,
      input_schema_uri: null,
      output_schema_uri: null,
      fallback_capability_id: null,
      trust_level: "trusted",
      created_at: "2026-03-28T10:00:00Z",
      updated_at: "2026-03-28T10:00:00Z"
    },
    scope_ref: "workspace:demo/project:demo",
    execution_state: "executable",
    reason_code: "within_budget",
    explanation: "Executable in the default local seed."
  }
] as const;

const remoteApprovalFixture = {
  id: "approval-1",
  workspace_id: "workspace-alpha",
  project_id: "project-auth",
  run_id: "run-approval",
  task_id: "task-approval",
  approval_type: "execution",
  target_ref: "run:run-approval",
  status: "pending",
  reason: "Needs approval",
  dedupe_key: "approval:1",
  decided_by: null,
  decision_note: null,
  decided_at: null,
  created_at: "2026-03-29T10:00:00Z",
  updated_at: "2026-03-29T10:00:00Z"
} as const;

const remoteInboxFixture = [
  {
    id: "inbox-approval-1",
    workspace_id: "workspace-alpha",
    project_id: "project-auth",
    run_id: "run-approval",
    approval_request_id: "approval-1",
    item_type: "approval_request",
    target_ref: "run:run-approval",
    status: "open",
    dedupe_key: "inbox:approval-1",
    title: "Execution approval required",
    message: "A governed run needs approval before execution.",
    created_at: "2026-03-29T10:00:00Z",
    updated_at: "2026-03-29T10:00:00Z",
    resolved_at: null
  }
] as const;

function createLocalWorkbenchClient(): HubClient {
  const transport: LocalHubTransport = {
    async invoke(command, payload) {
      switch (command) {
        case "hub:get_project_context":
          return localProjectContextFixture;
        case "hub:list_projects":
          expect(payload).toEqual({
            workspaceId: "demo"
          });
          return [localProjectContextFixture.project];
        case "hub:list_capability_visibility":
          return localCapabilityResolutionFixture;
        case "hub:get_connection_status":
          return {
            mode: "local",
            state: "connected",
            auth_state: "authenticated",
            active_server_count: 0,
            healthy_server_count: 0,
            servers: [],
            last_refreshed_at: "2026-03-29T10:00:00Z"
          };
        case "hub:list_automations":
        case "hub:list_inbox_items":
        case "hub:list_notifications":
          return [];
        default:
          throw new Error(`unexpected local command: ${command}`);
      }
    },
    async listen() {
      return () => undefined;
    }
  };

  return createLocalHubClient(transport);
}

function createRemoteWorkbenchClient(
  currentAuthState: () => "auth_required" | "authenticated" | "token_expired",
  currentProjects: () => readonly (typeof remoteProjectsFixture)[number][]
): HubClient {
  const transport: LocalHubTransport = {
    async invoke(command, payload) {
      switch (command) {
        case "hub:list_projects":
          return currentProjects();
        case "hub:get_project_context": {
          const commandPayload = payload as {
            workspaceId: string;
            projectId: string;
          };
          const project = currentProjects().find(
            (item) =>
              item.workspace_id === commandPayload.workspaceId &&
              item.id === commandPayload.projectId
          );
          if (!project) {
            throw new Error(
              `project \`${commandPayload.projectId}\` not found in workspace \`${commandPayload.workspaceId}\``
            );
          }
          return remoteProjectContextFixture(project.id);
        }
        case "hub:list_capability_visibility":
          return localCapabilityResolutionFixture;
        case "hub:get_connection_status":
          return {
            mode: "remote",
            state: "connected",
            auth_state: currentAuthState(),
            active_server_count: 1,
            healthy_server_count: currentAuthState() === "authenticated" ? 1 : 0,
            servers: [],
            last_refreshed_at: "2026-03-29T10:00:00Z"
          };
        case "hub:list_inbox_items":
          return currentAuthState() === "auth_required" ? [] : remoteInboxFixture;
        case "hub:get_approval_request":
          return remoteApprovalFixture;
        case "hub:list_notifications":
        case "hub:list_automations":
        case "hub:list_runs":
          return [];
        default:
          throw new Error(`unexpected remote command: ${command}`);
      }
    },
    async listen() {
      return () => undefined;
    }
  };

  return createLocalHubClient(transport);
}

async function mountRemoteShell(
  authState: () => "auth_required" | "authenticated" | "token_expired",
  authClient: RemoteHubAuthClient,
  currentProjects: () => readonly (typeof remoteProjectsFixture)[number][] = () =>
    remoteProjectsFixture
) {
  persistDesktopConnectionProfile(remoteProfile);
  configureDesktopConnectionRuntime({
    storage: window.localStorage,
    createLocalClient: () => createLocalWorkbenchClient(),
    createRemoteClient: () => createRemoteWorkbenchClient(authState, currentProjects),
    createRemoteAuthClient: () => authClient
  });

  const { pinia, router } = createDesktopPlugins(
    createRemoteWorkbenchClient(authState, currentProjects),
    true,
    {
      defaultRoute: resolveDesktopEntryRoute()
    }
  );

  await router.push("/");
  await router.isReady();

  const wrapper = mount(AppShell, {
    global: {
      plugins: [pinia, router]
    }
  });

  await flushPromises();

  return { wrapper, router };
}

describe("desktop remote connection surface", () => {
  beforeEach(() => {
    window.localStorage.clear();
    resetDesktopConnectionRuntime();
  });

  afterEach(() => {
    window.localStorage.clear();
    resetDesktopConnectionRuntime();
  });

  it("boots remote mode without a valid session into /connections", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(() => authState, {
      async login() {
        authState = "authenticated";
        return {
          access_token: "remote-token",
          session: remoteSessionFixture
        };
      },
      async getCurrentSession() {
        return remoteSessionFixture;
      },
      async logout() {
        authState = "auth_required";
      }
    });

    expect(router.currentRoute.value.fullPath).toBe("/connections");
    expect(wrapper.text()).toContain("Hub Connections");
    expect(wrapper.text()).toContain("auth_required");
  });

  it("logs in from ConnectionsView and enters the workspace workbench", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(() => authState, {
      async login() {
        authState = "authenticated";
        return {
          access_token: "remote-token",
          session: remoteSessionFixture
        };
      },
      async getCurrentSession() {
        return remoteSessionFixture;
      },
      async logout() {
        authState = "auth_required";
      }
    });

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe("/workspaces/workspace-alpha/projects");
    expect(wrapper.text()).toContain("Ops Project");
    expect(wrapper.text()).toContain("Auth Project");
  });

  it("selects a project from ProjectsView and persists the remembered project id", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return {
            access_token: "remote-token",
            session: remoteSessionFixture
          };
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      }
    );

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="project-open-project-auth"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-auth/tasks"
    );
    expect(loadDesktopConnectionProfile().projectId).toBe("project-auth");
    expect(wrapper.text()).toContain("Task Create");
  });

  it("reuses the remembered project on a later remote login", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return {
            access_token: "remote-token",
            session: remoteSessionFixture
          };
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      }
    );

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();
    await wrapper.get('[data-testid="project-open-project-auth"]').trigger("click");
    await flushPromises();

    await router.push("/connections");
    await flushPromises();
    await wrapper.get('[data-testid="remote-logout"]').trigger("click");
    await flushPromises();

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-auth/tasks"
    );
    expect(wrapper.text()).toContain("Task Create");
  });

  it("clears a stale remembered project and falls back to ProjectsView", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let availableProjects: readonly (typeof remoteProjectsFixture)[number][] =
      remoteProjectsFixture;

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return {
            access_token: "remote-token",
            session: remoteSessionFixture
          };
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      () => availableProjects
    );

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();
    await wrapper.get('[data-testid="project-open-project-auth"]').trigger("click");
    await flushPromises();

    await router.push("/connections");
    await flushPromises();
    await wrapper.get('[data-testid="remote-logout"]').trigger("click");
    await flushPromises();

    availableProjects = [remoteProjectsFixture[0]];

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe("/workspaces/workspace-alpha/projects");
    expect(loadDesktopConnectionProfile().projectId).toBeUndefined();
    expect(wrapper.text()).toContain("Ops Project");
  });

  it("logs out from remote mode and returns to the read-only connections surface", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(() => authState, {
      async login() {
        authState = "authenticated";
        return {
          access_token: "remote-token",
          session: remoteSessionFixture
        };
      },
      async getCurrentSession() {
        return remoteSessionFixture;
      },
      async logout() {
        authState = "auth_required";
      }
    });

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();
    await router.push("/connections");
    await flushPromises();

    await wrapper.get('[data-testid="remote-logout"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe("/connections");
    expect(wrapper.text()).toContain("auth_required");
    expect(wrapper.text()).toContain("Connect Remote Hub");
  });

  it("surfaces token expiry separately and keeps approval actions disabled", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "token_expired";

    const { wrapper, router } = await mountRemoteShell(() => authState, {
      async login() {
        authState = "authenticated";
        return {
          access_token: "remote-token",
          session: remoteSessionFixture
        };
      },
      async getCurrentSession() {
        if (authState === "token_expired") {
          throw new Error("token expired");
        }
        return remoteSessionFixture;
      },
      async logout() {
        authState = "auth_required";
      }
    });

    expect(wrapper.text()).toContain("token_expired");
    expect(wrapper.text()).toContain("Session expired");

    await router.push("/workspaces/workspace-alpha/inbox");
    await flushPromises();

    expect(
      wrapper.get('[data-testid="workspace-approve-approval-1"]').attributes("disabled")
    ).toBeDefined();
  });

  it("switches back to local mode without regressing the demo workbench path", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(() => authState, {
      async login() {
        authState = "authenticated";
        return {
          access_token: "remote-token",
          session: remoteSessionFixture
        };
      },
      async getCurrentSession() {
        return remoteSessionFixture;
      },
      async logout() {
        authState = "auth_required";
      }
    });

    await wrapper.get('[data-testid="connection-mode"]').setValue("local");
    await wrapper.get('[data-testid="connection-apply"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe("/workspaces/demo/projects/demo/tasks");
    expect(wrapper.text()).toContain("Task Create");
    expect(wrapper.text()).toContain("local");
  });
});
