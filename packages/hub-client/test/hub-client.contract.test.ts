import { describe, expect, it } from "vitest";

import {
  HUB_EVENT_CHANNEL,
  createLocalHubClient,
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
  status: "pending",
  reason: "Needs approval",
  dedupe_key: "approval:1",
  decided_by: null,
  decision_note: null,
  decided_at: null,
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
      await expect(client.createTask(taskCreateCommandFixture)).resolves.toMatchObject({
        id: "task-1"
      });
      await expect(client.startTask("task-1")).resolves.toMatchObject({
        run: { id: "run-1", status: "completed" }
      });
      await expect(client.getRunDetail("run-1")).resolves.toMatchObject({
        artifacts: [{ id: "artifact-1" }]
      });
      await expect(client.listArtifacts("run-1")).resolves.toHaveLength(1);
      await expect(
        client.listCapabilityVisibility("workspace-alpha", "project-slice1")
      ).resolves.toHaveLength(1);
      await expect(client.listInboxItems("workspace-alpha")).resolves.toEqual([]);
      await expect(client.listNotifications("workspace-alpha")).resolves.toEqual([]);
      await expect(client.getKnowledgeDetail("run-1")).resolves.toMatchObject({
        knowledge_space: { id: "knowledge-space-1" }
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
        case "hub:create_task":
          expect(payload).toEqual(taskCreateCommandFixture);
          return taskFixture;
        case "hub:start_task":
        case "hub:get_run_detail":
        case "hub:resolve_approval":
          return runDetailFixture;
        case "hub:list_artifacts":
          return runDetailFixture.artifacts;
        case "hub:list_capability_visibility":
          return capabilityVisibilityFixture;
        case "hub:list_inbox_items":
        case "hub:list_notifications":
          return [];
        case "hub:get_knowledge_detail":
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
      fetch: async (input, init) => {
        const method = init?.method ?? "GET";
        const url = String(input);

        if (method === "GET" && url === "http://hub.test/api/workspaces/workspace-alpha/projects/project-slice1/context") {
          return Response.json(projectContextFixture);
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
        if (method === "GET" && url === "http://hub.test/api/workspaces/workspace-alpha/projects/project-slice1/capabilities") {
          return Response.json(capabilityVisibilityFixture);
        }
        if (method === "GET" && url === "http://hub.test/api/runs/run-1/knowledge") {
          return Response.json(knowledgeDetailFixture);
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
  it("injects bearer tokens into remote hub requests", async () => {
    const client = createRemoteHubClient({
      baseUrl: "http://hub.test",
      getAccessToken: async () => "remote-token",
      fetch: async (input, init) => {
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
});
