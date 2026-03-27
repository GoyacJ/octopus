import { type ErrorObject } from "ajv";
import Ajv2020 from "ajv/dist/2020";
import addFormats from "ajv-formats";

import approvalRequestStatusSchema from "../../../schemas/governance/approval-request-status.schema.json";
import approvalRequestSchema from "../../../schemas/governance/approval-request.schema.json";
import approvalResolveCommandSchema from "../../../schemas/governance/approval-resolve-command.schema.json";
import budgetPolicySchema from "../../../schemas/governance/budget-policy.schema.json";
import capabilityBindingSchema from "../../../schemas/governance/capability-binding.schema.json";
import capabilityDescriptorSchema from "../../../schemas/governance/capability-descriptor.schema.json";
import capabilityGrantSchema from "../../../schemas/governance/capability-grant.schema.json";
import capabilityVisibilitySchema from "../../../schemas/governance/capability-visibility.schema.json";
import hubAuthErrorSchema from "../../../schemas/interop/hub-auth-error.schema.json";
import hubConnectionStatusSchema from "../../../schemas/interop/hub-connection-status.schema.json";
import hubEventSchema from "../../../schemas/interop/hub-event.schema.json";
import hubLoginCommandSchema from "../../../schemas/interop/hub-login-command.schema.json";
import hubLoginResponseSchema from "../../../schemas/interop/hub-login-response.schema.json";
import hubSessionSchema from "../../../schemas/interop/hub-session.schema.json";
import knowledgeSpaceSchema from "../../../schemas/context/knowledge-space.schema.json";
import projectContextSchema from "../../../schemas/context/project-context.schema.json";
import projectSchema from "../../../schemas/context/project.schema.json";
import workspaceSchema from "../../../schemas/context/workspace.schema.json";
import artifactSchema from "../../../schemas/observe/artifact.schema.json";
import artifactSummarySchema from "../../../schemas/observe/artifact-summary.schema.json";
import auditRecordSchema from "../../../schemas/observe/audit-record.schema.json";
import inboxItemStatusSchema from "../../../schemas/observe/inbox-item-status.schema.json";
import inboxItemSchema from "../../../schemas/observe/inbox-item.schema.json";
import knowledgeAssetStatusSchema from "../../../schemas/observe/knowledge-asset-status.schema.json";
import knowledgeAssetSchema from "../../../schemas/observe/knowledge-asset.schema.json";
import knowledgeCandidateStatusSchema from "../../../schemas/observe/knowledge-candidate-status.schema.json";
import knowledgeCandidateSchema from "../../../schemas/observe/knowledge-candidate.schema.json";
import knowledgeDetailSchema from "../../../schemas/observe/knowledge-detail.schema.json";
import knowledgeLineageRecordSchema from "../../../schemas/observe/knowledge-lineage-record.schema.json";
import knowledgePromoteCommandSchema from "../../../schemas/observe/knowledge-promote-command.schema.json";
import knowledgeSummarySchema from "../../../schemas/observe/knowledge-summary.schema.json";
import notificationStatusSchema from "../../../schemas/observe/notification-status.schema.json";
import notificationSchema from "../../../schemas/observe/notification.schema.json";
import policyDecisionLogSchema from "../../../schemas/observe/policy-decision-log.schema.json";
import traceRecordSchema from "../../../schemas/observe/trace-record.schema.json";
import automationSchema from "../../../schemas/runtime/automation.schema.json";
import automationDetailSchema from "../../../schemas/runtime/automation-detail.schema.json";
import automationLifecycleCommandSchema from "../../../schemas/runtime/automation-lifecycle-command.schema.json";
import automationStatusSchema from "../../../schemas/runtime/automation-status.schema.json";
import automationSummarySchema from "../../../schemas/runtime/automation-summary.schema.json";
import createAutomationCommandSchema from "../../../schemas/runtime/create-automation-command.schema.json";
import createAutomationResponseSchema from "../../../schemas/runtime/create-automation-response.schema.json";
import createTriggerInputSchema from "../../../schemas/runtime/create-trigger-input.schema.json";
import environmentLeaseStatusSchema from "../../../schemas/runtime/environment-lease-status.schema.json";
import environmentLeaseSchema from "../../../schemas/runtime/environment-lease.schema.json";
import manualDispatchCommandSchema from "../../../schemas/runtime/manual-dispatch-command.schema.json";
import runDetailSchema from "../../../schemas/runtime/run-detail.schema.json";
import runSchema from "../../../schemas/runtime/run.schema.json";
import runStatusSchema from "../../../schemas/runtime/run-status.schema.json";
import runSummarySchema from "../../../schemas/runtime/run-summary.schema.json";
import taskCreateCommandSchema from "../../../schemas/runtime/task-create-command.schema.json";
import taskSchema from "../../../schemas/runtime/task.schema.json";
import triggerDeliveryRetryCommandSchema from "../../../schemas/runtime/trigger-delivery-retry-command.schema.json";
import triggerDeliveryStatusSchema from "../../../schemas/runtime/trigger-delivery-status.schema.json";
import triggerDeliverySchema from "../../../schemas/runtime/trigger-delivery.schema.json";
import triggerSchema from "../../../schemas/runtime/trigger.schema.json";

import type {
  ApprovalRequest,
  ApprovalResolveCommand,
  AutomationDetail,
  AutomationLifecycleCommand,
  AutomationSummary,
  Artifact,
  ArtifactSummary,
  AuditRecord,
  CapabilityVisibility,
  CreateAutomationCommand,
  CreateAutomationResponse,
  HubAuthError,
  HubConnectionStatus,
  HubEvent,
  HubLoginCommand,
  HubLoginResponse,
  HubSession,
  InboxItem,
  KnowledgeDetail,
  KnowledgePromoteCommand,
  ManualDispatchCommand,
  KnowledgeSummary,
  Notification,
  ProjectContext,
  RunDetail,
  RunSummary,
  Task,
  TaskCreateCommand,
  TriggerDeliveryRetryCommand
} from "./contracts";

export * from "./contracts";

const schemaRegistry = {
  [workspaceSchema.$id]: workspaceSchema,
  [projectSchema.$id]: projectSchema,
  [projectContextSchema.$id]: projectContextSchema,
  [knowledgeSpaceSchema.$id]: knowledgeSpaceSchema,
  [taskSchema.$id]: taskSchema,
  [taskCreateCommandSchema.$id]: taskCreateCommandSchema,
  [runStatusSchema.$id]: runStatusSchema,
  [runSchema.$id]: runSchema,
  [runSummarySchema.$id]: runSummarySchema,
  [runDetailSchema.$id]: runDetailSchema,
  [automationStatusSchema.$id]: automationStatusSchema,
  [automationSchema.$id]: automationSchema,
  [createTriggerInputSchema.$id]: createTriggerInputSchema,
  [createAutomationCommandSchema.$id]: createAutomationCommandSchema,
  [createAutomationResponseSchema.$id]: createAutomationResponseSchema,
  [automationSummarySchema.$id]: automationSummarySchema,
  [automationDetailSchema.$id]: automationDetailSchema,
  [automationLifecycleCommandSchema.$id]: automationLifecycleCommandSchema,
  [manualDispatchCommandSchema.$id]: manualDispatchCommandSchema,
  [triggerDeliveryRetryCommandSchema.$id]: triggerDeliveryRetryCommandSchema,
  [triggerSchema.$id]: triggerSchema,
  [triggerDeliveryStatusSchema.$id]: triggerDeliveryStatusSchema,
  [triggerDeliverySchema.$id]: triggerDeliverySchema,
  [environmentLeaseStatusSchema.$id]: environmentLeaseStatusSchema,
  [environmentLeaseSchema.$id]: environmentLeaseSchema,
  [approvalRequestStatusSchema.$id]: approvalRequestStatusSchema,
  [approvalRequestSchema.$id]: approvalRequestSchema,
  [approvalResolveCommandSchema.$id]: approvalResolveCommandSchema,
  [budgetPolicySchema.$id]: budgetPolicySchema,
  [capabilityBindingSchema.$id]: capabilityBindingSchema,
  [capabilityDescriptorSchema.$id]: capabilityDescriptorSchema,
  [capabilityGrantSchema.$id]: capabilityGrantSchema,
  [capabilityVisibilitySchema.$id]: capabilityVisibilitySchema,
  [hubAuthErrorSchema.$id]: hubAuthErrorSchema,
  [artifactSchema.$id]: artifactSchema,
  [artifactSummarySchema.$id]: artifactSummarySchema,
  [auditRecordSchema.$id]: auditRecordSchema,
  [traceRecordSchema.$id]: traceRecordSchema,
  [inboxItemStatusSchema.$id]: inboxItemStatusSchema,
  [inboxItemSchema.$id]: inboxItemSchema,
  [notificationStatusSchema.$id]: notificationStatusSchema,
  [notificationSchema.$id]: notificationSchema,
  [policyDecisionLogSchema.$id]: policyDecisionLogSchema,
  [knowledgeCandidateStatusSchema.$id]: knowledgeCandidateStatusSchema,
  [knowledgeCandidateSchema.$id]: knowledgeCandidateSchema,
  [knowledgeAssetStatusSchema.$id]: knowledgeAssetStatusSchema,
  [knowledgeAssetSchema.$id]: knowledgeAssetSchema,
  [knowledgeLineageRecordSchema.$id]: knowledgeLineageRecordSchema,
  [knowledgeSummarySchema.$id]: knowledgeSummarySchema,
  [knowledgeDetailSchema.$id]: knowledgeDetailSchema,
  [knowledgePromoteCommandSchema.$id]: knowledgePromoteCommandSchema,
  [hubLoginCommandSchema.$id]: hubLoginCommandSchema,
  [hubSessionSchema.$id]: hubSessionSchema,
  [hubLoginResponseSchema.$id]: hubLoginResponseSchema,
  [hubConnectionStatusSchema.$id]: hubConnectionStatusSchema,
  [hubEventSchema.$id]: hubEventSchema
} as const;

export const surfaceSchemas = schemaRegistry;

const ajv = new Ajv2020({
  allErrors: true,
  strict: false
});

addFormats(ajv);

for (const schema of Object.values(schemaRegistry)) {
  ajv.addSchema(schema);
}

export class SchemaValidationError extends Error {
  readonly schemaId: string;
  readonly details: ErrorObject[] | null | undefined;

  constructor(schemaId: string, details: ErrorObject[] | null | undefined) {
    super(formatSchemaErrors(schemaId, details));
    this.name = "SchemaValidationError";
    this.schemaId = schemaId;
    this.details = details;
  }
}

function formatSchemaErrors(
  schemaId: string,
  details: ErrorObject[] | null | undefined
): string {
  if (!details || details.length === 0) {
    return `Schema validation failed for ${schemaId}.`;
  }

  const message = details
    .map((error) => `${error.instancePath || "/"} ${error.message ?? "invalid"}`)
    .join("; ");

  return `Schema validation failed for ${schemaId}: ${message}`;
}

function parseWithSchema<T>(schemaId: string, value: unknown): T {
  const validator = ajv.getSchema(schemaId);

  if (!validator) {
    throw new Error(`Schema ${schemaId} is not registered.`);
  }

  if (validator(value)) {
    return value as T;
  }

  throw new SchemaValidationError(schemaId, validator.errors);
}

function parseArrayWithItemSchema<T>(schemaId: string, value: unknown): T[] {
  const validator = ajv.compile({
    type: "array",
    items: { $ref: schemaId }
  });

  if (validator(value)) {
    return value as T[];
  }

  throw new SchemaValidationError(`${schemaId}[]`, validator.errors);
}

export function parseProjectContext(value: unknown): ProjectContext {
  return parseWithSchema<ProjectContext>(projectContextSchema.$id, value);
}

export function parseTaskCreateCommand(value: unknown): TaskCreateCommand {
  return parseWithSchema<TaskCreateCommand>(taskCreateCommandSchema.$id, value);
}

export function parseCreateAutomationCommand(
  value: unknown
): CreateAutomationCommand {
  return parseWithSchema<CreateAutomationCommand>(
    createAutomationCommandSchema.$id,
    value
  );
}

export function parseCreateAutomationResponse(
  value: unknown
): CreateAutomationResponse {
  return parseWithSchema<CreateAutomationResponse>(
    createAutomationResponseSchema.$id,
    value
  );
}

export function parseAutomationSummary(value: unknown): AutomationSummary {
  return parseWithSchema<AutomationSummary>(automationSummarySchema.$id, value);
}

export function parseAutomationSummaries(
  value: unknown
): AutomationSummary[] {
  return parseArrayWithItemSchema<AutomationSummary>(
    automationSummarySchema.$id,
    value
  );
}

export function parseAutomationDetail(value: unknown): AutomationDetail {
  return parseWithSchema<AutomationDetail>(automationDetailSchema.$id, value);
}

export function parseAutomationLifecycleCommand(
  value: unknown
): AutomationLifecycleCommand {
  return parseWithSchema<AutomationLifecycleCommand>(
    automationLifecycleCommandSchema.$id,
    value
  );
}

export function parseManualDispatchCommand(
  value: unknown
): ManualDispatchCommand {
  return parseWithSchema<ManualDispatchCommand>(
    manualDispatchCommandSchema.$id,
    value
  );
}

export function parseTriggerDeliveryRetryCommand(
  value: unknown
): TriggerDeliveryRetryCommand {
  return parseWithSchema<TriggerDeliveryRetryCommand>(
    triggerDeliveryRetryCommandSchema.$id,
    value
  );
}

export function parseTask(value: unknown): Task {
  return parseWithSchema<Task>(taskSchema.$id, value);
}

export function parseRunSummary(value: unknown): RunSummary {
  return parseWithSchema<RunSummary>(runSummarySchema.$id, value);
}

export function parseRunDetail(value: unknown): RunDetail {
  return parseWithSchema<RunDetail>(runDetailSchema.$id, value);
}

export function parseApprovalRequest(value: unknown): ApprovalRequest {
  return parseWithSchema<ApprovalRequest>(approvalRequestSchema.$id, value);
}

export function parseApprovalResolveCommand(
  value: unknown
): ApprovalResolveCommand {
  return parseWithSchema<ApprovalResolveCommand>(
    approvalResolveCommandSchema.$id,
    value
  );
}

export function parseCapabilityVisibility(value: unknown): CapabilityVisibility {
  return parseWithSchema<CapabilityVisibility>(
    capabilityVisibilitySchema.$id,
    value
  );
}

export function parseCapabilityVisibilities(
  value: unknown
): CapabilityVisibility[] {
  return parseArrayWithItemSchema<CapabilityVisibility>(
    capabilityVisibilitySchema.$id,
    value
  );
}

export function parseArtifact(value: unknown): Artifact {
  return parseWithSchema<Artifact>(artifactSchema.$id, value);
}

export function parseArtifactSummary(value: unknown): ArtifactSummary {
  return parseWithSchema<ArtifactSummary>(artifactSummarySchema.$id, value);
}

export function parseArtifacts(value: unknown): Artifact[] {
  return parseArrayWithItemSchema<Artifact>(artifactSchema.$id, value);
}

export function parseInboxItems(value: unknown): InboxItem[] {
  return parseArrayWithItemSchema<InboxItem>(inboxItemSchema.$id, value);
}

export function parseNotifications(value: unknown): Notification[] {
  return parseArrayWithItemSchema<Notification>(notificationSchema.$id, value);
}

export function parseKnowledgeSummary(value: unknown): KnowledgeSummary {
  return parseWithSchema<KnowledgeSummary>(knowledgeSummarySchema.$id, value);
}

export function parseKnowledgeDetail(value: unknown): KnowledgeDetail {
  return parseWithSchema<KnowledgeDetail>(knowledgeDetailSchema.$id, value);
}

export function parseKnowledgePromoteCommand(
  value: unknown
): KnowledgePromoteCommand {
  return parseWithSchema<KnowledgePromoteCommand>(
    knowledgePromoteCommandSchema.$id,
    value
  );
}

export function parseHubLoginCommand(value: unknown): HubLoginCommand {
  return parseWithSchema<HubLoginCommand>(hubLoginCommandSchema.$id, value);
}

export function parseHubSession(value: unknown): HubSession {
  return parseWithSchema<HubSession>(hubSessionSchema.$id, value);
}

export function parseHubLoginResponse(value: unknown): HubLoginResponse {
  return parseWithSchema<HubLoginResponse>(hubLoginResponseSchema.$id, value);
}

export function parseHubAuthError(value: unknown): HubAuthError {
  return parseWithSchema<HubAuthError>(hubAuthErrorSchema.$id, value);
}

export function parseHubConnectionStatus(value: unknown): HubConnectionStatus {
  return parseWithSchema<HubConnectionStatus>(hubConnectionStatusSchema.$id, value);
}

export function parseHubEvent(value: unknown): HubEvent {
  return parseWithSchema<HubEvent>(hubEventSchema.$id, value);
}
