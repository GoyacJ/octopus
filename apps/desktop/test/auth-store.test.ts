// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { useAuthStore } from '@/stores/auth'
import { useShellStore } from '@/stores/shell'
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
})
