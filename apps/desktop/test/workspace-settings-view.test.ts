// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import * as tauriClient from '@/tauri/client'
import { useNotificationStore } from '@/stores/notifications'
import { useWorkspaceStore } from '@/stores/workspace'
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
    app,
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for workspace settings state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Workspace settings view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    await router.push('/workspaces/ws-local/console/settings')
    await router.isReady()
    document.body.innerHTML = ''
  })

  it('renders the workspace settings page as the first console surface and saves canonical workspace fields', async () => {
    vi.spyOn(tauriClient as unknown as { pickAvatarImage: () => Promise<any> }, 'pickAvatarImage')
      .mockResolvedValue({
        fileName: 'workspace-hq.png',
        contentType: 'image/png',
        dataBase64: 'd29ya3NwYWNlLWhx',
        byteSize: 12,
      })
    vi.spyOn(tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> }, 'pickResourceDirectory')
      .mockResolvedValue('/Users/goya/Workspace HQ')
    const mounted = mountApp()
    const notificationStore = useNotificationStore()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="workspace-settings-view"]') !== null)

    expect(router.currentRoute.value.name).toBe('workspace-console-settings')
    expect(mounted.container.querySelector('[data-testid="workspace-settings-name-input"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-settings-avatar-preview"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-settings-avatar-pick"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-settings-directory-input"]')).not.toBeNull()

    await waitFor(() =>
      mounted.container.querySelector<HTMLButtonElement>('[data-testid="workspace-settings-directory-pick"]')?.disabled === false,
    )

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="workspace-settings-name-input"]')
    expect(nameInput).not.toBeNull()
    nameInput!.value = 'Workspace HQ'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="workspace-settings-avatar-pick"]')?.click()
    await waitFor(() =>
      mounted.container.querySelector<HTMLElement>('[data-testid="workspace-settings-avatar-file-label"]')?.textContent?.includes('workspace-hq.png') ?? false,
    )

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="workspace-settings-directory-pick"]')?.click()
    await waitFor(() =>
      mounted.container.querySelector<HTMLInputElement>('[data-testid="workspace-settings-directory-input"]')?.value === '/Users/goya/Workspace HQ',
    )

    const updateWorkspaceSpy = vi.spyOn(workspaceStore, 'updateWorkspace')

    await waitFor(() =>
      mounted.container.querySelector<HTMLButtonElement>('[data-testid="workspace-settings-save"]')?.disabled === false,
    )

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="workspace-settings-save"]')?.click()

    await waitFor(() => workspaceStore.activeWorkspace?.name === 'Workspace HQ')
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.source === 'workspace-project-governance'
        && notification.routeTo === '/workspaces/ws-local/console/settings'
        && notification.body?.includes('Workspace HQ'),
      ),
    )

    expect(updateWorkspaceSpy).toHaveBeenCalledWith(expect.objectContaining({
      name: 'Workspace HQ',
      mappedDirectory: '/Users/goya/Workspace HQ',
    }))
    expect(notificationStore.notificationsState.some(notification =>
      notification.source === 'workspace-project-governance'
      && notification.routeTo === '/workspaces/ws-local/console/settings'
      && notification.actionLabel
      && notification.body?.includes('Workspace HQ'),
    )).toBe(true)
    expect(workspaceStore.activeWorkspace?.mappedDirectory).toBe('/Users/goya/Workspace HQ')
    expect(mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent).toContain('Workspace HQ')
    expect(mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]')?.textContent).toContain('Workspace HQ')
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger-icon"] img') !== null)
    expect(mounted.container.querySelector<HTMLImageElement>('[data-testid="sidebar-workspace-menu-trigger-icon"] img')?.src)
      .toContain('data:image/png;base64,d29ya3NwYWNlLWhx')

    mounted.destroy()
  })
})
