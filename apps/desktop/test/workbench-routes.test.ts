import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import { createLocalHubClient, type LocalHubTransport } from "@octopus/hub-client";

import AppShell from "../src/App.vue";
import { createDesktopPlugins } from "../src/app";

const projectContextFixture = {
  workspace: {
    id: "workspace-alpha",
    slug: "workspace-alpha",
    display_name: "Workspace Alpha",
    created_at: "2026-03-26T10:00:00Z",
    updated_at: "2026-03-26T10:00:00Z"
  },
  project: {
    id: "project-slice1",
    workspace_id: "workspace-alpha",
    slug: "project-slice1",
    display_name: "Project Slice 1",
    created_at: "2026-03-26T10:00:00Z",
    updated_at: "2026-03-26T10:00:00Z"
  }
};

const demoContextFixture = {
  workspace: {
    id: "demo",
    slug: "demo",
    display_name: "Demo Workspace",
    created_at: "2026-03-28T08:00:00Z",
    updated_at: "2026-03-28T08:00:00Z"
  },
  project: {
    id: "demo",
    workspace_id: "demo",
    slug: "demo",
    display_name: "Demo Project",
    created_at: "2026-03-28T08:00:00Z",
    updated_at: "2026-03-28T08:00:00Z"
  }
};

const capabilityResolutionFixture = [
  {
    descriptor: {
      id: "capability-write-note",
      slug: "capability-write-note",
      kind: "core",
      source: "octopus-runtime",
      platform: "local",
      risk_level: "low",
      requires_approval: false,
      input_schema_uri: null,
      output_schema_uri: null,
      fallback_capability_id: null,
      trust_level: "trusted",
      created_at: "2026-03-26T10:00:00Z",
      updated_at: "2026-03-26T10:00:00Z"
    },
    scope_ref: "workspace:workspace-alpha/project:project-slice1",
    execution_state: "executable",
    reason_code: "within_budget",
    explanation:
      "Executable because the capability is bound, granted, and within the current budget."
  }
];

const runSummaryFixture = {
  id: "run-1",
  task_id: "task-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  title: "Write note",
  run_type: "task",
  status: "completed",
  approval_request_id: null,
  attempt_count: 1,
  max_attempts: 2,
  last_error: null,
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:01Z",
  started_at: "2026-03-26T10:00:00Z",
  completed_at: "2026-03-26T10:00:01Z",
  terminated_at: null
} as const;

const approvalFixture = {
  id: "approval-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
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
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:00Z"
} as const;

const inboxApprovalFixture = {
  id: "inbox-approval-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  run_id: "run-approval",
  approval_request_id: "approval-1",
  item_type: "approval_request",
  target_ref: "run:run-approval",
  status: "open",
  dedupe_key: "inbox:approval-1",
  title: "Execution approval required",
  message: "A governed run needs approval before execution.",
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:00Z",
  resolved_at: null
} as const;

const notificationFixture = {
  id: "notification-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  run_id: "run-approval",
  approval_request_id: "approval-1",
  target_ref: "run:run-approval",
  status: "pending",
  dedupe_key: "notification:approval-1",
  title: "Approval pending",
  message: "A run is waiting for approval.",
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:00Z"
} as const;

const hubConnectionStatusFixture = {
  mode: "local",
  state: "connected",
  auth_state: "authenticated",
  active_server_count: 0,
  healthy_server_count: 0,
  servers: [],
  last_refreshed_at: "2026-03-26T10:00:01Z"
};

function buildTransport(): LocalHubTransport {
  return {
    async invoke(command, payload) {
      switch (command) {
        case "hub:get_project_context": {
          const workspaceId = (payload as { workspaceId?: string } | undefined)
            ?.workspaceId;
          return workspaceId === "demo" ? demoContextFixture : projectContextFixture;
        }
        case "hub:list_capability_visibility":
          return capabilityResolutionFixture;
        case "hub:list_runs":
          return [runSummaryFixture];
        case "hub:list_inbox_items":
          return [inboxApprovalFixture];
        case "hub:list_notifications":
          return [notificationFixture];
        case "hub:get_approval_request":
          return approvalFixture;
        case "hub:get_connection_status":
          return hubConnectionStatusFixture;
        case "hub:list_automations":
          return [];
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    },
    async listen() {
      return () => undefined;
    }
  };
}

async function mountAt(path: string) {
  const client = createLocalHubClient(buildTransport());
  const { pinia, router } = createDesktopPlugins(client, true);
  await router.push(path);
  await router.isReady();

  const wrapper = mount(AppShell, {
    global: {
      plugins: [pinia, router]
    }
  });

  await flushPromises();

  return { wrapper, router };
}

describe("desktop task workbench routes", () => {
  it("redirects the default entry to the demo tasks route", async () => {
    const { router } = await mountAt("/");

    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/demo/projects/demo/tasks"
    );
  });

  it("renders the tasks route as a focused task surface", async () => {
    const { wrapper } = await mountAt(
      "/workspaces/workspace-alpha/projects/project-slice1/tasks"
    );

    expect(wrapper.text()).toContain("Task Create");
    expect(wrapper.text()).not.toContain("Approval Inbox");
    expect(wrapper.text()).not.toContain("Hub Connections");
  });

  it("renders the runs route with recent project runs", async () => {
    const { wrapper } = await mountAt(
      "/workspaces/workspace-alpha/projects/project-slice1/runs"
    );

    expect(wrapper.text()).toContain("Recent Runs");
    expect(wrapper.text()).toContain("Write note");
    expect(wrapper.text()).toContain("completed");
  });

  it("renders the inbox route as the action surface", async () => {
    const { wrapper } = await mountAt("/workspaces/workspace-alpha/inbox");

    expect(wrapper.text()).toContain("Approval Inbox");
    expect(wrapper.text()).toContain("Execution approval required");
    expect(wrapper.text()).toContain("Approve");
  });

  it("renders the notifications route separately from inbox actions", async () => {
    const { wrapper } = await mountAt("/workspaces/workspace-alpha/notifications");

    expect(wrapper.text()).toContain("Notifications");
    expect(wrapper.text()).toContain("Approval pending");
    expect(wrapper.text()).not.toContain("Approve");
  });

  it("renders the connections route with explicit hub state", async () => {
    const { wrapper } = await mountAt("/connections");

    expect(wrapper.text()).toContain("Hub Connections");
    expect(wrapper.text()).toContain("connected");
    expect(wrapper.text()).toContain("authenticated");
  });
});
