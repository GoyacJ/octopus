import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { createLocalHubClient, type LocalHubTransport } from "@octopus/hub-client";

import AppShell from "../src/App.vue";
import { createDesktopPlugins } from "../src/app";

const preferencesStorageKey = "octopus.desktop.preferences";

function projectContextFixture(workspaceId: string, projectId: string) {
  return {
    workspace: {
      id: workspaceId,
      slug: workspaceId,
      display_name: workspaceId === "demo" ? "Demo Workspace" : "Workspace Alpha",
      created_at: "2026-03-30T09:00:00Z",
      updated_at: "2026-03-30T09:00:00Z"
    },
    project: {
      id: projectId,
      workspace_id: workspaceId,
      slug: projectId,
      display_name: projectId === "demo" ? "Demo Project" : "Project Slice 1",
      created_at: "2026-03-30T09:00:00Z",
      updated_at: "2026-03-30T09:00:00Z"
    }
  };
}

const runSummaryFixture = {
  id: "run-1",
  task_id: "task-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  title: "Review failed nightly sync",
  run_type: "task",
  status: "failed",
  approval_request_id: "approval-1",
  attempt_count: 1,
  max_attempts: 2,
  last_error: "Gateway timeout during MCP execution",
  created_at: "2026-03-30T09:00:00Z",
  updated_at: "2026-03-30T09:04:00Z",
  started_at: "2026-03-30T09:00:00Z",
  completed_at: null,
  terminated_at: null
} as const;

const approvalFixture = {
  id: "approval-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  run_id: "run-1",
  task_id: "task-1",
  approval_type: "execution",
  target_ref: "run:run-1",
  status: "pending",
  reason: "Needs approval",
  dedupe_key: "approval:run-1",
  decided_by: null,
  decision_note: null,
  decided_at: null,
  created_at: "2026-03-30T09:00:00Z",
  updated_at: "2026-03-30T09:00:00Z"
} as const;

const inboxFixture = [
  {
    id: "inbox-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    run_id: "run-1",
    approval_request_id: "approval-1",
    item_type: "approval_request",
    target_ref: "run:run-1",
    status: "open",
    dedupe_key: "inbox:run-1",
    title: "Execution approval required",
    message: "A governed run needs approval before execution.",
    created_at: "2026-03-30T09:00:00Z",
    updated_at: "2026-03-30T09:00:00Z",
    resolved_at: null
  }
] as const;

const notificationFixture = [
  {
    id: "notification-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    run_id: "run-1",
    approval_request_id: "approval-1",
    target_ref: "run:run-1",
    status: "pending",
    dedupe_key: "notification:run-1",
    title: "Approval pending",
    message: "A run is waiting for approval.",
    created_at: "2026-03-30T09:00:00Z",
    updated_at: "2026-03-30T09:00:00Z"
  }
] as const;

const projectKnowledgeFixture = {
  knowledge_space: {
    id: "knowledge-space-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    owner_ref: "workspace_admin:alice",
    display_name: "Project Slice 1 Knowledge",
    created_at: "2026-03-30T09:00:00Z",
    updated_at: "2026-03-30T09:00:00Z"
  },
  entries: [
    {
      kind: "candidate",
      id: "candidate-1",
      knowledge_space_id: "knowledge-space-1",
      capability_id: "capability-write-note",
      status: "candidate",
      source_run_id: "run-1",
      source_artifact_id: "artifact-1",
      source_candidate_id: null,
      provenance_source: "builtin",
      trust_level: "trusted",
      created_at: "2026-03-30T09:04:00Z"
    }
  ]
} as const;

const taskFixture = {
  id: "task-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  source_kind: "manual",
  automation_id: null,
  title: "Prepare weekly summary",
  instruction: "Summarize project health for this week",
  action: {
    kind: "emit_text",
    content: "weekly summary"
  },
  capability_id: "capability-write-note",
  estimated_cost: 2,
  idempotency_key: "workspace-alpha:project-slice1:thread-test",
  created_at: "2026-03-30T09:05:00Z",
  updated_at: "2026-03-30T09:05:00Z"
} as const;

const runDetailFixture = {
  task: taskFixture,
  run: {
    id: "run-created",
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
    checkpoint_seq: 1,
    resume_token: null,
    last_error: null,
    created_at: "2026-03-30T09:05:00Z",
    updated_at: "2026-03-30T09:05:10Z",
    started_at: "2026-03-30T09:05:00Z",
    completed_at: "2026-03-30T09:05:10Z",
    terminated_at: null
  },
  artifacts: [
    {
      id: "artifact-1",
      workspace_id: "workspace-alpha",
      project_id: "project-slice1",
      run_id: "run-created",
      task_id: "task-1",
      artifact_type: "execution_output",
      content: "weekly summary",
      provenance_source: "builtin",
      source_descriptor_id: "builtin:emit_text",
      source_invocation_id: null,
      trust_level: "trusted",
      knowledge_gate_status: "eligible",
      created_at: "2026-03-30T09:05:10Z",
      updated_at: "2026-03-30T09:05:10Z"
    }
  ],
  audits: [],
  approvals: [],
  inbox_items: [],
  notifications: [],
  policy_decisions: [],
  traces: [],
  knowledge_candidates: [],
  knowledge_assets: [],
  knowledge_lineage: [],
  model_selection_decision: null
} as const;

const knowledgeDetailFixture = {
  knowledge_space: projectKnowledgeFixture.knowledge_space,
  candidates: [],
  assets: [],
  lineage: []
} as const;

function buildTransport(counters?: {
  createTaskCalls: number;
  startTaskCalls: number;
}) {
  const transport: LocalHubTransport = {
    async invoke(command, payload) {
      switch (command) {
        case "hub:get_project_context": {
          const params = payload as { workspaceId: string; projectId: string };
          return projectContextFixture(params.workspaceId, params.projectId);
        }
        case "hub:list_runs":
          return [runSummaryFixture];
        case "hub:list_inbox_items":
          return inboxFixture;
        case "hub:list_notifications":
          return notificationFixture;
        case "hub:get_approval_request":
          return approvalFixture;
        case "hub:get_connection_status":
          return {
            mode: "local",
            state: "connected",
            auth_state: "authenticated",
            active_server_count: 1,
            healthy_server_count: 1,
            servers: [],
            last_refreshed_at: "2026-03-30T09:05:00Z"
          };
        case "hub:get_project_knowledge":
          return projectKnowledgeFixture;
        case "hub:list_automations":
          return [];
        case "hub:create_task":
          if (counters) {
            counters.createTaskCalls += 1;
          }
          return taskFixture;
        case "hub:start_task":
          if (counters) {
            counters.startTaskCalls += 1;
          }
          return runDetailFixture;
        case "hub:get_run_detail":
          return runDetailFixture;
        case "hub:list_artifacts":
          return [
            {
              id: "artifact-1",
              workspace_id: "workspace-alpha",
              project_id: "project-slice1",
              run_id: "run-created",
              task_id: "task-1",
              artifact_type: "execution_output",
              content: "weekly summary",
              trust_level: "trusted",
              provenance_source: "builtin",
              source_descriptor_id: "builtin:emit_text",
              source_invocation_id: null,
              knowledge_gate_status: "eligible",
              created_at: "2026-03-30T09:05:10Z",
              updated_at: "2026-03-30T09:05:10Z"
            }
          ];
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

  return createLocalHubClient(transport);
}

async function mountAt(path: string, transport = buildTransport()) {
  const { pinia, router } = createDesktopPlugins(transport, true);
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

describe("desktop target-state redesign", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  afterEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("redirects the default entry to the demo dashboard route", async () => {
    const { router, wrapper } = await mountAt("/");

    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/demo/projects/demo/dashboard"
    );
    expect(wrapper.text()).toContain("Dashboard");
    expect(wrapper.text()).toContain("Conversation");
  });

  it("keeps drafting local until explicit confirmation creates a formal run", async () => {
    const counters = {
      createTaskCalls: 0,
      startTaskCalls: 0
    };
    const { wrapper, router } = await mountAt(
      "/workspaces/workspace-alpha/projects/project-slice1/conversation",
      buildTransport(counters)
    );

    await wrapper.get('[data-testid="conversation-input"]').setValue(
      "Prepare weekly summary"
    );
    await wrapper.get('[data-testid="conversation-send"]').trigger("click");
    await flushPromises();

    expect(counters.createTaskCalls).toBe(0);
    expect(counters.startTaskCalls).toBe(0);
    expect(wrapper.text()).toContain("Prepare weekly summary");
    expect(wrapper.text()).toContain("proposal_ready");

    await wrapper.get('[data-testid="proposal-confirm"]').trigger("click");
    await flushPromises();

    expect(counters.createTaskCalls).toBe(1);
    expect(counters.startTaskCalls).toBe(1);
    expect(router.currentRoute.value.fullPath).toBe("/runs/run-created");
  });

  it("updates locale and theme preferences without translating user-authored content", async () => {
    const { wrapper } = await mountAt(
      "/workspaces/workspace-alpha/projects/project-slice1/conversation"
    );

    await wrapper.get('[data-testid="shell-open-preferences"]').trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("Preferences");

    await wrapper.get('[data-testid="preferences-locale"]').setValue("zh-CN");
    await wrapper.get('[data-testid="preferences-theme"]').setValue("dark");
    await flushPromises();

    expect(wrapper.text()).toContain("控制台");
    expect(wrapper.text()).toContain("对话");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(window.localStorage.getItem(preferencesStorageKey)).toContain("\"locale\":\"zh-CN\"");
    expect(window.localStorage.getItem(preferencesStorageKey)).toContain(
      "\"themeMode\":\"dark\""
    );

    await wrapper.get('[data-testid="shell-open-conversation"]').trigger("click");
    await flushPromises();
    await wrapper.get('[data-testid="conversation-input"]').setValue(
      "Prepare weekly summary"
    );
    await wrapper.get('[data-testid="conversation-send"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Prepare weekly summary");
  });
});
