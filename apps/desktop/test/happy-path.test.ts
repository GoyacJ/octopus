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

const modelSelectionDecisionFixture = {
  id: "selection-1",
  run_id: "run-1",
  model_profile_id: "profile-default-reasoning",
  requested_intent: "web_research",
  decision_outcome: "selected",
  selected_model_key: "openai:gpt-5.4",
  selected_provider_id: "provider-openai",
  required_feature_tags: ["supports_structured_output", "supports_builtin_web_search"],
  missing_feature_tags: [],
  requires_approval: false,
  decision_reason: "best matching features within tenant policy",
  created_at: "2026-03-30T10:00:00Z"
} as const;

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
  policy_decisions: [
    {
      id: "policy-1",
      workspace_id: "workspace-alpha",
      project_id: "project-slice1",
      run_id: "run-1",
      task_id: "task-1",
      capability_id: "capability-write-note",
      decision: "allow",
      reason: "within_budget",
      estimated_cost: 1,
      approval_request_id: null,
      created_at: "2026-03-26T10:00:00Z"
    }
  ],
  model_selection_decision: modelSelectionDecisionFixture,
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

const projectKnowledgeIndexFixture = {
  knowledge_space: knowledgeDetailFixture.knowledge_space,
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
      created_at: "2026-03-26T10:00:01Z"
    }
  ]
};

const emptyProjectKnowledgeIndexFixture = {
  knowledge_space: knowledgeDetailFixture.knowledge_space,
  entries: []
};

const retryableRunDetailFixture = {
  ...runDetailFixture,
  run: {
    ...runDetailFixture.run,
    status: "failed",
    attempt_count: 1,
    resume_token: "resume-1",
    last_error: "network_glitch",
    updated_at: "2026-03-26T10:00:02Z",
    completed_at: null,
    terminated_at: "2026-03-26T10:00:02Z"
  },
  artifacts: [],
  knowledge_candidates: [],
  knowledge_assets: [],
  knowledge_lineage: []
};

const retryableKnowledgeDetailFixture = {
  ...knowledgeDetailFixture,
  candidates: [],
  assets: [],
  lineage: []
};

const recoveredRunDetailFixture = {
  ...runDetailFixture,
  run: {
    ...runDetailFixture.run,
    attempt_count: 2,
    resume_token: null,
    last_error: null,
    updated_at: "2026-03-26T10:00:03Z",
    completed_at: "2026-03-26T10:00:03Z",
    terminated_at: null
  }
};

const runningRunDetailFixture = {
  ...runDetailFixture,
  run: {
    ...runDetailFixture.run,
    status: "running",
    checkpoint_seq: 1,
    resume_token: null,
    last_error: null,
    updated_at: "2026-03-26T10:00:00Z",
    completed_at: null,
    terminated_at: null
  },
  artifacts: [],
  knowledge_candidates: [],
  knowledge_assets: [],
  knowledge_lineage: []
};

const terminatedRunDetailFixture = {
  ...runDetailFixture,
  run: {
    ...runDetailFixture.run,
    status: "terminated",
    checkpoint_seq: 1,
    resume_token: null,
    last_error: "desktop_operator_stopped",
    updated_at: "2026-03-26T10:00:04Z",
    completed_at: null,
    terminated_at: "2026-03-26T10:00:04Z"
  },
  artifacts: [],
  knowledge_candidates: [],
  knowledge_assets: [],
  knowledge_lineage: []
};

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

const retryableRunSummaryFixture = {
  ...runSummaryFixture,
  status: "failed",
  last_error: "network_glitch",
  completed_at: null,
  terminated_at: "2026-03-26T10:00:02Z"
};

const recoveredRunSummaryFixture = {
  ...runSummaryFixture,
  attempt_count: 2,
  updated_at: "2026-03-26T10:00:03Z",
  completed_at: "2026-03-26T10:00:03Z"
};

const terminatedRunSummaryFixture = {
  ...runSummaryFixture,
  status: "terminated",
  last_error: "desktop_operator_stopped",
  updated_at: "2026-03-26T10:00:04Z",
  completed_at: null,
  terminated_at: "2026-03-26T10:00:04Z"
};

const promotionApprovalFixture = {
  id: "approval-promotion-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  run_id: "run-1",
  task_id: "task-1",
  approval_type: "knowledge_promotion",
  target_ref: "knowledge_candidate:candidate-1",
  status: "pending",
  reason: "Promote candidate to verified shared knowledge",
  dedupe_key: "knowledge_promotion:candidate-1:approval-promotion-1",
  decided_by: null,
  decision_note: null,
  decided_at: null,
  created_at: "2026-03-26T10:00:02Z",
  updated_at: "2026-03-26T10:00:02Z"
} as const;

const approvedPromotionApprovalFixture = {
  ...promotionApprovalFixture,
  status: "approved",
  decided_by: "workspace_admin:desktop_operator",
  decision_note: "promoted",
  decided_at: "2026-03-26T10:00:03Z",
  updated_at: "2026-03-26T10:00:03Z"
} as const;

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

function createRunViewTransport(
  overrides: Partial<Record<string, () => unknown>> = {}
): LocalHubTransport {
  return {
    async invoke(command) {
      const override = overrides[command];
      if (override) {
        return override();
      }

      switch (command) {
        case "hub:get_project_context":
          return projectContextFixture;
        case "hub:list_capability_visibility":
          return capabilityResolutionFixture;
        case "hub:get_connection_status":
          return hubConnectionStatusFixture;
        case "hub:list_runs":
          return [runSummaryFixture];
        case "hub:list_automations":
        case "hub:list_inbox_items":
        case "hub:list_notifications":
          return [];
        case "hub:get_run_detail":
          return runDetailFixture;
        case "hub:list_artifacts":
          return runDetailFixture.artifacts;
        case "hub:get_knowledge_detail":
          return knowledgeDetailFixture;
        case "hub:get_project_knowledge":
          return projectKnowledgeIndexFixture;
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    },
    async listen() {
      return () => undefined;
    }
  };
}

describe("desktop local happy path", () => {
  it("creates a task and shows run, artifact, and knowledge state", async () => {
    const transport: LocalHubTransport = {
      async invoke(command) {
        switch (command) {
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility":
            return capabilityResolutionFixture;
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:list_automations":
            return [];
          case "hub:list_runs":
            return [runSummaryFixture];
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
    await router.push("/workspaces/workspace-alpha/projects/project-slice1/tasks");
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
    expect(wrapper.text()).toContain("within_budget");
  });

  it("refreshes workspace governance explainability when the task estimated cost changes", async () => {
    const seenCosts: number[] = [];

    const transport: LocalHubTransport = {
      async invoke(command, payload) {
        switch (command) {
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility": {
            const estimatedCost = (payload as { estimatedCost?: number } | undefined)
              ?.estimatedCost ?? 0;
            seenCosts.push(estimatedCost);
            return [
              {
                ...capabilityResolutionFixture[0],
                execution_state: estimatedCost > 5 ? "approval_required" : "executable",
                reason_code:
                  estimatedCost > 5 ? "budget_soft_limit_exceeded" : "within_budget",
                explanation:
                  estimatedCost > 5
                    ? `Approval required because the estimated cost ${estimatedCost} exceeds the soft cost limit 5.`
                    : `Executable because the estimated cost ${estimatedCost} is within the current budget.`
              }
            ];
          }
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:list_runs":
            return [runSummaryFixture];
          case "hub:list_automations":
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
    await router.push("/workspaces/workspace-alpha/projects/project-slice1/tasks");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();
    expect(wrapper.text()).toContain(
      "Executable because the estimated cost 1 is within the current budget."
    );

    const taskCostInput = wrapper.findAll('input[type="number"]')[0];
    await taskCostInput.setValue(7);
    await flushPromises();

    expect(seenCosts).toContain(1);
    expect(seenCosts).toContain(7);
    expect(wrapper.text()).toContain(
      "Approval required because the estimated cost 7 exceeds the soft cost limit 5."
    );
  });

  it("shows token expiry separately from disconnect and falls back to read-only mode", async () => {
    const transport: LocalHubTransport = {
      async invoke(command) {
        switch (command) {
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility":
            return capabilityResolutionFixture;
          case "hub:get_connection_status":
            return {
              ...hubConnectionStatusFixture,
              mode: "remote",
              state: "connected",
              auth_state: "token_expired"
            };
          case "hub:list_automations":
            return [];
          case "hub:list_runs":
            return [runSummaryFixture];
          case "hub:list_inbox_items":
            return [inboxApprovalFixture];
          case "hub:list_notifications":
            return [notificationFixture];
          case "hub:get_approval_request":
            return approvalFixture;
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
    await router.push("/workspaces/workspace-alpha/projects/project-slice1/tasks");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();

    expect(wrapper.text()).toContain("token_expired");
    expect(wrapper.get('[data-testid="create-start"]').attributes("disabled")).toBeDefined();
    await router.push("/workspaces/workspace-alpha/inbox");
    await flushPromises();

    expect(wrapper.text()).toContain("Approval Inbox");
    expect(
      wrapper.get('[data-testid="workspace-approve-approval-1"]').attributes("disabled")
    ).toBeDefined();
  });

  it("loads workspace governance approvals and resolves an inbox item inline", async () => {
    let inboxOpen = true;
    let resolveCount = 0;

    const transport: LocalHubTransport = {
      async invoke(command) {
        switch (command) {
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility":
            return capabilityResolutionFixture;
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:list_automations":
            return [];
          case "hub:list_runs":
            return [runSummaryFixture];
          case "hub:list_inbox_items":
            return inboxOpen ? [inboxApprovalFixture] : [];
          case "hub:list_notifications":
            return inboxOpen ? [notificationFixture] : [];
          case "hub:get_approval_request":
            return approvalFixture;
          case "hub:resolve_approval":
            inboxOpen = false;
            resolveCount += 1;
            return {
              ...runDetailFixture,
              run: {
                ...runDetailFixture.run,
                id: "run-approval"
              },
              approvals: [
                {
                  ...approvalFixture,
                  status: "approved",
                  decided_by: "workspace_admin:desktop_operator",
                  decision_note: "approved",
                  decided_at: "2026-03-26T10:00:01Z",
                  updated_at: "2026-03-26T10:00:01Z"
                }
              ]
            };
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
    await router.push("/workspaces/workspace-alpha/projects/project-slice1/inbox");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();

    expect(wrapper.text()).toContain("Execution approval required");
    expect(wrapper.text()).toContain("run:run-approval");

    await wrapper.get('[data-testid="workspace-approve-approval-1"]').trigger("click");
    await flushPromises();

    expect(resolveCount).toBe(1);
    expect(wrapper.text()).toContain("No approval requests are waiting.");
  });

  it("requests knowledge promotion from the run view and reflects pending and approved state", async () => {
    let promotionState: "idle" | "pending" | "approved" = "idle";

    const promotionRunDetail = () => ({
      ...runDetailFixture,
      approvals:
        promotionState === "idle"
          ? []
          : [promotionState === "pending" ? promotionApprovalFixture : approvedPromotionApprovalFixture]
    });

    const promotionKnowledgeDetail = () => ({
      ...knowledgeDetailFixture,
      candidates: [
        {
          ...knowledgeDetailFixture.candidates[0],
          status: promotionState === "approved" ? "verified_shared" : "candidate"
        }
      ],
      assets:
        promotionState === "approved"
          ? [
              {
                id: "asset-1",
                knowledge_space_id: "knowledge-space-1",
                source_candidate_id: "candidate-1",
                capability_id: "capability-write-note",
                status: "verified_shared",
                content: "hello",
                trust_level: "verified",
                created_at: "2026-03-26T10:00:03Z",
                updated_at: "2026-03-26T10:00:03Z"
              }
            ]
          : [],
      lineage:
        promotionState === "approved"
          ? [
              {
                id: "lineage-1",
                workspace_id: "workspace-alpha",
                project_id: "project-slice1",
                run_id: "run-1",
                task_id: "task-1",
                source_ref: "knowledge_candidate:candidate-1",
                target_ref: "knowledge_asset:asset-1",
                relation_type: "promoted_from",
                created_at: "2026-03-26T10:00:03Z"
              }
            ]
          : []
    });

    const transport: LocalHubTransport = {
      async invoke(command) {
        switch (command) {
          case "hub:get_run_detail":
            return promotionRunDetail();
          case "hub:list_artifacts":
            return runDetailFixture.artifacts;
          case "hub:get_knowledge_detail":
            return promotionKnowledgeDetail();
          case "hub:request_knowledge_promotion":
            promotionState = "pending";
            return promotionApprovalFixture;
          case "hub:resolve_approval":
            promotionState = "approved";
            return promotionRunDetail();
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:get_project_context":
            return projectContextFixture;
          case "hub:list_capability_visibility":
            return capabilityResolutionFixture;
          case "hub:list_inbox_items":
          case "hub:list_notifications":
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

    const client = createLocalHubClient(transport);
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push("/runs/run-1");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();
    expect(wrapper.text()).toContain("candidate");

    await wrapper.get('[data-testid="request-promotion-candidate-1"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("knowledge_promotion");
    expect(wrapper.text()).toContain("pending");

    await wrapper.get('[data-testid="run-approve-approval-promotion-1"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("verified_shared");
    expect(wrapper.text()).toContain("asset-1");
  });

  it("shows the run-scoped model selection decision on the authoritative run surface", async () => {
    const client = createLocalHubClient(createRunViewTransport());
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push("/runs/run-1");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();

    expect(wrapper.text()).toContain("Model Selection Decision");
    expect(wrapper.text()).toContain("web_research");
    expect(wrapper.text()).toContain("openai:gpt-5.4");
    expect(wrapper.text()).toContain("provider-openai");
    expect(wrapper.text()).toContain(
      "best matching features within tenant policy"
    );
  });

  it("retries a failed run from the run view and refreshes dependent surfaces", async () => {
    let runPhase: "failed" | "completed" = "failed";
    let retryCalls = 0;
    let getRunDetailCalls = 0;
    let listRunsCalls = 0;
    let projectKnowledgeCalls = 0;
    let inboxCalls = 0;
    let notificationCalls = 0;

    const transport = createRunViewTransport({
      "hub:get_run_detail": () => {
        getRunDetailCalls += 1;
        return runPhase === "failed"
          ? retryableRunDetailFixture
          : recoveredRunDetailFixture;
      },
      "hub:list_artifacts": () =>
        runPhase === "failed" ? [] : recoveredRunDetailFixture.artifacts,
      "hub:get_knowledge_detail": () =>
        runPhase === "failed"
          ? retryableKnowledgeDetailFixture
          : knowledgeDetailFixture,
      "hub:list_runs": () => {
        listRunsCalls += 1;
        return [
          runPhase === "failed"
            ? retryableRunSummaryFixture
            : recoveredRunSummaryFixture
        ];
      },
      "hub:get_project_knowledge": () => {
        projectKnowledgeCalls += 1;
        return runPhase === "failed"
          ? emptyProjectKnowledgeIndexFixture
          : projectKnowledgeIndexFixture;
      },
      "hub:list_inbox_items": () => {
        inboxCalls += 1;
        return [];
      },
      "hub:list_notifications": () => {
        notificationCalls += 1;
        return [];
      },
      "hub:retry_run": () => {
        retryCalls += 1;
        runPhase = "completed";
        return recoveredRunDetailFixture;
      }
    });

    const client = createLocalHubClient(transport);
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push("/runs/run-1");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();
    expect(wrapper.text()).toContain("failed");
    expect(wrapper.get('[data-testid="retry-run"]').attributes("disabled")).toBeUndefined();

    await wrapper.get('[data-testid="retry-run"]').trigger("click");
    await flushPromises();

    expect(retryCalls).toBe(1);
    expect(getRunDetailCalls).toBe(2);
    expect(listRunsCalls).toBe(1);
    expect(projectKnowledgeCalls).toBe(1);
    expect(inboxCalls).toBe(1);
    expect(notificationCalls).toBe(1);
    expect(wrapper.text()).toContain("completed");
    expect(wrapper.text()).toContain("Attempts: 2/2");
    expect(wrapper.find('[data-testid="retry-run"]').exists()).toBe(false);
  });

  it("terminates a running run from the run view and refreshes dependent surfaces", async () => {
    let runPhase: "running" | "terminated" = "running";
    let terminateCalls = 0;
    let listRunsCalls = 0;
    let projectKnowledgeCalls = 0;
    let inboxCalls = 0;
    let notificationCalls = 0;

    const transport = createRunViewTransport({
      "hub:get_run_detail": () =>
        runPhase === "running" ? runningRunDetailFixture : terminatedRunDetailFixture,
      "hub:list_artifacts": () => [],
      "hub:get_knowledge_detail": () => retryableKnowledgeDetailFixture,
      "hub:list_runs": () => {
        listRunsCalls += 1;
        return [
          runPhase === "running" ? runSummaryFixture : terminatedRunSummaryFixture
        ];
      },
      "hub:get_project_knowledge": () => {
        projectKnowledgeCalls += 1;
        return emptyProjectKnowledgeIndexFixture;
      },
      "hub:list_inbox_items": () => {
        inboxCalls += 1;
        return [];
      },
      "hub:list_notifications": () => {
        notificationCalls += 1;
        return [];
      },
      "hub:terminate_run": () => {
        terminateCalls += 1;
        runPhase = "terminated";
        return terminatedRunDetailFixture;
      }
    });

    const client = createLocalHubClient(transport);
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push("/runs/run-1");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();
    expect(wrapper.text()).toContain("running");
    expect(wrapper.get('[data-testid="terminate-run"]').attributes("disabled")).toBeUndefined();

    await wrapper.get('[data-testid="terminate-run"]').trigger("click");
    await flushPromises();

    expect(terminateCalls).toBe(1);
    expect(listRunsCalls).toBe(1);
    expect(projectKnowledgeCalls).toBe(1);
    expect(inboxCalls).toBe(1);
    expect(notificationCalls).toBe(1);
    expect(wrapper.text()).toContain("terminated");
    expect(wrapper.find('[data-testid="terminate-run"]').exists()).toBe(false);
  });

  it("disables run control actions when the connection is read-only", async () => {
    const retryTransport = createRunViewTransport({
      "hub:get_run_detail": () => retryableRunDetailFixture,
      "hub:list_artifacts": () => [],
      "hub:get_knowledge_detail": () => retryableKnowledgeDetailFixture,
      "hub:get_connection_status": () => ({
        ...hubConnectionStatusFixture,
        mode: "remote",
        auth_state: "token_expired"
      })
    });

    const retryClient = createLocalHubClient(retryTransport);
    const retryPlugins = createDesktopPlugins(retryClient, true);
    await retryPlugins.router.push("/runs/run-1");
    await retryPlugins.router.isReady();

    const retryWrapper = mount(AppShell, {
      global: {
        plugins: [retryPlugins.pinia, retryPlugins.router]
      }
    });

    await flushPromises();
    expect(retryWrapper.get('[data-testid="retry-run"]').attributes("disabled")).toBeDefined();

    const terminateTransport = createRunViewTransport({
      "hub:get_run_detail": () => runningRunDetailFixture,
      "hub:list_artifacts": () => [],
      "hub:get_knowledge_detail": () => retryableKnowledgeDetailFixture,
      "hub:get_connection_status": () => ({
        ...hubConnectionStatusFixture,
        mode: "remote",
        auth_state: "auth_required"
      })
    });

    const terminateClient = createLocalHubClient(terminateTransport);
    const terminatePlugins = createDesktopPlugins(terminateClient, true);
    await terminatePlugins.router.push("/runs/run-1");
    await terminatePlugins.router.isReady();

    const terminateWrapper = mount(AppShell, {
      global: {
        plugins: [terminatePlugins.pinia, terminatePlugins.router]
      }
    });

    await flushPromises();
    expect(
      terminateWrapper.get('[data-testid="terminate-run"]').attributes("disabled")
    ).toBeDefined();
  });

  it("shows a run control error banner without faking refreshed state", async () => {
    const retryError = new Error("retry rejected by runtime");
    let retryCalls = 0;
    let getRunDetailCalls = 0;
    let listRunsCalls = 0;
    let projectKnowledgeCalls = 0;

    const transport = createRunViewTransport({
      "hub:get_run_detail": () => {
        getRunDetailCalls += 1;
        return retryableRunDetailFixture;
      },
      "hub:list_artifacts": () => [],
      "hub:get_knowledge_detail": () => retryableKnowledgeDetailFixture,
      "hub:list_runs": () => {
        listRunsCalls += 1;
        return [retryableRunSummaryFixture];
      },
      "hub:get_project_knowledge": () => {
        projectKnowledgeCalls += 1;
        return emptyProjectKnowledgeIndexFixture;
      },
      "hub:retry_run": () => {
        retryCalls += 1;
        throw retryError;
      }
    });

    const client = createLocalHubClient(transport);
    const { pinia, router } = createDesktopPlugins(client, true);
    await router.push("/runs/run-1");
    await router.isReady();

    const wrapper = mount(AppShell, {
      global: {
        plugins: [pinia, router]
      }
    });

    await flushPromises();
    await wrapper.get('[data-testid="retry-run"]').trigger("click");
    await flushPromises();

    expect(retryCalls).toBe(1);
    expect(getRunDetailCalls).toBe(1);
    expect(listRunsCalls).toBe(0);
    expect(projectKnowledgeCalls).toBe(0);
    expect(wrapper.text()).toContain(retryError.message);
    expect(wrapper.text()).toContain("failed");
    expect(wrapper.find('[data-testid="retry-run"]').exists()).toBe(true);
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
            return capabilityResolutionFixture;
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:list_inbox_items":
          case "hub:list_notifications":
            return [];
          case "hub:list_runs":
            return [runSummaryFixture];
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
    await router.push("/workspaces/workspace-alpha/projects/project-slice1/tasks");
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
            return capabilityResolutionFixture;
          case "hub:get_connection_status":
            return hubConnectionStatusFixture;
          case "hub:list_inbox_items":
          case "hub:list_notifications":
            return [];
          case "hub:list_runs":
            return [runSummaryFixture];
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

    await router.push("/workspaces/workspace-alpha/projects/project-slice1/tasks");
    await flushPromises();

    await wrapper.get('[data-testid="automation-create"]').trigger("click");
    await flushPromises();

    expect(createAttempts).toBe(1);
    expect(wrapper.text()).toContain(schemaError.message);
    expect(router.currentRoute.value.fullPath).toBe(
      "/workspaces/workspace-alpha/projects/project-slice1/tasks"
    );
  });
});
