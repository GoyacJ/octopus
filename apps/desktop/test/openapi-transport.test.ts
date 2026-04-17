// @vitest-environment jsdom

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type {
  AccessAuditListResponse,
  ArtifactVersionReference,
  AuditRecord,
  ClientAppRecord,
  CreateDeliverableVersionInput,
  DeliverableDetail,
  DeliverableSummary,
  DeliverableVersionContent,
  DeliverableVersionSummary,
  ForkDeliverableInput,
  HealthcheckStatus,
  InboxItemRecord,
  KnowledgeEntryRecord,
  NotificationRecord,
  ProjectRecord,
  PromoteDeliverableInput,
  RuntimeConfiguredModelProbeResult,
  RuntimeEffectiveConfig,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  WorkspaceSkillFileDocument,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema/generated'

import {
  fetchHostOpenApi,
  fetchWorkspaceOpenApi,
  normalizeComparableApiPath,
  openWorkspaceOpenApiStream,
} from '@/tauri/shared'

const fetchSpy = vi.fn()

function createWorkspaceConnection(): WorkspaceConnectionRecord {
  return {
    workspaceConnectionId: 'conn-local',
    workspaceId: 'ws-local',
    label: 'Local Runtime',
    baseUrl: 'http://127.0.0.1:43127',
    transportSecurity: 'loopback',
    authMode: 'session-token',
    status: 'connected',
  }
}

function createWorkspaceSession(
  connection: WorkspaceConnectionRecord,
): WorkspaceSessionTokenEnvelope {
  return {
    workspaceConnectionId: connection.workspaceConnectionId,
    token: 'workspace-session-token',
    issuedAt: 1,
    session: {
      id: 'sess-1',
      workspaceId: connection.workspaceId,
      userId: 'user-owner',
      clientAppId: 'octopus-desktop',
      token: 'workspace-session-token',
      status: 'active',
      createdAt: 1,
      expiresAt: undefined,
    },
  }
}

describe('OpenAPI transport helpers', () => {
  beforeEach(() => {
    fetchSpy.mockReset()
    vi.stubGlobal('fetch', fetchSpy)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('normalizes server, spec, and template paths into one comparable form', () => {
    expect(normalizeComparableApiPath('/api/v1/host/notifications/:notification_id/read'))
      .toBe('/api/v1/host/notifications/{param}/read')
    expect(normalizeComparableApiPath('/api/v1/host/notifications/{notificationId}/read'))
      .toBe('/api/v1/host/notifications/{param}/read')
    expect(normalizeComparableApiPath('/api/v1/host/notifications/${id}/read'))
      .toBe('/api/v1/host/notifications/{param}/read')
    expect(normalizeComparableApiPath('/api/v1/workspace/catalog/skills/${skillId}/files/${relativePath}'))
      .toBe('/api/v1/workspace/catalog/skills/{param}/files/{param}')
  })

  it('keeps background runtime event kinds in the generated transport contract', () => {
    const generated = readFileSync(
      resolve(import.meta.dirname, '../../../packages/schema/src/generated.ts'),
      'utf8',
    )

    expect(generated).toContain('"background.started"')
    expect(generated).toContain('"background.paused"')
    expect(generated).toContain('"background.completed"')
    expect(generated).toContain('"background.failed"')
  })

  it('removes opaque runtime escape hatches from the generated public transport contract', () => {
    const generated = readFileSync(
      resolve(import.meta.dirname, '../../../packages/schema/src/generated.ts'),
      'utf8',
    )

    expect(generated).not.toContain('export type RuntimePermissionEnvelope = Record<string, unknown>')
    expect(generated).not.toContain('payload?: Record<string, unknown>')
    expect(generated).not.toContain('serializedSession:')
    expect(generated).not.toContain('compactionMetadata?:')
  })

  it('keeps the progressive access experience transport contract in generated and exported schema surfaces', () => {
    const generated = readFileSync(
      resolve(import.meta.dirname, '../../../packages/schema/src/generated.ts'),
      'utf8',
    )
    const accessControlSchema = readFileSync(
      resolve(import.meta.dirname, '../../../packages/schema/src/access-control.ts'),
      'utf8',
    )

    expect(generated).toContain('export interface AccessExperienceResponse')
    expect(generated).toContain('export interface AccessExperienceSummary')
    expect(generated).toContain('export interface AccessSectionGrant')
    expect(generated).toContain('export interface AccessRoleTemplate')
    expect(generated).toContain('export interface AccessRolePreset')
    expect(generated).toContain('export interface AccessCapabilityBundle')
    expect(generated).toContain('source: AccessRoleSource')
    expect(generated).toContain('export type AccessRoleSource = "system" | "custom"')
    expect(generated).toContain('editable: boolean')
    expect(generated).toContain('"/api/v1/access/experience": {')
    expect(generated).toContain('operationId: "getAccessExperience"')
    expect(generated).toContain('export interface AccessMemberSummary')
    expect(generated).toContain('primaryPresetCode: string | null')
    expect(generated).toContain('effectiveRoles: AccessMemberRoleSummary[]')
    expect(generated).toContain('export interface AccessMemberRoleSummary')
    expect(generated).toContain('source: AccessRoleSource')
    expect(generated).toContain('export interface AccessUserPresetUpdateRequest')
    expect(generated).toContain('"/api/v1/access/members": {')
    expect(generated).toContain('operationId: "listAccessMembers"')
    expect(generated).toContain('"/api/v1/access/users/{userId}/preset": {')
    expect(generated).toContain('operationId: "updateAccessUserPreset"')

    expect(accessControlSchema).toContain('AccessExperienceResponse as OpenApiAccessExperienceResponse')
    expect(accessControlSchema).toContain('export type AccessExperienceResponse = OpenApiAccessExperienceResponse')
    expect(accessControlSchema).toContain('AccessMemberSummary as OpenApiAccessMemberSummary')
    expect(accessControlSchema).toContain('AccessUserPresetUpdateRequest as OpenApiAccessUserPresetUpdateRequest')
  })

  it('keeps deliverable-first schemas and paths in the generated transport contract', () => {
    const legacySummaryAliasInterface = ['export interface ', 'Artifact', 'Record'].join('')
    const legacyWorkspaceArtifactsPath = ['"/api/v1', 'artifacts"'].join('/')
    const generated = readFileSync(
      resolve(import.meta.dirname, '../../../packages/schema/src/generated.ts'),
      'utf8',
    )

    expect(generated).toContain('export interface DeliverableSummary')
    expect(generated).toContain('export interface DeliverableDetail')
    expect(generated).toContain('export interface DeliverableVersionSummary')
    expect(generated).toContain('export interface DeliverableVersionContent')
    expect(generated).toContain('export interface ArtifactVersionReference')
    expect(generated).toContain('export interface CreateDeliverableVersionInput')
    expect(generated).toContain('export interface PromoteDeliverableInput')
    expect(generated).toContain('export interface ForkDeliverableInput')
    expect(generated).toContain('"/api/v1/workspace/deliverables"')
    expect(generated).toContain('"/api/v1/projects/{projectId}/deliverables"')
    expect(generated).toContain('"/api/v1/deliverables/{deliverableId}"')
    expect(generated).toContain('"/api/v1/deliverables/{deliverableId}/versions"')
    expect(generated).toContain('"/api/v1/deliverables/{deliverableId}/versions/{version}"')
    expect(generated).toContain('"/api/v1/deliverables/{deliverableId}/promote"')
    expect(generated).toContain('"/api/v1/deliverables/{deliverableId}/fork"')
    expect(generated).toContain('artifactRefs: ArtifactVersionReference[]')
    expect(generated).toContain('deliverableRefs?: ArtifactVersionReference[]')
    expect(generated).not.toContain(legacySummaryAliasInterface)
    expect(generated).not.toContain(legacyWorkspaceArtifactsPath)
  })

  it('models deliverable detail, versions, content, and actions as typed transport records', () => {
    const latestVersionRef: ArtifactVersionReference = {
      artifactId: 'artifact-1',
      version: 2,
      title: 'Runtime Delivery Summary',
      previewKind: 'markdown',
      updatedAt: 10,
    }
    const summary: DeliverableSummary = {
      id: 'artifact-1',
      workspaceId: 'ws-local',
      projectId: 'proj-redesign',
      conversationId: 'conv-1',
      title: 'Runtime Delivery Summary',
      status: 'review',
      previewKind: 'markdown',
      latestVersion: 2,
      updatedAt: 10,
      promotionState: 'not-promoted',
      latestVersionRef,
    }
    const detail: DeliverableDetail = {
      ...summary,
      sessionId: 'rt-1',
      runId: 'run-1',
    }
    const versionSummary: DeliverableVersionSummary = {
      artifactId: 'artifact-1',
      version: 2,
      title: 'Runtime Delivery Summary',
      previewKind: 'markdown',
      updatedAt: 10,
    }
    const versionContent: DeliverableVersionContent = {
      artifactId: 'artifact-1',
      version: 2,
      previewKind: 'markdown',
      editable: true,
      textContent: '# Runtime Delivery Summary',
    }
    const createVersionInput: CreateDeliverableVersionInput = {
      previewKind: 'markdown',
      textContent: '# Runtime Delivery Summary v2',
    }
    const promoteInput: PromoteDeliverableInput = {
      summary: 'Promote this deliverable into project knowledge.',
    }
    const forkInput: ForkDeliverableInput = {
      title: 'Follow-up conversation',
    }

    expect(detail.latestVersionRef.version).toBe(versionSummary.version)
    expect(versionContent.artifactId).toBe(summary.id)
    expect(createVersionInput.previewKind).toBe('markdown')
    expect(promoteInput.summary).toContain('project knowledge')
    expect(forkInput.title).toContain('Follow-up')
  })

  it('uses generated OpenAPI paths for host requests', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<HealthcheckStatus> => ({
        status: 'ok',
        host: 'web',
        mode: 'local',
        cargoWorkspace: false,
        backend: {
          state: 'ready',
          transport: 'http',
        },
      }),
    })

    const payload = await fetchHostOpenApi(
      'http://127.0.0.1:43127',
      'desktop-test-token',
      '/api/v1/host/health',
      'get',
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/host/health',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
    expect(payload.status).toBe('ok')
  })

  it('uses generated OpenAPI paths for workspace requests', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<RuntimeEffectiveConfig> => ({
        effectiveConfig: { locale: 'zh-CN' },
        effectiveConfigHash: 'hash-1',
        sources: [],
        validation: {
          valid: true,
          errors: [],
          warnings: [],
        },
        secretReferences: [],
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    const payload = await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/runtime/config',
      'get',
      { session },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/config',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = fetchSpy.mock.calls[0]?.[1] as RequestInit
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
    expect(payload.effectiveConfigHash).toBe('hash-1')
  })

  it('resolves generated host path templates before making the request', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<NotificationRecord> => ({
        id: 'notif-1',
        scopeKind: 'app',
        level: 'info',
        title: 'Saved',
        body: 'Preferences updated.',
        source: 'settings',
        createdAt: 1,
      }),
    })

    const payload = await fetchHostOpenApi(
      'http://127.0.0.1:43127',
      'desktop-test-token',
      '/api/v1/host/notifications/{notificationId}/read',
      'post',
      {
        pathParams: {
          notificationId: 'notif-1',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/host/notifications/notif-1/read',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )
    expect(payload.id).toBe('notif-1')
  })

  it('resolves generated workspace path templates before making the request', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<ProjectRecord> => ({
        id: 'proj-redesign',
        workspaceId: 'ws-local',
        name: 'Redesign',
        status: 'active',
        description: 'Main redesign project',
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    const payload = await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/projects/{projectId}',
      'patch',
      {
        session,
        body: JSON.stringify({
          name: 'Redesign',
          description: 'Main redesign project',
          status: 'active',
        }),
        pathParams: {
          projectId: 'proj-redesign',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )

    const request = fetchSpy.mock.calls[0]?.[1] as RequestInit
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(payload.id).toBe('proj-redesign')
  })

  it('builds project resource import requests with path params and JSON bodies', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'res-imported-folder',
        workspaceId: 'ws-local',
        projectId: 'proj-redesign',
        kind: 'folder',
        name: 'design-assets',
        origin: 'source',
        status: 'healthy',
        updatedAt: 1,
        tags: [],
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    await (fetchWorkspaceOpenApi as any)(
      connection,
      '/api/v1/projects/{projectId}/resources/import',
      'post',
      {
        session,
        pathParams: {
          projectId: 'proj-redesign',
        },
        body: JSON.stringify({
          name: 'design-assets',
          rootDirName: 'design-assets',
          scope: 'project',
          visibility: 'public',
          files: [
            {
              fileName: 'brief.md',
              contentType: 'text/markdown',
              dataBase64: 'IyBCcmllZg==',
              byteSize: 7,
              relativePath: 'brief.md',
            },
          ],
        }),
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/resources/import',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
        body: JSON.stringify({
          name: 'design-assets',
          rootDirName: 'design-assets',
          scope: 'project',
          visibility: 'public',
          files: [
            {
              fileName: 'brief.md',
              contentType: 'text/markdown',
              dataBase64: 'IyBCcmllZg==',
              byteSize: 7,
              relativePath: 'brief.md',
            },
          ],
        }),
      }),
    )
  })

  it('builds workspace filesystem directory requests with query params', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        currentPath: '/remote/projects',
        parentPath: '/remote',
        entries: [
          {
            name: 'design',
            path: '/remote/projects/design',
          },
        ],
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    await (fetchWorkspaceOpenApi as any)(
      connection,
      '/api/v1/workspace/filesystem/directories',
      'get',
      {
        session,
        queryParams: {
          path: '/remote/projects',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/filesystem/directories?path=%2Fremote%2Fprojects',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
  })

  it('encodes nested relativePath params for generated workspace skill file routes', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<WorkspaceSkillFileDocument> => ({
        skillId: 'skill-octopus',
        sourceKey: 'workspace:skill-octopus',
        path: 'docs/guide.md',
        displayPath: 'skills/skill-octopus/docs/guide.md',
        byteSize: 128,
        isText: true,
        content: '# Guide',
        contentType: 'text/markdown',
        language: 'markdown',
        readonly: false,
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    const payload = await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}',
      'get',
      {
        session,
        pathParams: {
          skillId: 'skill-octopus',
          relativePath: 'docs/guide.md',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-octopus/files/docs%2Fguide.md',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
    expect(payload.path).toBe('docs/guide.md')
  })

  it('resolves generated runtime config scope paths before making the request', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<RuntimeEffectiveConfig> => ({
        effectiveConfig: { locale: 'zh-CN' },
        effectiveConfigHash: 'hash-scope',
        sources: [],
        validation: {
          valid: true,
          errors: [],
          warnings: [],
        },
        secretReferences: [],
      }),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)
    const payload = await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/runtime/config/scopes/{scope}',
      'patch',
      {
        session,
        body: JSON.stringify({
          scope: 'workspace',
          patch: { locale: 'zh-CN' },
        }),
        pathParams: {
          scope: 'workspace',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/config/scopes/workspace',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )
    expect(payload.effectiveConfigHash).toBe('hash-scope')
  })

  it('resolves generated runtime config validate and probe paths with session headers', async () => {
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeEffectiveConfig['validation']> => ({
          valid: true,
          errors: [],
          warnings: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeConfiguredModelProbeResult> => ({
          valid: true,
          reachable: true,
          configuredModelId: 'anthropic-primary',
          configuredModelName: 'Claude Primary',
          requestId: 'probe-1',
          consumedTokens: 10,
          errors: [],
          warnings: [],
        }),
      })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)

    await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/runtime/config/validate',
      'post',
      {
        session,
        body: JSON.stringify({
          scope: 'workspace',
          patch: { locale: 'zh-CN' },
        }),
      },
    )

    await fetchWorkspaceOpenApi(
      connection,
      '/api/v1/runtime/config/configured-models/probe',
      'post',
      {
        session,
        body: JSON.stringify({
          scope: 'workspace',
          configuredModelId: 'anthropic-primary',
          patch: { configuredModels: {} },
        }),
      },
    )

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/runtime/config/validate',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/runtime/config/configured-models/probe',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )
    const firstHeaders = fetchSpy.mock.calls[0]?.[1]?.headers as Headers
    const secondHeaders = fetchSpy.mock.calls[1]?.[1]?.headers as Headers
    expect(firstHeaders.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(secondHeaders.get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('keeps managed configured-model credential paths and probe apiKey override in the generated transport contract', () => {
    const generated = readFileSync(
      resolve(import.meta.dirname, '../../../packages/schema/src/generated.ts'),
      'utf8',
    )

    expect(generated).toContain('"/api/v1/runtime/config/configured-models/{configuredModelId}/credential"')
    expect(generated).toContain('operationId: "upsertRuntimeConfiguredModelCredential"')
    expect(generated).toContain('operationId: "deleteRuntimeConfiguredModelCredential"')
    expect(generated).toContain('export interface RuntimeConfiguredModelCredentialUpsertInput')
    expect(generated).toContain('export interface RuntimeConfiguredModelCredentialRecord')
    expect(generated).toContain('apiKey?: string')
  })

  it('resolves generated runtime session and approval paths with path params, query params, and idempotency headers', async () => {
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeSessionSummary[]> => ([
          {
            id: 'rt-1',
            conversationId: 'conv-1',
            projectId: 'proj-redesign',
            title: 'Runtime Session',
            sessionKind: 'project',
            status: 'running',
            updatedAt: 2,
            configSnapshotId: 'cfg-1',
            effectiveConfigHash: 'hash-1',
            startedFromScopeSet: ['workspace'],
            selectedActorRef: 'agent:agent-architect',
            manifestRevision: 'manifest-1',
            sessionPolicy: {
              selectedActorRef: 'agent:agent-architect',
              selectedConfiguredModelId: 'anthropic-primary',
              executionPermissionMode: 'workspace-write',
              configSnapshotId: 'cfg-1',
              manifestRevision: 'manifest-1',
              capabilityPolicy: {},
              memoryPolicy: {},
              delegationPolicy: {},
              approvalPreference: {},
            },
            activeRunId: 'run-1',
            subrunCount: 0,
            workflow: {
              workflowRunId: 'wf-run-1',
              label: 'Runtime workflow',
              status: 'running',
              totalSteps: 3,
              completedSteps: 1,
              currentStepId: 'run-1',
              currentStepLabel: 'Leader plan',
              backgroundCapable: true,
              updatedAt: 2,
            },
            pendingMailbox: {
              mailboxRef: 'mailbox-1',
              channel: 'leader-hub',
              status: 'pending',
              pendingCount: 1,
              totalMessages: 1,
              updatedAt: 2,
            },
            backgroundRun: {
              runId: 'run-1',
              workflowRunId: 'wf-run-1',
              status: 'running',
              backgroundCapable: true,
              updatedAt: 2,
            },
            memorySummary: {
              summary: 'No durable memories selected.',
              durableMemoryCount: 0,
              selectedMemoryIds: [],
            },
            capabilityPlanSummary: {
              activatedTools: [],
              approvedTools: [],
              authResolvedTools: [],
              availableResources: [],
              deferredTools: [],
              discoverableSkills: [],
              grantedTools: [],
              hiddenCapabilities: [],
              pendingTools: [],
              providerFallbacks: [],
              visibleTools: [],
            },
            capabilityStateRef: 'capstate-1',
            pendingMediation: {
              mediationKind: 'none',
            },
            providerStateSummary: [],
            lastExecutionOutcome: {
              outcome: 'success',
            },
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeSessionDetail> => ({
          summary: {
            id: 'rt-1',
            conversationId: 'conv-1',
            projectId: 'proj-redesign',
            title: 'Runtime Session',
            sessionKind: 'project',
            status: 'running',
            updatedAt: 2,
            configSnapshotId: 'cfg-1',
            effectiveConfigHash: 'hash-1',
            startedFromScopeSet: ['workspace'],
            selectedActorRef: 'agent:agent-architect',
            manifestRevision: 'manifest-1',
            sessionPolicy: {
              selectedActorRef: 'agent:agent-architect',
              selectedConfiguredModelId: 'anthropic-primary',
              executionPermissionMode: 'workspace-write',
              configSnapshotId: 'cfg-1',
              manifestRevision: 'manifest-1',
              capabilityPolicy: {},
              memoryPolicy: {},
              delegationPolicy: {},
              approvalPreference: {},
            },
            activeRunId: 'run-1',
            subrunCount: 0,
            workflow: {
              workflowRunId: 'wf-run-1',
              label: 'Runtime workflow',
              status: 'running',
              totalSteps: 3,
              completedSteps: 1,
              currentStepId: 'run-1',
              currentStepLabel: 'Leader plan',
              backgroundCapable: true,
              updatedAt: 2,
            },
            pendingMailbox: {
              mailboxRef: 'mailbox-1',
              channel: 'leader-hub',
              status: 'pending',
              pendingCount: 1,
              totalMessages: 1,
              updatedAt: 2,
            },
            backgroundRun: {
              runId: 'run-1',
              workflowRunId: 'wf-run-1',
              status: 'running',
              backgroundCapable: true,
              updatedAt: 2,
            },
            memorySummary: {
              summary: 'No durable memories selected.',
              durableMemoryCount: 0,
              selectedMemoryIds: [],
            },
            capabilityPlanSummary: {
              activatedTools: [],
              approvedTools: [],
              authResolvedTools: [],
              availableResources: [],
              deferredTools: [],
              discoverableSkills: [],
              grantedTools: [],
              hiddenCapabilities: [],
              pendingTools: [],
              providerFallbacks: [],
              visibleTools: [],
            },
            capabilityStateRef: 'capstate-1',
            pendingMediation: {
              mediationKind: 'none',
            },
            providerStateSummary: [],
            lastExecutionOutcome: {
              outcome: 'success',
            },
          },
          run: {
            id: 'run-1',
            sessionId: 'rt-1',
            conversationId: 'conv-1',
            status: 'running',
            currentStep: 'awaiting_approval',
            startedAt: 1,
            updatedAt: 2,
            actorRef: 'agent:agent-architect',
            runKind: 'primary',
            workflowRun: 'wf-run-1',
            mailboxRef: 'mailbox-1',
            backgroundState: 'running',
            workerDispatch: {
              totalSubruns: 2,
              activeSubruns: 1,
              completedSubruns: 1,
              failedSubruns: 0,
            },
            capabilityPlanSummary: {
              activatedTools: [],
              approvedTools: [],
              authResolvedTools: [],
              availableResources: [],
              deferredTools: [],
              discoverableSkills: [],
              grantedTools: [],
              hiddenCapabilities: [],
              pendingTools: [],
              providerFallbacks: [],
              visibleTools: [],
            },
            capabilityStateRef: 'capstate-1',
            configSnapshotId: 'cfg-1',
            effectiveConfigHash: 'hash-1',
            startedFromScopeSet: ['workspace'],
            pendingMediation: {
              mediationKind: 'none',
            },
            providerStateSummary: [],
            lastExecutionOutcome: {
              outcome: 'success',
            },
            usageSummary: {
              inputTokens: 0,
              outputTokens: 0,
              totalTokens: 0,
            },
            artifactRefs: [],
            traceContext: {
              sessionId: 'rt-1',
              traceId: 'trace-1',
              turnId: 'turn-1',
            },
            checkpoint: {
              serializedSession: {
                sessionId: 'rt-1',
                runId: 'run-1',
              },
              capabilityPlanSummary: {
                activatedTools: [],
                approvedTools: [],
                authResolvedTools: [],
                availableResources: [],
                deferredTools: [],
                discoverableSkills: [],
                grantedTools: [],
                hiddenCapabilities: [],
                pendingTools: [],
                providerFallbacks: [],
                visibleTools: [],
              },
              capabilityStateRef: 'capstate-1',
              currentIterationIndex: 0,
              pendingMediation: {
                mediationKind: 'none',
              },
              lastExecutionOutcome: {
                outcome: 'success',
              },
              usageSummary: {
                inputTokens: 0,
                outputTokens: 0,
                totalTokens: 0,
              },
            },
          },
          subruns: [
            {
              runId: 'run-sub-1',
              parentRunId: 'run-1',
              actorRef: 'agent:agent-architect',
              label: 'Architect',
              status: 'completed',
              runKind: 'subrun',
              delegatedByToolCallId: 'tool-1',
              startedAt: 1,
              updatedAt: 2,
            },
          ],
          handoffs: [
            {
              handoffRef: 'handoff-1',
              mailboxRef: 'mailbox-1',
              senderActorRef: 'team:team-workspace-core',
              receiverActorRef: 'agent:agent-architect',
              state: 'delivered',
              artifactRefs: ['artifact-1'],
              updatedAt: 2,
            },
          ],
          workflow: {
            workflowRunId: 'wf-run-1',
            label: 'Runtime workflow',
            status: 'running',
            totalSteps: 3,
            completedSteps: 1,
            currentStepId: 'run-1',
            currentStepLabel: 'Leader plan',
            backgroundCapable: true,
            updatedAt: 2,
          },
          pendingMailbox: {
            mailboxRef: 'mailbox-1',
            channel: 'leader-hub',
            status: 'pending',
            pendingCount: 1,
            totalMessages: 1,
            updatedAt: 2,
          },
          backgroundRun: {
            runId: 'run-1',
            workflowRunId: 'wf-run-1',
            status: 'running',
            backgroundCapable: true,
            updatedAt: 2,
          },
          messages: [],
          trace: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeRunSnapshot> => ({
          id: 'run-1',
          sessionId: 'rt-1',
          conversationId: 'conv-1',
          status: 'running',
          currentStep: 'awaiting_approval',
          startedAt: 1,
          updatedAt: 2,
          actorRef: 'agent:agent-architect',
          runKind: 'primary',
          workflowRun: 'wf-run-1',
          mailboxRef: 'mailbox-1',
          backgroundState: 'running',
          workerDispatch: {
            totalSubruns: 2,
            activeSubruns: 1,
            completedSubruns: 1,
            failedSubruns: 0,
          },
          capabilityPlanSummary: {
            activatedTools: [],
            approvedTools: [],
            authResolvedTools: [],
            availableResources: [],
            deferredTools: [],
            discoverableSkills: [],
            grantedTools: [],
            hiddenCapabilities: [],
            pendingTools: [],
            providerFallbacks: [],
            visibleTools: [],
          },
          capabilityStateRef: 'capstate-1',
          configSnapshotId: 'cfg-1',
          effectiveConfigHash: 'hash-1',
          startedFromScopeSet: ['workspace'],
          pendingMediation: {
            mediationKind: 'none',
          },
          providerStateSummary: [],
          lastExecutionOutcome: {
            outcome: 'success',
          },
          usageSummary: {
            inputTokens: 0,
            outputTokens: 0,
            totalTokens: 0,
          },
          artifactRefs: [],
          traceContext: {
            sessionId: 'rt-1',
            traceId: 'trace-1',
            turnId: 'turn-1',
          },
          checkpoint: {
            serializedSession: {
              sessionId: 'rt-1',
              runId: 'run-1',
            },
            capabilityPlanSummary: {
              activatedTools: [],
              approvedTools: [],
              authResolvedTools: [],
              availableResources: [],
              deferredTools: [],
              discoverableSkills: [],
              grantedTools: [],
              hiddenCapabilities: [],
              pendingTools: [],
              providerFallbacks: [],
              visibleTools: [],
            },
            capabilityStateRef: 'capstate-1',
            currentIterationIndex: 0,
            pendingMediation: {
              mediationKind: 'none',
            },
            lastExecutionOutcome: {
              outcome: 'success',
            },
            usageSummary: {
              inputTokens: 0,
              outputTokens: 0,
              totalTokens: 0,
            },
          },
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        status: 204,
        headers: new Headers(),
        text: async () => '',
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<RuntimeRunSnapshot> => ({
          id: 'run-1',
          sessionId: 'rt-1',
          conversationId: 'conv-1',
          status: 'completed',
          currentStep: 'runtime.run.completed',
          startedAt: 1,
          updatedAt: 3,
          actorRef: 'agent:agent-architect',
          runKind: 'primary',
          workflowRun: 'wf-run-1',
          mailboxRef: 'mailbox-1',
          backgroundState: 'completed',
          workerDispatch: {
            totalSubruns: 2,
            activeSubruns: 0,
            completedSubruns: 2,
            failedSubruns: 0,
          },
          capabilityPlanSummary: {
            activatedTools: [],
            approvedTools: [],
            authResolvedTools: [],
            availableResources: [],
            deferredTools: [],
            discoverableSkills: [],
            grantedTools: [],
            hiddenCapabilities: [],
            pendingTools: [],
            providerFallbacks: [],
            visibleTools: [],
          },
          capabilityStateRef: 'capstate-1',
          configSnapshotId: 'cfg-1',
          effectiveConfigHash: 'hash-1',
          startedFromScopeSet: ['workspace'],
          pendingMediation: {
            mediationKind: 'none',
          },
          providerStateSummary: [],
          lastExecutionOutcome: {
            outcome: 'success',
          },
          usageSummary: {
            inputTokens: 0,
            outputTokens: 0,
            totalTokens: 0,
          },
          artifactRefs: [],
          traceContext: {
            sessionId: 'rt-1',
            traceId: 'trace-1',
            turnId: 'turn-1',
          },
          checkpoint: {
            serializedSession: {
              sessionId: 'rt-1',
              runId: 'run-1',
            },
            capabilityPlanSummary: {
              activatedTools: [],
              approvedTools: [],
              authResolvedTools: [],
              availableResources: [],
              deferredTools: [],
              discoverableSkills: [],
              grantedTools: [],
              hiddenCapabilities: [],
              pendingTools: [],
              providerFallbacks: [],
              visibleTools: [],
            },
            capabilityStateRef: 'capstate-1',
            currentIterationIndex: 0,
            pendingMediation: {
              mediationKind: 'none',
            },
            lastExecutionOutcome: {
              outcome: 'success',
            },
            usageSummary: {
              inputTokens: 0,
              outputTokens: 0,
              totalTokens: 0,
            },
          },
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => [],
      })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)

    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions', 'get', { session })
    const runtimeDetailPayload = await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}', 'get', {
      session,
      pathParams: { sessionId: 'rt-1' },
    })
    const runtimeRunPayload = await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}/turns', 'post', {
      session,
      pathParams: { sessionId: 'rt-1' },
      idempotencyKey: 'idem-turn-1',
      body: JSON.stringify({
        content: 'hello',
        permissionMode: 'workspace-write',
      }),
    })
    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}/approvals/{approvalId}', 'post', {
      session,
      pathParams: {
        sessionId: 'rt-1',
        approvalId: 'approval-1',
      },
      idempotencyKey: 'idem-approval-1',
      body: JSON.stringify({
        decision: 'approve',
      }),
    })
    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}/memory-proposals/{proposalId}', 'post', {
      session,
      pathParams: {
        sessionId: 'rt-1',
        proposalId: 'memory-proposal-1',
      },
      idempotencyKey: 'idem-memory-proposal-1',
      body: JSON.stringify({
        decision: 'approve',
        note: 'Approved durable memory candidate.',
      }),
    })
    await fetchWorkspaceOpenApi(connection, '/api/v1/runtime/sessions/{sessionId}/events', 'get', {
      session,
      pathParams: { sessionId: 'rt-1' },
      queryParams: { after: 'evt-1' },
    })

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/runtime/sessions',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/turns',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/approvals/approval-1',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/memory-proposals/memory-proposal-1',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      6,
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/events?after=evt-1',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    const turnHeaders = fetchSpy.mock.calls[2]?.[1]?.headers as Headers
    expect(turnHeaders.get('Idempotency-Key')).toBe('idem-turn-1')
    const approvalHeaders = fetchSpy.mock.calls[3]?.[1]?.headers as Headers
    expect(approvalHeaders.get('Idempotency-Key')).toBe('idem-approval-1')
    const memoryProposalHeaders = fetchSpy.mock.calls[4]?.[1]?.headers as Headers
    expect(memoryProposalHeaders.get('Idempotency-Key')).toBe('idem-memory-proposal-1')
    expect(runtimeDetailPayload.workflow?.workflowRunId).toBe('wf-run-1')
    expect(runtimeDetailPayload.pendingMailbox?.mailboxRef).toBe('mailbox-1')
    expect(runtimeDetailPayload.backgroundRun?.workflowRunId).toBe('wf-run-1')
    expect(runtimeDetailPayload.subruns[0]?.runKind).toBe('subrun')
    expect(runtimeDetailPayload.handoffs[0]?.mailboxRef).toBe('mailbox-1')
    expect(runtimeRunPayload.workflowRun).toBe('wf-run-1')
    expect(runtimeRunPayload.mailboxRef).toBe('mailbox-1')
    expect(runtimeRunPayload.workerDispatch?.totalSubruns).toBe(2)
  })

  it('extends generated workspace transport paths to apps, audit, inbox, and knowledge routes', async () => {
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<ClientAppRecord[]> => ([
          {
            id: 'octopus-web',
            name: 'Octopus Web',
            platform: 'web',
            status: 'active',
            firstParty: true,
            allowedOrigins: ['http://127.0.0.1'],
            allowedHosts: ['127.0.0.1'],
            sessionPolicy: 'session_token',
            defaultScopes: ['workspace'],
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<ClientAppRecord> => ({
          id: 'octopus-mobile',
          name: 'Octopus Mobile',
          platform: 'mobile',
          status: 'disabled',
          firstParty: true,
          allowedOrigins: [],
          allowedHosts: [],
          sessionPolicy: 'session_token',
          defaultScopes: ['workspace'],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<AccessAuditListResponse> => ({
          items: [
            {
              id: 'audit-1',
              workspaceId: 'ws-local',
              actorType: 'user',
              actorId: 'user-owner',
              action: 'runtime.session.create',
              resource: 'runtime-session',
              outcome: 'success',
              createdAt: 1,
            } satisfies AuditRecord,
          ],
          nextCursor: undefined,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<InboxItemRecord[]> => ([
          {
            id: 'inbox-1',
            workspaceId: 'ws-local',
            projectId: 'proj-redesign',
            itemType: 'approval',
            title: 'Need approval',
            description: 'Runtime needs approval.',
            status: 'pending',
            priority: 'high',
            actionable: true,
            routeTo: '/workspaces/ws-local/projects/proj-redesign/settings',
            actionLabel: 'Review approval',
            createdAt: 1,
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async (): Promise<KnowledgeEntryRecord[]> => ([
          {
            id: 'knowledge-1',
            workspaceId: 'ws-local',
            projectId: 'proj-redesign',
            title: 'Knowledge Entry',
            scope: 'project',
            status: 'active',
            sourceType: 'document',
            sourceRef: 'doc://knowledge-1',
            updatedAt: 1,
          },
        ]),
      })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)

    await fetchWorkspaceOpenApi(connection, '/api/v1/apps', 'get', { session })
    await fetchWorkspaceOpenApi(connection, '/api/v1/apps', 'post', {
      session,
      body: JSON.stringify({
        id: 'octopus-mobile',
        name: 'Octopus Mobile',
        platform: 'mobile',
        status: 'disabled',
        firstParty: true,
        allowedOrigins: [],
        allowedHosts: [],
        sessionPolicy: 'session_token',
        defaultScopes: ['workspace'],
      }),
    })
    await fetchWorkspaceOpenApi(connection, '/api/v1/access/audit', 'get', { session })
    await fetchWorkspaceOpenApi(connection, '/api/v1/inbox', 'get', { session })
    await fetchWorkspaceOpenApi(connection, '/api/v1/knowledge', 'get', { session })

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/apps',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/apps',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/access/audit',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/inbox',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/knowledge',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
  })

  it('uses generated runtime event paths for stream requests and resume headers', async () => {
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'text/event-stream' }),
      body: new ReadableStream(),
    })

    const connection = createWorkspaceConnection()
    const session = createWorkspaceSession(connection)

    await openWorkspaceOpenApiStream(
      connection,
      '/api/v1/runtime/sessions/{sessionId}/events',
      {
        session,
        pathParams: {
          sessionId: 'rt-1',
        },
        queryParams: {
          after: 'evt-1/next',
        },
        headers: {
          Accept: 'text/event-stream',
          'Last-Event-ID': 'evt-1/next',
        },
      },
    )

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/sessions/rt-1/events?after=evt-1%2Fnext',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = fetchSpy.mock.calls[0]?.[1] as RequestInit
    const headers = request.headers as Headers
    expect(headers.get('Accept')).toBe('text/event-stream')
    expect(headers.get('Last-Event-ID')).toBe('evt-1/next')
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
  })
})
