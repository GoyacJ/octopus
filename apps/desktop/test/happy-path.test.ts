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

const taskFixture = {
  id: "task-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  source_kind: "manual",
  automation_id: null,
  title: "Write note",
  instruction: "Emit a deterministic artifact",
  action: {
    kind: "emit_text",
    content: "hello"
  },
  capability_id: "capability-write-note",
  estimated_cost: 1,
  idempotency_key: "task-1",
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:00Z"
};

const runDetailFixture = {
  run: {
    id: "run-1",
    task_id: "task-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    automation_id: null,
    trigger_delivery_id: null,
    run_type: "task",
    status: "completed",
    approval_request_id: null,
    idempotency_key: "run:task:task-1",
    attempt_count: 1,
    max_attempts: 2,
    checkpoint_seq: 3,
    resume_token: null,
    last_error: null,
    created_at: "2026-03-26T10:00:00Z",
    updated_at: "2026-03-26T10:00:01Z",
    started_at: "2026-03-26T10:00:00Z",
    completed_at: "2026-03-26T10:00:01Z",
    terminated_at: null
  },
  task: taskFixture,
  artifacts: [
    {
      id: "artifact-1",
      workspace_id: "workspace-alpha",
      project_id: "project-slice1",
      run_id: "run-1",
      task_id: "task-1",
      artifact_type: "execution_output",
      content: "hello",
      provenance_source: "builtin",
      source_descriptor_id: "builtin:emit_text",
      source_invocation_id: null,
      trust_level: "trusted",
      knowledge_gate_status: "eligible",
      created_at: "2026-03-26T10:00:01Z",
      updated_at: "2026-03-26T10:00:01Z"
    }
  ],
  audits: [],
  traces: [],
  approvals: [],
  inbox_items: [],
  notifications: [],
  policy_decisions: [],
  knowledge_candidates: [
    {
      id: "candidate-1",
      knowledge_space_id: "knowledge-space-1",
      source_run_id: "run-1",
      source_task_id: "task-1",
      source_artifact_id: "artifact-1",
      capability_id: "capability-write-note",
      status: "candidate",
      content: "hello",
      provenance_source: "builtin",
      source_trust_level: "trusted",
      dedupe_key: "knowledge_candidate:artifact:artifact-1",
      created_at: "2026-03-26T10:00:01Z",
      updated_at: "2026-03-26T10:00:01Z"
    }
  ],
  knowledge_assets: [],
  knowledge_lineage: []
};

const knowledgeDetailFixture = {
  knowledge_space: {
    id: "knowledge-space-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    owner_ref: "workspace_admin:alice",
    display_name: "Project Slice 1 Knowledge",
    created_at: "2026-03-26T10:00:00Z",
    updated_at: "2026-03-26T10:00:00Z"
  },
  candidates: runDetailFixture.knowledge_candidates,
  assets: [],
  lineage: []
};

const capabilityVisibilityFixture = [
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
    scope_ref: "project:project-slice1",
    visibility: "visible",
    reason_code: "project_scope_grant_active",
    explanation: "Visible because the project-scoped capability grant is active."
  }
];

const hubConnectionStatusFixture = {
  mode: "local",
  state: "connected",
  auth_state: "authenticated",
  active_server_count: 0,
  healthy_server_count: 0,
  servers: [],
  last_refreshed_at: "2026-03-26T10:00:01Z"
};

const manualAutomationResponseFixture = {
  automation: {
    id: "automation-manual-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    trigger_id: "trigger-manual-1",
    status: "active",
    title: "Manual automation",
    instruction: "Dispatch on demand",
    action: {
      kind: "emit_text",
      content: "manual artifact"
    },
    capability_id: "capability-write-note",
    estimated_cost: 1,
    created_at: "2026-03-26T10:00:00Z",
    updated_at: "2026-03-26T10:00:00Z"
  },
  trigger: {
    id: "trigger-manual-1",
    automation_id: "automation-manual-1",
    trigger_type: "manual_event",
    config: {},
    created_at: "2026-03-26T10:00:00Z",
    updated_at: "2026-03-26T10:00:00Z"
  },
  webhook_secret: null
};

const manualAutomationDetailFixture = {
  automation: manualAutomationResponseFixture.automation,
  trigger: manualAutomationResponseFixture.trigger,
  recent_deliveries: [
    {
      id: "delivery-manual-1",
      trigger_id: "trigger-manual-1",
      run_id: "run-manual-1",
      status: "succeeded",
      dedupe_key: "manual-dispatch-1",
      payload: {
        source: "desktop"
      },
      attempt_count: 1,
      last_error: null,
      created_at: "2026-03-26T10:05:00Z",
      updated_at: "2026-03-26T10:05:01Z"
    }
  ],
  last_run_summary: {
    id: "run-manual-1",
    task_id: "task-manual-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    title: "Manual automation",
    run_type: "automation",
    status: "completed",
    approval_request_id: null,
    attempt_count: 1,
    max_attempts: 2,
    last_error: null,
    created_at: "2026-03-26T10:05:00Z",
    updated_at: "2026-03-26T10:05:01Z",
    started_at: "2026-03-26T10:05:00Z",
    completed_at: "2026-03-26T10:05:01Z",
    terminated_at: null
  }
};

const failingAutomationSummaryFixture = {
  automation: {
    id: "automation-failing-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    trigger_id: "trigger-failing-1",
    status: "active",
    title: "Failing automation",
    instruction: "Always fail",
    action: {
      kind: "always_fail",
      message: "boom"
    },
    capability_id: "capability-write-note",
    estimated_cost: 1,
    created_at: "2026-03-26T10:10:00Z",
    updated_at: "2026-03-26T10:10:00Z"
  },
  trigger: {
    id: "trigger-failing-1",
    automation_id: "automation-failing-1",
    trigger_type: "manual_event",
    config: {},
    created_at: "2026-03-26T10:10:00Z",
    updated_at: "2026-03-26T10:10:00Z"
  },
  recent_deliveries: [
    {
      id: "delivery-failed-1",
      trigger_id: "trigger-failing-1",
      run_id: "run-failed-1",
      status: "failed",
      dedupe_key: "manual-dispatch-failed-1",
      payload: {
        source: "desktop"
      },
      attempt_count: 1,
      last_error: "boom",
      created_at: "2026-03-26T10:11:00Z",
      updated_at: "2026-03-26T10:11:01Z"
    }
  ],
  last_run_summary: {
    id: "run-failed-1",
    task_id: "task-failed-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    title: "Failing automation",
    run_type: "automation",
    status: "failed",
    approval_request_id: null,
    attempt_count: 1,
    max_attempts: 2,
    last_error: "boom",
    created_at: "2026-03-26T10:11:00Z",
    updated_at: "2026-03-26T10:11:01Z",
    started_at: "2026-03-26T10:11:00Z",
    completed_at: null,
    terminated_at: "2026-03-26T10:11:01Z"
  }
};

const automationListFixture = [
  manualAutomationDetailFixture,
  failingAutomationSummaryFixture
];

describe("desktop local happy path", () => {
  it("creates a task and shows run, artifact, and knowledge state", async () => {
    const transport: LocalHubTransport = {
      async invoke(command) {
        switch (command) {
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility":
            return capabilityVisibilityFixture;
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:list_automations":
            return [];
          case "hub:list_inbox_items":
          case "hub:list_notifications":
            return [];
          case "hub:create_task":
            return taskFixture;
          case "hub:start_task":
          case "hub:get_run_detail":
            return runDetailFixture;
          case "hub:list_artifacts":
            return runDetailFixture.artifacts;
          case "hub:get_knowledge_detail":
            return knowledgeDetailFixture;
          default:
            throw new Error(`unexpected command: ${command}`);
        }
      },
      async listen() {
        return () => undefined;
      }
    };

    const client = createLocalHubClient(transport);
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push("/workspaces/workspace-alpha/projects/project-slice1");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();
    expect(wrapper.text()).toContain("Workspace Alpha");

    await wrapper.get('[data-testid="create-start"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.fullPath).toBe("/runs/run-1");
    expect(wrapper.text()).toContain("completed");
    expect(wrapper.text()).toContain("hello");
    expect(wrapper.text()).toContain("Project Slice 1 Knowledge");
    expect(wrapper.text()).toContain("candidate-1");
  });

  it("shows token expiry separately from disconnect and falls back to read-only mode", async () => {
    const transport: LocalHubTransport = {
      async invoke(command) {
        switch (command) {
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility":
            return capabilityVisibilityFixture;
          case "hub:get_connection_status":
            return {
              ...hubConnectionStatusFixture,
              mode: "remote",
              state: "connected",
              auth_state: "token_expired"
            };
          case "hub:list_automations":
            return [];
          case "hub:list_inbox_items":
          case "hub:list_notifications":
            return [];
          default:
            throw new Error(`unexpected command: ${command}`);
        }
      },
      async listen() {
        return () => undefined;
      }
    };

    const client = createLocalHubClient(transport);
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push("/workspaces/workspace-alpha/projects/project-slice1");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();

    expect(wrapper.text()).toContain("token_expired");
    expect(wrapper.get('[data-testid="create-start"]').attributes("disabled")).toBeDefined();
  });

  it("creates an automation, opens detail, and shows manual dispatch state", async () => {
    const seenCommands: string[] = [];
    const transport: LocalHubTransport = {
      async invoke(command) {
        seenCommands.push(command);
        switch (command) {
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility":
            return capabilityVisibilityFixture;
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:list_inbox_items":
          case "hub:list_notifications":
            return [];
          case "hub:list_automations":
            return automationListFixture;
          case "hub:create_automation":
            return manualAutomationResponseFixture;
          case "hub:get_automation_detail":
          case "hub:manual_dispatch":
            return manualAutomationDetailFixture;
          default:
            throw new Error(`unexpected command: ${command}`);
        }
      },
      async listen() {
        return () => undefined;
      }
    };

    const client = createLocalHubClient(transport);
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push("/workspaces/workspace-alpha/projects/project-slice1");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();
    expect(wrapper.text()).toContain("Manual automation");
    expect(wrapper.text()).toContain("Failing automation");

    await wrapper.get('[data-testid="automation-create"]').trigger("click");
    await flushPromises();

    expect(seenCommands).toContain("hub:create_automation");
    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-slice1/automations/automation-manual-1"
    );
    expect(wrapper.text()).toContain("Dispatch on demand");
    expect(wrapper.text()).toContain("succeeded");
    expect(wrapper.text()).toContain("completed");

    await wrapper.get('[data-testid="manual-dispatch"]').trigger("click");
    await flushPromises();

    expect(seenCommands.filter((command) => command === "hub:manual_dispatch")).toHaveLength(1);
    expect(wrapper.text()).toContain("delivery-manual-1");
  });

  it("surfaces automation lifecycle, retry, and schema errors without leaving the detail view", async () => {
    const invalidTransition = new Error(
      "trigger lifecycle cannot transition from `active` to `active`"
    );
    const retryError = new Error(
      "trigger delivery `delivery-succeeded-1` cannot transition from `succeeded` to `retry_scheduled`"
    );
    const schemaError = new Error("Invalid create automation command");
    let createAttempts = 0;
    let detailPhase = "invalid-lifecycle";
    const transport: LocalHubTransport = {
      async invoke(command) {
        switch (command) {
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility":
            return capabilityVisibilityFixture;
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:list_inbox_items":
          case "hub:list_notifications":
            return [];
          case "hub:list_automations":
            return [
              manualAutomationDetailFixture,
              {
                ...failingAutomationSummaryFixture,
                recent_deliveries: [
                  {
                    ...failingAutomationSummaryFixture.recent_deliveries[0],
                    id: "delivery-succeeded-1",
                    status: "succeeded",
                    last_error: null
                  }
                ]
              }
            ];
          case "hub:get_automation_detail":
            if (detailPhase === "invalid-lifecycle") {
              return manualAutomationDetailFixture;
            }
            return {
              ...failingAutomationSummaryFixture,
              recent_deliveries: [
                {
                  ...failingAutomationSummaryFixture.recent_deliveries[0],
                  id: "delivery-succeeded-1",
                  status: "succeeded",
                  last_error: null
                }
              ]
            };
          case "hub:activate_automation":
            throw invalidTransition;
          case "hub:retry_trigger_delivery":
            throw retryError;
          case "hub:create_automation":
            createAttempts += 1;
            throw schemaError;
          default:
            throw new Error(`unexpected command: ${command}`);
        }
      },
      async listen() {
        return () => undefined;
      }
    };

    const client = createLocalHubClient(transport);
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push(
      "/workspaces/workspace-alpha/projects/project-slice1/automations/automation-manual-1"
    );
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();
    await wrapper.get('[data-testid="automation-activate"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain(invalidTransition.message);
    expect(router.currentRoute.value.fullPath).toContain("/automations/automation-manual-1");

    detailPhase = "retry-error";
    await router.push(
      "/workspaces/workspace-alpha/projects/project-slice1/automations/automation-failing-1"
    );
    await flushPromises();

    await wrapper.get('[data-testid="retry-delivery-delivery-succeeded-1"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain(retryError.message);
    expect(router.currentRoute.value.fullPath).toContain("/automations/automation-failing-1");

    await router.push("/workspaces/workspace-alpha/projects/project-slice1");
    await flushPromises();

    await wrapper.get('[data-testid="automation-create"]').trigger("click");
    await flushPromises();

    expect(createAttempts).toBe(1);
    expect(wrapper.text()).toContain(schemaError.message);
    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-slice1"
    );
  });
});
