// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useShellStore } from '@/stores/shell'
import type { WorkspaceClient } from '@/tauri/workspace-client'
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

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for shell state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

function configureWorkspaceClient(
  transform: (client: WorkspaceClient) => WorkspaceClient,
) {
  const createWorkspaceClientMock = vi.mocked(tauriClient.createWorkspaceClient)
  const baseImplementation = createWorkspaceClientMock.getMockImplementation()
  expect(baseImplementation).toBeTypeOf('function')

  createWorkspaceClientMock.mockImplementation((context) =>
    transform(baseImplementation!(context) as WorkspaceClient) as ReturnType<typeof tauriClient.createWorkspaceClient>,
  )
}

describe('Workbench shell layout', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders the topbar and sidebar from the real shell and workspace fixtures', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    const localWorkspaceLabel = String(i18n.global.t('topbar.localWorkspace'))
    await waitFor(() =>
      (mounted.container.textContent?.includes(localWorkspaceLabel) ?? false)
      && (mounted.container.textContent?.includes('Desktop Redesign') ?? false),
    )

    expect(mounted.container.querySelector('[data-testid="workbench-topbar"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain(localWorkspaceLabel)
    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.querySelector('[data-testid="global-search-trigger"]')).not.toBeNull()

    mounted.destroy()
  })

  it('renders topbar breadcrumbs by route section instead of always showing workspace and project', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/dashboard')
    await router.isReady()

    const mounted = mountApp()
    const localWorkspaceLabel = String(i18n.global.t('topbar.localWorkspace'))
    await waitFor(() =>
      (mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent?.includes(localWorkspaceLabel) ?? false)
      && (mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent?.includes('Desktop Redesign') ?? false),
    )

    const breadcrumbText = () =>
      mounted.container
        .querySelector('[data-testid="topbar-breadcrumbs"]')
        ?.textContent
        ?.replace(/\s+/g, ' ')
        .trim() ?? ''

    expect(breadcrumbText()).toContain('网易Lobster')
    expect(breadcrumbText()).toContain(localWorkspaceLabel)
    expect(breadcrumbText()).toContain('Desktop Redesign')
    expect(breadcrumbText()).toContain(String(i18n.global.t('sidebar.navigation.dashboard')))

    await router.push('/workspaces/ws-local/projects')
    await waitFor(() => router.currentRoute.value.name === 'workspace-projects')

    expect(breadcrumbText()).toContain('网易Lobster')
    expect(breadcrumbText()).toContain(localWorkspaceLabel)
    expect(breadcrumbText()).not.toContain('Desktop Redesign')
    expect(breadcrumbText()).toContain(String(i18n.global.t('sidebar.navigation.projects')))

    await router.push('/settings')
    await waitFor(() => router.currentRoute.value.name === 'app-settings')

    expect(breadcrumbText()).toContain('网易Lobster')
    expect(breadcrumbText()).not.toContain('Local Workspace')
    expect(breadcrumbText()).not.toContain('Desktop Redesign')
    expect(breadcrumbText()).toContain(String(i18n.global.t('topbar.settings')))

    mounted.destroy()
  })

  it('keeps the topbar workspace breadcrumb aligned with the sidebar label for loopback workspaces', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    const shell = useShellStore()
    const localWorkspaceLabel = String(i18n.global.t('topbar.localWorkspace'))

    await waitFor(() =>
      (mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent?.includes(localWorkspaceLabel) ?? false),
    )

    shell.workspaceConnectionsState = shell.workspaceConnectionsState.map(connection =>
      connection.workspaceConnectionId === 'conn-local'
        ? { ...connection, label: 'Local Runtime' }
        : connection,
    )

    await waitFor(() =>
      (mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent?.includes(localWorkspaceLabel) ?? false)
      && (mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]')?.textContent?.includes(localWorkspaceLabel) ?? false),
    )

    expect(mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent).toContain(localWorkspaceLabel)
    expect(mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent).not.toContain('Local Runtime')
    expect(mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]')?.textContent).toContain(localWorkspaceLabel)

    mounted.destroy()
  })

  it('renders translated local workspace labels and connection dots in the footer workspace menu', async () => {
    const originalLocale = i18n.global.locale.value
    i18n.global.locale.value = 'zh-CN'

    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    try {
      const shell = useShellStore()
      const localWorkspaceLabel = String(i18n.global.t('topbar.localWorkspace'))

      await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]') !== null)

      shell.workspaceConnectionsState = shell.workspaceConnectionsState.map(connection => {
        if (connection.workspaceConnectionId === 'conn-local') {
          return { ...connection, label: 'Local Runtime' }
        }

        if (connection.workspaceConnectionId === 'conn-enterprise') {
          return { ...connection, status: 'unreachable' }
        }

        return connection
      })

      mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
      await waitFor(() => document.body.querySelector('[data-testid="sidebar-workspace-menu-item-conn-local"]') !== null)
      await waitFor(() =>
        (document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-item-conn-local"]')?.textContent?.includes(localWorkspaceLabel) ?? false)
        && (document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-status-dot-conn-enterprise"]')?.className.includes('bg-status-error') ?? false),
      )

      const localItem = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-item-conn-local"]')
      const localDot = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-status-dot-conn-local"]')
      const enterpriseDot = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-status-dot-conn-enterprise"]')

      expect(localItem?.textContent).toContain(localWorkspaceLabel)
      expect(localItem?.textContent).not.toContain('Local Runtime')
      expect(localItem?.textContent).not.toContain(String(i18n.global.t('common.selected')))
      expect(localDot?.className).toContain('bg-status-success')
      expect(enterpriseDot?.className).toContain('bg-status-error')
    } finally {
      mounted.destroy()
      i18n.global.locale.value = originalLocale
    }
  })

  it('does not blank on the root entry route before any explicit navigation', async () => {
    await router.push('/')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() =>
      (mounted.container.textContent?.includes('Local Workspace') ?? false)
      || (mounted.container.textContent?.includes('Octopus') ?? false),
    )

    expect(mounted.container.textContent?.trim().length ?? 0).toBeGreaterThan(0)

    mounted.destroy()
  })

  it('updates theme and locale preferences through the topbar controls', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    const shell = useShellStore()
    await waitFor(() => mounted.container.querySelector('[data-testid="topbar-theme-toggle"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-toggle"]')?.click()
    await nextTick()
    Array.from(mounted.container.querySelectorAll('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('topbar.themeModes.light'))))?.click()
    await waitFor(() => shell.preferences.theme === 'light')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-locale-toggle"]')?.click()
    await nextTick()
    Array.from(mounted.container.querySelectorAll('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('topbar.localeModes.en-US'))))?.click()
    await waitFor(() => shell.preferences.locale === 'en-US')

    expect(shell.preferences.theme).toBe('light')
    expect(shell.preferences.locale).toBe('en-US')

    mounted.destroy()
  })

  it('opens the pet panel from the sidebar and sends messages through runtime', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/dashboard')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="desktop-pet-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="desktop-pet-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="desktop-pet-chat"]') !== null)

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="desktop-pet-input"]')
    expect(input).not.toBeNull()
    input!.value = '宠物你好'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    document.body.querySelector<HTMLButtonElement>('[data-testid="desktop-pet-send"]')?.click()

    await waitFor(() => (document.body.querySelector('[data-testid="desktop-pet-messages"]')?.textContent?.includes('宠物你好') ?? false))
    await waitFor(() => (document.body.querySelector('[data-testid="desktop-pet-messages"]')?.textContent?.includes('Completed request') ?? false), 3000)

    expect(document.body.querySelector('[data-testid="desktop-pet-messages"]')?.textContent).toContain('宠物你好')
    expect(document.body.querySelector('[data-testid="desktop-pet-messages"]')?.textContent).toContain('Completed request')
    expect(mounted.container.textContent).not.toContain('小章 proj-redesign')

    mounted.destroy()
  })

  it('keeps the pet draft and shows the runtime error when submission fails', async () => {
    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async submitUserTurn() {
          throw new Error('missing configured credential env var `ANTHROPIC_API_KEY` for provider `anthropic`')
        },
      },
    }))

    await router.push('/workspaces/ws-local/projects/proj-redesign/dashboard')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="desktop-pet-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="desktop-pet-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="desktop-pet-chat"]') !== null)

    const input = document.body.querySelector<HTMLInputElement>('[data-testid="desktop-pet-input"]')
    expect(input).not.toBeNull()
    input!.value = '宠物你好'
    input!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    document.body.querySelector<HTMLButtonElement>('[data-testid="desktop-pet-send"]')?.click()

    await waitFor(() => (document.body.querySelector('[data-testid="desktop-pet-error"]')?.textContent?.includes('missing configured credential env var') ?? false))

    expect((document.body.querySelector('[data-testid="desktop-pet-input"]') as HTMLInputElement | null)?.value).toBe('宠物你好')
    expect(document.body.querySelector('[data-testid="desktop-pet-error"]')?.textContent).toContain('missing configured credential env var `ANTHROPIC_API_KEY` for provider `anthropic`')
    expect(document.body.querySelector('[data-testid="desktop-pet-error"]')?.getAttribute('role')).toBe('alert')

    mounted.destroy()
  })

  it('clamps the pet panel inside the viewport when the trigger is near the left edge', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/dashboard')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="desktop-pet-trigger"]') !== null)

    Object.defineProperty(window, 'innerWidth', {
      configurable: true,
      writable: true,
      value: 320,
    })
    Object.defineProperty(window, 'innerHeight', {
      configurable: true,
      writable: true,
      value: 900,
    })

    const trigger = mounted.container.querySelector<HTMLElement>('[data-testid="desktop-pet-trigger"]')
    expect(trigger).not.toBeNull()
    trigger!.getBoundingClientRect = () => ({
      x: 24,
      y: 720,
      width: 44,
      height: 44,
      top: 720,
      right: 68,
      bottom: 764,
      left: 24,
      toJSON: () => ({}),
    })

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="desktop-pet-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="desktop-pet-panel"]') !== null)

    const panel = document.body.querySelector<HTMLElement>('[data-testid="desktop-pet-panel"]')
    expect(panel).not.toBeNull()
    panel!.getBoundingClientRect = () => ({
      x: 0,
      y: 0,
      width: 352,
      height: 320,
      top: 0,
      right: 352,
      bottom: 320,
      left: 0,
      toJSON: () => ({}),
    })

    window.dispatchEvent(new Event('resize'))
    await nextTick()

    expect(panel?.style.left).toBe('16px')
    expect(panel?.style.visibility).toBe('visible')

    mounted.destroy()
  })

  it('renders the notification trigger in the top-right action cluster', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="topbar-notification-trigger"]') !== null)

    expect(mounted.container.querySelector('[data-testid="topbar-notification-trigger"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="topbar-profile-trigger"]')).not.toBeNull()

    mounted.destroy()
  })

  it('switches workspace scope from the footer workspace menu', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-workspace-menu-item-conn-enterprise"]') !== null)

    document.body
      .querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-item-conn-enterprise"]')
      ?.click()

    await waitFor(() => String(router.currentRoute.value.params.workspaceId) === 'ws-enterprise')
    expect(String(router.currentRoute.value.params.workspaceId)).toBe('ws-enterprise')

    mounted.destroy()
  })

  it('removes the sidebar top workspace list and exposes workspace switching actions from the footer menu', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]') !== null)

    expect(mounted.container.querySelector('[data-testid="sidebar-workspace-list-top"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-workspace-navigation-menu"]')).toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-workspace-menu-list"]') !== null)

    expect(document.body.querySelector('[data-testid="sidebar-workspace-menu-list"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="sidebar-workspace-navigation-menu"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="sidebar-workspace-nav-workspace-projects"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="sidebar-connect-workspace-trigger"]')).not.toBeNull()
    expect(document.body.textContent).toContain(String(i18n.global.t('sidebar.workspaceMenu.title')))

    mounted.destroy()
  })

  it('navigates to the project management workspace page from the footer workspace menu', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-workspace-nav-workspace-projects"]') !== null)

    document.body
      .querySelector<HTMLAnchorElement>('[data-testid="sidebar-workspace-nav-workspace-projects"]')
      ?.click()

    await waitFor(() => router.currentRoute.value.name === 'workspace-projects')
    expect(router.currentRoute.value.name).toBe('workspace-projects')

    mounted.destroy()
  })

  it('creates a project from the sidebar quick-create popover and lands on project settings', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-create-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-create-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-project-create-popover"]') !== null)

    const nameInput = document.body.querySelector<HTMLInputElement>('[data-testid="sidebar-project-create-name-input"]')
    const descriptionInput = document.body.querySelector<HTMLTextAreaElement>('[data-testid="sidebar-project-create-description-input"]')
    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()

    nameInput!.value = 'Strategy Launch'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Launch checklist and delivery alignment.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    document.body.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-create-submit"]')?.click()

    await waitFor(() =>
      router.currentRoute.value.name === 'project-settings'
      && String(router.currentRoute.value.params.projectId).includes('strategy-launch'),
    )

    expect(mounted.container.querySelector('[data-testid="sidebar-project-proj-strategy-launch"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Strategy Launch')

    mounted.destroy()
  })

  it('shows the project settings menu item in each project module list', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-proj-redesign"]') !== null)

    const settingsLink = mounted.container.querySelector<HTMLAnchorElement>('[data-testid="sidebar-project-module-proj-redesign-settings"]')
    expect(settingsLink).not.toBeNull()

    settingsLink?.click()

    await waitFor(() => router.currentRoute.value.name === 'project-settings')
    expect(router.currentRoute.value.name).toBe('project-settings')

    mounted.destroy()
  })

  it('only expands the selected project and opens another project when its collapsed card is clicked', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-proj-redesign"]') !== null)

    expect(mounted.container.querySelector('[data-testid="sidebar-project-module-proj-redesign-settings"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-project-module-proj-governance-settings"]')).toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-summary-proj-governance"]')?.click()

    await waitFor(() =>
      router.currentRoute.value.name === 'project-dashboard'
      && String(router.currentRoute.value.params.projectId) === 'proj-governance',
    )

    expect(mounted.container.querySelector('[data-testid="sidebar-project-module-proj-redesign-settings"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-project-module-proj-governance-settings"]')).not.toBeNull()

    mounted.destroy()
  })

  it('opens a delete confirmation for collapsed projects and removes the project after confirm', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-delete-trigger-proj-governance"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-delete-trigger-proj-governance"]')?.click()

    await waitFor(() => document.body.querySelector('[data-testid="sidebar-project-delete-dialog"]') !== null)
    expect(document.body.textContent).toContain('Workspace Governance')

    document.body.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-delete-confirm"]')?.click()

    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-proj-governance"]') === null)
    expect(mounted.container.querySelector('[data-testid="sidebar-project-proj-redesign"]')).not.toBeNull()

    mounted.destroy()
  })
})
