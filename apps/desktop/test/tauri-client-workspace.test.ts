// @vitest-environment jsdom

import { describe, expect, it } from 'vitest'

import type {
  ApiErrorEnvelope,
  BindPetConversationInput,
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

  it('calls workspace pet endpoints through the workspace client adapter', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())
    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
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
          projectId: 'proj-redesign',
          conversationId: 'conversation-1',
          sessionId: 'rt-conversation-1',
          updatedAt: 12,
        }),
      })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])
    const connection = payload.workspaceConnections?.[0]
    const session = createWorkspaceSession(connection!)
    const workspaceClient = client.createWorkspaceClient({ connection: connection!, session })

    await workspaceClient.pet.getSnapshot('proj-redesign')
    await workspaceClient.pet.savePresence({
      petId: 'pet-octopus',
      chatOpen: true,
      motionState: 'chat',
    } satisfies SavePetPresenceInput, 'proj-redesign')
    await workspaceClient.pet.bindConversation({
      petId: 'pet-octopus',
      conversationId: 'conversation-1',
      sessionId: 'rt-conversation-1',
    } satisfies BindPetConversationInput, 'proj-redesign')

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/pet',
      expect.objectContaining({ method: 'GET' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/pet/presence',
      expect.objectContaining({ method: 'PATCH' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/projects/proj-redesign/pet/conversation',
      expect.objectContaining({ method: 'PUT' }),
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
          routeTo: '/workspaces/ws-local/projects/proj-redesign/runtime',
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
    expect(records[0]?.routeTo).toBe('/workspaces/ws-local/projects/proj-redesign/runtime')
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

  it('lists workspace artifacts through the workspace API with the session token', async () => {
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

    const artifacts = await workspaceClient.artifacts.listWorkspace()

    expect(artifacts[0]?.title).toBe('Runtime Delivery Summary')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/artifacts',
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

    await workspaceClient.catalog.setToolDisabled({
      sourceKey: 'builtin:bash',
      disabled: true,
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/workspace/catalog/tool-catalog/disable',
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
