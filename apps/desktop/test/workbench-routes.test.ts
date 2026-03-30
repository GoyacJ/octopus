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

const projectKnowledgeIndexFixture = {
  knowledge_space: {
    id: "knowledge-space-1",
    workspace_id: "workspace-alpha",
    project_id: "project-slice1",
    owner_ref: "workspace_admin:alice",
    display_name: "Project Slice 1 Knowledge",
    created_at: "2026-03-26T10:00:00Z",
    updated_at: "2026-03-26T10:00:00Z"
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
      created_at: "2026-03-26T10:00:01Z"
    },
    {
      kind: "asset",
      id: "asset-1",
      knowledge_space_id: "knowledge-space-1",
      capability_id: "capability-write-note",
      status: "verified_shared",
      source_run_id: null,
      source_artifact_id: null,
      source_candidate_id: "candidate-1",
      provenance_source: null,
      trust_level: "verified",
      created_at: "2026-03-26T10:00:02Z"
    }
  ]
};

const modelProviderFixture = {
  id: "provider-openai",
  display_name: "OpenAI",
  provider_family: "openai",
  status: "active",
  default_base_url: "https://api.openai.com/v1",
  protocol_families: ["openai_responses_compatible"],
  created_at: "2026-03-30T10:00:00Z",
  updated_at: "2026-03-30T10:00:00Z"
} as const;

const modelCatalogItemFixture = {
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

const modelProfileFixture = {
  id: "profile-default-reasoning",
  display_name: "Default Reasoning",
  scope_ref: "tenant:workspace-alpha",
  primary_model_key: "openai:gpt-5.4",
  fallback_model_keys: ["openai:gpt-5.4-mini"],
  created_at: "2026-03-30T10:00:00Z",
  updated_at: "2026-03-30T10:00:00Z"
} as const;

const workspaceModelPolicyFixture = {
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

function buildTransport(
  overrides: Partial<Record<string, (payload: unknown) => unknown>> = {}
): LocalHubTransport {
  return {
    async invoke(command, payload) {
      const override = overrides[command];
      if (override) {
        return override(payload);
      }

      switch (command) {
        case "hub:get_project_context": {
          const params = payload as
            | { workspaceId?: string; projectId?: string }
            | undefined;
          if (params?.workspaceId === "demo") {
            return demoContextFixture;
          }
          if (params?.projectId === "project-empty") {
            return {
              workspace: projectContextFixture.workspace,
              project: {
                ...projectContextFixture.project,
                id: "project-empty",
                slug: "project-empty",
                display_name: "Project Empty"
              }
            };
          }
          return projectContextFixture;
        }
        case "hub:list_capability_visibility":
          return capabilityResolutionFixture;
        case "hub:list_runs":
          return [runSummaryFixture];
        case "hub:get_project_knowledge": {
          const projectId = (payload as { projectId?: string } | undefined)?.projectId;
          if (projectId === "project-empty") {
            return {
              knowledge_space: {
                ...projectKnowledgeIndexFixture.knowledge_space,
                project_id: "project-empty",
                display_name: "Project Empty Knowledge"
              },
              entries: []
            };
          }
          return projectKnowledgeIndexFixture;
        }
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
        case "hub:list_model_providers":
          return [modelProviderFixture];
        case "hub:list_model_catalog_items":
          return [modelCatalogItemFixture];
        case "hub:list_model_profiles":
          return [modelProfileFixture];
        case "hub:get_workspace_model_policy":
          return workspaceModelPolicyFixture;
        default:
          throw new Error(`unexpected command: ${command}`);
      }
    },
    async listen() {
      return () => undefined;
    }
  };
}

async function mountAt(path: string, transport = buildTransport()) {
  const client = createLocalHubClient(transport);
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

  it("renders the workspace models route as a read-only governance surface", async () => {
    const { wrapper } = await mountAt("/workspaces/workspace-alpha/models");

    expect(wrapper.text()).toContain("Models");
    expect(wrapper.text()).toContain("Read-only");
    expect(wrapper.text()).toContain("OpenAI");
    expect(wrapper.text()).toContain("openai:gpt-5.4");
    expect(wrapper.text()).toContain("Default Reasoning");
    expect(wrapper.text()).toContain("Preview models require approval");
  });

  it("renders explicit empty-state copy when no workspace model governance truth exists", async () => {
    const { wrapper } = await mountAt(
      "/workspaces/workspace-alpha/models",
      buildTransport({
        "hub:list_model_providers": () => [],
        "hub:list_model_catalog_items": () => [],
        "hub:list_model_profiles": () => [],
        "hub:get_workspace_model_policy": () => null
      })
    );

    expect(wrapper.text()).toContain("Models");
    expect(wrapper.text()).toContain("No model providers are recorded");
    expect(wrapper.text()).toContain("No workspace model policy is recorded");
    expect(wrapper.text()).toContain("Read-only");
  });

  it("renders the knowledge route from a dedicated project loader with traceability links", async () => {
    const { wrapper } = await mountAt(
      "/workspaces/workspace-alpha/projects/project-slice1/knowledge",
      buildTransport({
        "hub:get_run_detail": () => {
          throw new Error("knowledge route must not hydrate run detail");
        },
        "hub:get_knowledge_detail": () => {
          throw new Error("knowledge route must not reuse run-scoped knowledge detail");
        }
      })
    );

    expect(wrapper.text()).toContain("Project Knowledge");
    expect(wrapper.text()).toContain("Project Slice 1 Knowledge");
    expect(wrapper.text()).toContain("candidate");
    expect(wrapper.text()).toContain("verified_shared");
    expect(wrapper.text()).not.toContain("Request Promotion");
    expect(wrapper.get('[data-testid="knowledge-open-run-run-1"]').attributes("href")).toContain(
      "/runs/run-1"
    );
    expect(wrapper.get('[data-testid="knowledge-open-inbox"]').attributes("href")).toContain(
      "/workspaces/workspace-alpha/inbox"
    );
  });

  it("renders an empty knowledge state for a project with no visible entries", async () => {
    const { wrapper } = await mountAt(
      "/workspaces/workspace-alpha/projects/project-empty/knowledge"
    );

    expect(wrapper.text()).toContain("Project Empty Knowledge");
    expect(wrapper.text()).toContain("No shared knowledge entries are visible for this project yet.");
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
