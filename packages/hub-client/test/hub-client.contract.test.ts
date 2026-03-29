import { LOCAL_HUB_TRANSPORT } from "@octopus/schema-ts";
import { describe, expect, it } from "vitest";

import {
  HUB_EVENT_CHANNEL,
  createLocalHubClient,
  createRemoteHubAuthClient,
  createRemoteHubClient,
  type EventSourceLike,
  type EventSourceMessage,
  type HubClient,
  type LocalHubTransport
} from "../src/index";

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

const taskCreateCommandFixture = {
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  title: "Write note",
  instruction: "Emit a deterministic artifact",
  action: {
    kind: "emit_text",
    content: "hello"
  },
  capability_id: "capability-write-note",
  estimated_cost: 1,
  idempotency_key: "task-1"
} as const;

const createAutomationCommandFixture = {
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  title: "Automation note",
  instruction: "Run from manual event",
  action: {
    kind: "emit_text",
    content: "hello"
  },
  capability_id: "capability-write-note",
  estimated_cost: 1,
  trigger: {
    trigger_type: "manual_event",
    config: {}
  }
} as const;

const automationFixture = {
  id: "automation-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  trigger_id: "trigger-1",
  status: "active",
  title: "Automation note",
  instruction: "Run from manual event",
  action: {
    kind: "emit_text",
    content: "hello"
  },
  capability_id: "capability-write-note",
  estimated_cost: 1,
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:00Z"
} as const;

const triggerFixture = {
  id: "trigger-1",
  automation_id: "automation-1",
  trigger_type: "manual_event",
  config: {},
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:00Z"
} as const;

const triggerDeliveryFixture = {
  id: "delivery-1",
  trigger_id: "trigger-1",
  run_id: "run-1",
  status: "succeeded",
  dedupe_key: "delivery-1",
  payload: {
    source: "manual"
  },
  attempt_count: 1,
  last_error: null,
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:01Z"
} as const;

const runSummaryFixture = {
  id: "run-1",
  task_id: "task-1",
  workspace_id: "workspace-alpha",
  project_id: "project-slice1",
  title: "Automation note",
  run_type: "automation",
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

const createAutomationResponseFixture = {
  automation: automationFixture,
  trigger: triggerFixture,
  webhook_secret: null
} as const;

const automationDetailFixture = {
  automation: automationFixture,
  trigger: triggerFixture,
  recent_deliveries: [triggerDeliveryFixture],
  last_run_summary: runSummaryFixture
} as const;

const taskFixture = {
  id: "task-1",
  ...taskCreateCommandFixture,
  source_kind: "manual",
  automation_id: null,
  created_at: "2026-03-26T10:00:00Z",
  updated_at: "2026-03-26T10:00:00Z"
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
};

const promotionApprovalFixture = {
  ...approvalFixture,
  id: "approval-promotion-1",
  run_id: "run-1",
  task_id: "task-1",
  approval_type: "knowledge_promotion",
  target_ref: "knowledge_candidate:candidate-1",
  reason: "Promote candidate to verified shared knowledge",
  dedupe_key: "knowledge_promotion:candidate-1:approval-promotion-1"
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
    execution_state: "approval_required",
    reason_code: "budget_soft_limit_exceeded",
    explanation: "Approval required because the estimated cost 7 exceeds the soft cost limit 5."
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

const hubEventFixture = {
  event_type: "hub.connection.updated",
  sequence: 1,
  occurred_at: "2026-03-26T10:00:01Z",
  payload: hubConnectionStatusFixture
};

type SuiteFactory = () => {
  client: HubClient;
  emitEvent: (event: unknown) => void;
};

function runHubClientContractSuite(name: string, factory: SuiteFactory) {
  describe(name, () => {
    it("loads the minimum GA surface resources through one client shape", async () => {
      const { client } = factory();

      await expect(
        client.getProjectContext("workspace-alpha", "project-slice1")
      ).resolves.toMatchObject({
        project: { id: "project-slice1" }
      });
      await expect(
        client.listAutomations("workspace-alpha", "project-slice1")
      ).resolves.toMatchObject([
        { automation: { id: "automation-1", status: "active" } }
      ]);
      await expect(client.createAutomation(createAutomationCommandFixture)).resolves.toMatchObject({
        automation: { id: "automation-1" },
        trigger: { id: "trigger-1" }
      });
      await expect(client.getAutomationDetail("automation-1")).resolves.toMatchObject({
        automation: { id: "automation-1" },
        recent_deliveries: [{ id: "delivery-1" }]
      });
      await expect(client.pauseAutomation("automation-1")).resolves.toMatchObject({
        automation: { id: "automation-1" }
      });
      await expect(client.activateAutomation("automation-1")).resolves.toMatchObject({
        automation: { id: "automation-1" }
      });
      await expect(client.archiveAutomation("automation-1")).resolves.toMatchObject({
        automation: { id: "automation-1" }
      });
      await expect(
        client.manualDispatch({
          trigger_id: "trigger-1",
          dedupe_key: "manual-1",
          payload: { source: "manual" }
        })
      ).resolves.toMatchObject({
        automation: { id: "automation-1" }
      });
      await expect(
        client.retryTriggerDelivery({
          delivery_id: "delivery-1"
        })
      ).resolves.toMatchObject({
        automation: { id: "automation-1" }
      });
      await expect(client.createTask(taskCreateCommandFixture)).resolves.toMatchObject({
        id: "task-1"
      });
      await expect(client.startTask("task-1")).resolves.toMatchObject({
        run: { id: "run-1", status: "completed" }
      });
      await expect(client.getRunDetail("run-1")).resolves.toMatchObject({
        artifacts: [{ id: "artifact-1" }]
      });
      await expect(
        client.listRuns("workspace-alpha", "project-slice1")
      ).resolves.toMatchObject([{ id: "run-1", title: "Automation note" }]);
      await expect(client.getApprovalRequest("approval-1")).resolves.toMatchObject({
        id: "approval-1",
        target_ref: "run:run-approval"
      });
      await expect(client.listArtifacts("run-1")).resolves.toHaveLength(1);
      await expect(
        client.listCapabilityResolutions("workspace-alpha", "project-slice1", 7)
      ).resolves.toMatchObject([
        {
          execution_state: "approval_required",
          reason_code: "budget_soft_limit_exceeded"
        }
      ]);
      await expect(client.listInboxItems("workspace-alpha")).resolves.toEqual([]);
      await expect(client.listNotifications("workspace-alpha")).resolves.toEqual([]);
      await expect(client.getKnowledgeDetail("run-1")).resolves.toMatchObject({
        knowledge_space: { id: "knowledge-space-1" }
      });
      await expect(
        client.getProjectKnowledge("workspace-alpha", "project-slice1")
      ).resolves.toEqual(
        expect.objectContaining({
          knowledge_space: expect.objectContaining({ id: "knowledge-space-1" }),
          entries: expect.arrayContaining([
            expect.objectContaining({ kind: "candidate", id: "candidate-1" }),
            expect.objectContaining({ kind: "asset", id: "asset-1" })
          ])
        })
      );
      await expect(
        client.requestKnowledgePromotion({
          candidate_id: "candidate-1",
          actor_ref: "workspace_admin:alice",
          note: "promote"
        })
      ).resolves.toMatchObject({
        id: "approval-promotion-1",
        approval_type: "knowledge_promotion",
        target_ref: "knowledge_candidate:candidate-1"
      });
      await expect(
        client.promoteKnowledge({
          candidate_id: "candidate-1",
          actor_ref: "workspace_admin:alice",
          note: "approve"
        })
      ).resolves.toMatchObject({
        candidates: [{ id: "candidate-1" }]
      });
      await expect(client.getHubConnectionStatus()).resolves.toMatchObject({
        state: "connected"
      });
      await expect(
        client.resolveApproval({
          approval_id: "approval-1",
          decision: "approve",
          actor_ref: "workspace_admin:alice",
          note: "ship it"
        })
      ).resolves.toMatchObject({
        run: { id: "run-1" }
      });
    });

    it("subscribes to normalized hub events", async () => {
      const { client, emitEvent } = factory();
      const seen: string[] = [];

      const unsubscribe = await client.subscribe((event) => {
        seen.push(event.event_type);
      });

      emitEvent(hubEventFixture);
      expect(seen).toEqual(["hub.connection.updated"]);

      await unsubscribe();
    });
  });
}

runHubClientContractSuite("local adapter", () => {
  let eventHandler: ((payload: unknown) => void) | undefined;

  const transport: LocalHubTransport = {
    async invoke(command, payload) {
      switch (command) {
        case "hub:get_project_context":
          expect(payload).toEqual({
            workspaceId: "workspace-alpha",
            projectId: "project-slice1"
          });
          return projectContextFixture;
        case "hub:list_automations":
          expect(payload).toEqual({
            workspaceId: "workspace-alpha",
            projectId: "project-slice1"
          });
          return [automationDetailFixture];
        case "hub:create_automation":
          expect(payload).toEqual(createAutomationCommandFixture);
          return createAutomationResponseFixture;
        case "hub:get_automation_detail":
        case "hub:activate_automation":
        case "hub:pause_automation":
        case "hub:archive_automation":
        case "hub:manual_dispatch":
        case "hub:retry_trigger_delivery":
          return automationDetailFixture;
        case "hub:create_task":
          expect(payload).toEqual(taskCreateCommandFixture);
          return taskFixture;
        case "hub:list_runs":
          expect(payload).toEqual({
            workspaceId: "workspace-alpha",
            projectId: "project-slice1"
          });
          return [runSummaryFixture];
        case "hub:start_task":
        case "hub:get_run_detail":
        case "hub:resolve_approval":
          return runDetailFixture;
        case "hub:get_approval_request":
          expect(payload).toEqual({ approvalId: "approval-1" });
          return approvalFixture;
        case "hub:list_artifacts":
          return runDetailFixture.artifacts;
        case "hub:list_capability_visibility":
          expect(payload).toEqual({
            workspaceId: "workspace-alpha",
            projectId: "project-slice1",
            estimatedCost: 7
          });
          return capabilityResolutionFixture;
        case "hub:list_inbox_items":
        case "hub:list_notifications":
          return [];
        case "hub:get_knowledge_detail":
          return knowledgeDetailFixture;
        case "hub:get_project_knowledge":
          expect(payload).toEqual({
            workspaceId: "workspace-alpha",
            projectId: "project-slice1"
          });
          return projectKnowledgeIndexFixture;
        case "hub:request_knowledge_promotion":
          expect(payload).toEqual({
            candidate_id: "candidate-1",
            actor_ref: "workspace_admin:alice",
            note: "promote"
          });
          return promotionApprovalFixture;
        case "hub:promote_knowledge":
          return knowledgeDetailFixture;
        case "hub:get_connection_status":
          return hubConnectionStatusFixture;
        default:
          throw new Error(`unexpected local command: ${command}`);
      }
    },
    async listen(channel, handler) {
      expect(channel).toBe(HUB_EVENT_CHANNEL);
      eventHandler = handler;
      return () => {
        eventHandler = undefined;
      };
    }
  };

  return {
    client: createLocalHubClient(transport),
    emitEvent(event) {
      eventHandler?.(event);
    }
  };
});

describe("local adapter transport ownership", () => {
  it("reuses the interop-owned local transport command directory", async () => {
    const commands: string[] = [];
    const client = createLocalHubClient({
      async invoke(command) {
        commands.push(command);
        return projectContextFixture;
      },
      async listen() {
        return async () => {};
      }
    });

    await client.getProjectContext("workspace-alpha", "project-slice1");

    expect(commands).toEqual([
      LOCAL_HUB_TRANSPORT.commands.get_project_context
    ]);
    expect(HUB_EVENT_CHANNEL).toBe(LOCAL_HUB_TRANSPORT.event_channel);
  });
});

class FakeEventSource implements EventSourceLike {
  onmessage: ((event: EventSourceMessage) => void) | null = null;
  onerror: ((error: unknown) => void) | null = null;
  closed = false;

  close() {
    this.closed = true;
  }

  emit(event: unknown) {
    this.onmessage?.({
      data: JSON.stringify(event)
    });
  }
}

runHubClientContractSuite("remote adapter", () => {
  const eventSource = new FakeEventSource();

  return {
    client: createRemoteHubClient({
      baseUrl: "http://hub.test",
      fetch: async (input: RequestInfo | URL, init?: RequestInit) => {
        const method = init?.method ?? "GET";
        const url = String(input);

        if (method === "GET" && url === "http://hub.test/api/workspaces/workspace-alpha/projects/project-slice1/context") {
          return Response.json(projectContextFixture);
        }
        if (method === "GET" && url === "http://hub.test/api/workspaces/workspace-alpha/projects/project-slice1/automations") {
          return Response.json([automationDetailFixture]);
        }
        if (method === "POST" && url === "http://hub.test/api/workspaces/workspace-alpha/projects/project-slice1/automations") {
          return Response.json(createAutomationResponseFixture);
        }
        if (method === "GET" && url === "http://hub.test/api/automations/automation-1") {
          return Response.json(automationDetailFixture);
        }
        if (method === "POST" && url === "http://hub.test/api/automations/automation-1/activate") {
          return Response.json(automationDetailFixture);
        }
        if (method === "POST" && url === "http://hub.test/api/automations/automation-1/pause") {
          return Response.json(automationDetailFixture);
        }
        if (method === "POST" && url === "http://hub.test/api/automations/automation-1/archive") {
          return Response.json(automationDetailFixture);
        }
        if (method === "POST" && url === "http://hub.test/api/triggers/trigger-1/manual-dispatch") {
          return Response.json(automationDetailFixture);
        }
        if (method === "POST" && url === "http://hub.test/api/trigger-deliveries/delivery-1/retry") {
          return Response.json(automationDetailFixture);
        }
        if (method === "POST" && url === "http://hub.test/api/tasks") {
          return Response.json(taskFixture);
        }
        if (method === "POST" && url === "http://hub.test/api/tasks/task-1/start") {
          return Response.json(runDetailFixture);
        }
        if (method === "GET" && url === "http://hub.test/api/runs/run-1") {
          return Response.json(runDetailFixture);
        }
        if (
          method === "GET" &&
          url ===
            "http://hub.test/api/workspaces/workspace-alpha/projects/project-slice1/runs"
        ) {
          return Response.json([runSummaryFixture]);
        }
        if (method === "GET" && url === "http://hub.test/api/approvals/approval-1") {
          return Response.json(approvalFixture);
        }
        if (method === "GET" && url === "http://hub.test/api/runs/run-1/artifacts") {
          return Response.json(runDetailFixture.artifacts);
        }
        if (method === "POST" && url === "http://hub.test/api/approvals/approval-1/resolve") {
          return Response.json(runDetailFixture);
        }
        if (method === "GET" && url === "http://hub.test/api/workspaces/workspace-alpha/inbox") {
          return Response.json([]);
        }
        if (method === "GET" && url === "http://hub.test/api/workspaces/workspace-alpha/notifications") {
          return Response.json([]);
        }
        if (
          method === "GET" &&
          url ===
            "http://hub.test/api/workspaces/workspace-alpha/projects/project-slice1/capabilities?estimated_cost=7"
        ) {
          return Response.json(capabilityResolutionFixture);
        }
        if (method === "GET" && url === "http://hub.test/api/runs/run-1/knowledge") {
          return Response.json(knowledgeDetailFixture);
        }
        if (
          method === "GET" &&
          url ===
            "http://hub.test/api/workspaces/workspace-alpha/projects/project-slice1/knowledge"
        ) {
          return Response.json(projectKnowledgeIndexFixture);
        }
        if (
          method === "POST" &&
          url ===
            "http://hub.test/api/knowledge/candidates/candidate-1/request-promotion"
        ) {
          return Response.json(promotionApprovalFixture);
        }
        if (method === "POST" && url === "http://hub.test/api/knowledge/candidates/candidate-1/promote") {
          return Response.json(knowledgeDetailFixture);
        }
        if (method === "GET" && url === "http://hub.test/api/hub/connection") {
          return Response.json(hubConnectionStatusFixture);
        }

        throw new Error(`unexpected remote request: ${method} ${url}`);
      },
      createEventSource(url) {
        expect(url).toBe("http://hub.test/api/events");
        return eventSource;
      }
    }),
    emitEvent(event) {
      eventSource.emit(event);
    }
  };
});

describe("remote auth-aware adapter behavior", () => {
  it("logs in through the remote auth surface and parses the session response", async () => {
    const authClient = createRemoteHubAuthClient({
      baseUrl: "http://hub.test",
      fetch: async (input: RequestInfo | URL, init?: RequestInit) => {
        expect(String(input)).toBe("http://hub.test/api/auth/login");
        expect(init?.method).toBe("POST");
        expect(new Headers(init?.headers).get("content-type")).toBe("application/json");
        expect(init?.body).toBe(
          JSON.stringify({
            workspace_id: "workspace-alpha",
            email: "admin@octopus.local",
            password: "octopus-bootstrap-password"
          })
        );

        return Response.json({
          access_token: "remote-token",
          session: {
            session_id: "session-1",
            user_id: "user-1",
            email: "admin@octopus.local",
            workspace_id: "workspace-alpha",
            actor_ref: "workspace_admin:bootstrap_admin",
            issued_at: "2026-03-29T10:00:00Z",
            expires_at: "2026-03-29T12:00:00Z"
          }
        });
      }
    } as any);

    await expect(
      authClient.login({
        workspace_id: "workspace-alpha",
        email: "admin@octopus.local",
        password: "octopus-bootstrap-password"
      })
    ).resolves.toMatchObject({
      access_token: "remote-token",
      session: {
        workspace_id: "workspace-alpha",
        email: "admin@octopus.local"
      }
    });
  });

  it("injects bearer tokens when reading the current remote session", async () => {
    const authClient = createRemoteHubAuthClient({
      baseUrl: "http://hub.test",
      getAccessToken: async () => "remote-token",
      fetch: async (input: RequestInfo | URL, init?: RequestInit) => {
        expect(String(input)).toBe("http://hub.test/api/auth/session");
        expect(init?.method).toBe("GET");
        expect(new Headers(init?.headers).get("authorization")).toBe(
          "Bearer remote-token"
        );

        return Response.json({
          session_id: "session-1",
          user_id: "user-1",
          email: "admin@octopus.local",
          workspace_id: "workspace-alpha",
          actor_ref: "workspace_admin:bootstrap_admin",
          issued_at: "2026-03-29T10:00:00Z",
          expires_at: "2026-03-29T12:00:00Z"
        });
      }
    } as any);

    await expect(authClient.getCurrentSession()).resolves.toMatchObject({
      session_id: "session-1",
      workspace_id: "workspace-alpha"
    });
  });

  it("logs out through the remote auth surface with bearer auth", async () => {
    const authClient = createRemoteHubAuthClient({
      baseUrl: "http://hub.test",
      getAccessToken: async () => "remote-token",
      fetch: async (input: RequestInfo | URL, init?: RequestInit) => {
        expect(String(input)).toBe("http://hub.test/api/auth/logout");
        expect(init?.method).toBe("POST");
        expect(new Headers(init?.headers).get("authorization")).toBe(
          "Bearer remote-token"
        );

        return new Response(null, {
          status: 204
        });
      }
    } as any);

    await expect(authClient.logout()).resolves.toBeUndefined();
  });

  it("injects bearer tokens into remote hub requests", async () => {
    const client = createRemoteHubClient({
      baseUrl: "http://hub.test",
      getAccessToken: async () => "remote-token",
      fetch: async (input: RequestInfo | URL, init?: RequestInit) => {
        expect(String(input)).toBe("http://hub.test/api/hub/connection");
        expect(new Headers(init?.headers).get("authorization")).toBe(
          "Bearer remote-token"
        );

        return Response.json({
          ...hubConnectionStatusFixture,
          mode: "remote",
          auth_state: "authenticated"
        });
      }
    } as any);

    await expect(client.getHubConnectionStatus()).resolves.toMatchObject({
      auth_state: "authenticated"
    });
  });

  it("normalizes token expiry into an auth-aware client error", async () => {
    const client = createRemoteHubClient({
      baseUrl: "http://hub.test",
      getAccessToken: async () => "remote-token",
      fetch: async () =>
        new Response(
          JSON.stringify({
            error: "token expired",
            error_code: "token_expired",
            auth_state: "token_expired"
          }),
          {
            status: 401,
            headers: {
              "content-type": "application/json"
            }
          }
        )
    } as any);

    await expect(client.listInboxItems("workspace-alpha")).rejects.toMatchObject({
      name: "HubClientAuthError",
      kind: "token_expired",
      authState: "token_expired",
      status: 401
    });
  });

  it("normalizes workspace mismatch into an auth-aware client error", async () => {
    const client = createRemoteHubClient({
      baseUrl: "http://hub.test",
      getAccessToken: async () => "remote-token",
      fetch: async () =>
        new Response(
          JSON.stringify({
            error: "workspace membership required",
            error_code: "workspace_forbidden",
            auth_state: "authenticated"
          }),
          {
            status: 403,
            headers: {
              "content-type": "application/json"
            }
          }
        )
    } as any);

    await expect(
      client.listRuns("workspace-alpha", "project-slice1")
    ).rejects.toMatchObject({
      name: "HubClientAuthError",
      kind: "workspace_forbidden",
      authState: "authenticated",
      status: 403
    });
  });
});
