import { describe, expect, it } from "vitest";

import {
  parseHubEvent,
  parseRunDetail,
  parseTaskCreateCommand
} from "../src/index";

describe("schema-ts contract parsers", () => {
  it("accepts a valid task create command", () => {
    expect(
      parseTaskCreateCommand({
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
      })
    ).toMatchObject({
      capability_id: "capability-write-note",
      workspace_id: "workspace-alpha"
    });
  });

  it("rejects an invalid task create command", () => {
    expect(() =>
      parseTaskCreateCommand({
        workspace_id: "workspace-alpha",
        project_id: "project-slice1",
        title: "",
        instruction: "Emit a deterministic artifact",
        action: {
          kind: "emit_text",
          content: "hello"
        },
        capability_id: "capability-write-note",
        estimated_cost: 1,
        idempotency_key: "task-1"
      })
    ).toThrow(/title/i);
  });

  it("accepts a run detail payload composed from shared schemas", () => {
    expect(
      parseRunDetail({
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
          idempotency_key: "run-task-1",
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
        task: {
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
        },
        artifacts: [],
        audits: [],
        traces: [],
        approvals: [],
        inbox_items: [],
        notifications: [],
        policy_decisions: [],
        knowledge_candidates: [],
        knowledge_assets: [],
        knowledge_lineage: []
      }).run.status
    ).toBe("completed");
  });

  it("accepts a hub event payload", () => {
    expect(
      parseHubEvent({
        event_type: "hub.connection.updated",
        sequence: 1,
        occurred_at: "2026-03-26T10:00:01Z",
        payload: {
          mode: "local",
          state: "connected",
          active_server_count: 0,
          healthy_server_count: 0,
          servers: [],
          last_refreshed_at: "2026-03-26T10:00:01Z"
        }
      }).event_type
    ).toBe("hub.connection.updated");
  });
});
