// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { createAppRouter } from '@/router'
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

function mountApp(pinia = createPinia()) {
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)
  const router = createAppRouter()

  const app = createApp(App)
  app.use(pinia)
  app.use(i18n)
  app.use(router)
  app.mount(container)

  return {
    app,
    container,
    router,
    async destroy() {
      app.unmount()
      await nextTick()
      await new Promise(resolve => window.setTimeout(resolve, 0))
      container.remove()
      await nextTick()
    },
  }
}

async function mountRoutedApp(path: string) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const mounted = mountApp(pinia)
  await mounted.router.push(path)
  await mounted.router.isReady()
  await flushUi()
  return mounted
}

async function flushUi(timeoutMs = 1500) {
  const startedAt = Date.now()
  let previousMarkup = ''
  let stableTicks = 0

  while (Date.now() - startedAt < timeoutMs) {
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))

    const nextMarkup = document.body.innerHTML
    if (nextMarkup === previousMarkup) {
      stableTicks += 1
      if (stableTicks >= 3) {
        return
      }
    } else {
      previousMarkup = nextMarkup
      stableTicks = 0
    }
  }
}

async function waitForSelector(container: HTMLElement, selector: string, timeoutMs = 3000) {
  const startedAt = Date.now()
  while (!container.querySelector(selector)) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for selector: ${selector}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

async function waitForText(container: HTMLElement, value: string, timeoutMs = 3000) {
  const startedAt = Date.now()
  while (!(container.textContent?.includes(value) ?? false)) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for text: ${value}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

function clickSelector(container: HTMLElement, selector: string) {
  const element = container.querySelector<HTMLElement>(selector)
  if (!element) {
    throw new Error(`Missing element for click: ${selector}`)
  }
  element.click()
}

function updateInput(container: HTMLElement, selector: string, value: string) {
  const input = container.querySelector<HTMLInputElement>(selector)
  if (!input) {
    throw new Error(`Missing input: ${selector}`)
  }
  input.value = value
  input.dispatchEvent(new Event('input', { bubbles: true }))
  input.dispatchEvent(new Event('change', { bubbles: true }))
}

function updateSelect(container: HTMLElement, selector: string, value: string) {
  const select = container.querySelector<HTMLSelectElement>(selector)
  if (!select) {
    throw new Error(`Missing select: ${selector}`)
  }
  select.value = value
  select.dispatchEvent(new Event('change', { bubbles: true }))
}

describe('workspace access centers', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    i18n.global.locale.value = 'zh-CN'
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders fixed progressive access sections instead of raw admin menu tabs', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/members')

    await waitForSelector(mounted.container, '[data-testid="access-control-sections"]')
    await waitForSelector(mounted.container, '[data-testid="access-members-view"]')

    expect(mounted.container.textContent).toContain('成员')
    expect(mounted.container.textContent).toContain('访问')
    expect(mounted.container.textContent).toContain('治理')
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-members"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-access"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-governance"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-access-control-users"]')).toBeNull()

    await mounted.destroy()
  })

  it('keeps the sidebar access-control entry visible when menu policy hides the root menu but a section stays allowed', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.menuPolicies = [{
          menuId: 'menu-workspace-access-control',
          enabled: true,
          order: 100,
          group: 'Security',
          visibility: 'hidden',
        }]
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/overview?project=proj-redesign')

    await waitForSelector(mounted.container, '[data-testid="sidebar-workspace-menu-trigger"]')
    clickSelector(mounted.container, '[data-testid="sidebar-workspace-menu-trigger"]')
    await waitForSelector(document.body, '[data-testid="sidebar-workspace-navigation-menu"]')

    expect(document.body.querySelector('[data-testid="sidebar-workspace-nav-workspace-access-control"]')).not.toBeNull()

    await mounted.destroy()
  })

  it('hides the sidebar access-control entry when all sections are denied even if the root menu is forced visible', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-operator'
        state.roles = state.roles.map(role => role.id === 'role-operator'
          ? {
              ...role,
              permissionCodes: ['workspace.overview.read'],
            }
          : role)
        state.menuPolicies = [{
          menuId: 'menu-workspace-access-control',
          enabled: true,
          order: 100,
          group: 'Security',
          visibility: 'visible',
        }]
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/overview?project=proj-redesign')

    await waitForSelector(mounted.container, '[data-testid="sidebar-workspace-menu-trigger"]')
    clickSelector(mounted.container, '[data-testid="sidebar-workspace-menu-trigger"]')
    await waitForSelector(document.body, '[data-testid="sidebar-workspace-navigation-menu"]')

    expect(document.body.querySelector('[data-testid="sidebar-workspace-nav-workspace-access-control"]')).toBeNull()

    await mounted.destroy()
  })

  it('sends personal workspaces to the access surface and keeps governance low-noise', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        state.currentUserId = 'user-owner'
        state.users = state.users.filter(user => user.id === 'user-owner')
        state.userOrgAssignments = state.userOrgAssignments.filter(assignment => assignment.userId === 'user-owner')
        state.roleBindings = state.roleBindings.filter(binding => binding.subjectId === 'user-owner')
        state.dataPolicies = []
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control')

    await waitForSelector(mounted.container, '[data-testid="access-permissions-view"]')
    expect(mounted.router.currentRoute.value.name).toBe('workspace-access-control-access')

    await mounted.router.push('/workspaces/ws-local/access-control/governance')
    await flushUi()

    await waitForSelector(mounted.container, '[data-testid="access-governance-view"]')
    await waitForText(mounted.container, '还没有需要展开的治理对象')
    expect(mounted.container.querySelector('[data-testid="access-governance-sections"]')).toBeNull()

    await mounted.destroy()
  })

  it('keeps default team workspaces low-noise in governance when they only use basic project access policies', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/governance')

    await waitForSelector(mounted.container, '[data-testid="access-governance-view"]')
    await waitForText(mounted.container, '还没有需要展开的治理对象')
    expect(mounted.container.querySelector('[data-testid="access-governance-sections"]')).toBeNull()

    await mounted.destroy()
  })

  it('lets team workspaces create a member and assign a preset from the members surface', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control')

    await waitForSelector(mounted.container, '[data-testid="access-members-view"]')
    expect(mounted.router.currentRoute.value.name).toBe('workspace-access-control-members')

    clickSelector(mounted.container, '[data-testid="access-members-create-button"]')
    await waitForSelector(mounted.container, '[data-testid="access-members-create-dialog"]')
    updateInput(mounted.container, '[data-testid="access-members-create-username"]', 'qa.lead')
    updateInput(mounted.container, '[data-testid="access-members-create-display-name"]', 'QA Lead')
    clickSelector(mounted.container, '[data-testid="access-members-create-submit"]')

    await waitForText(mounted.container, 'QA Lead')

    clickSelector(mounted.container, '[data-testid="access-members-assign-preset-user-operator"]')
    await waitForSelector(mounted.container, '[data-testid="access-members-assign-dialog"]')
    updateSelect(mounted.container, '[data-testid="access-members-assign-preset-select"]', 'owner')
    clickSelector(mounted.container, '[data-testid="access-members-assign-submit"]')

    await waitForText(mounted.container, '已将 所有者 应用给 Workspace Operator')
    await mounted.destroy()
  })

  it('supports direct preset assignment from the access surface', async () => {
    i18n.global.locale.value = 'en-US'
    installWorkspaceApiFixture({ locale: 'en-US' })

    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/access')

    await waitForSelector(mounted.container, '[data-testid="access-permissions-view"]')
    await waitForText(mounted.container, 'Owner')
    updateSelect(mounted.container, '[data-testid="access-assign-member-select"]', 'user-operator')
    clickSelector(mounted.container, '[data-testid="access-preset-card-owner"]')
    clickSelector(mounted.container, '[data-testid="access-assign-submit"]')

    await waitForText(mounted.container, 'Applied Owner to Workspace Operator')
    await mounted.destroy()
  })

  it('keeps enterprise governance surfaces available when the workspace carries advanced governance state', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-enterprise') {
          return
        }

        state.menuPolicies = [{
          menuId: 'menu-workspace-console-projects',
          enabled: true,
          order: 110,
          group: 'Operations',
          visibility: 'visible',
        }]
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-enterprise/access-control/governance')

    await waitForSelector(mounted.container, '[data-testid="access-governance-view"]')
    await waitForSelector(mounted.container, '[data-testid="access-governance-sections"]')
    expect(mounted.container.textContent).toContain('组织结构')
    expect(mounted.container.textContent).toContain('角色')
    expect(mounted.container.textContent).toContain('策略')
    expect(mounted.container.textContent).toContain('菜单')
    expect(mounted.container.textContent).toContain('资源')
    expect(mounted.container.textContent).toContain('会话')

    await mounted.destroy()
  })
})
