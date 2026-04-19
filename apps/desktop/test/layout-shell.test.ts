// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useAuthStore } from '@/stores/auth'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'
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

interface ClientCallCounters {
  workspaceGet: number
  projectsList: number
  workspaceOverview: number
  accessUsers: number
  accessRoles: number
  accessPermissions: number
  runtimeBootstrap: number
}

function createClientCallCounters(): ClientCallCounters {
  return {
    workspaceGet: 0,
    projectsList: 0,
    workspaceOverview: 0,
    accessUsers: 0,
    accessRoles: 0,
    accessPermissions: 0,
    runtimeBootstrap: 0,
  }
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

  it('keeps the workbench main canvas on a fixed-height shell so pages can own their internal scroll areas', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="workbench-main-canvas"]') !== null)

    const shellRoot = mounted.container.querySelector<HTMLElement>('[data-testid="workbench-shell"]')
    const main = mounted.container.querySelector<HTMLElement>('[data-testid="workbench-main"]')
    const canvas = mounted.container.querySelector<HTMLElement>('[data-testid="workbench-main-canvas"]')

    expect(shellRoot).not.toBeNull()
    expect(shellRoot?.className).toContain('bg-sidebar')

    expect(main).not.toBeNull()
    expect(main?.className).toContain('bg-[color-mix(in_srgb,var(--background)_92%,var(--sidebar)_8%)]')

    expect(canvas).not.toBeNull()
    expect(canvas?.className).toContain('h-full')
    expect(canvas?.className).not.toContain('min-h-full')
    expect(canvas?.className).toContain('min-w-0')

    mounted.destroy()
  })

  it('keeps the topbar action triggers on neutral hover states', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="topbar-profile-trigger"]') !== null)

    const searchTrigger = mounted.container.querySelector<HTMLElement>('[data-testid="global-search-trigger"]')
    const settingsTrigger = mounted.container.querySelector<HTMLElement>('[data-testid="topbar-settings-button"]')
    const notificationTrigger = mounted.container.querySelector<HTMLElement>('[data-testid="topbar-notification-trigger"]')
    const profileTrigger = mounted.container.querySelector<HTMLElement>('[data-testid="topbar-profile-trigger"]')

    expect(searchTrigger?.className).toContain('hover:bg-subtle')
    expect(searchTrigger?.className).not.toContain('hover:bg-accent')
    expect(settingsTrigger?.className).toContain('hover:bg-subtle')
    expect(settingsTrigger?.className).not.toContain('hover:bg-accent')
    expect(notificationTrigger?.className).toContain('hover:bg-subtle')
    expect(notificationTrigger?.className).not.toContain('hover:bg-accent')
    expect(profileTrigger?.className).toContain('hover:bg-subtle')
    expect(profileTrigger?.className).not.toContain('hover:bg-accent')

    mounted.destroy()
  })

  it('renders the topbar search trigger as an integrated shell control instead of a floating card', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="global-search-trigger"]') !== null)

    const searchTrigger = mounted.container.querySelector<HTMLElement>('[data-testid="global-search-trigger"]')

    expect(searchTrigger?.className).toContain('border-border')
    expect(searchTrigger?.className).toContain('bg-surface')
    expect(searchTrigger?.className).not.toContain('shadow-xs')

    mounted.destroy()
  })

  it('uses bordered accent-soft open states across topbar shell triggers', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() =>
      mounted.container.querySelector('[data-testid="topbar-theme-toggle"]') !== null
      && mounted.container.querySelector('[data-testid="topbar-notification-trigger"]') !== null
      && mounted.container.querySelector('[data-testid="topbar-profile-trigger"]') !== null,
    )

    const themeToggle = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-toggle"]')
    const notificationTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-notification-trigger"]')
    const profileTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-profile-trigger"]')

    expect(themeToggle?.className).toContain('border-transparent')
    expect(themeToggle?.className).toContain('hover:border-border')
    expect(notificationTrigger?.className).toContain('border-transparent')
    expect(notificationTrigger?.className).toContain('hover:border-border')
    expect(profileTrigger?.className).toContain('border-transparent')
    expect(profileTrigger?.className).toContain('hover:border-border')

    themeToggle?.click()
    await waitFor(() => document.body.querySelector('[data-testid="topbar-theme-menu"]') !== null)

    expect(themeToggle?.className).toContain('border-border-strong')
    expect(themeToggle?.className).toContain('bg-accent')
    expect(themeToggle?.className).not.toContain('shadow-xs')

    themeToggle?.click()
    await waitFor(() => document.body.querySelector('[data-testid="topbar-theme-menu"]') === null)

    notificationTrigger?.click()
    await waitFor(() => document.body.querySelector('[data-testid="ui-message-center"]') !== null)

    expect(notificationTrigger?.className).toContain('border-border-strong')
    expect(notificationTrigger?.className).toContain('bg-accent')
    expect(notificationTrigger?.querySelector('svg')?.getAttribute('class')).toContain('text-text-primary')

    notificationTrigger?.click()
    await waitFor(() => document.body.querySelector('[data-testid="ui-message-center"]') === null)

    profileTrigger?.click()
    await waitFor(() => document.body.querySelector('[data-testid="topbar-account-menu"]') !== null)

    expect(profileTrigger?.className).toContain('border-border-strong')
    expect(profileTrigger?.className).toContain('bg-accent')
    expect(profileTrigger?.className).not.toContain('shadow-xs')

    mounted.destroy()
  })

  it('renders the topbar account menu as an integrated shell with calm intro and action bands', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    try {
      await waitFor(() => mounted.container.querySelector('[data-testid="topbar-profile-trigger"]') !== null)

      mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-profile-trigger"]')?.click()
      await waitFor(() => document.body.querySelector('[data-testid="topbar-account-menu"]') !== null)

      const intro = document.body.querySelector<HTMLElement>('[data-testid="topbar-account-menu-intro"]')
      const actions = document.body.querySelector<HTMLElement>('[data-testid="topbar-account-menu-actions"]')

      expect(intro).not.toBeNull()
      expect(intro?.className).toContain('border-b')
      expect(intro?.className).toContain('bg-subtle')

      expect(actions).not.toBeNull()
      expect(actions?.className).toContain('border-t')
      expect(actions?.className).toContain('bg-subtle')
    } finally {
      mounted.destroy()
    }
  })

  it('renders the topbar theme and locale menus through the shared selection menu shell', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    try {
      const shell = useShellStore()
      await waitFor(() =>
        mounted.container.querySelector('[data-testid="topbar-theme-toggle"]') !== null
        && mounted.container.querySelector('[data-testid="topbar-locale-toggle"]') !== null,
      )

      mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-toggle"]')?.click()
      await waitFor(() => document.body.querySelector('[data-testid="topbar-theme-menu"]') !== null)

      const themeMenu = document.body.querySelector<HTMLElement>('[data-testid="topbar-theme-menu"]')
      const themeHeader = themeMenu?.firstElementChild as HTMLElement | null
      const activeThemeOption = document.body.querySelector<HTMLElement>(`[data-testid="topbar-theme-option-${shell.preferences.theme}"]`)
      const inactiveThemeKey = shell.preferences.theme === 'light' ? 'dark' : 'light'
      const inactiveThemeOption = document.body.querySelector<HTMLElement>(`[data-testid="topbar-theme-option-${inactiveThemeKey}"]`)

      expect(themeMenu).not.toBeNull()
      expect(themeMenu?.textContent).toContain(String(i18n.global.t('topbar.theme')))
      expect(themeMenu?.textContent).toContain(String(i18n.global.t('topbar.themeMenuLabel')))
      expect(themeHeader?.className).toContain('border-b')
      expect(themeHeader?.className).toContain('bg-subtle')
      expect(activeThemeOption).not.toBeNull()
      expect(activeThemeOption?.className).toContain('border-border-strong')
      expect(activeThemeOption?.className).toContain('bg-accent')
      expect(inactiveThemeOption?.className).toContain('hover:bg-subtle')
      expect(inactiveThemeOption?.className).not.toContain('hover:bg-accent')

      mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-toggle"]')?.click()
      await waitFor(() => document.body.querySelector('[data-testid="topbar-theme-menu"]') === null)

      mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-locale-toggle"]')?.click()
      await waitFor(() => document.body.querySelector('[data-testid="topbar-locale-menu"]') !== null)

      const localeMenu = document.body.querySelector<HTMLElement>('[data-testid="topbar-locale-menu"]')
      const localeHeader = localeMenu?.firstElementChild as HTMLElement | null
      const activeLocaleOption = document.body.querySelector<HTMLElement>(`[data-testid="topbar-locale-option-${shell.preferences.locale}"]`)

      expect(localeMenu).not.toBeNull()
      expect(localeMenu?.textContent).toContain(String(i18n.global.t('topbar.locale')))
      expect(localeMenu?.textContent).toContain(String(i18n.global.t('topbar.localeMenuLabel')))
      expect(localeHeader?.className).toContain('border-b')
      expect(localeHeader?.className).toContain('bg-subtle')
      expect(activeLocaleOption).not.toBeNull()
      expect(activeLocaleOption?.className).toContain('border-border-strong')
      expect(activeLocaleOption?.className).toContain('bg-accent')
    } finally {
      mounted.destroy()
    }
  })

  it('opens and closes the topbar message center from the notification trigger', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="topbar-notification-trigger"]') !== null)

    const trigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-notification-trigger"]')
    expect(document.body.querySelector('[data-testid="ui-message-center"]')).toBeNull()

    trigger?.click()
    await waitFor(() => document.body.querySelector('[data-testid="ui-message-center"]') !== null)

    trigger?.click()
    await waitFor(() => document.body.querySelector('[data-testid="ui-message-center"]') === null)

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

    expect(breadcrumbText()).toContain('Octopus')
    expect(breadcrumbText()).toContain(localWorkspaceLabel)
    expect(breadcrumbText()).toContain('Desktop Redesign')
    expect(breadcrumbText()).toContain(String(i18n.global.t('sidebar.navigation.dashboard')))

    await router.push('/workspaces/ws-local/console/projects')
    await waitFor(() => router.currentRoute.value.name === 'workspace-console-projects')

    expect(breadcrumbText()).toContain('Octopus')
    expect(breadcrumbText()).toContain(localWorkspaceLabel)
    expect(breadcrumbText()).not.toContain('Desktop Redesign')
    expect(breadcrumbText()).toContain(String(i18n.global.t('sidebar.navigation.console')))

    await router.push('/settings')
    await waitFor(() => router.currentRoute.value.name === 'app-settings')

    expect(breadcrumbText()).toContain('Octopus')
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
    const canonicalWorkspaceLabel = 'Local Workspace'
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
      (mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent?.includes(canonicalWorkspaceLabel) ?? false)
      && (mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]')?.textContent?.includes(canonicalWorkspaceLabel) ?? false),
    )

    expect(mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent).toContain(canonicalWorkspaceLabel)
    expect(mounted.container.querySelector('[data-testid="topbar-breadcrumbs"]')?.textContent).not.toContain('Local Runtime')
    expect(mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]')?.textContent).toContain(canonicalWorkspaceLabel)

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
      const canonicalWorkspaceLabel = 'Local Workspace'

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
        (document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-item-conn-local"]')?.textContent?.includes(canonicalWorkspaceLabel) ?? false)
        && (document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-status-dot-conn-enterprise"]')?.className.includes('bg-status-error') ?? false),
      )

      const localItem = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-item-conn-local"]')
      const localDot = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-status-dot-conn-local"]')
      const enterpriseDot = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-status-dot-conn-enterprise"]')

      expect(localItem?.textContent).toContain(canonicalWorkspaceLabel)
      expect(localItem?.textContent).not.toContain('Local Runtime')
      expect(localItem?.textContent).not.toContain(String(i18n.global.t('common.selected')))
      expect(localDot?.className).toContain('bg-status-success')
      expect(enterpriseDot?.className).toContain('bg-status-error')
    } finally {
      mounted.destroy()
      i18n.global.locale.value = originalLocale
    }
  })

  it('uses the logo-inspired accent treatment for the footer workspace trigger icon', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    try {
      await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]') !== null)

      const triggerIcon = mounted.container.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-trigger-icon"]')
      const trigger = mounted.container.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-trigger"]')

      expect(trigger).not.toBeNull()
      expect(trigger?.className).toContain('hover:bg-subtle')
      expect(trigger?.className).not.toContain('workspace-menu-trigger')
      expect(triggerIcon).not.toBeNull()
      expect(triggerIcon?.className).toContain('bg-primary/10')
      expect(triggerIcon?.className).toContain('text-primary')
      expect(triggerIcon?.className).not.toContain('workspace-menu-trigger__icon')

      mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
      await waitFor(() => document.body.querySelector('[data-testid="sidebar-workspace-menu-list"]') !== null)

      const openTrigger = mounted.container.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-trigger"]')
      expect(openTrigger?.className).toContain('bg-accent')
      expect(openTrigger?.className).toContain('border-border-strong')
    } finally {
      mounted.destroy()
    }
  })

  it('renders the footer workspace menu as an integrated shell instead of a loose stacked popover', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    try {
      await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]') !== null)

      mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
      await waitFor(() => document.body.querySelector('[data-testid="sidebar-workspace-menu-list"]') !== null)

      const intro = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-intro"]')
      const actions = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-actions"]')
      const connectAction = document.body.querySelector<HTMLElement>('[data-testid="sidebar-connect-workspace-trigger"]')

      expect(intro).not.toBeNull()
      expect(intro?.className).toContain('border-b')
      expect(intro?.className).toContain('bg-subtle')

      expect(actions).not.toBeNull()
      expect(actions?.className).toContain('border-t')
      expect(actions?.className).toContain('bg-subtle')

      expect(connectAction).not.toBeNull()
      expect(connectAction?.className).toContain('justify-start')
    } finally {
      mounted.destroy()
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

  it('does not rerun workspace, admin access, or runtime bootstrap on same-workspace menu navigation', async () => {
    const counts = createClientCallCounters()
    configureWorkspaceClient(client => ({
      ...client,
      workspace: {
        ...client.workspace,
        async get() {
          counts.workspaceGet += 1
          return await client.workspace.get()
        },
        async getOverview() {
          counts.workspaceOverview += 1
          return await client.workspace.getOverview()
        },
      },
      projects: {
        ...client.projects,
        async list() {
          counts.projectsList += 1
          return await client.projects.list()
        },
      },
      accessControl: {
        ...client.accessControl,
        async listUsers() {
          counts.accessUsers += 1
          return await client.accessControl.listUsers()
        },
        async listRoles() {
          counts.accessRoles += 1
          return await client.accessControl.listRoles()
        },
        async listPermissionDefinitions() {
          counts.accessPermissions += 1
          return await client.accessControl.listPermissionDefinitions()
        },
      },
      runtime: {
        ...client.runtime,
        async bootstrap() {
          counts.runtimeBootstrap += 1
          return await client.runtime.bootstrap()
        },
      },
    }))

    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="workspace-overview-view"]') !== null)
    await new Promise(resolve => window.setTimeout(resolve, 120))
    await nextTick()

    const baseline = { ...counts }

    await router.push('/workspaces/ws-local/console/projects')
    await waitFor(() => router.currentRoute.value.name === 'workspace-console-projects')

    await router.push('/workspaces/ws-local/projects/proj-redesign/resources')
    await waitFor(() => router.currentRoute.value.name === 'project-resources')

    expect(counts.workspaceGet - baseline.workspaceGet).toBe(0)
    expect(counts.projectsList - baseline.projectsList).toBe(0)
    expect(counts.workspaceOverview - baseline.workspaceOverview).toBe(0)
    expect(counts.accessUsers - baseline.accessUsers).toBe(0)
    expect(counts.accessRoles - baseline.accessRoles).toBe(0)
    expect(counts.accessPermissions - baseline.accessPermissions).toBe(0)
    expect(counts.runtimeBootstrap - baseline.runtimeBootstrap).toBe(0)

    mounted.destroy()
  })

  it('updates theme and locale preferences through the topbar controls', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    const shell = useShellStore()
    await waitFor(() => mounted.container.querySelector('[data-testid="topbar-theme-toggle"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-toggle"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="ui-popover-content"]') !== null)

    const selectedThemeButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t(`topbar.themeModes.${shell.preferences.theme}`))))
    const themeLightButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('topbar.themeModes.light'))))

    expect(selectedThemeButton?.className).toContain('border-border-strong')
    expect(selectedThemeButton?.className).toContain('bg-accent')
    expect(selectedThemeButton?.className).not.toContain('shadow-xs')
    expect(themeLightButton?.className).toContain('hover:bg-subtle')
    expect(themeLightButton?.className).not.toContain('hover:bg-accent')
    themeLightButton?.click()
    await waitFor(() => shell.preferences.theme === 'light')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-locale-toggle"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="ui-popover-content"]') !== null)

    const selectedLocaleButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t(`topbar.localeModes.${shell.preferences.locale}`))))
    const localeEnglishButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('topbar.localeModes.en-US'))))

    expect(selectedLocaleButton?.className).toContain('border-border-strong')
    expect(selectedLocaleButton?.className).toContain('bg-accent')
    expect(selectedLocaleButton?.className).not.toContain('shadow-xs')
    expect(localeEnglishButton?.className).toContain('hover:bg-subtle')
    expect(localeEnglishButton?.className).not.toContain('hover:bg-accent')
    localeEnglishButton?.click()
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
    expect(document.body.querySelector('[data-testid="sidebar-workspace-nav-workspace-console"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="sidebar-connect-workspace-trigger"]')).not.toBeNull()
    expect(document.body.textContent).toContain(String(i18n.global.t('sidebar.workspaceMenu.title')))

    mounted.destroy()
  })

  it('renders the connect workspace dialog as an integrated shell overlay with shared error callout styling', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-connect-workspace-trigger"]') !== null)

    document.body.querySelector<HTMLButtonElement>('[data-testid="sidebar-connect-workspace-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="connect-workspace-dialog"]') !== null)

    const auth = useAuthStore()
    auth.error = 'Invalid workspace credentials'
    await nextTick()

    const dialog = document.body.querySelector<HTMLElement>('[data-testid="connect-workspace-dialog"]')
    const form = document.body.querySelector<HTMLElement>('[data-testid="connect-workspace-form"]')
    const intro = document.body.querySelector<HTMLElement>('[data-testid="connect-workspace-intro"]')
    const errorCallout = document.body.querySelector<HTMLElement>('[data-testid="connect-workspace-error"]')
    const actions = form?.lastElementChild as HTMLElement | null

    expect(dialog).not.toBeNull()
    expect(dialog?.className).toContain('overflow-hidden')
    expect(dialog?.className).toContain('p-0')

    expect(intro).not.toBeNull()
    expect(intro?.className).toContain('border-b')
    expect(intro?.className).toContain('border-border')
    expect(intro?.className).toContain('bg-subtle')
    expect(intro?.className).not.toContain('rounded-[var(--radius-l)]')
    expect(intro?.className).not.toContain('shadow-xs')

    expect(actions).not.toBeNull()
    expect(actions?.className).toContain('border-t')
    expect(actions?.className).toContain('bg-subtle')

    expect(errorCallout).not.toBeNull()
    expect(errorCallout?.className).toContain('bg-[color-mix(in_srgb,var(--color-status-error-soft)_72%,var(--surface)_28%)]')
    expect(errorCallout?.className).toContain('border-[color-mix(in_srgb,var(--color-status-error)_18%,var(--border))]')
    expect(errorCallout?.textContent).toContain('Invalid workspace credentials')
    expect(errorCallout?.innerHTML).not.toContain('text-destructive')
    expect(errorCallout?.className).not.toContain('border-destructive/20')
    expect(errorCallout?.className).not.toContain('bg-destructive/5')
    expect(errorCallout?.className).not.toContain('text-destructive')

    mounted.destroy()
  })

  it('navigates to the first console workspace surface from the footer workspace menu', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-workspace-menu-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-workspace-nav-workspace-console"]') !== null)

    document.body
      .querySelector<HTMLAnchorElement>('[data-testid="sidebar-workspace-nav-workspace-console"]')
      ?.click()

    await waitFor(() => router.currentRoute.value.name === 'workspace-console-settings')
    expect(router.currentRoute.value.name).toBe('workspace-console-settings')

    mounted.destroy()
  })

  it('creates a project from the sidebar quick-create popover and lands on project settings', async () => {
    vi.spyOn(tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> }, 'pickResourceDirectory')
      .mockResolvedValue('/workspace/projects/strategy-launch/resources')

    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-create-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-create-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-project-create-popover"]') !== null)

    const nameInput = document.body.querySelector<HTMLInputElement>('[data-testid="sidebar-project-create-name-input"]')
    const descriptionInput = document.body.querySelector<HTMLTextAreaElement>('[data-testid="sidebar-project-create-description-input"]')
    const presetSelect = document.body.querySelector<HTMLSelectElement>('[data-testid="sidebar-project-create-preset-select"]')
    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()
    expect(presetSelect).not.toBeNull()

    presetSelect!.value = 'documentation'
    presetSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    nameInput!.value = 'Strategy Launch'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Launch checklist and delivery alignment.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))
    document.body.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-create-resource-directory-pick"]')?.click()
    await waitFor(() =>
      document.body.querySelector<HTMLInputElement>('[data-testid="sidebar-project-create-resource-directory-path"]')?.value
        === '/workspace/projects/strategy-launch/resources',
    )

    await waitFor(() => {
      const submit = document.body.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-create-submit"]')
      return Boolean(submit && !submit.disabled)
    })

    document.body.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-create-submit"]')?.click()

    await waitFor(
      () =>
        router.currentRoute.value.name === 'project-settings'
        && String(router.currentRoute.value.params.projectId).includes('strategy-launch'),
      4000,
    )

    expect(mounted.container.querySelector('[data-testid="sidebar-project-proj-strategy-launch"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Strategy Launch')
    const created = workspaceStore.projects.find(project => project.name === 'Strategy Launch')
    expect(created?.assignments).toBeUndefined()
    expect(created?.presetCode).toBe('documentation')
    expect(workspaceStore.getProjectSettings(created?.id ?? '').models?.allowedConfiguredModelIds).toHaveLength(1)

    mounted.destroy()
  })

  it('renders the sidebar quick-create popover as an integrated shell instead of a flat floating form', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-create-trigger"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-create-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-project-create-popover"]') !== null)

    const popover = document.body.querySelector<HTMLElement>('[data-testid="sidebar-project-create-popover"]')
    const intro = document.body.querySelector<HTMLElement>('[data-testid="sidebar-project-create-intro"]')
    const actions = document.body.querySelector<HTMLElement>('[data-testid="sidebar-project-create-actions"]')

    expect(popover).not.toBeNull()
    expect(popover?.className).not.toContain('shadow-xs')

    expect(intro).not.toBeNull()
    expect(intro?.className).toContain('border-b')
    expect(intro?.className).toContain('bg-subtle')

    expect(actions).not.toBeNull()
    expect(actions?.className).toContain('border-t')
    expect(actions?.className).toContain('bg-subtle')

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

  it('shows the project settings entry for governance reviewers who are not project members', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-operator'
        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        ;(project as any).ownerUserId = 'user-owner'
        ;(project as any).memberUserIds = ['user-owner']
      },
    })

    await router.push('/workspaces/ws-local/projects/proj-redesign/settings')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() =>
      router.currentRoute.value.name === 'project-settings'
      && mounted.container.querySelector('[data-testid="sidebar-project-module-proj-redesign-settings"]') !== null,
    )

    expect(mounted.container.querySelector('[data-testid="sidebar-project-module-proj-redesign-settings"]')).not.toBeNull()

    mounted.destroy()
  })

  it('routes governance reviewers to project settings when they open a non-member project from the sidebar', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-operator'

        const redesign = state.projects.find(item => item.id === 'proj-redesign')
        const governance = state.projects.find(item => item.id === 'proj-governance')
        if (!redesign || !governance) {
          throw new Error('Expected fixture projects')
        }

        ;(redesign as any).ownerUserId = 'user-owner'
        ;(redesign as any).memberUserIds = ['user-owner']
        ;(governance as any).memberUserIds = ['user-operator']
      },
    })

    await router.push('/workspaces/ws-local/overview?project=proj-governance')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-summary-proj-redesign"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-project-summary-proj-redesign"]')?.click()

    await waitFor(() =>
      router.currentRoute.value.name === 'project-settings'
      && String(router.currentRoute.value.params.projectId) === 'proj-redesign',
    )

    expect(router.currentRoute.value.name).toBe('project-settings')
    expect(router.currentRoute.value.params.projectId).toBe('proj-redesign')

    mounted.destroy()
  })

  it('keeps sidebar project modules and workspace menu rows on neutral hover while preserving brand-soft active rows', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/settings')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-module-proj-redesign-settings"]') !== null)

    const settingsLink = mounted.container.querySelector<HTMLElement>('[data-testid="sidebar-project-module-proj-redesign-settings"]')
    const deliverablesLink = mounted.container.querySelector<HTMLElement>('[data-testid="sidebar-project-module-proj-redesign-deliverables"]')

    expect(settingsLink?.className).toContain('border-border-strong')
    expect(settingsLink?.className).toContain('bg-accent')
    expect(settingsLink?.className).not.toContain('shadow-xs')
    expect(settingsLink?.className).not.toContain('hover:bg-accent')
    expect(deliverablesLink?.className).toContain('hover:bg-subtle')
    expect(deliverablesLink?.className).not.toContain('hover:bg-accent')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-menu-trigger"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="sidebar-workspace-nav-workspace-console"]') !== null)

    const workspaceConsoleLink = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-nav-workspace-console"]')
    const enterpriseConnection = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-item-conn-enterprise"]')
    const localConnection = document.body.querySelector<HTMLElement>('[data-testid="sidebar-workspace-menu-item-conn-local"]')

    expect(workspaceConsoleLink?.className).toContain('hover:bg-subtle')
    expect(workspaceConsoleLink?.className).not.toContain('hover:bg-accent')
    expect(enterpriseConnection?.className).toContain('hover:bg-subtle')
    expect(enterpriseConnection?.className).not.toContain('hover:bg-accent')
    expect(localConnection?.className).toContain('bg-accent')
    expect(localConnection?.className).toContain('border-border-strong')
    expect(localConnection?.className).not.toContain('shadow-xs')

    mounted.destroy()
  })

  it('shows the project tasks menu item in each project module list', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-proj-redesign"]') !== null)

    const tasksLink = mounted.container.querySelector<HTMLAnchorElement>('[data-testid="sidebar-project-module-proj-redesign-tasks"]')
    expect(tasksLink).not.toBeNull()

    tasksLink?.click()

    await waitFor(() => router.currentRoute.value.name === 'project-tasks')
    expect(router.currentRoute.value.name).toBe('project-tasks')

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

  it('keeps sidebar project groups integrated with the rail instead of floating card styling', async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()

    const mounted = mountApp()
    await waitFor(() => mounted.container.querySelector('[data-testid="sidebar-project-proj-redesign"]') !== null)

    const expandedProject = mounted.container.querySelector<HTMLElement>('[data-testid="sidebar-project-proj-redesign"]')
    const collapsedProject = mounted.container.querySelector<HTMLElement>('[data-testid="sidebar-project-proj-governance"]')
    const collapsedSummary = mounted.container.querySelector<HTMLElement>('[data-testid="sidebar-project-summary-proj-governance"]')

    expect(expandedProject?.className).not.toContain('shadow-xs')
    expect(collapsedProject?.className).not.toContain('shadow-xs')
    expect(collapsedSummary?.className).toContain('hover:bg-subtle')
    expect(collapsedSummary?.className).not.toContain('group-hover:-translate-x-1')

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
