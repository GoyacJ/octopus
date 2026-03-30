import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  HubClientAuthError,
  HubClientTransportError,
  createLocalHubClient,
  type HubClient,
  type HubAuthError,
  type HubSession,
  type LocalHubTransport,
  type RemoteHubAuthClient
} from "@octopus/hub-client";

import AppShell from "../src/App.vue";
import { createDesktopPlugins } from "../src/app";
import {
  createConfiguredDesktopHubClient,
  configureDesktopConnectionRuntime,
  initializeDesktopConnection,
  loadDesktopConnectionProfile,
  persistDesktopConnectionProfile,
  resetDesktopConnectionRuntime,
  resolveDesktopEntryRoute,
  useConnectionStore,
  type DesktopConnectionProfile,
  type DesktopConnectionRuntimeOptions,
  type PersistedRemoteSession
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
  expires_at: "2099-03-29T12:00:00Z"
} as const;

const remoteRefreshTokenFixture = "refresh-token" as const;
const remoteRefreshExpiresAtFixture = "2099-04-05T12:00:00Z" as const;

function createRemoteLoginResponse(overrides: Record<string, unknown> = {}) {
  return {
    access_token: "remote-token",
    refresh_token: remoteRefreshTokenFixture,
    refresh_expires_at: remoteRefreshExpiresAtFixture,
    session: remoteSessionFixture,
    ...overrides
  };
}

function createRemoteRefreshResponse(overrides: Record<string, unknown> = {}) {
  return {
    access_token: "remote-token-next",
    refresh_token: "refresh-token-next",
    refresh_expires_at: "2099-04-06T12:00:00Z",
    session: remoteSessionFixture,
    ...overrides
  };
}

function createPersistedRemoteSession(
  overrides: Partial<PersistedRemoteSession> = {}
): PersistedRemoteSession {
  const session = overrides.session ?? remoteSessionFixture;
  return {
    baseUrl: remoteProfile.baseUrl,
    workspaceId: remoteProfile.workspaceId,
    email: remoteProfile.email,
    accessToken: "remote-token",
    refreshToken: remoteRefreshTokenFixture,
    refreshTokenExpiresAt: remoteRefreshExpiresAtFixture,
    session,
    ...overrides
  };
}

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

const remoteModelProviderFixture = {
  id: "provider-openai",
  display_name: "OpenAI",
  provider_family: "openai",
  status: "active",
  default_base_url: "https://api.openai.com/v1",
  protocol_families: ["openai_responses_compatible"],
  created_at: "2026-03-30T10:00:00Z",
  updated_at: "2026-03-30T10:00:00Z"
} as const;

const remoteModelCatalogItemFixture = {
  id: "catalog-openai-gpt-5-4",
  provider_id: "provider-openai",
  model_key: "openai:gpt-5.4",
  provider_model_id: "gpt-5.4",
  release_channel: "ga",
  modality_tags: ["text_in", "text_out", "image_in"],
  feature_tags: ["supports_structured_output", "supports_builtin_web_search"],
  context_window: 1050000,
  max_output_tokens: 128000,
  created_at: "2026-03-30T10:00:00Z",
  updated_at: "2026-03-30T10:00:00Z"
} as const;

const remoteModelProfileFixture = {
  id: "profile-default-reasoning",
  display_name: "Default Reasoning",
  scope_ref: "tenant:workspace-alpha",
  primary_model_key: "openai:gpt-5.4",
  fallback_model_keys: ["openai:gpt-5.4-mini"],
  created_at: "2026-03-30T10:00:00Z",
  updated_at: "2026-03-30T10:00:00Z"
} as const;

const remoteWorkspaceModelPolicyFixture = {
  id: "tenant-policy-workspace-alpha",
  tenant_id: "workspace-alpha",
  allowed_model_keys: ["openai:gpt-5.4", "openai:gpt-5.4-mini"],
  denied_model_keys: [],
  allowed_provider_ids: ["provider-openai"],
  denied_release_channels: ["experimental"],
  require_approval_for_preview: true,
  created_at: "2026-03-30T10:00:00Z",
  updated_at: "2026-03-30T10:00:00Z"
} as const;

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
  currentProjects: () => readonly (typeof remoteProjectsFixture)[number][],
  currentConnectionState: () => "connected" | "disconnected" = () => "connected",
  currentModelReadBehavior: () =>
    "ok" | "empty" | "forbidden" | "transport_error" | "auth_required" = () => "ok"
): HubClient {
  function modelReadResult<T>(value: T): T {
    switch (currentModelReadBehavior()) {
      case "ok":
        return value;
      case "empty":
        return ([] as T[])[0] as T;
      case "forbidden":
        throw new HubClientAuthError(403, {
          error: "workspace model governance is forbidden",
          error_code: "workspace_forbidden",
          auth_state: "authenticated"
        } satisfies HubAuthError);
      case "transport_error":
        throw new HubClientTransportError("remote hub unavailable");
      case "auth_required":
        throw new HubClientAuthError(401, {
          error: "authentication required",
          error_code: "auth_required",
          auth_state: "auth_required"
        } satisfies HubAuthError);
    }
  }

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
            state: currentConnectionState(),
            auth_state: currentAuthState(),
            active_server_count: 1,
            healthy_server_count:
              currentConnectionState() === "connected" &&
              currentAuthState() === "authenticated"
                ? 1
                : 0,
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
        case "hub:list_model_providers":
          return currentModelReadBehavior() === "empty"
            ? []
            : modelReadResult([remoteModelProviderFixture]);
        case "hub:list_model_catalog_items":
          return currentModelReadBehavior() === "empty"
            ? []
            : modelReadResult([remoteModelCatalogItemFixture]);
        case "hub:list_model_profiles":
          return currentModelReadBehavior() === "empty"
            ? []
            : modelReadResult([remoteModelProfileFixture]);
        case "hub:get_workspace_model_policy":
          return currentModelReadBehavior() === "empty"
            ? null
            : modelReadResult(remoteWorkspaceModelPolicyFixture);
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

interface MountRemoteShellOptions {
  currentProjects?: () => readonly (typeof remoteProjectsFixture)[number][];
  currentConnectionState?: () => "connected" | "disconnected";
  currentModelReadBehavior?: () =>
    "ok" | "empty" | "forbidden" | "transport_error" | "auth_required";
  initializeConnection?: boolean;
  runtime?: Partial<DesktopConnectionRuntimeOptions>;
  seedProfile?: Partial<DesktopConnectionProfile> | null;
}

async function mountRemoteShell(
  authState: () => "auth_required" | "authenticated" | "token_expired",
  authClient: Partial<RemoteHubAuthClient> &
    Pick<RemoteHubAuthClient, "login" | "getCurrentSession" | "logout">,
  options: MountRemoteShellOptions = {}
) {
  if (options.seedProfile !== null) {
    persistDesktopConnectionProfile(options.seedProfile ?? remoteProfile);
  }

  configureDesktopConnectionRuntime({
    storage: window.localStorage,
    createLocalClient: () => createLocalWorkbenchClient(),
    createRemoteClient: () =>
      createRemoteWorkbenchClient(
        authState,
        options.currentProjects ?? (() => remoteProjectsFixture),
        options.currentConnectionState,
        options.currentModelReadBehavior
      ),
    createRemoteAuthClient: () => ({
      async refreshSession() {
        throw new Error("refreshSession not configured for this test");
      },
      ...authClient
    }),
    ...options.runtime
  });

  if (options.initializeConnection) {
    await initializeDesktopConnection();
  }

  const { pinia, router } = createDesktopPlugins(createConfiguredDesktopHubClient(), true, {
    defaultRoute: resolveDesktopEntryRoute()
  });

  await router.push("/");
  await router.isReady();

  const wrapper = mount(AppShell, {
    global: {
      plugins: [pinia, router]
    }
  });

  await flushPromises();

  return { wrapper, router, pinia };
}

function createAuthError(details: Partial<HubAuthError>): HubClientAuthError {
  return new HubClientAuthError(401, {
    error: details.error ?? "authentication failed",
    error_code: details.error_code ?? "auth_required",
    auth_state: details.auth_state ?? "auth_required"
  });
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
        return createRemoteLoginResponse();
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
        return createRemoteLoginResponse();
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

  it("persists the remote session cache after login", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let persistedSession: PersistedRemoteSession | null = null;

    const { wrapper } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: null,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        }
      }
    );

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();

    expect(persistedSession).toEqual(
      createPersistedRemoteSession({
        workspaceId: remoteSessionFixture.workspace_id,
        email: remoteSessionFixture.email
      })
    );
  });

  it("selects a project from ProjectsView and persists the remembered project id", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
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
          return createRemoteLoginResponse();
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

  it("restores the cached remote session on app restart and reuses the remembered project route", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let persistedSession: PersistedRemoteSession | null = null;

    const firstShell = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        }
      }
    );

    await firstShell.wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await firstShell.wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();
    await firstShell.wrapper.get('[data-testid="project-open-project-auth"]').trigger("click");
    await flushPromises();
    firstShell.wrapper.unmount();

    resetDesktopConnectionRuntime();

    const restartedShell = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        runtime: {
          async loadPersistedRemoteSession(profile) {
            expect(profile.workspaceId).toBe("workspace-alpha");
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        },
        seedProfile: null
      }
    );

    expect(restartedShell.router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-auth/tasks"
    );
    expect(restartedShell.wrapper.text()).toContain("Task Create");
  });

  it("refreshes expired cached access before resolving the remote entry route", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "authenticated";
    let persistedSession: PersistedRemoteSession | null = createPersistedRemoteSession({
      session: {
        ...remoteSessionFixture,
        expires_at: "2020-03-29T12:00:00Z"
      }
    });

    persistDesktopConnectionProfile({
      ...remoteProfile,
      projectId: "project-auth"
    });

    const refreshed = createRemoteRefreshResponse();
    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          return createRemoteLoginResponse();
        },
        async refreshSession() {
          return refreshed;
        },
        async getCurrentSession() {
          throw new Error("startup restore should refresh before session validation");
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        },
        seedProfile: null
      }
    );

    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-auth/tasks"
    );
    expect(wrapper.text()).toContain("Task Create");
    expect(persistedSession).toEqual(
      createPersistedRemoteSession({
        accessToken: "remote-token-next",
        refreshToken: "refresh-token-next",
        refreshTokenExpiresAt: "2099-04-06T12:00:00Z"
      })
    );
  });

  it("clears cached auth state when startup refresh fails with a hard auth error", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let persistedSession: PersistedRemoteSession | null = createPersistedRemoteSession({
      session: {
        ...remoteSessionFixture,
        expires_at: "2020-03-29T12:00:00Z"
      }
    });

    persistDesktopConnectionProfile({
      ...remoteProfile,
      projectId: "project-auth"
    });

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          return createRemoteLoginResponse();
        },
        async refreshSession() {
          authState = "token_expired";
          throw createAuthError({
            error: "refresh expired",
            error_code: "token_expired",
            auth_state: "token_expired"
          });
        },
        async getCurrentSession() {
          throw new Error("startup restore should not use access-session validation");
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        },
        seedProfile: null
      }
    );

    expect(router.currentRoute.value.fullPath).toBe("/connections");
    expect(wrapper.text()).toContain("token_expired");
    expect(persistedSession).toBeNull();
  });

  it("preserves cached refresh state when startup refresh only hits a transport failure", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let persistedSession: PersistedRemoteSession | null = createPersistedRemoteSession({
      session: {
        ...remoteSessionFixture,
        expires_at: "2020-03-29T12:00:00Z"
      }
    });

    persistDesktopConnectionProfile({
      ...remoteProfile,
      projectId: "project-auth"
    });

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          return createRemoteLoginResponse();
        },
        async refreshSession() {
          throw new HubClientTransportError("remote hub unavailable");
        },
        async getCurrentSession() {
          throw new Error("startup restore should not use access-session validation");
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        currentConnectionState: () => "disconnected",
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        },
        seedProfile: null
      }
    );

    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-auth/tasks"
    );
    await router.push("/connections");
    await flushPromises();
    expect(wrapper.text()).toContain("read-only");
    expect(persistedSession).not.toBeNull();
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
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        currentProjects: () => availableProjects
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
    let persistedSession: PersistedRemoteSession | null = null;

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        }
      }
    );

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
    expect(persistedSession).toBeNull();
  });

  it("surfaces token expiry separately and keeps approval actions disabled", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "token_expired";

    const { wrapper, router } = await mountRemoteShell(() => authState, {
      async login() {
        authState = "authenticated";
        return createRemoteLoginResponse();
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
    let persistedSession: PersistedRemoteSession | null = null;

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        }
      }
    );

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();

    await router.push("/connections");
    await flushPromises();
    await wrapper.get('[data-testid="connection-mode"]').setValue("local");
    await wrapper.get('[data-testid="connection-apply"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe("/workspaces/demo/projects/demo/tasks");
    expect(wrapper.text()).toContain("Task Create");
    expect(wrapper.text()).toContain("local");
    expect(persistedSession).toBeNull();
  });

  it("clears the cached remote session when restore hits an auth-required response", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let persistedSession: PersistedRemoteSession | null = createPersistedRemoteSession();

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          authState = "token_expired";
          throw createAuthError({
            error: "session expired",
            error_code: "token_expired",
            auth_state: "token_expired"
          });
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        }
      }
    );

    expect(router.currentRoute.value.fullPath).toBe("/connections");
    expect(wrapper.text()).toContain("token_expired");
    expect(persistedSession).toBeNull();
  });

  it("keeps the cached session summary when restore only hits a transport failure", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let persistedSession: PersistedRemoteSession | null = createPersistedRemoteSession();

    persistDesktopConnectionProfile({
      ...remoteProfile,
      projectId: "project-auth"
    });

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          throw new HubClientTransportError("remote hub unavailable");
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        currentConnectionState: () => "disconnected",
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        },
        seedProfile: null
      }
    );

    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-auth/tasks"
    );
    await router.push("/connections");
    await flushPromises();

    expect(wrapper.text()).toContain("read-only");
    expect(persistedSession).not.toBeNull();
  });

  it("shows a global degraded banner after cached restore lands on a workbench route", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let persistedSession: PersistedRemoteSession | null = createPersistedRemoteSession();

    persistDesktopConnectionProfile({
      ...remoteProfile,
      projectId: "project-auth"
    });

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          throw new HubClientTransportError("remote hub unavailable");
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        currentConnectionState: () => "disconnected",
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        },
        seedProfile: null
      }
    );

    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-auth/tasks"
    );
    expect(
      wrapper.get('[data-testid="connection-banner"]').attributes("data-kind")
    ).toBe("restored_but_disconnected");
  });

  it("shows the memory-only warning in the global shell after remote login", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: null,
              storageAvailable: false,
              warning:
                "Secure session storage is unavailable. Remote sign-in will stay memory-only."
            };
          },
          async savePersistedRemoteSession() {
            return {
              storageAvailable: false,
              warning:
                "Secure session storage is unavailable. Remote sign-in will stay memory-only."
            };
          },
          async clearPersistedRemoteSession() {
            return {
              storageAvailable: false,
              warning:
                "Secure session storage is unavailable. Remote sign-in will stay memory-only."
            };
          }
        }
      }
    );

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe("/workspaces/workspace-alpha/projects");
    expect(
      wrapper.get('[data-testid="connection-banner"]').attributes("data-kind")
    ).toBe("memory_only_storage");
  });

  it("clears the global degraded banner after route-entry refresh sees the connection recover", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let connectionState: "connected" | "disconnected" = "disconnected";
    let persistedSession: PersistedRemoteSession | null = createPersistedRemoteSession();

    persistDesktopConnectionProfile({
      ...remoteProfile,
      projectId: "project-auth"
    });

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          throw new HubClientTransportError("remote hub unavailable");
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        currentConnectionState: () => connectionState,
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        },
        seedProfile: null
      }
    );

    expect(
      wrapper.get('[data-testid="connection-banner"]').attributes("data-kind")
    ).toBe("restored_but_disconnected");

    authState = "authenticated";
    connectionState = "connected";

    await router.push("/workspaces/workspace-alpha/projects");
    await flushPromises();

    expect(wrapper.find('[data-testid="connection-banner"]').exists()).toBe(false);
  });

  it("clears degraded session context when switching to a different remote workspace", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";
    let persistedSession: PersistedRemoteSession | null = createPersistedRemoteSession();

    persistDesktopConnectionProfile({
      ...remoteProfile,
      projectId: "project-auth"
    });

    const { wrapper, router, pinia } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          throw new HubClientTransportError("remote hub unavailable");
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        initializeConnection: true,
        currentConnectionState: () => "disconnected",
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: persistedSession,
              storageAvailable: true
            };
          },
          async savePersistedRemoteSession(session) {
            persistedSession = session;
            return {
              storageAvailable: true
            };
          },
          async clearPersistedRemoteSession() {
            persistedSession = null;
            return {
              storageAvailable: true
            };
          }
        },
        seedProfile: null
      }
    );

    expect(
      wrapper.get('[data-testid="connection-banner"]').attributes("data-kind")
    ).toBe("restored_but_disconnected");

    const connection = useConnectionStore(pinia);
    await connection.applyProfile({
      mode: "remote",
      baseUrl: remoteProfile.baseUrl,
      workspaceId: "workspace-beta",
      email: "ops@octopus.local"
    });
    await router.push("/workspaces/workspace-beta/projects");
    await flushPromises();

    expect(loadDesktopConnectionProfile().workspaceId).toBe("workspace-beta");
    expect(loadDesktopConnectionProfile().projectId).toBeUndefined();
    expect(
      wrapper.get('[data-testid="connection-banner"]').attributes("data-kind")
    ).toBe("auth_required");
  });

  it("falls back to memory-only login when the secure session store is unavailable", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "auth_required";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          authState = "authenticated";
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        runtime: {
          async loadPersistedRemoteSession() {
            return {
              session: null,
              storageAvailable: false,
              warning:
                "Secure session storage is unavailable. Remote sign-in will stay memory-only."
            };
          },
          async savePersistedRemoteSession() {
            return {
              storageAvailable: false,
              warning:
                "Secure session storage is unavailable. Remote sign-in will stay memory-only."
            };
          },
          async clearPersistedRemoteSession() {
            return {
              storageAvailable: false,
              warning:
                "Secure session storage is unavailable. Remote sign-in will stay memory-only."
            };
          }
        }
      }
    );

    await wrapper.get('[data-testid="remote-password"]').setValue(
      "octopus-bootstrap-password"
    );
    await wrapper.get('[data-testid="remote-login"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe("/workspaces/workspace-alpha/projects");
    await router.push("/connections");
    await flushPromises();
    expect(wrapper.text()).toContain("memory-only");
  });

  it("renders the remote models page when authenticated and connected", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "authenticated";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      }
    );

    await router.push("/workspaces/workspace-alpha/models");
    await flushPromises();

    expect(wrapper.text()).toContain("Models");
    expect(wrapper.text()).toContain("OpenAI");
    expect(wrapper.text()).toContain("Preview models require approval");
  });

  it("shows explicit remote offline copy on the models page when transport is unavailable", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "authenticated";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        currentConnectionState: () => "disconnected",
        currentModelReadBehavior: () => "transport_error"
      }
    );

    await router.push("/workspaces/workspace-alpha/models");
    await flushPromises();

    expect(wrapper.text()).toContain("Models");
    expect(wrapper.text()).toContain("Remote hub is offline");
    expect(wrapper.text()).toContain("Read-only");
  });

  it("shows forbidden copy on the models page when workspace model governance is not accessible", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "authenticated";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          return remoteSessionFixture;
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        currentModelReadBehavior: () => "forbidden"
      }
    );

    await router.push("/workspaces/workspace-alpha/models");
    await flushPromises();

    expect(wrapper.text()).toContain("Models");
    expect(wrapper.text()).toContain(
      "You do not have permission to read workspace model governance"
    );
    expect(wrapper.text()).toContain("Read-only");
  });

  it("shows auth-required read-only copy on the models page when the remote session expires", async () => {
    let authState: "auth_required" | "authenticated" | "token_expired" = "token_expired";

    const { wrapper, router } = await mountRemoteShell(
      () => authState,
      {
        async login() {
          return createRemoteLoginResponse();
        },
        async getCurrentSession() {
          throw new HubClientAuthError(401, {
            error: "token expired",
            error_code: "token_expired",
            auth_state: "token_expired"
          });
        },
        async logout() {
          authState = "auth_required";
        }
      },
      {
        currentModelReadBehavior: () => "auth_required"
      }
    );

    await router.push("/workspaces/workspace-alpha/models");
    await flushPromises();

    expect(wrapper.text()).toContain("Models");
    expect(wrapper.text()).toContain("Sign in again to load workspace model governance");
    expect(wrapper.text()).toContain("Read-only");
  });
});
