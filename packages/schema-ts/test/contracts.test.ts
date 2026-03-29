import { describe, expect, it } from "vitest";

import {
  LOCAL_HUB_TRANSPORT,
  parseApprovalRequest,
  parseAutomationDetail,
  parseAutomationLifecycleCommand,
  parseAutomationSummary,
  parseCapabilityResolution,
  parseCapabilityResolutions,
  parseCreateAutomationCommand,
  parseCreateAutomationResponse,
  parseHubAuthError,
  parseHubConnectionStatus,
  parseHubEvent,
  parseHubLoginCommand,
  parseHubLoginResponse,
  parseKnowledgeDetail,
  parseProjectKnowledgeIndex,
  parseLocalHubTransportContract,
  parseManualDispatchCommand,
  parseRequestKnowledgePromotionCommand,
  parseRunDetail,
  parseRunRetryCommand,
  parseRunSummaries,
  parseRunTerminateCommand,
  parseTaskCreateCommand,
  parseTriggerDeliveryRetryCommand
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

  it("accepts the minimum automation surface contracts", () => {
    expect(
      parseCreateAutomationCommand({
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
      }).trigger.trigger_type
    ).toBe("manual_event");

    expect(
      parseCreateAutomationResponse({
        automation: {
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
        },
        trigger: {
          id: "trigger-1",
          automation_id: "automation-1",
          trigger_type: "manual_event",
          config: {},
          created_at: "2026-03-26T10:00:00Z",
          updated_at: "2026-03-26T10:00:00Z"
        },
        webhook_secret: null
      }).automation.status
    ).toBe("active");

    expect(
      parseAutomationSummary({
        automation: {
          id: "automation-1",
          workspace_id: "workspace-alpha",
          project_id: "project-slice1",
          trigger_id: "trigger-1",
          status: "paused",
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
        },
        trigger: {
          id: "trigger-1",
          automation_id: "automation-1",
          trigger_type: "manual_event",
          config: {},
          created_at: "2026-03-26T10:00:00Z",
          updated_at: "2026-03-26T10:00:00Z"
        },
        recent_deliveries: [],
        last_run_summary: null
      }).automation.status
    ).toBe("paused");

    expect(
      parseAutomationDetail({
        automation: {
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
        },
        trigger: {
          id: "trigger-1",
          automation_id: "automation-1",
          trigger_type: "manual_event",
          config: {},
          created_at: "2026-03-26T10:00:00Z",
          updated_at: "2026-03-26T10:00:00Z"
        },
        recent_deliveries: [
          {
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
          }
        ],
        last_run_summary: {
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
        }
      }).last_run_summary?.run_type
    ).toBe("automation");

    expect(
      parseAutomationLifecycleCommand({
        automation_id: "automation-1",
        action: "archive"
      }).action
    ).toBe("archive");

    expect(
      parseManualDispatchCommand({
        trigger_id: "trigger-1",
        dedupe_key: "manual-1",
        payload: {
          source: "manual"
        }
      }).trigger_id
    ).toBe("trigger-1");

    expect(
      parseTriggerDeliveryRetryCommand({
        delivery_id: "delivery-1"
      }).delivery_id
    ).toBe("delivery-1");

    expect(
      parseRunRetryCommand({
        run_id: "run-1"
      }).run_id
    ).toBe("run-1");

    expect(
      parseRunTerminateCommand({
        run_id: "run-1",
        reason: "desktop_operator_stopped"
      }).reason
    ).toBe("desktop_operator_stopped");
  });

  it("accepts the shared local hub transport owner contract", () => {
    const contract = parseLocalHubTransportContract(LOCAL_HUB_TRANSPORT);

    expect(contract.event_channel).toBe("hub://events");
    expect(contract.commands.get_project_context).toBeTruthy();
    expect(contract.commands.get_project_knowledge).toBeTruthy();
    expect(contract.commands.list_projects).toBeTruthy();
    expect(contract.commands.list_runs).toBeTruthy();
    expect(contract.commands.retry_run).toBeTruthy();
    expect(contract.commands.terminate_run).toBeTruthy();
    expect(contract.commands.get_connection_status).toBeTruthy();
    expect(Object.values(contract.commands)).toHaveLength(27);
  });

  it("accepts run summary arrays for the recent-runs workbench surface", () => {
    expect(
      parseRunSummaries([
        {
          id: "run-2",
          task_id: "task-2",
          workspace_id: "workspace-alpha",
          project_id: "project-slice1",
          title: "Follow up note",
          run_type: "task",
          status: "waiting_approval",
          approval_request_id: "approval-1",
          attempt_count: 1,
          max_attempts: 2,
          last_error: null,
          created_at: "2026-03-26T10:02:00Z",
          updated_at: "2026-03-26T10:02:01Z",
          started_at: "2026-03-26T10:02:00Z",
          completed_at: null,
          terminated_at: null
        },
        {
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
        }
      ])[0].id
    ).toBe("run-2");
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

  it("accepts capability resolutions with governance explainability", () => {
    expect(
      parseCapabilityResolution({
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
          created_at: "2026-03-28T10:00:00Z",
          updated_at: "2026-03-28T10:00:00Z"
        },
        scope_ref: "workspace:workspace-alpha/project:project-slice1",
        execution_state: "approval_required",
        reason_code: "budget_soft_limit_exceeded",
        explanation:
          "Approval required because the estimated cost 7 exceeds the soft cost limit 5."
      }).execution_state
    ).toBe("approval_required");

    expect(
      parseCapabilityResolutions([
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
            created_at: "2026-03-28T10:00:00Z",
            updated_at: "2026-03-28T10:00:00Z"
          },
          scope_ref: "workspace:workspace-alpha/project:project-slice1",
          execution_state: "executable",
          reason_code: "within_budget",
          explanation:
            "Executable because the capability is bound, granted, and within the current budget."
        }
      ])[0].reason_code
    ).toBe("within_budget");
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
          auth_state: "authenticated",
          active_server_count: 0,
          healthy_server_count: 0,
          servers: [],
          last_refreshed_at: "2026-03-26T10:00:01Z"
        }
      }).event_type
    ).toBe("hub.connection.updated");
  });

  it("accepts an auth-aware hub connection status payload", () => {
    expect(
      parseHubConnectionStatus({
        mode: "remote",
        state: "connected",
        auth_state: "token_expired",
        active_server_count: 1,
        healthy_server_count: 1,
        servers: [],
        last_refreshed_at: "2026-03-26T10:00:01Z"
      }).auth_state
    ).toBe("token_expired");
  });

  it("accepts a remote login command and response", () => {
    expect(
      parseHubLoginCommand({
        workspace_id: "workspace-alpha",
        email: "admin@octopus.local",
        password: "octopus-bootstrap-password"
      }).workspace_id
    ).toBe("workspace-alpha");

    expect(
      parseHubLoginResponse({
        access_token: "jwt-token",
        session: {
          session_id: "session-1",
          user_id: "remote-user-bootstrap-admin",
          email: "admin@octopus.local",
          workspace_id: "workspace-alpha",
          actor_ref: "workspace_admin:bootstrap_admin",
          issued_at: "2026-03-26T10:00:00Z",
          expires_at: "2026-03-26T11:00:00Z"
        }
      }).session.actor_ref
    ).toBe("workspace_admin:bootstrap_admin");
  });

  it("accepts a structured auth failure payload", () => {
    expect(
      parseHubAuthError({
        error: "token expired",
        error_code: "token_expired",
        auth_state: "token_expired"
      }).error_code
    ).toBe("token_expired");
  });

  it("accepts governance interaction approval detail and request-promotion payloads", () => {
    expect(
      parseApprovalRequest({
        id: "approval-knowledge-1",
        workspace_id: "workspace-alpha",
        project_id: "project-slice1",
        run_id: "run-1",
        task_id: "task-1",
        approval_type: "knowledge_promotion",
        target_ref: "knowledge_candidate:candidate-1",
        status: "pending",
        reason: "knowledge_promotion_requested",
        dedupe_key: "knowledge_promotion:candidate-1:approval-knowledge-1",
        decided_by: null,
        decision_note: null,
        decided_at: null,
        created_at: "2026-03-28T10:00:00Z",
        updated_at: "2026-03-28T10:00:00Z"
      }).target_ref
    ).toBe("knowledge_candidate:candidate-1");

    expect(
      parseRequestKnowledgePromotionCommand({
        candidate_id: "candidate-1",
        actor_ref: "workspace_admin:alice",
        note: "request review"
      }).candidate_id
    ).toBe("candidate-1");
  });

  it("accepts verified_shared as the knowledge candidate lifecycle state", () => {
    expect(
      parseKnowledgeDetail({
        knowledge_space: {
          id: "knowledge-space-1",
          workspace_id: "workspace-alpha",
          project_id: "project-slice1",
          owner_ref: "workspace_admin:alice",
          display_name: "Project Slice 1 Knowledge",
          created_at: "2026-03-26T10:00:00Z",
          updated_at: "2026-03-26T10:00:00Z"
        },
        candidates: [
          {
            id: "candidate-1",
            knowledge_space_id: "knowledge-space-1",
            source_run_id: "run-1",
            source_task_id: "task-1",
            source_artifact_id: "artifact-1",
            capability_id: "capability-write-note",
            status: "verified_shared",
            content: "hello",
            provenance_source: "builtin",
            source_trust_level: "trusted",
            dedupe_key: "knowledge_candidate:artifact:artifact-1",
            created_at: "2026-03-26T10:00:01Z",
            updated_at: "2026-03-26T10:00:02Z"
          }
        ],
        assets: [],
        lineage: []
      }).candidates[0].status
    ).toBe("verified_shared");
  });

  it("accepts a project-scoped knowledge index that reuses KnowledgeSummary rows", () => {
    expect(
      parseProjectKnowledgeIndex({
        knowledge_space: {
          id: "knowledge-space-1",
          workspace_id: "workspace-alpha",
          project_id: "project-slice1",
          owner_ref: "workspace_admin:alice",
          display_name: "Project Slice 1 Knowledge",
          created_at: "2026-03-29T10:00:00Z",
          updated_at: "2026-03-29T10:00:00Z"
        },
        entries: [
          {
            kind: "candidate",
            id: "candidate-2",
            knowledge_space_id: "knowledge-space-1",
            capability_id: "capability-write-note",
            status: "candidate",
            source_run_id: "run-2",
            source_artifact_id: "artifact-2",
            source_candidate_id: null,
            provenance_source: "builtin",
            trust_level: "trusted",
            created_at: "2026-03-29T10:00:02Z"
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
            created_at: "2026-03-29T10:00:01Z"
          }
        ]
      }).entries[0].kind
    ).toBe("candidate");
  });
});
