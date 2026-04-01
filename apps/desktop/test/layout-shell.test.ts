// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, h, nextTick } from 'vue'

import i18n from '@/plugins/i18n'
import { router } from '@/router'
import WorkbenchLayout from '@/layouts/WorkbenchLayout.vue'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

function mountLayout() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  const app = createApp({
    render: () => h(WorkbenchLayout, null, {
      default: () => h('div', { 'data-testid': 'workbench-slot' }, 'slot'),
    }),
  })

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

async function flushUi() {
  await nextTick()
  await new Promise((resolve) => window.setTimeout(resolve, 0))
  await nextTick()
}

describe('Workbench shell layout', () => {
  beforeEach(async () => {
    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await router.isReady()
    document.body.innerHTML = ''
    vi.spyOn(window, 'confirm').mockReturnValue(true)
  })

  it('renders the topbar chrome with brand, search trigger, and function menu', async () => {
    const mounted = mountLayout()

    await flushUi()

    expect(mounted.container.querySelector('[data-testid="workbench-topbar"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="brand-title"]')?.textContent).toContain('Octopus')
    expect(mounted.container.querySelector('[data-testid="global-search-trigger"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="topbar-menu"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="topbar-left-sidebar-toggle"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="topbar-locale-toggle"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="topbar-actions"]')).not.toBeNull()

    mounted.destroy()
  })

  it('opens topbar menus, updates theme and locale preferences, and manages workspaces', async () => {
    const mounted = mountLayout()
    const shell = useShellStore()
    const workbench = useWorkbenchStore()

    await flushUi()

    const themeButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-toggle"]')
    expect(themeButton).not.toBeNull()

    themeButton?.click()
    await flushUi()

    expect(mounted.container.querySelector('[data-testid="topbar-theme-menu"]')).not.toBeNull()

    const lightThemeOption = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-option-light"]')
    expect(lightThemeOption).not.toBeNull()
    lightThemeOption?.click()
    await flushUi()

    expect(shell.preferences.theme).toBe('light')

    themeButton?.click()
    await flushUi()

    const darkThemeOption = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-option-dark"]')
    expect(darkThemeOption).not.toBeNull()
    darkThemeOption?.click()
    await flushUi()

    expect(shell.preferences.theme).toBe('dark')

    themeButton?.click()
    await flushUi()

    const systemThemeOption = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-theme-option-system"]')
    expect(systemThemeOption).not.toBeNull()
    systemThemeOption?.click()
    await flushUi()

    expect(shell.preferences.theme).toBe('system')

    const localeButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-locale-toggle"]')
    expect(localeButton).not.toBeNull()

    localeButton?.click()
    await flushUi()

    expect(mounted.container.querySelector('[data-testid="topbar-locale-menu"]')).not.toBeNull()

    const englishLocaleOption = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-locale-option-en-US"]')
    expect(englishLocaleOption).not.toBeNull()
    englishLocaleOption?.click()
    await flushUi()

    expect(shell.preferences.locale).toBe('en-US')

    localeButton?.click()
    await flushUi()

    const chineseLocaleOption = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-locale-option-zh-CN"]')
    expect(chineseLocaleOption).not.toBeNull()
    chineseLocaleOption?.click()
    await flushUi()

    expect(shell.preferences.locale).toBe('zh-CN')

    const profileTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-profile-trigger"]')
    expect(profileTrigger).not.toBeNull()

    profileTrigger?.click()
    await flushUi()

    expect(mounted.container.querySelector('[data-testid="topbar-account-menu"]')).not.toBeNull()

    const workspaceButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="workspace-switch-ws-enterprise"]')
    workspaceButton?.click()
    await flushUi()

    expect(workbench.currentWorkspaceId).toBe('ws-enterprise')
    expect(workbench.currentProjectId).toBe('proj-launch')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-enterprise')

    const reopenProfileTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-profile-trigger"]')
    reopenProfileTrigger?.click()
    await flushUi()

    const addWorkspaceButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="add-workspace-button"]')
    addWorkspaceButton?.click()
    await flushUi()

    expect(workbench.workspaces.length).toBe(3)
    expect(workbench.currentWorkspaceId).toBe('ws-mock-3')
    expect(router.currentRoute.value.params.workspaceId).toBe('ws-mock-3')

    const reopenAfterAdd = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-profile-trigger"]')
    reopenAfterAdd?.click()
    await flushUi()

    const removeEnterprise = mounted.container.querySelector<HTMLButtonElement>('[data-testid="remove-workspace-ws-enterprise"]')
    expect(removeEnterprise).not.toBeNull()
    removeEnterprise?.click()
    await flushUi()

    expect(workbench.workspaces.some((workspace) => workspace.id === 'ws-enterprise')).toBe(false)

    const removeActiveWorkspace = mounted.container.querySelector<HTMLButtonElement>('[data-testid="remove-workspace-ws-mock-3"]')
    expect(removeActiveWorkspace).not.toBeNull()
    removeActiveWorkspace?.click()
    await flushUi()

    expect(workbench.currentWorkspaceId).toBe('ws-local')
    expect(workbench.workspaces).toHaveLength(1)

    const accountMenu = mounted.container.querySelector('[data-testid="topbar-account-menu"]')
    if (accountMenu) {
      const closeLastWorkspaceMenu = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-profile-trigger"]')
      closeLastWorkspaceMenu?.click()
      await flushUi()
    }

    const reopenLastWorkspaceMenu = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-profile-trigger"]')
    reopenLastWorkspaceMenu?.click()
    await flushUi()

    const removeLastWorkspace = mounted.container.querySelector<HTMLButtonElement>('[data-testid="remove-workspace-ws-local"]')
    expect(removeLastWorkspace?.hasAttribute('disabled')).toBe(true)

    mounted.destroy()
  })

  it('keeps the topbar settings button active while the settings page is open', async () => {
    const mounted = mountLayout()

    await flushUi()

    const settingsButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="topbar-settings-button"]')
    expect(settingsButton).not.toBeNull()
    expect(settingsButton?.classList.contains('active')).toBe(false)

    settingsButton?.click()
    await flushUi()

    expect(router.currentRoute.value.name).toBe('settings')
    expect(settingsButton?.classList.contains('active')).toBe(true)

    mounted.destroy()
  })

  it('keeps the global layout free of conversation detail chrome', async () => {
    const mounted = mountLayout()
    const shell = useShellStore()

    shell.toggleLeftSidebar()
    shell.toggleRightSidebar()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="sidebar-rail"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="context-rail"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-detail-panel"]')).toBeNull()

    mounted.destroy()
  })

  it('renders the restructured project modules and exposes workspace navigation through a dropdown in the left sidebar', async () => {
    const mounted = mountLayout()
    const workbench = useWorkbenchStore()

    await flushUi()

    const projectTree = mounted.container.querySelector('[data-testid="sidebar-project-tree"]')
    expect(projectTree).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-project-tree-scroll"]')).not.toBeNull()

    const projectButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-node-proj-governance"]')
    expect(projectButton).not.toBeNull()
    projectButton?.click()
    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(workbench.currentProjectId).toBe('proj-governance')
    expect(router.currentRoute.value.name).toBe('project-dashboard')
    expect(router.currentRoute.value.params.projectId).toBe('proj-governance')
    expect(mounted.container.querySelector('[data-testid="project-module-proj-governance-dashboard"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-module-proj-governance-agents"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-module-proj-governance-resources"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-module-proj-governance-knowledge"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-module-proj-governance-trace"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-module-proj-governance-conversations"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="add-project-button"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-bottom-navigation"]')).not.toBeNull()

    const workspaceTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="sidebar-workspace-trigger"]')
    expect(workspaceTrigger).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-workspace-menu"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-nav-workspace-overview"]')).toBeNull()

    workspaceTrigger?.click()
    await flushUi()

    expect(mounted.container.querySelector('[data-testid="sidebar-workspace-menu"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-nav-workspace-overview"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-nav-knowledge"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-nav-agents"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-nav-models"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-nav-tools"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-nav-automations"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="sidebar-nav-user-center"]')).not.toBeNull()

    const userCenterLink = mounted.container.querySelector<HTMLAnchorElement>('[data-testid="sidebar-nav-user-center"]')
    userCenterLink?.click()
    await flushUi()

    expect(router.currentRoute.value.name).toBe('user-center-profile')
    expect(mounted.container.querySelector('[data-testid="sidebar-workspace-menu"]')).toBeNull()

    mounted.destroy()
  })

  it('keeps the expanded project free of delete affordance and still removes a collapsed project', async () => {
    const mounted = mountLayout()
    const workbench = useWorkbenchStore()

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="remove-project-proj-redesign"]')).toBeNull()

    const deleteButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="remove-project-proj-governance"]')
    expect(deleteButton).not.toBeNull()

    deleteButton?.click()
    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(workbench.workspaceProjects.some((item) => item.id === 'proj-governance')).toBe(false)
    expect(workbench.currentProjectId).toBe('proj-redesign')
    expect(router.currentRoute.value.name).toBe('project-dashboard')
    expect(router.currentRoute.value.params.projectId).toBe('proj-redesign')

    mounted.destroy()
  })

  it('opens a create-project dialog and routes to the new project dashboard after naming it', async () => {
    const mounted = mountLayout()
    const workbench = useWorkbenchStore()

    await nextTick()

    const beforeCount = workbench.workspaceProjects.length
    const addProjectButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="add-project-button"]')
    expect(addProjectButton).not.toBeNull()

    addProjectButton?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="project-create-dialog"]')).not.toBeNull()

    const input = mounted.container.querySelector<HTMLInputElement>('[data-testid="project-create-input"]')
    expect(input).not.toBeNull()
    if (input) {
      input.value = 'Project Atlas'
      input.dispatchEvent(new Event('input'))
    }

    await nextTick()

    const confirmButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-create-confirm"]')
    expect(confirmButton?.disabled).toBe(false)
    confirmButton?.click()

    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(workbench.workspaceProjects.length).toBe(beforeCount + 1)
    expect(workbench.projectConversations).toHaveLength(1)
    expect(workbench.activeProject?.name).toBe('Project Atlas')
    expect(router.currentRoute.value.name).toBe('project-dashboard')
    expect(router.currentRoute.value.params.projectId).toBe(workbench.currentProjectId)

    mounted.destroy()
  })
})
