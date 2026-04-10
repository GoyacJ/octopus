// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
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

async function mountRoutedApp(path: string) {
  const pinia = createPinia()
  setActivePinia(pinia)
  await router.push(path)
  await router.isReady()
  return mountApp(pinia)
}

async function waitForText(container: HTMLElement, value: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!(container.textContent?.includes(value) ?? false)) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for text: ${value}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

async function waitForSelector(container: HTMLElement, selector: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!container.querySelector(selector)) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for selector: ${selector}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

async function findInput(container: HTMLElement, selector: string) {
  await waitForSelector(container, selector)
  const input = container.querySelector(selector)
  if (!(input instanceof HTMLInputElement)) {
    throw new Error(`Expected input for selector: ${selector}`)
  }
  return input
}

async function findSelect(container: HTMLElement, selector: string) {
  await waitForSelector(container, selector)
  const select = container.querySelector(selector)
  if (!(select instanceof HTMLSelectElement)) {
    throw new Error(`Expected select for selector: ${selector}`)
  }
  return select
}

function updateInput(input: HTMLInputElement, value: string) {
  input.value = value
  input.dispatchEvent(new Event('input', { bubbles: true }))
  input.dispatchEvent(new Event('change', { bubbles: true }))
}

function updateSelect(select: HTMLSelectElement, value: string) {
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

  it('renders permission center tabs with RBAC pages only', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/permission-center/users')

    await waitForSelector(mounted.container, '[data-testid="permission-center-tabs"]')
    await waitForText(mounted.container, 'Lin Zhou')

    expect(mounted.container.querySelector('[data-testid="permission-center-users-shell"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="permission-center-users-inspector"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-permission-center-users"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-permission-center-roles"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-permission-center-permissions"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-permission-center-menus"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('成员管理')
    expect(mounted.container.textContent).toContain('角色管理')
    expect(mounted.container.textContent).toContain('权限管理')
    expect(mounted.container.textContent).toContain('导航管理')
    expect(mounted.container.textContent).not.toContain('基本资料')
    expect(mounted.container.textContent).not.toContain('宠物')

    mounted.destroy()
  })

  it('groups console and permission menus inside the permission center role editor', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/permission-center/roles')

    await waitForText(mounted.container, 'Owner')

    expect(mounted.container.querySelector('[data-testid="permission-center-roles-shell"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="roles-menu-group-console"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="roles-menu-group-permission-center"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="roles-menu-menu-workspace-console-projects"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="roles-menu-menu-workspace-permission-center-users"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('控制台')
    expect(mounted.container.textContent).toContain('权限中心')
    expect(mounted.container.textContent).not.toContain('用户中心')
    expect(mounted.container.querySelector('[data-testid="roles-menu-menu-workspace-personal-center-profile"]')).toBeNull()

    mounted.destroy()
  })

  it('renders personal center profile and pet pages on their own route surface', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/personal-center/profile')

    await waitForSelector(mounted.container, '[data-testid="personal-center-tabs"]')
    await waitForText(mounted.container, 'Octopus Owner')

    expect(mounted.container.querySelector('[data-testid="personal-center-profile-view"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-workspace-personal-center-profile"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-workspace-personal-center-pet"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="profile-access-card"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="profile-access-roles"]')?.textContent).toContain('Owner')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).toContain('项目管理')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).toContain('成员管理')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).not.toContain('基本资料')

    mounted.destroy()
  })

  it('persists personal pet settings through the personal center route', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/personal-center/pet')

    await waitForSelector(mounted.container, '[data-testid="personal-center-pet-view"]')

    updateInput(await findInput(mounted.container, '[data-testid="personal-center-pet-display-name"]'), '章鱼助手')
    updateSelect(await findSelect(mounted.container, '[data-testid="personal-center-pet-model-select"]'), 'anthropic-alt')
    updateSelect(await findSelect(mounted.container, '[data-testid="personal-center-pet-permission-select"]'), 'workspace-write')

    const greetingInput = mounted.container.querySelector('[data-testid="personal-center-pet-greeting-input"]')
    if (!(greetingInput instanceof HTMLTextAreaElement)) {
      throw new Error('Expected greeting textarea')
    }
    greetingInput.value = '欢迎回来，我已经准备好了。'
    greetingInput.dispatchEvent(new Event('input', { bubbles: true }))

    const summaryInput = mounted.container.querySelector('[data-testid="personal-center-pet-summary-input"]')
    if (!(summaryInput instanceof HTMLTextAreaElement)) {
      throw new Error('Expected summary textarea')
    }
    summaryInput.value = '负责陪伴与执行工作区任务的章鱼助手。'
    summaryInput.dispatchEvent(new Event('input', { bubbles: true }))

    const saveButton = mounted.container.querySelector('[data-testid="personal-center-pet-save"]')
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error('Expected pet save button')
    }
    saveButton.click()

    await waitForText(mounted.container, '"permissionMode": "workspace-write"')
    expect(mounted.container.textContent).toContain('章鱼助手')
    expect(mounted.container.textContent).toContain('欢迎回来，我已经准备好了。')

    mounted.destroy()
  })

  it('renders the workspace console as a tabbed shell with the first business surface active', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/console')

    await waitForSelector(mounted.container, '[data-testid="workspace-console-view"]')
    await waitForSelector(mounted.container, '[data-testid="workspace-console-tabs"]')
    await waitForText(mounted.container, '项目管理')

    expect(mounted.container.textContent).toContain('控制台')
    expect(mounted.container.textContent).toContain('项目管理')
    expect(mounted.container.textContent).toContain('知识')
    expect(mounted.container.textContent).toContain('资源')
    expect(mounted.container.textContent).toContain('数字员工')
    expect(mounted.container.textContent).toContain('模型')
    expect(mounted.container.textContent).toContain('工具')
    expect(mounted.container.querySelector('[data-testid="workspace-console-nav"]')).toBeNull()

    mounted.destroy()
  })
})
