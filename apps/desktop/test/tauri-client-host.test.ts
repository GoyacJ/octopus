// @vitest-environment jsdom

import { describe, expect, it, vi } from 'vitest'

import {
  createHostBootstrap,
  createHostUpdateStatus,
  createNotificationRecord,
  fetchSpy,
  firstRequest,
  installTauriClientTestHooks,
  invokeSpy,
  loadClientModule,
} from './tauri-client-test-helpers'

describe('host client transport', () => {
  installTauriClientTestHooks()

  it('exposes host backend metadata without turning it into the business transport', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign', [])

    expect(invokeSpy).toHaveBeenCalledWith('bootstrap_shell')
    expect(fetchSpy).not.toHaveBeenCalled()
    expect(payload.hostState.platform).toBe('tauri')
    expect(payload.backend?.transport).toBe('http')
    expect(payload.workspaceConnections?.[0]).toMatchObject({
      workspaceConnectionId: 'conn-local',
      workspaceId: 'ws-local',
      transportSecurity: 'loopback',
      authMode: 'session-token',
      status: 'connected',
    })
  })

  it('does not expose the removed legacy runtime helper exports', async () => {
    invokeSpy.mockResolvedValue(createHostBootstrap())

    const client = await loadClientModule()

    expect('bootstrapRuntime' in client).toBe(false)
    expect('createRuntimeSession' in client).toBe(false)
    expect('loadRuntimeSession' in client).toBe(false)
    expect('listRuntimeSessions' in client).toBe(false)
    expect('pollRuntimeEvents' in client).toBe(false)
    expect('resolveRuntimeApproval' in client).toBe(false)
    expect('submitRuntimeUserTurn' in client).toBe(false)
  })

  it('creates notifications through the Tauri shell bridge', async () => {
    const notification = createNotificationRecord({
      id: 'notif-created',
      scopeKind: 'workspace',
    })
    invokeSpy.mockResolvedValue(notification)

    const client = await loadClientModule()
    const result = await client.createNotification({
      scopeKind: 'workspace',
      scopeOwnerId: 'ws-local',
      level: 'success',
      title: 'Workspace synced',
      body: 'The workspace status is up to date.',
      source: 'workspace-store',
      toastDurationMs: 30_000,
    })

    expect(invokeSpy).toHaveBeenCalledWith('create_notification', {
      input: {
        scopeKind: 'workspace',
        scopeOwnerId: 'ws-local',
        level: 'success',
        title: 'Workspace synced',
        body: 'The workspace status is up to date.',
        source: 'workspace-store',
        toastDurationMs: 30_000,
      },
    })
    expect(result.id).toBe('notif-created')
  })

  it('fans out notification events to local subscribers after successful creation', async () => {
    const notification = createNotificationRecord({
      id: 'notif-fanout',
      toastVisibleUntil: 30_000,
    })
    invokeSpy.mockResolvedValue(notification)

    const client = await loadClientModule()
    const received = [] as typeof notification[]
    const unsubscribe = client.subscribeToNotifications((event) => {
      received.push(event)
    })

    await client.createNotification({
      scopeKind: 'app',
      level: 'info',
      title: 'Heads up',
      body: 'New notification.',
      source: 'test-suite',
    })

    expect(received).toEqual([notification])

    unsubscribe()

    await client.createNotification({
      scopeKind: 'app',
      level: 'info',
      title: 'Second',
      body: 'Should not be received.',
      source: 'test-suite',
    })

    expect(received).toEqual([notification])
  })

  it('uses browser host HTTP endpoints when VITE_HOST_RUNTIME=browser', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => createHostBootstrap({
        hostState: {
          platform: 'web',
          mode: 'local',
          appVersion: '0.1.0-test',
          cargoWorkspace: true,
          shell: 'browser',
        },
      }),
    })

    const client = await loadClientModule()
    const payload = await client.bootstrapShellHost('ws-local', 'proj-redesign')

    expect(invokeSpy).not.toHaveBeenCalled()
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/host/bootstrap',
      expect.objectContaining({
        method: 'GET',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer browser-host-token')
    expect(payload.hostState.platform).toBe('web')
    expect(payload.workspaceConnections?.[0]?.workspaceConnectionId).toBe('conn-local')
  })

  it('persists browser host preferences through the host HTTP API instead of local storage fallback', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    fetchSpy.mockResolvedValue({
      ok: true,
      headers: new Headers({ 'Content-Type': 'application/json' }),
      json: async () => ({
        theme: 'dark',
        locale: 'en-US',
        compactSidebar: true,
        leftSidebarCollapsed: true,
        rightSidebarCollapsed: false,
        fontSize: 15,
        fontFamily: 'Inter, sans-serif',
        fontStyle: 'sans',
        defaultWorkspaceId: 'ws-local',
        lastVisitedRoute: '/workspaces/ws-local/overview?project=proj-redesign',
      }),
    })

    const client = await loadClientModule()
    const preferences = await client.savePreferences({
      theme: 'dark',
      locale: 'en-US',
      compactSidebar: true,
      leftSidebarCollapsed: true,
      rightSidebarCollapsed: false,
      fontSize: 15,
      fontFamily: 'Inter, sans-serif',
      fontStyle: 'sans',
      defaultWorkspaceId: 'ws-local',
      lastVisitedRoute: '/workspaces/ws-local/overview?project=proj-redesign',
    })

    expect(preferences.theme).toBe('dark')
    expect(fetchSpy).toHaveBeenCalledWith(
      'http://127.0.0.1:43127/api/v1/host/preferences',
      expect.objectContaining({
        method: 'PUT',
        headers: expect.any(Headers),
      }),
    )

    const request = firstRequest()
    const headers = request.headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer browser-host-token')
    expect(headers.get('Content-Type')).toBe('application/json')
  })

  it('bridges host update actions through Tauri commands', async () => {
    invokeSpy
      .mockResolvedValueOnce(createHostUpdateStatus())
      .mockResolvedValueOnce(createHostUpdateStatus({
        state: 'up_to_date',
        lastCheckedAt: 1_710_000_000_000,
      }))
      .mockResolvedValueOnce(createHostUpdateStatus({
        state: 'downloading',
        progress: {
          downloadedBytes: 512,
          totalBytes: 1024,
          percent: 50,
        },
      }))
      .mockResolvedValueOnce(createHostUpdateStatus({
        state: 'installing',
      }))

    const client = await loadClientModule()
    const updateStatus = await (client as typeof client & {
      getHostUpdateStatus: () => Promise<unknown>
      checkHostUpdate: (channel: string) => Promise<unknown>
      downloadHostUpdate: () => Promise<unknown>
      installHostUpdate: () => Promise<unknown>
    }).getHostUpdateStatus()

    expect(updateStatus).toMatchObject({
      currentVersion: '0.2.0',
      currentChannel: 'formal',
      state: 'idle',
    })
    expect(invokeSpy).toHaveBeenCalledWith('get_host_update_status')

    await (client as typeof client & {
      checkHostUpdate: (channel: string) => Promise<unknown>
    }).checkHostUpdate('preview')
    expect(invokeSpy).toHaveBeenCalledWith('check_host_update', {
      channel: 'preview',
    })

    await (client as typeof client & {
      downloadHostUpdate: () => Promise<unknown>
    }).downloadHostUpdate()
    expect(invokeSpy).toHaveBeenCalledWith('download_host_update')

    await (client as typeof client & {
      installHostUpdate: () => Promise<unknown>
    }).installHostUpdate()
    expect(invokeSpy).toHaveBeenCalledWith('install_host_update')
  })

  it('uses browser host update HTTP endpoints when VITE_HOST_RUNTIME=browser', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => createHostUpdateStatus(),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => createHostUpdateStatus({
          currentChannel: 'preview',
          state: 'update_available',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => createHostUpdateStatus({
          state: 'downloaded',
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => createHostUpdateStatus({
          state: 'installing',
        }),
      })

    const client = await loadClientModule()
    await (client as typeof client & {
      getHostUpdateStatus: () => Promise<unknown>
      checkHostUpdate: (channel: string) => Promise<unknown>
      downloadHostUpdate: () => Promise<unknown>
      installHostUpdate: () => Promise<unknown>
    }).getHostUpdateStatus()
    await (client as typeof client & {
      checkHostUpdate: (channel: string) => Promise<unknown>
    }).checkHostUpdate('preview')
    await (client as typeof client & {
      downloadHostUpdate: () => Promise<unknown>
    }).downloadHostUpdate()
    await (client as typeof client & {
      installHostUpdate: () => Promise<unknown>
    }).installHostUpdate()

    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/host/update-status',
      expect.objectContaining({ method: 'GET' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/host/update-check',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/host/update-download',
      expect.objectContaining({ method: 'POST' }),
    )
    expect(fetchSpy).toHaveBeenNthCalledWith(
      4,
      'http://127.0.0.1:43127/api/v1/host/update-install',
      expect.objectContaining({ method: 'POST' }),
    )
  })

  it('creates, lists, and deletes host workspace connections through Tauri commands', async () => {
    invokeSpy.mockImplementation(async (command: string) => {
      if (command === 'list_workspace_connections') {
        return [{
          workspaceConnectionId: 'conn-enterprise',
          workspaceId: 'ws-enterprise',
          label: 'Enterprise Workspace',
          baseUrl: 'https://enterprise.example.test',
          transportSecurity: 'trusted',
          authMode: 'session-token',
          status: 'connected',
          lastUsedAt: 42,
        }]
      }

      if (command === 'create_workspace_connection') {
        return {
          workspaceConnectionId: 'conn-enterprise',
          workspaceId: 'ws-enterprise',
          label: 'Enterprise Workspace',
          baseUrl: 'https://enterprise.example.test',
          transportSecurity: 'trusted',
          authMode: 'session-token',
          status: 'connected',
          lastUsedAt: 42,
        }
      }

      if (command === 'delete_workspace_connection') {
        return null
      }

      return createHostBootstrap()
    })

    const client = await loadClientModule()

    const listed = await client.listWorkspaceConnections()
    expect(listed).toHaveLength(1)
    expect(invokeSpy).toHaveBeenCalledWith('list_workspace_connections')

    const created = await client.createWorkspaceConnection({
      workspaceId: 'ws-enterprise',
      label: 'Enterprise Workspace',
      baseUrl: 'https://enterprise.example.test',
      transportSecurity: 'trusted',
      authMode: 'session-token',
    })
    expect(created.workspaceConnectionId).toBe('conn-enterprise')
    expect(invokeSpy).toHaveBeenCalledWith('create_workspace_connection', {
      input: {
        workspaceId: 'ws-enterprise',
        label: 'Enterprise Workspace',
        baseUrl: 'https://enterprise.example.test',
        transportSecurity: 'trusted',
        authMode: 'session-token',
      },
    })

    await client.deleteWorkspaceConnection('conn-enterprise')
    expect(invokeSpy).toHaveBeenCalledWith('delete_workspace_connection', {
      workspaceConnectionId: 'conn-enterprise',
    })
  })

  it('creates, lists, and deletes host workspace connections through browser host HTTP endpoints', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')
    delete (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__

    fetchSpy
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ([{
          workspaceConnectionId: 'conn-enterprise',
          workspaceId: 'ws-enterprise',
          label: 'Enterprise Workspace',
          baseUrl: 'https://enterprise.example.test',
          transportSecurity: 'trusted',
          authMode: 'session-token',
          status: 'connected',
          lastUsedAt: 42,
        }]),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers({ 'Content-Type': 'application/json' }),
        json: async () => ({
          workspaceConnectionId: 'conn-enterprise',
          workspaceId: 'ws-enterprise',
          label: 'Enterprise Workspace',
          baseUrl: 'https://enterprise.example.test',
          transportSecurity: 'trusted',
          authMode: 'session-token',
          status: 'connected',
          lastUsedAt: 42,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        headers: new Headers(),
        json: async () => null,
      })

    const client = await loadClientModule()

    const listed = await client.listWorkspaceConnections()
    expect(listed[0]?.workspaceConnectionId).toBe('conn-enterprise')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:43127/api/v1/host/workspace-connections',
      expect.objectContaining({ method: 'GET', headers: expect.any(Headers) }),
    )

    await client.createWorkspaceConnection({
      workspaceId: 'ws-enterprise',
      label: 'Enterprise Workspace',
      baseUrl: 'https://enterprise.example.test',
      transportSecurity: 'trusted',
      authMode: 'session-token',
    })
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:43127/api/v1/host/workspace-connections',
      expect.objectContaining({ method: 'POST', headers: expect.any(Headers) }),
    )

    await client.deleteWorkspaceConnection('conn-enterprise')
    expect(fetchSpy).toHaveBeenNthCalledWith(
      3,
      'http://127.0.0.1:43127/api/v1/host/workspace-connections/conn-enterprise',
      expect.objectContaining({ method: 'DELETE', headers: expect.any(Headers) }),
    )
  })

  it('bridges agent bundle archive picking through the Tauri shell bridge', async () => {
    invokeSpy.mockResolvedValue(null)

    const client = await loadClientModule()
    await (client as typeof client & {
      pickAgentBundleArchive: () => Promise<unknown>
    }).pickAgentBundleArchive()

    expect(invokeSpy).toHaveBeenCalledWith('pick_agent_bundle_archive')
  })

  it('bridges agent bundle export saving for folders and zip archives', async () => {
    invokeSpy.mockResolvedValue(undefined)

    const client = await loadClientModule()
    const payload = {
      rootDirName: 'finance-bundle',
      fileCount: 1,
      agentCount: 1,
      teamCount: 0,
      skillCount: 1,
      mcpCount: 0,
      avatarCount: 1,
      files: [
        {
          fileName: 'Analyst.md',
          contentType: 'text/markdown',
          byteSize: 24,
          dataBase64: 'IyBBbmFseXN0Cg==',
          relativePath: 'finance-bundle/Analyst/Analyst.md',
        },
      ],
      issues: [],
    }

    await (client as typeof client & {
      saveAgentBundleExport: (result: typeof payload, format: 'folder' | 'zip') => Promise<void>
    }).saveAgentBundleExport(payload, 'folder')
    expect(invokeSpy).toHaveBeenCalledWith('save_agent_bundle_folder', { exportPayload: payload })

    await (client as typeof client & {
      saveAgentBundleExport: (result: typeof payload, format: 'folder' | 'zip') => Promise<void>
    }).saveAgentBundleExport(payload, 'zip')
    expect(invokeSpy).toHaveBeenCalledWith('save_agent_bundle_zip', { exportPayload: payload })
  })
})
