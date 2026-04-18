// @vitest-environment jsdom

import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import type {
  ApiErrorEnvelope,
  ArtifactVersionReference,
  BindPetConversationInput,
  CreateDeliverableVersionInput,
  DeliverableDetail,
  DeliverableSummary,
  DeliverableVersionContent,
  DeliverableVersionSummary,
  ForkDeliverableInput,
  PromoteDeliverableInput,
  RegisterBootstrapAdminRequest,
  SavePetPresenceInput,
} from '@octopus/schema'

import {
  createHostBootstrap,
  createWorkspaceSession,
  fetchSpy,
  firstRequest,
  installTauriClientTestHooks,
  invokeSpy,
  loadClientModule,
} from './tauri-client-test-helpers'

describe('workspace client transport', () => {
  installTauriClientTestHooks()

  it('re-exports canonical deliverable transport records from @octopus/schema', () => {
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
      artifactId: summary.id,
      version: summary.latestVersion,
      title: summary.title,
      previewKind: summary.previewKind,
      updatedAt: summary.updatedAt,
    }
    const versionContent: DeliverableVersionContent = {
      artifactId: summary.id,
      version: summary.latestVersion,
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
      title: 'Deliverable follow-up',
    }

    expect(detail.latestVersionRef.artifactId).toBe(versionSummary.artifactId)
    expect(versionContent.version).toBe(summary.latestVersion)
    expect(createVersionInput.previewKind).toBe('markdown')
    expect(promoteInput.summary).toContain('project knowledge')
    expect(forkInput.title).toContain('follow-up')
  })

  it('adds project task schema exports to the canonical @octopus/schema surfaces', () => {
    const repoRoot = resolve(import.meta.dirname, '../../..')
    const taskSchemaPath = resolve(repoRoot, 'packages/schema/src/task.ts')
    const indexSchemaPath = resolve(repoRoot, 'packages/schema/src/index.ts')

    expect(existsSync(taskSchemaPath)).toBe(true)

    const taskSchema = readFileSync(taskSchemaPath, 'utf8')
    const indexSchema = readFileSync(indexSchemaPath, 'utf8')

    expect(taskSchema).toContain('TaskSummary as OpenApiTaskSummary')
    expect(taskSchema).toContain('TaskDetail as OpenApiTaskDetail')
    expect(taskSchema).toContain('TaskRunSummary as OpenApiTaskRunSummary')
    expect(taskSchema).toContain('TaskContextBundle as OpenApiTaskContextBundle')
    expect(taskSchema).toContain('TaskFailureCategory as OpenApiTaskFailureCategory')
    expect(taskSchema).toContain('TaskAnalyticsSummary as OpenApiTaskAnalyticsSummary')
    expect(indexSchema).toContain("export * from './task'")
  })

  it('requires a workspace session token before workspace-plane calls can be made', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]

    expect(connection).toBeDefined()

    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
    })

    await expect(workspaceClient.workspace.get()).rejects.toThrow(/workspace session/i)
    expect(fetchSpy).not.toHaveBeenCalled()
  })

  it('uses the workspace HTTP protocol and workspace session token for authenticated read calls', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'ws-local',
        name: 'Local Workspace',
        slug: 'local-workspace',
        deployment: 'local',
        bootstrapStatus: 'ready',
        host: '127.0.0.1',
        listenAddress: 'http://127.0.0.1:43127',
        defaultProjectId: 'proj-redesign',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session,
    })

    const workspace = await workspaceClient.workspace.get()

    expect(workspace.name).toBe('Local Workspace')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer workspace-session-token')
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
    expect(headers.get('X-Request-Id')).toMatch(/^req-/)
  })

  it('calls resource import, detail, content, children, promote, and remote directory endpoints through the workspace client adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'res-folder-1',
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
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'res-folder-1',
          workspaceId: 'ws-local',
          projectId: 'proj-redesign',
          kind: 'folder',
          name: 'design-assets',
          origin: 'source',
          status: 'healthy',
          scope: 'project',
          visibility: 'public',
          ownerUserId: 'user-owner',
          storagePath: 'data/projects/proj-redesign/resources/design-assets',
          previewKind: 'folder',
          updatedAt: 1,
          tags: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          resourceId: 'res-folder-1',
          previewKind: 'folder',
          fileName: 'design-assets',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ([
          {
            name: 'brief.md',
            relativePath: 'brief.md',
            kind: 'file',
            previewKind: 'markdown',
            contentType: 'text/markdown',
            byteSize: 7,
            updatedAt: 1,
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'res-folder-1',
          workspaceId: 'ws-local',
          projectId: 'proj-redesign',
          kind: 'folder',
          name: 'design-assets',
          origin: 'source',
          status: 'healthy',
          scope: 'workspace',
          visibility: 'public',
          ownerUserId: 'user-owner',
          storagePath: 'data/projects/proj-redesign/resources/design-assets',
          previewKind: 'folder',
          updatedAt: 2,
          tags: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          currentPath: '/remote/projects',
          parentPath: '/remote',
          entries: [
            {
              name: 'design-assets',
              path: '/remote/projects/design-assets',
            },
          ],
        }),
      })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({ connection: connection!, session })

    await (workspaceClient.resources as any).importProject('proj-redesign', {
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
    })
    await (workspaceClient.resources as any).getDetail('res-folder-1')
    await (workspaceClient.resources as any).getContent('res-folder-1')
    await (workspaceClient.resources as any).listChildren('res-folder-1')
    await (workspaceClient.resources as any).promote('res-folder-1', { scope: 'workspace' })
    await (workspaceClient.filesystem as any).listDirectories('/remote/projects')

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/resources/import',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/resources/res-folder-1',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/resources/res-folder-1/content',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/resources/res-folder-1/children',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/resources/res-folder-1/promote',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      6,
      'http://127.0.0.1:43127/api/v1/workspace/filesystem/directories?path=%2Fremote%2Fprojects',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
  })

  it('calls project promotion request endpoints through the workspace client adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ([
          {
            id: 'promotion-1',
            workspaceId: 'ws-local',
            projectId: 'proj-redesign',
            assetType: 'resource',
            assetId: 'proj-redesign-res-3',
            requestedByUserId: 'user-owner',
            submittedByOwnerUserId: 'user-owner',
            requiredWorkspaceCapability: 'resource.publish',
            status: 'pending',
            createdAt: 1,
            updatedAt: 1,
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'promotion-2',
          workspaceId: 'ws-local',
          projectId: 'proj-redesign',
          assetType: 'resource',
          assetId: 'proj-redesign-res-3',
          requestedByUserId: 'user-owner',
          submittedByOwnerUserId: 'user-owner',
          requiredWorkspaceCapability: 'resource.publish',
          status: 'pending',
          createdAt: 2,
          updatedAt: 2,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ([
          {
            id: 'promotion-2',
            workspaceId: 'ws-local',
            projectId: 'proj-redesign',
            assetType: 'resource',
            assetId: 'proj-redesign-res-3',
            requestedByUserId: 'user-owner',
            submittedByOwnerUserId: 'user-owner',
            requiredWorkspaceCapability: 'resource.publish',
            status: 'pending',
            createdAt: 2,
            updatedAt: 2,
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'promotion-2',
          workspaceId: 'ws-local',
          projectId: 'proj-redesign',
          assetType: 'resource',
          assetId: 'proj-redesign-res-3',
          requestedByUserId: 'user-owner',
          submittedByOwnerUserId: 'user-owner',
          requiredWorkspaceCapability: 'resource.publish',
          status: 'approved',
          reviewedByUserId: 'user-owner',
          reviewComment: 'Looks good.',
          createdAt: 2,
          updatedAt: 3,
          reviewedAt: 3,
        }),
      })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({ connection: connection!, session })

    await (workspaceClient.projects as any).listPromotionRequests('proj-redesign')
    await (workspaceClient.projects as any).createPromotionRequest('proj-redesign', {
      assetType: 'resource',
      assetId: 'proj-redesign-res-3',
    })
    await (workspaceClient.workspace as any).listPromotionRequests()
    await (workspaceClient.workspace as any).reviewPromotionRequest('promotion-2', {
      approved: true,
      reviewComment: 'Looks good.',
    })

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/promotion-requests',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/promotion-requests',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/workspace/promotion-requests',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/workspace/promotion-requests/promotion-2/review',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
  })

  it('calls project task endpoints through the workspace client adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ([
          {
            id: 'task-1',
            projectId: 'proj-redesign',
            title: 'Prepare launch checklist',
            goal: 'Create a launch-ready checklist.',
            defaultActorRef: 'agent:workspace-core',
            status: 'running',
            scheduleSpec: '0 9 * * 1-5',
            nextRunAt: 20,
            lastRunAt: 10,
            latestResultSummary: null,
            latestFailureCategory: null,
            latestTransition: null,
            viewStatus: 'healthy',
            attentionReasons: [],
            attentionUpdatedAt: null,
            activeTaskRunId: 'task-run-1',
            analyticsSummary: {
              runCount: 1,
              manualRunCount: 1,
              scheduledRunCount: 0,
              completionCount: 0,
              failureCount: 0,
              takeoverCount: 0,
              approvalRequiredCount: 0,
              averageRunDurationMs: 0,
              lastSuccessfulRunAt: null,
            },
            updatedAt: 10,
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'task-1',
          projectId: 'proj-redesign',
          title: 'Prepare launch checklist',
          goal: 'Create a launch-ready checklist.',
          brief: 'Focus on dependencies and sequencing.',
          defaultActorRef: 'agent:workspace-core',
          status: 'ready',
          scheduleSpec: null,
          nextRunAt: null,
          lastRunAt: null,
          latestResultSummary: null,
          latestFailureCategory: null,
          latestTransition: null,
          viewStatus: 'configured',
          attentionReasons: [],
          attentionUpdatedAt: null,
          activeTaskRunId: null,
          analyticsSummary: {
            runCount: 0,
            manualRunCount: 0,
            scheduledRunCount: 0,
            completionCount: 0,
            failureCount: 0,
            takeoverCount: 0,
            approvalRequiredCount: 0,
            averageRunDurationMs: 0,
            lastSuccessfulRunAt: null,
          },
          contextBundle: {
            refs: [],
            pinnedInstructions: '',
            resolutionMode: 'explicit_only',
            lastResolvedAt: null,
          },
          latestDeliverableRefs: [],
          latestArtifactRefs: [],
          runHistory: [],
          interventionHistory: [],
          activeRun: null,
          createdBy: 'user-owner',
          updatedBy: null,
          createdAt: 10,
          updatedAt: 10,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'task-1',
          projectId: 'proj-redesign',
          title: 'Prepare launch checklist',
          goal: 'Create a launch-ready checklist.',
          brief: 'Focus on dependencies and sequencing.',
          defaultActorRef: 'agent:workspace-core',
          status: 'ready',
          scheduleSpec: null,
          nextRunAt: null,
          lastRunAt: null,
          latestResultSummary: null,
          latestFailureCategory: null,
          latestTransition: null,
          viewStatus: 'configured',
          attentionReasons: [],
          attentionUpdatedAt: null,
          activeTaskRunId: null,
          analyticsSummary: {
            runCount: 0,
            manualRunCount: 0,
            scheduledRunCount: 0,
            completionCount: 0,
            failureCount: 0,
            takeoverCount: 0,
            approvalRequiredCount: 0,
            averageRunDurationMs: 0,
            lastSuccessfulRunAt: null,
          },
          contextBundle: {
            refs: [],
            pinnedInstructions: '',
            resolutionMode: 'explicit_only',
            lastResolvedAt: null,
          },
          latestDeliverableRefs: [],
          latestArtifactRefs: [],
          runHistory: [],
          interventionHistory: [],
          activeRun: null,
          createdBy: 'user-owner',
          updatedBy: null,
          createdAt: 10,
          updatedAt: 10,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'task-1',
          projectId: 'proj-redesign',
          title: 'Prepare launch checklist',
          goal: 'Create a launch-ready checklist.',
          brief: 'Updated brief.',
          defaultActorRef: 'agent:workspace-core',
          status: 'ready',
          scheduleSpec: null,
          nextRunAt: null,
          lastRunAt: null,
          latestResultSummary: null,
          latestFailureCategory: null,
          latestTransition: null,
          viewStatus: 'configured',
          attentionReasons: [],
          attentionUpdatedAt: null,
          activeTaskRunId: null,
          analyticsSummary: {
            runCount: 0,
            manualRunCount: 0,
            scheduledRunCount: 0,
            completionCount: 0,
            failureCount: 0,
            takeoverCount: 0,
            approvalRequiredCount: 0,
            averageRunDurationMs: 0,
            lastSuccessfulRunAt: null,
          },
          contextBundle: {
            refs: [],
            pinnedInstructions: '',
            resolutionMode: 'explicit_only',
            lastResolvedAt: null,
          },
          latestDeliverableRefs: [],
          latestArtifactRefs: [],
          runHistory: [],
          interventionHistory: [],
          activeRun: null,
          createdBy: 'user-owner',
          updatedBy: 'user-owner',
          createdAt: 10,
          updatedAt: 11,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'task-run-1',
          taskId: 'task-1',
          triggerType: 'manual',
          status: 'running',
          sessionId: 'rt-1',
          conversationId: 'conv-1',
          runtimeRunId: 'run-1',
          actorRef: 'agent:workspace-core',
          startedAt: 12,
          completedAt: null,
          resultSummary: null,
          failureCategory: null,
          failureSummary: null,
          viewStatus: 'healthy',
          attentionReasons: [],
          attentionUpdatedAt: null,
          deliverableRefs: [],
          artifactRefs: [],
          latestTransition: null,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'task-run-2',
          taskId: 'task-1',
          triggerType: 'rerun',
          status: 'running',
          sessionId: 'rt-2',
          conversationId: 'conv-2',
          runtimeRunId: 'run-2',
          actorRef: 'agent:workspace-core',
          startedAt: 13,
          completedAt: null,
          resultSummary: null,
          failureCategory: null,
          failureSummary: null,
          viewStatus: 'healthy',
          attentionReasons: [],
          attentionUpdatedAt: null,
          deliverableRefs: [],
          artifactRefs: [],
          latestTransition: null,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ([
          {
            id: 'task-run-2',
            taskId: 'task-1',
            triggerType: 'rerun',
            status: 'running',
            sessionId: 'rt-2',
            conversationId: 'conv-2',
            runtimeRunId: 'run-2',
            actorRef: 'agent:workspace-core',
            startedAt: 13,
            completedAt: null,
            resultSummary: null,
            failureCategory: null,
            failureSummary: null,
            viewStatus: 'healthy',
            attentionReasons: [],
            attentionUpdatedAt: null,
            deliverableRefs: [],
            artifactRefs: [],
            latestTransition: null,
          },
        ]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'intervention-1',
          taskId: 'task-1',
          taskRunId: 'task-run-2',
          type: 'comment',
          payload: {
            note: 'Please keep the checklist concise.',
          },
          createdBy: 'user-owner',
          createdAt: 14,
          appliedToSessionId: null,
          status: 'accepted',
        }),
      })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({ connection: connection!, session })

    await workspaceClient.tasks.listProject('proj-redesign')
    await workspaceClient.tasks.createProject('proj-redesign', {
      title: 'Prepare launch checklist',
      goal: 'Create a launch-ready checklist.',
      brief: 'Focus on dependencies and sequencing.',
      defaultActorRef: 'agent:workspace-core',
      scheduleSpec: null,
      contextBundle: {
        refs: [],
        pinnedInstructions: '',
        resolutionMode: 'explicit_only',
        lastResolvedAt: null,
      },
    })
    await workspaceClient.tasks.getDetail('proj-redesign', 'task-1')
    await workspaceClient.tasks.updateProject('proj-redesign', 'task-1', {
      brief: 'Updated brief.',
    })
    await workspaceClient.tasks.launch('proj-redesign', 'task-1', {
      actorRef: 'agent:workspace-core',
    })
    await workspaceClient.tasks.rerun('proj-redesign', 'task-1', {
      actorRef: 'agent:workspace-core',
      sourceTaskRunId: 'task-run-1',
    })
    await workspaceClient.tasks.listRuns('proj-redesign', 'task-1')
    await workspaceClient.tasks.createIntervention('proj-redesign', 'task-1', {
      approvalId: 'approval-task-run-2',
      taskRunId: 'task-run-2',
      type: 'comment',
      payload: {
        note: 'Please keep the checklist concise.',
      },
    })

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/tasks',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/tasks',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/tasks/task-1',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/tasks/task-1',
      expect.objectContaining({ method: 'PATCH', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/tasks/task-1/launch',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      6,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/tasks/task-1/rerun',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      7,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/tasks/task-1/runs',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      8,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/tasks/task-1/interventions',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
  })

  it('calls personal pet home, project-context, and dashboard endpoints through the workspace client adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          workspaceId: 'ws-local',
          ownerUserId: 'user-owner',
          contextScope: 'home',
          profile: {
            id: 'pet-octopus',
            displayName: '小章',
            species: 'octopus',
            ownerUserId: 'user-owner',
            avatarLabel: 'Octopus mascot',
            summary: 'Octopus 首席吉祥物，负责卖萌和加油。',
            greeting: '嗨！我是小章，今天也要加油哦！',
            mood: 'happy',
            favoriteSnack: '新鲜小虾',
            promptHints: ['最近有什么好消息？'],
            fallbackAsset: 'octopus',
          },
          presence: {
            petId: 'pet-octopus',
            isVisible: true,
            chatOpen: false,
            motionState: 'idle',
            unreadCount: 0,
            lastInteractionAt: 0,
            position: { x: 0, y: 0 },
          },
          messages: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          petId: 'pet-octopus',
          workspaceId: 'ws-local',
          ownerUserId: 'user-owner',
          species: 'octopus',
          mood: 'happy',
          memoryCount: 4,
          knowledgeCount: 7,
          resourceCount: 3,
          reminderCount: 2,
          activeConversationCount: 1,
          lastInteractionAt: 12,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          petId: 'pet-octopus',
          isVisible: true,
          chatOpen: true,
          motionState: 'chat',
          unreadCount: 0,
          lastInteractionAt: 12,
          position: { x: 0, y: 0 },
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          petId: 'pet-octopus',
          workspaceId: 'ws-local',
          conversationId: 'conversation-1',
          sessionId: 'rt-conversation-1',
          ownerUserId: 'user-owner',
          contextScope: 'home',
          updatedAt: 12,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          workspaceId: 'ws-local',
          ownerUserId: 'user-owner',
          contextScope: 'project',
          projectId: 'proj-redesign',
          profile: {
            id: 'pet-octopus',
            displayName: '小章',
            species: 'octopus',
            ownerUserId: 'user-owner',
            avatarLabel: 'Octopus mascot',
            summary: 'Octopus 首席吉祥物，负责卖萌和加油。',
            greeting: '嗨！我是小章，今天也要加油哦！',
            mood: 'focused',
            favoriteSnack: '新鲜小虾',
            promptHints: ['最近有什么好消息？'],
            fallbackAsset: 'octopus',
          },
          presence: {
            petId: 'pet-octopus',
            isVisible: true,
            chatOpen: true,
            motionState: 'chat',
            unreadCount: 1,
            lastInteractionAt: 14,
            position: { x: 10, y: 18 },
          },
          messages: [],
          binding: {
            petId: 'pet-octopus',
            workspaceId: 'ws-local',
            projectId: 'proj-redesign',
            conversationId: 'conversation-2',
            sessionId: 'rt-conversation-2',
            ownerUserId: 'user-owner',
            contextScope: 'project',
            updatedAt: 14,
          },
        }),
      })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({ connection: connection!, session })

    const homeSnapshot = await workspaceClient.pet.getSnapshot()
    const dashboard = await workspaceClient.pet.getDashboard()
    const homePresence = await workspaceClient.pet.savePresence({
      petId: 'pet-octopus',
      chatOpen: true,
      motionState: 'chat',
    } satisfies SavePetPresenceInput)
    const homeBinding = await workspaceClient.pet.bindConversation({
      petId: 'pet-octopus',
      conversationId: 'conversation-1',
      sessionId: 'rt-conversation-1',
    } satisfies BindPetConversationInput)
    const projectSnapshot = await workspaceClient.pet.getSnapshot('proj-redesign')

    expect(homeSnapshot.contextScope).toBe('home')
    expect(homeSnapshot.ownerUserId).toBe('user-owner')
    expect(dashboard.knowledgeCount).toBe(7)
    expect(homePresence.motionState).toBe('chat')
    expect(homeBinding.contextScope).toBe('home')
    expect(homeBinding.projectId).toBeUndefined()
    expect(projectSnapshot.contextScope).toBe('project')
    expect(projectSnapshot.projectId).toBe('proj-redesign')
    expect(projectSnapshot.binding?.contextScope).toBe('project')

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/workspace/pet',
      expect.objectContaining({ method: 'GET' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/workspace/pet/dashboard',
      expect.objectContaining({ method: 'GET' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/workspace/pet/presence',
      expect.objectContaining({ method: 'PATCH' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/workspace/pet/conversation',
      expect.objectContaining({ method: 'PUT' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/pet',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('submits a pet home runtime session without requiring projectId in the transport body', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        summary: {
          id: 'rt-pet-home',
          conversationId: 'conversation-home',
          projectId: '',
          title: 'Pet Home',
          sessionKind: 'pet',
          status: 'draft',
          updatedAt: 10,
          lastMessagePreview: null,
          configSnapshotId: 'config-home',
          effectiveConfigHash: 'hash-home',
          startedFromScopeSet: ['user', 'workspace'],
          selectedActorRef: 'agent:agent-architect',
          manifestRevision: 'asset-manifest-v2',
          sessionPolicy: {
            selectedActorRef: 'agent:agent-architect',
            selectedConfiguredModelId: '',
            executionPermissionMode: 'readonly',
            configSnapshotId: 'config-home',
            manifestRevision: 'asset-manifest-v2',
            capabilityPolicy: {},
            memoryPolicy: {},
            delegationPolicy: {},
            approvalPreference: {},
          },
          activeRunId: 'run-home',
          subrunCount: 0,
          memorySummary: { totalCount: 0, byScope: {}, writableScopes: [] },
          memorySelectionSummary: {
            selectedCount: 0,
            sharedCount: 0,
            privateCount: 0,
            staleCount: 0,
          },
          pendingMemoryProposalCount: 0,
          memoryStateRef: 'memory-home',
          capabilityPlanSummary: {
            visibleTools: [],
            deferredTools: [],
            discoverableSkills: [],
            availableResources: [],
            hiddenCapabilities: [],
            activatedTools: [],
            grantedTools: [],
            pendingTools: [],
            approvedTools: [],
            authResolvedTools: [],
            providerFallbacks: [],
          },
          providerStateSummary: [],
          authStateSummary: {
            pendingChallenges: 0,
            challengedTargets: [],
            resolvedTargets: [],
          },
          policyDecisionSummary: {
            allowedCount: 0,
            deniedCount: 0,
            deferredCount: 0,
            hiddenCount: 0,
          },
        },
        selectedActorRef: 'agent:agent-architect',
        manifestRevision: 'asset-manifest-v2',
        sessionPolicy: {
          selectedActorRef: 'agent:agent-architect',
          selectedConfiguredModelId: '',
          executionPermissionMode: 'readonly',
          configSnapshotId: 'config-home',
          manifestRevision: 'asset-manifest-v2',
          capabilityPolicy: {},
          memoryPolicy: {},
          delegationPolicy: {},
          approvalPreference: {},
        },
        activeRunId: 'run-home',
        subrunCount: 0,
        memorySummary: { totalCount: 0, byScope: {}, writableScopes: [] },
        memorySelectionSummary: {
          selectedCount: 0,
          sharedCount: 0,
          privateCount: 0,
          staleCount: 0,
        },
        pendingMemoryProposalCount: 0,
        memoryStateRef: 'memory-home',
        capabilityPlanSummary: {
          visibleTools: [],
          deferredTools: [],
          discoverableSkills: [],
          availableResources: [],
          hiddenCapabilities: [],
          activatedTools: [],
          grantedTools: [],
          pendingTools: [],
          approvedTools: [],
          authResolvedTools: [],
          providerFallbacks: [],
        },
        providerStateSummary: [],
        authStateSummary: {
          pendingChallenges: 0,
          challengedTargets: [],
          resolvedTargets: [],
        },
        policyDecisionSummary: {
          allowedCount: 0,
          deniedCount: 0,
          deferredCount: 0,
          hiddenCount: 0,
        },
        run: {
          id: 'run-home',
          sessionId: 'rt-pet-home',
          conversationId: 'conversation-home',
          status: 'draft',
          currentStep: 'ready',
          startedAt: 10,
          updatedAt: 10,
          selectedMemory: [],
          configSnapshotId: 'config-home',
          effectiveConfigHash: 'hash-home',
          startedFromScopeSet: ['user', 'workspace'],
          runKind: 'primary',
          parentRunId: null,
          actorRef: 'agent:agent-architect',
          delegatedByToolCallId: null,
          approvalState: 'not-required',
          approvalTarget: null,
          authTarget: null,
          usageSummary: { promptTokens: 0, completionTokens: 0, totalTokens: 0 },
          artifactRefs: [],
          traceContext: { sessionId: 'rt-pet-home', branchKey: 'main' },
          checkpoint: {
            currentIterationIndex: 0,
            usageSummary: { promptTokens: 0, completionTokens: 0, totalTokens: 0 },
            capabilityPlanSummary: {
              visibleTools: [],
              deferredTools: [],
              discoverableSkills: [],
              availableResources: [],
              hiddenCapabilities: [],
              activatedTools: [],
              grantedTools: [],
              pendingTools: [],
              approvedTools: [],
              authResolvedTools: [],
              providerFallbacks: [],
            },
          },
          capabilityPlanSummary: {
            visibleTools: [],
            deferredTools: [],
            discoverableSkills: [],
            availableResources: [],
            hiddenCapabilities: [],
            activatedTools: [],
            grantedTools: [],
            pendingTools: [],
            approvedTools: [],
            authResolvedTools: [],
            providerFallbacks: [],
          },
          providerStateSummary: [],
          pendingMediation: null,
        },
        subruns: [],
        handoffs: [],
        messages: [],
        trace: [],
        pendingApproval: null,
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({ connection: connection!, session })

    await workspaceClient.runtime.createSession({
      conversationId: 'conversation-home',
      title: 'Pet Home',
      sessionKind: 'pet',
      selectedActorRef: 'agent:agent-architect',
      executionPermissionMode: 'readonly',
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/runtime/sessions',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          conversationId: 'conversation-home',
          title: 'Pet Home',
          sessionKind: 'pet',
          selectedActorRef: 'agent:agent-architect',
          executionPermissionMode: 'readonly',
        }),
      }),
    )
  })

  it('calls the workspace inbox endpoint through the workspace client adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ([
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

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({ connection: connection!, session })

    const records = await workspaceClient.inbox.list()

    expect(records[0]?.actionable).toBe(true)
    expect(records[0]?.routeTo).toBe('/workspaces/ws-local/projects/proj-redesign/settings')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/inbox',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )
  })

  it('submits first-owner registration through the public auth endpoint without an existing session', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        session: {
          id: 'sess-owner',
          workspaceId: 'ws-local',
          userId: 'user-owner',
          clientAppId: 'octopus-desktop',
          token: 'token-owner',
          status: 'active',
          createdAt: 1,
        },
        workspace: {
          id: 'ws-local',
          name: 'Local Workspace',
          slug: 'local-workspace',
          deployment: 'local',
          bootstrapStatus: 'ready',
          ownerUserId: 'user-owner',
          host: '127.0.0.1',
          listenAddress: 'http://127.0.0.1:43127',
          defaultProjectId: 'proj-redesign',
        },
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
    })

    const requestBody: RegisterBootstrapAdminRequest = {
      clientAppId: 'octopus-desktop',
      username: 'owner',
      displayName: 'Workspace Owner',
      password: 'owner-owner',
      confirmPassword: 'owner-owner',
      workspaceId: 'ws-local',
      avatar: {
        fileName: 'owner-avatar.png',
        contentType: 'image/png',
        dataBase64: 'iVBORw0KGgo=',
        byteSize: 8,
      },
    }

    const response = await workspaceClient.enterpriseAuth.bootstrapAdmin(requestBody)

    expect(response.session.userId).toBe('user-owner')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/system/auth/bootstrap-admin',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBeNull()
    expect(headers.get('X-Workspace-Id')).toBe('ws-local')
  })

  it('throws a typed API error for workspace auth failures', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: false,
      status: 401,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async (): Promise<ApiErrorEnvelope> => ({
        error: {
          code: 'SESSION_EXPIRED',
          message: 'session expired',
          details: null,
          requestId: 'req-auth-1',
          retryable: false,
        },
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session,
    })

    await expect(workspaceClient.workspace.get()).rejects.toMatchObject({
      code: 'SESSION_EXPIRED',
      status: 401,
      requestId: 'req-auth-1',
      retryable: false,
    })
  })

  it('lists workspace deliverables through the workspace API with the session token', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ([
        {
          id: 'artifact-1',
          workspaceId: 'ws-local',
          projectId: 'proj-redesign',
          title: 'Runtime Delivery Summary',
          status: 'review',
          latestVersion: 2,
          updatedAt: 10,
        },
      ]),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const deliverables = await workspaceClient.deliverables.listWorkspace()

    expect((workspaceClient as Record<string, unknown>).artifacts).toBeUndefined()
    expect(deliverables[0]?.title).toBe('Runtime Delivery Summary')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/deliverables',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
    const request = firstRequest()
    expect((request.headers as Headers).get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('lists project deliverables through the workspace adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ([
        {
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
          latestVersionRef: {
            artifactId: 'artifact-1',
            version: 2,
            title: 'Runtime Delivery Summary',
            previewKind: 'markdown',
            updatedAt: 10,
          },
        },
      ]),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const deliverables = await (workspaceClient.projects as any).listDeliverables('proj-redesign')

    expect(deliverables[0]?.id).toBe('artifact-1')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/deliverables',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
    const request = firstRequest()
    expect((request.headers as Headers).get('Authorization')).toBe('Bearer workspace-session-token')
  })

  it('uses authenticated project create endpoint for workspace project management', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'proj-new',
        workspaceId: 'ws-local',
        name: 'New Project',
        status: 'active',
        description: 'Created from the workspace surface.',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.projects.create({
      name: 'New Project',
      description: 'Created from the workspace surface.',
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )
  })

  it('uses authenticated project update endpoint for archive/restore actions', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'proj-redesign',
        workspaceId: 'ws-local',
        name: 'Desktop Redesign',
        status: 'archived',
        description: 'Archived project.',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.projects.update('proj-redesign', {
      name: 'Desktop Redesign',
      description: 'Archived project.',
      status: 'archived',
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )
  })

  it('fetches the formal capability management projection through the workspace catalog adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        entries: [],
        assets: [],
        skillPackages: [],
        mcpServerPackages: [],
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const projection = await workspaceClient.catalog.getManagementProjection()

    expect(projection.entries).toEqual([])
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/catalog/management-projection',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )
  })

  it('updates the current user profile through the workspace personal center profile endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'user-owner',
        username: 'owner',
        displayName: 'Workspace Owner',
        avatar: 'data:image/png;base64,b3duZXI=',
        status: 'active',
        passwordState: 'set',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    const avatar: AvatarUploadPayload = {
      fileName: 'owner-avatar.png',
      contentType: 'image/png',
      dataBase64: 'b3duZXI=',
      byteSize: 5,
    }

    await workspaceClient.profile.updateCurrentUserProfile({
      displayName: 'Workspace Owner',
      avatar,
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/personal-center/profile',
      expect.objectContaining({
        method: 'PATCH',
        headers: expect.any(Headers),
      }),
    )
  })

  it('changes the current user password through the workspace personal center profile password endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        success: true,
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.profile.changeCurrentUserPassword({
      currentPassword: 'owner-owner',
      newPassword: 'owner-owner-2',
      confirmPassword: 'owner-owner-2',
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/workspace/personal-center/profile/password',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    expect(request.body).toBe(JSON.stringify({
      currentPassword: 'owner-owner',
      newPassword: 'owner-owner-2',
      confirmPassword: 'owner-owner-2',
    }))
  })

  it('creates access-control users through the enterprise users endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'access-user-alpha',
        username: 'member-alpha',
        displayName: 'Member Alpha',
        status: 'active',
        passwordState: 'reset-required',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.accessControl.createUser({
      username: 'member-alpha',
      displayName: 'Member Alpha',
      status: 'active',
      password: 'member-alpha-temp',
      confirmPassword: 'member-alpha-temp',
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/access/users',
      expect.objectContaining({
        method: 'POST',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    expect(request.body).toBe(JSON.stringify({
      username: 'member-alpha',
      displayName: 'Member Alpha',
      status: 'active',
      password: 'member-alpha-temp',
      confirmPassword: 'member-alpha-temp',
    }))
  })

  it('updates access-control users through the enterprise user detail endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        id: 'access-user-beta',
        username: 'member-beta',
        displayName: 'Member Beta',
        status: 'active',
        passwordState: 'set',
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.accessControl.updateUser('access-user-beta', {
      username: 'member-beta',
      displayName: 'Member Beta',
      status: 'active',
      resetPassword: true,
    })

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/access/users/access-user-beta',
      expect.objectContaining({
        method: 'PUT',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    expect(request.body).toBe(JSON.stringify({
      username: 'member-beta',
      displayName: 'Member Beta',
      status: 'active',
      resetPassword: true,
    }))
  })

  it('deletes access-control users through the enterprise user detail endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      status: 204,
      headers: new Headers(),
      text: async () => '',
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.accessControl.deleteUser('access-user-beta')

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/access/users/access-user-beta',
      expect.objectContaining({
        method: 'DELETE',
        headers: expect.any(Headers),
      }),
    )
  })

  it('deletes access-control roles through the enterprise role detail endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      status: 204,
      headers: new Headers(),
      text: async () => '',
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.accessControl.deleteRole('role-operator')

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/access/roles/role-operator',
      expect.objectContaining({
        method: 'DELETE',
        headers: expect.any(Headers),
      }),
    )
  })

  it('deletes enterprise data policies through the access-control policy endpoint', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy.mockResolvedValue({
      ok: true,
      status: 204,
      headers: new Headers(),
      text: async () => '',
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session: createWorkspaceSession(connection!),
    })

    await workspaceClient.accessControl.deleteDataPolicy('policy-project-redesign')

    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/access/policies/data-policies/policy-project-redesign',
      expect.objectContaining({
        method: 'DELETE',
        headers: expect.any(Headers),
      }),
    )
  })

  it('calls the workspace tool management routes through the catalog adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({ entries: [] }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({ entries: [] }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'skill-workspace-ops-helper',
          sourceKey: 'skill:data/skills/ops-helper/SKILL.md',
          name: 'ops-helper',
          description: 'Helpful local skill.',
          content: '---\nname: ops-helper\n---\n',
          displayPath: 'data/skills/ops-helper/SKILL.md',
          rootPath: 'data/skills/ops-helper',
          tree: [],
          relativePath: 'data/skills/ops-helper/SKILL.md',
          workspaceOwned: true,
          sourceOrigin: 'skills_dir',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          serverName: 'ops',
          sourceKey: 'mcp:ops',
          displayPath: 'config/runtime/workspace.json',
          scope: 'workspace',
          config: {
            type: 'http',
            url: 'https://ops.example.test/mcp',
          },
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          skillId: 'skill-workspace-ops-helper',
          sourceKey: 'skill:data/skills/ops-helper/SKILL.md',
          displayPath: 'data/skills/ops-helper',
          rootPath: 'data/skills/ops-helper',
          tree: [],
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          skillId: 'skill-workspace-ops-helper',
          sourceKey: 'skill:data/skills/ops-helper/SKILL.md',
          path: 'notes/overview.md',
          displayPath: 'data/skills/ops-helper/notes/overview.md',
          byteSize: 12,
          isText: true,
          content: '# Overview',
          contentType: 'text/markdown',
          language: 'markdown',
          readonly: false,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          skillId: 'skill-workspace-ops-helper',
          sourceKey: 'skill:data/skills/ops-helper/SKILL.md',
          path: 'notes/overview.md',
          displayPath: 'data/skills/ops-helper/notes/overview.md',
          byteSize: 14,
          isText: true,
          content: '# Updated',
          contentType: 'text/markdown',
          language: 'markdown',
          readonly: false,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'skill-workspace-imported',
          sourceKey: 'skill:data/skills/imported/SKILL.md',
          name: 'imported',
          description: 'Imported skill.',
          content: '---\nname: imported\n---\n',
          displayPath: 'data/skills/imported/SKILL.md',
          rootPath: 'data/skills/imported',
          tree: [],
          relativePath: 'data/skills/imported/SKILL.md',
          workspaceOwned: true,
          sourceOrigin: 'skills_dir',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'skill-workspace-foldered',
          sourceKey: 'skill:data/skills/foldered/SKILL.md',
          name: 'foldered',
          description: 'Folder import.',
          content: '---\nname: foldered\n---\n',
          displayPath: 'data/skills/foldered/SKILL.md',
          rootPath: 'data/skills/foldered',
          tree: [],
          relativePath: 'data/skills/foldered/SKILL.md',
          workspaceOwned: true,
          sourceOrigin: 'skills_dir',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          id: 'skill-workspace-copied',
          sourceKey: 'skill:data/skills/copied/SKILL.md',
          name: 'copied',
          description: 'Copied skill.',
          content: '---\nname: copied\n---\n',
          displayPath: 'data/skills/copied/SKILL.md',
          rootPath: 'data/skills/copied',
          tree: [],
          relativePath: 'data/skills/copied/SKILL.md',
          workspaceOwned: true,
          sourceOrigin: 'skills_dir',
        }),
      })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({
      connection: connection!,
      session,
    })

    await workspaceClient.catalog.setAssetDisabled({
      sourceKey: 'builtin:bash',
      disabled: true,
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/management-projection/disable',
      expect.objectContaining({ method: 'PATCH', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.createSkill({
      slug: 'ops-helper',
      content: '---\nname: ops-helper\n---\n',
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.getSkill('skill-workspace-ops-helper')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-workspace-ops-helper',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.getMcpServer('ops')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/mcp-servers/ops',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.getSkillTree('skill-workspace-ops-helper')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      5,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-workspace-ops-helper/tree',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.getSkillFile('skill-workspace-ops-helper', 'notes/overview.md')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      6,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-workspace-ops-helper/files/notes%2Foverview.md',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.updateSkillFile('skill-workspace-ops-helper', 'notes/overview.md', {
      content: '# Updated',
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      7,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-workspace-ops-helper/files/notes%2Foverview.md',
      expect.objectContaining({ method: 'PATCH', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.importSkillArchive({
      slug: 'imported',
      archive: {
        fileName: 'imported.zip',
        contentType: 'application/zip',
        dataBase64: 'UEsDBA==',
        byteSize: 8,
      },
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      8,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/import-archive',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.importSkillFolder({
      slug: 'foldered',
      files: [{
        relativePath: 'foldered/SKILL.md',
        fileName: 'SKILL.md',
        contentType: 'text/markdown',
        dataBase64: 'IyBza2lsbA==',
        byteSize: 8,
      }],
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      9,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/import-folder',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )

    await workspaceClient.catalog.copySkillToManaged('skill-external-help', {
      slug: 'copied',
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      10,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/skills/skill-external-help/copy-to-managed',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )
  })
})
