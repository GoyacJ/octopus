import { flushPromises } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LOCAL_HUB_TRANSPORT } from "@octopus/schema-ts";

import {
  clearWindowLocalHubTransport,
  toTauriInvokeCommand
} from "../src/tauri-local-bridge";

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

function responseForCommand(command: string): unknown {
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
}

describe("desktop bootstrap smoke", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockClear();
    unlistenMock.mockClear();
    invokeMock.mockImplementation(async (normalizedCommand: string) => {
      const command = Object.values(LOCAL_HUB_TRANSPORT.commands).find(
        (candidate) => toTauriInvokeCommand(candidate) === normalizedCommand
      );
      if (!command) {
        throw new Error(`unexpected tauri invoke command: ${normalizedCommand}`);
      }
      return responseForCommand(command);
    });
    document.body.innerHTML = '<div id="app"></div>';
    window.history.replaceState({}, "", "/");
    clearWindowLocalHubTransport();
    (
      globalThis as { __OCTOPUS_DISABLE_AUTO_BOOTSTRAP__?: boolean }
    ).__OCTOPUS_DISABLE_AUTO_BOOTSTRAP__ = true;
    vi.resetModules();
  });

  afterEach(() => {
    clearWindowLocalHubTransport();
    delete (
      globalThis as { __OCTOPUS_DISABLE_AUTO_BOOTSTRAP__?: boolean }
    ).__OCTOPUS_DISABLE_AUTO_BOOTSTRAP__;
    document.body.innerHTML = "";
  });

  it("boots into the demo workspace route through the tauri local bridge", async () => {
    const { bootstrap } = await import("../src/main");

    await bootstrap();
    await flushPromises();

    expect(window.__OCTOPUS_LOCAL_HUB__).toBeDefined();
    expect(invokeMock).toHaveBeenCalledWith(
      toTauriInvokeCommand(LOCAL_HUB_TRANSPORT.commands.get_project_context),
      {
        workspaceId: "demo",
        projectId: "demo"
      }
    );
    expect(window.location.pathname).toBe("/workspaces/demo/projects/demo");
    expect(document.body.textContent).toContain("Demo Workspace");
    expect(document.body.textContent).not.toContain(
      "No local Hub transport bridge is registered"
    );
  });
});
