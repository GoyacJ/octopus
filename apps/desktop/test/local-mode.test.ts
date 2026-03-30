import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { createLocalHubClient, type LocalHubTransport } from "@octopus/hub-client";
import { LOCAL_HUB_TRANSPORT } from "@octopus/schema-ts";

import AppShell from "../src/App.vue";
import { createDesktopPlugins } from "../src/app";
import {
  clearWindowLocalHubTransport,
  registerTauriLocalHubTransport,
  toTauriInvokeCommand
} from "../src/tauri-local-bridge";

const { invokeMock, listenMock, unlistenMock } = vi.hoisted(() => {
  const invokeMock = vi.fn();
  const unlistenMock = vi.fn();
  const listenMock = vi.fn(
    async (
      _event: string,
      handler: (event: { payload: unknown }) => void
    ) => {
      handler({
        payload: {
          event_type: "hub.connection.updated",
          sequence: 1,
          occurred_at: "2026-03-28T10:00:00Z",
          payload: {
            mode: "local",
            state: "connected",
            auth_state: "authenticated",
            active_server_count: 0,
            healthy_server_count: 0,
            servers: [],
            last_refreshed_at: "2026-03-28T10:00:00Z"
          }
        }
      });
      return unlistenMock;
    }
  );

  return {
    invokeMock,
    listenMock,
    unlistenMock
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock
}));

const projectContextFixture = {
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

const capabilityResolutionFixture = [
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

const connectionStatusFixture = {
  mode: "local",
  state: "connected",
  auth_state: "authenticated",
  active_server_count: 0,
  healthy_server_count: 0,
  servers: [],
  last_refreshed_at: "2026-03-28T10:00:00Z"
} as const;

function createWorkspaceTransport(): LocalHubTransport {
  return {
    async invoke(command) {
      switch (command) {
        case LOCAL_HUB_TRANSPORT.commands.get_project_context:
          return projectContextFixture;
        case LOCAL_HUB_TRANSPORT.commands.list_capability_visibility:
          return capabilityResolutionFixture;
        case LOCAL_HUB_TRANSPORT.commands.get_connection_status:
          return connectionStatusFixture;
        case LOCAL_HUB_TRANSPORT.commands.list_inbox_items:
        case LOCAL_HUB_TRANSPORT.commands.list_notifications:
        case LOCAL_HUB_TRANSPORT.commands.list_automations:
          return [];
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    },
    async listen() {
      return async () => {};
    }
  };
}

describe("desktop tauri local bridge", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockClear();
    unlistenMock.mockClear();
    clearWindowLocalHubTransport();
  });

  it("registers the window local transport through tauri invoke and event APIs", async () => {
    invokeMock.mockResolvedValue(projectContextFixture);

    const transport = await registerTauriLocalHubTransport();
    expect(window.__OCTOPUS_LOCAL_HUB__).toBe(transport);

    await transport?.invoke(LOCAL_HUB_TRANSPORT.commands.get_project_context, {
      workspaceId: "demo",
      projectId: "demo"
    });
    expect(invokeMock).toHaveBeenCalledWith(
      toTauriInvokeCommand(LOCAL_HUB_TRANSPORT.commands.get_project_context),
      {
        workspaceId: "demo",
        projectId: "demo"
      }
    );

    const receivedPayloads: unknown[] = [];
    const unsubscribe = await transport?.listen(
      LOCAL_HUB_TRANSPORT.event_channel,
      (payload) => {
        receivedPayloads.push(payload);
      }
    );

    expect(listenMock).toHaveBeenCalledWith(
      LOCAL_HUB_TRANSPORT.event_channel,
      expect.any(Function)
    );
    expect(receivedPayloads).toHaveLength(1);

    await unsubscribe?.();
    expect(unlistenMock).toHaveBeenCalledTimes(1);
  });
});

describe("desktop local mode trigger restrictions", () => {
  it("disables unsupported external-ingress trigger types in local mode", async () => {
    const client = createLocalHubClient(createWorkspaceTransport());
    const { pinia, router } = createDesktopPlugins(client, true);

    await router.push("/workspaces/demo/projects/demo/tasks");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();

    const triggerOptions = wrapper
      .findAll("select option")
      .map((option) => ({
        value: option.attributes("value"),
        disabled: (option.element as HTMLOptionElement).disabled
      }));

    expect(
      triggerOptions.find((option) => option.value === "manual_event")?.disabled
    ).toBe(false);
    expect(
      triggerOptions.find((option) => option.value === "cron")?.disabled
    ).toBe(false);
    expect(
      triggerOptions.find((option) => option.value === "webhook")?.disabled
    ).toBe(true);
    expect(
      triggerOptions.find((option) => option.value === "mcp_event")?.disabled
    ).toBe(true);
    expect(wrapper.text()).toContain(
      "Local host only supports manual_event and cron"
    );
  });
});
