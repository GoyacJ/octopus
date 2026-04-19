// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'
import { vi } from 'vitest'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useWorkspaceStore } from '@/stores/workspace'
import * as tauriClient from '@/tauri/client'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})

function mountApp() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  const app = createApp(App)
  app.use(pinia)
  app.use(i18n)
  app.use(router)
  app.mount(container)

  return {
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function flushUi() {
  await nextTick()
  await new Promise(resolve => window.setTimeout(resolve, 0))
  await nextTick()
}

async function waitFor(predicate: () => boolean, timeoutMs = 3000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for auth gate state')
    }
    await flushUi()
  }
}

describe('App auth gate', () => {
  beforeEach(async () => {
    vi.unstubAllEnvs()
    vi.stubEnv('VITE_HOST_RUNTIME', 'tauri')
    window.localStorage.clear()
    document.body.innerHTML = ''
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()
  })

  it('shows the registration gate for an uninitialized local workspace', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: false,
      localSetupRequired: true,
      preloadWorkspaceSessions: false,
    })

    const mounted = mountApp()
    await flushUi()

    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.register.title')))
    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.fields.username')))

    mounted.destroy()
  })

  it('prefills the bootstrap mapped directory and persists the selected directory into the canonical workspace summary', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: false,
      localSetupRequired: true,
      preloadWorkspaceSessions: false,
    })
    vi.spyOn(tauriClient as unknown as { pickAvatarImage: () => Promise<any> }, 'pickAvatarImage')
      .mockResolvedValue({
        fileName: 'owner-avatar.png',
        contentType: 'image/png',
        dataBase64: 'b3duZXI=',
        byteSize: 5,
      })
    vi.spyOn(tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> }, 'pickResourceDirectory')
      .mockResolvedValue('/Users/goya/Workspace Launchpad')

    const mounted = mountApp()

    try {
      await waitFor(() => document.body.textContent?.includes(String(i18n.global.t('authGate.register.title'))) ?? false)

      const mappedDirectoryInput = document.body.querySelector<HTMLInputElement>('[data-testid="auth-gate-mapped-directory-input"]')
      expect(mappedDirectoryInput).not.toBeNull()
      expect(mappedDirectoryInput?.value).toBe('/Users/goya/Octopus')

      document.body.querySelector<HTMLButtonElement>('[data-testid="auth-gate-avatar-pick"]')?.click()
      await waitFor(() => (document.body.textContent?.includes('owner-avatar.png') ?? false))

      document.body.querySelector<HTMLButtonElement>('[data-testid="auth-gate-mapped-directory-pick"]')?.click()
      await waitFor(() =>
        document.body.querySelector<HTMLInputElement>('[data-testid="auth-gate-mapped-directory-input"]')?.value === '/Users/goya/Workspace Launchpad',
      )

      const usernameInput = document.body.querySelector<HTMLInputElement>('[data-testid="auth-gate-username-input"]')
      const displayNameInput = document.body.querySelector<HTMLInputElement>('[data-testid="auth-gate-display-name-input"]')
      const passwordInput = document.body.querySelector<HTMLInputElement>('[data-testid="auth-gate-password-input"]')
      const confirmPasswordInput = document.body.querySelector<HTMLInputElement>('[data-testid="auth-gate-confirm-password-input"]')

      expect(usernameInput).not.toBeNull()
      expect(displayNameInput).not.toBeNull()
      expect(passwordInput).not.toBeNull()
      expect(confirmPasswordInput).not.toBeNull()

      usernameInput!.value = 'owner'
      usernameInput!.dispatchEvent(new Event('input', { bubbles: true }))
      displayNameInput!.value = 'Workspace Owner'
      displayNameInput!.dispatchEvent(new Event('input', { bubbles: true }))
      passwordInput!.value = 'secret-123'
      passwordInput!.dispatchEvent(new Event('input', { bubbles: true }))
      confirmPasswordInput!.value = 'secret-123'
      confirmPasswordInput!.dispatchEvent(new Event('input', { bubbles: true }))

      document.body.querySelector<HTMLButtonElement>('[data-testid="auth-gate-submit"]')?.click()

      await waitFor(() => !document.body.textContent?.includes(String(i18n.global.t('authGate.register.title'))))
      const workspaceStore = useWorkspaceStore()
      await workspaceStore.ensureWorkspaceBootstrap('conn-local', { force: true })
      expect(workspaceStore.activeWorkspace?.mappedDirectory).toBe('/Users/goya/Workspace Launchpad')
    } finally {
      mounted.destroy()
    }
  })

  it('shows the login gate after a persisted session expires', async () => {
    installWorkspaceApiFixture({
      localOwnerReady: true,
      localSetupRequired: false,
      preloadWorkspaceSessions: true,
      localSessionValid: false,
    })

    const mounted = mountApp()
    await flushUi()

    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.login.title')))
    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.fields.password')))

    mounted.destroy()
  })

  it('redirects browser host to the dedicated login route when no session exists', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')

    installWorkspaceApiFixture({
      localOwnerReady: true,
      localSetupRequired: false,
      preloadWorkspaceSessions: false,
    })

    const mounted = mountApp()
    await flushUi()
    await flushUi()

    expect(router.currentRoute.value.name).toBe('auth-login')
    expect(document.body.textContent).toContain(String(i18n.global.t('authGate.login.title')))
    const loginView = document.body.querySelector<HTMLElement>('[data-testid="browser-auth-login-view"]')
    expect(loginView?.className).toContain('border-t')
    expect(loginView?.className).toContain('border-border')
    expect(loginView?.className).not.toContain('rounded-[var(--radius-l)]')
    expect(loginView?.className).not.toContain('bg-card')

    mounted.destroy()
  })

  it('renders the browser login route as an integrated auth shell with a calm intro band', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')

    installWorkspaceApiFixture({
      localOwnerReady: true,
      localSetupRequired: false,
      preloadWorkspaceSessions: false,
    })

    const mounted = mountApp()

    try {
      await flushUi()
      await flushUi()

      const authShell = document.body.querySelector<HTMLElement>('[data-testid="browser-auth-shell"]')
      const introBand = document.body.querySelector<HTMLElement>('[data-testid="browser-auth-intro"]')

      expect(router.currentRoute.value.name).toBe('auth-login')
      expect(authShell).not.toBeNull()
      expect(authShell?.className).toContain('bg-surface')
      expect(authShell?.className).not.toContain('bg-card')
      expect(authShell?.className).not.toContain('shadow-sm')

      expect(introBand).not.toBeNull()
      expect(introBand?.className).toContain('bg-subtle')
      expect(introBand?.className).not.toContain('bg-muted/35')
    } finally {
      mounted.destroy()
    }
  })

  it('redirects an authenticated browser host away from the login route', async () => {
    vi.stubEnv('VITE_HOST_RUNTIME', 'browser')
    vi.stubEnv('VITE_HOST_API_BASE_URL', 'http://127.0.0.1:43127')
    vi.stubEnv('VITE_HOST_AUTH_TOKEN', 'browser-host-token')

    installWorkspaceApiFixture({
      localOwnerReady: true,
      localSetupRequired: false,
      preloadWorkspaceSessions: true,
      localSessionValid: true,
    })

    await router.push('/login')
    await router.isReady()

    const mounted = mountApp()
    await flushUi()
    await flushUi()

    expect(router.currentRoute.value.name).toBe('workspace-overview')
    expect(document.body.textContent).not.toContain(String(i18n.global.t('authGate.login.title')))

    mounted.destroy()
  })
})
