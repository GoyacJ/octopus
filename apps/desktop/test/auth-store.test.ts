// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { createDefaultShellPreferences } from '@octopus/schema'

import { useAuthStore } from '@/stores/auth'
import { useShellStore } from '@/stores/shell'
import * as tauriClient from '@/tauri/client'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

describe('useAuthStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    window.localStorage.clear()
    vi.restoreAllMocks()
  })

  async function bootstrapShell() {
    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign')
    return shell
  }

  it('defaults to first-owner registration when the workspace is not initialized', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: false,
      localSetupRequired: true,
      preloadWorkspaceSessions: false,
    })

    await bootstrapShell()
    const auth = useAuthStore()

    await auth.bootstrapAuth()

    expect(auth.dialogOpen).toBe(true)
    expect(auth.mode).toBe('register')
    expect(auth.reason).toBe('first-launch')
  })

  it('requires login when the owner exists but no persisted session is available', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: true,
      localSetupRequired: false,
      preloadWorkspaceSessions: false,
    })

    await bootstrapShell()
    const auth = useAuthStore()

    await auth.bootstrapAuth()

    expect(auth.dialogOpen).toBe(true)
    expect(auth.mode).toBe('login')
    expect(auth.reason).toBe('missing-session')
  })

  it('accepts a persisted valid session and keeps the auth gate closed on startup', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: true,
      localSetupRequired: false,
      preloadWorkspaceSessions: true,
      localSessionValid: true,
    })

    await bootstrapShell()
    const auth = useAuthStore()

    await auth.bootstrapAuth()

    expect(auth.dialogOpen).toBe(false)
    expect(auth.isAuthenticated).toBe(true)
  })

  it('clears an invalid persisted session and falls back to login', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: true,
      localSetupRequired: false,
      preloadWorkspaceSessions: true,
      localSessionValid: false,
    })

    const shell = await bootstrapShell()
    const auth = useAuthStore()

    await auth.bootstrapAuth()

    expect(auth.dialogOpen).toBe(true)
    expect(auth.mode).toBe('login')
    expect(auth.reason).toBe('session-expired')
    expect(shell.activeWorkspaceSession).toBeNull()
  })

  it('connects a remote workspace, persists its session, and activates the saved connection', async () => {
    const shell = useShellStore()
    shell.preferencesState = createDefaultShellPreferences('ws-local', 'proj-redesign')
    shell.workspaceConnectionsState = [
      {
        workspaceConnectionId: 'conn-local',
        workspaceId: 'ws-local',
        label: 'Local Workspace',
        baseUrl: 'http://127.0.0.1:43127',
        transportSecurity: 'loopback',
        authMode: 'session-token',
        status: 'connected',
      },
    ]
    shell.activeWorkspaceConnectionId = 'conn-local'

    const enterpriseWorkspace = {
      id: 'ws-enterprise',
      name: 'Enterprise Workspace',
      slug: 'enterprise-workspace',
      deployment: 'remote',
      bootstrapStatus: 'ready',
      ownerUserId: 'user-owner',
      host: 'enterprise.example.test',
      listenAddress: 'https://enterprise.example.test',
      defaultProjectId: 'proj-launch',
    } as const

    vi.spyOn(tauriClient, 'savePreferences').mockImplementation(async preferences => preferences)
    vi.spyOn(tauriClient, 'createWorkspaceConnection').mockResolvedValue({
      workspaceConnectionId: 'conn-enterprise',
      workspaceId: 'ws-enterprise',
      label: 'Enterprise Workspace',
      baseUrl: 'https://enterprise.example.test',
      transportSecurity: 'trusted',
      authMode: 'session-token',
      status: 'connected',
    })
    vi.spyOn(tauriClient, 'createWorkspaceClient').mockImplementation(({ connection }) => {
      if (!connection.workspaceId) {
        return {
          system: {
            bootstrap: async () => ({
              workspace: enterpriseWorkspace,
              transportSecurity: 'trusted',
              authMode: 'session-token',
              setupRequired: false,
              ownerReady: true,
            }),
          },
        } as ReturnType<typeof tauriClient.createWorkspaceClient>
      }

      return {
        auth: {
          login: async () => ({
            workspace: enterpriseWorkspace,
            session: {
              id: 'sess-enterprise',
              workspaceId: 'ws-enterprise',
              userId: 'user-owner',
              clientAppId: 'octopus-desktop',
              token: 'token-enterprise',
              status: 'active',
              createdAt: 1,
              expiresAt: undefined,
              roleIds: ['role-owner'],
              scopeProjectIds: [],
            },
          }),
        },
      } as ReturnType<typeof tauriClient.createWorkspaceClient>
    })

    const auth = useAuthStore()

    const connection = await auth.connectWorkspace({
      baseUrl: 'https://enterprise.example.test/',
      username: 'owner',
      password: 'secret',
    })

    expect(connection.workspaceConnectionId).toBe('conn-enterprise')
    expect(shell.activeWorkspaceConnectionId).toBe('conn-enterprise')
    expect(shell.activeWorkspaceSession?.token).toBe('token-enterprise')
    expect(auth.isAuthenticated).toBe(true)
  })
})
