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
  await nextTick()
  await new Promise(resolve => window.setTimeout(resolve, 0))
  await new Promise(resolve => window.setTimeout(resolve, 0))
  return mounted
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

async function waitForSelectorToDisappear(container: HTMLElement, selector: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (container.querySelector(selector)) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for selector to disappear: ${selector}`)
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

async function waitForAttributeValue(
  container: HTMLElement,
  selector: string,
  attribute: string,
  value: string,
  timeoutMs = 2000,
) {
  const startedAt = Date.now()
  while (container.querySelector(selector)?.getAttribute(attribute) !== value) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for ${attribute}=${value} on selector: ${selector}`)
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

function findPageHeaderDescription(container: HTMLElement, title: string) {
  const heading = Array.from(container.querySelectorAll('h1, h2')).find(node => node.textContent?.trim() === title)
  if (!heading?.parentElement) {
    throw new Error(`Expected page heading: ${title}`)
  }
  const paragraphs = heading.parentElement.querySelectorAll('p')
  return paragraphs.item(paragraphs.length - 1)?.textContent?.trim() ?? ''
}

describe('workspace access centers', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    i18n.global.locale.value = 'zh-CN'
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders access control tabs with enterprise access pages only', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/users')

    await waitForSelector(mounted.container, '[data-testid="access-control-tabs"]')
    await waitForText(mounted.container, 'Lin Zhou')

    expect(mounted.container.querySelector('[data-testid="access-control-users-shell"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="access-control-users-toolbar"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="access-control-user-create-button"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('请选择用户')
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-access-control-users"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-access-control-org"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-access-control-roles"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-access-control-policies"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-access-control-menus"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-access-control-resources"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-access-control-sessions"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('用户管理')
    expect(mounted.container.textContent).toContain('组织管理')
    expect(mounted.container.textContent).toContain('角色管理')
    expect(mounted.container.textContent).toContain('权限与策略')
    expect(mounted.container.textContent).toContain('菜单管理')
    expect(mounted.container.textContent).toContain('资源与授权')
    expect(mounted.container.textContent).toContain('会话与审计')
    expect(mounted.container.textContent).not.toContain('基本资料')
    expect(mounted.container.textContent).not.toContain('宠物')

    await mounted.destroy()
  })

  it('renders localized access control header copy and paginates long user lists', async () => {
    i18n.global.locale.value = 'en-US'
    installWorkspaceApiFixture({ extraAccessUsersCount: 10, locale: 'en-US' })

    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/users')

    await waitForSelector(mounted.container, '[data-testid="access-control-tabs"]')
    await waitForText(mounted.container, 'Access User 01')

    expect(findPageHeaderDescription(mounted.container, 'Access Control')).toBe(
      'Review workspace identities, access bindings, policies, and audit posture from one workbench.',
    )
    expect(mounted.container.textContent).toContain('Create user')
    expect(mounted.container.textContent).toContain('Select a user')
    expect(mounted.container.querySelector('[data-testid="ui-pagination"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('1 / 2')

    await mounted.destroy()
  })

  it('does not render password setup status copy in the users workbench', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/users')

    await waitForSelector(mounted.container, '[data-testid="access-control-user-record-user-owner"]')

    expect(mounted.container.textContent).not.toContain('已设置')
    expect(mounted.container.textContent).not.toContain('需要重置')
    expect(mounted.container.textContent).not.toContain('临时密码')

    await mounted.destroy()
  })

  it('uses pagination sections without hard divider lines in access control pages', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/users')

    await waitForSelector(mounted.container, '[data-testid="ui-pagination"]')

    expect(mounted.container.innerHTML).not.toContain('border-t border-border/70')

    await mounted.destroy()
  })

  it('renders access control role and policy projections on dedicated routes', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/roles')

    await waitForText(mounted.container, 'Owner')

    expect(mounted.container.querySelector('[data-testid="access-control-roles-shell"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="access-control-roles-toolbar"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="access-control-role-create-button"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Owner')
    expect(mounted.container.textContent).toContain('Operator')
    expect(mounted.container.textContent).toContain('请选择角色')

    await mounted.destroy()

    const policiesMounted = await mountRoutedApp('/workspaces/ws-local/access-control/policies')
    await waitForSelector(policiesMounted.container, '[data-testid="access-control-policies-shell"]')
    const policySearchInput = await findInput(
      policiesMounted.container,
      'input[placeholder*="权限"], input[placeholder*="permission"], input[placeholder*="capability"]',
    )
    updateInput(policySearchInput, 'tool.mcp.invoke')
    await waitForText(policiesMounted.container, 'tool.mcp.invoke')
    expect(policiesMounted.container.querySelector('[data-testid="access-control-policies-section-tabs"]')).not.toBeNull()
    expect(policiesMounted.container.textContent).toContain('权限目录')
    expect(policiesMounted.container.textContent).toContain('tool.mcp.invoke')
    expect(policiesMounted.container.textContent).toContain('请选择权限')

    await policiesMounted.destroy()
  })

  it('renders audit logs inside the sessions and audit surface without legacy todo copy', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/sessions')

    await waitForSelector(mounted.container, '[data-testid="access-control-sessions-shell"]')

    expect(mounted.container.textContent).toContain('会话管理')
    expect(mounted.container.querySelector('[data-testid="access-control-sessions-toolbar"]')).not.toBeNull()
    expect(mounted.container.textContent).not.toContain('审计日志下一步补齐')

    const auditTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-audit"]')
    if (!(auditTab instanceof HTMLButtonElement)) {
      throw new Error('Expected audit tab button')
    }
    auditTab.click()

    await waitForText(mounted.container, '审计日志')

    await mounted.destroy()
  })

  it('renders precise protected tool resource types in the resource authorization view', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/resources')

    await waitForSelector(mounted.container, '[data-testid="access-control-resources-shell"]')
    const searchInput = await findInput(mounted.container, 'input[placeholder*="资源"], input[placeholder*="Search resource"]')
    updateInput(searchInput, 'tool.skill')
    await waitForText(mounted.container, 'tool.skill')
    updateInput(searchInput, 'tool.builtin')
    await waitForText(mounted.container, 'tool.builtin')
    updateInput(searchInput, 'tool.mcp')
    await waitForText(mounted.container, 'tool.mcp')

    expect(mounted.container.querySelector('[data-testid="access-control-resources-toolbar"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('tool.mcp')
    expect(mounted.container.textContent).not.toContain('tool / mcp')

    await mounted.destroy()
  })

  it('creates a user from the access control users page dialog', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/users')

    await waitForSelector(mounted.container, '[data-testid="access-control-user-create-button"]')

    const createButton = mounted.container.querySelector('[data-testid="access-control-user-create-button"]')
    if (!(createButton instanceof HTMLButtonElement)) {
      throw new Error('Expected user create button')
    }
    createButton.click()

    await waitForSelector(document.body, '[data-testid="access-control-user-form-username"]')

    updateInput(await findInput(document.body as unknown as HTMLElement, '[data-testid="access-control-user-form-username"]'), 'new-user')
    updateInput(await findInput(document.body as unknown as HTMLElement, '[data-testid="access-control-user-form-display-name"]'), 'New User')
    updateInput(await findInput(document.body as unknown as HTMLElement, '[data-testid="access-control-user-form-password"]'), 'password123')
    updateInput(await findInput(document.body as unknown as HTMLElement, '[data-testid="access-control-user-form-confirm-password"]'), 'password123')

    const saveButton = document.body.querySelector('[data-testid="access-control-user-form-save"]')
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error('Expected user save button')
    }
    saveButton.click()

    await waitForSelector(document.body, '[data-testid="ui-toast-viewport"]')
    await waitForText(document.body as unknown as HTMLElement, '用户已保存')
    await waitForText(document.body as unknown as HTMLElement, 'New User（new-user）')
    expect(mounted.container.textContent).toContain('new-user')

    await mounted.destroy()
  })

  it('creates a menu policy from the access control menus page', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/menus')

    await waitForSelector(mounted.container, '[data-testid="access-control-menu-select"]')

    const selectButtons = mounted.container.querySelectorAll('[data-testid="access-control-menu-select"]')
    const firstSelectButton = selectButtons.item(0)
    if (!(firstSelectButton instanceof HTMLElement)) {
      throw new Error('Expected menu select button')
    }
    firstSelectButton.click()

    updateInput(await findInput(mounted.container, '[data-testid="access-control-menu-order"]'), '400')
    updateInput(await findInput(mounted.container, '[data-testid="access-control-menu-group"]'), 'workspace')

    const saveButton = mounted.container.querySelector('[data-testid="access-control-menu-save-policy"]')
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error('Expected menu policy save button')
    }
    saveButton.click()

    await waitForSelector(document.body, '[data-testid="ui-toast-viewport"]')
    await waitForText(document.body as unknown as HTMLElement, '菜单策略已保存')
    expect(mounted.container.querySelector('[data-testid="access-control-menus-toolbar"]')).not.toBeNull()

    await mounted.destroy()
  })

  it('toggles a user status directly from the left list and shows a toast', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/users')

    await waitForSelector(mounted.container, '[data-testid="access-control-user-record-user-operator"] [role="switch"]')

    const userSwitch = mounted.container.querySelector('[data-testid="access-control-user-record-user-operator"] [role="switch"]')
    if (!(userSwitch instanceof HTMLButtonElement)) {
      throw new Error('Expected user status switch')
    }

    expect(userSwitch.getAttribute('aria-checked')).toBe('true')
    userSwitch.click()

    await waitForText(document.body as unknown as HTMLElement, '用户已停用')
    await waitForAttributeValue(mounted.container, '[data-testid="access-control-user-record-user-operator"] [role="switch"]', 'aria-checked', 'false')

    await mounted.destroy()
  })

  it('supports cross-page bulk delete for users and reports the batch result in a toast', async () => {
    installWorkspaceApiFixture({ extraAccessUsersCount: 10 })

    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/users')

    await waitForSelector(mounted.container, '[data-testid="access-control-user-record-user-extra-01"]')

    const firstPageCheckbox = mounted.container.querySelector<HTMLInputElement>(
      '[data-testid="access-control-user-select-user-extra-01"] input[type="checkbox"]',
    )
    if (!(firstPageCheckbox instanceof HTMLInputElement)) {
      throw new Error('Expected first-page bulk-select checkbox')
    }
    firstPageCheckbox.click()

    const nextPageButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="ui-pagination-next"]')
    if (!(nextPageButton instanceof HTMLButtonElement)) {
      throw new Error('Expected next-page button')
    }
    nextPageButton.click()

    await waitForText(mounted.container, 'Access User 09')

    const secondPageCheckbox = mounted.container.querySelector<HTMLInputElement>(
      '[data-testid="access-control-user-select-user-extra-09"] input[type="checkbox"]',
    )
    if (!(secondPageCheckbox instanceof HTMLInputElement)) {
      throw new Error('Expected second-page bulk-select checkbox')
    }
    secondPageCheckbox.click()

    await waitForText(mounted.container, '已选 2 项')

    const bulkDeleteButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="access-control-user-bulk-delete-button"]')
    if (!(bulkDeleteButton instanceof HTMLButtonElement)) {
      throw new Error('Expected bulk-delete button')
    }
    bulkDeleteButton.click()

    await waitForSelector(document.body, '[data-testid="access-control-user-bulk-delete-confirm"]')

    const confirmButton = document.body.querySelector<HTMLButtonElement>('[data-testid="access-control-user-bulk-delete-confirm"]')
    if (!(confirmButton instanceof HTMLButtonElement)) {
      throw new Error('Expected bulk-delete confirm button')
    }
    confirmButton.click()

    await waitForSelector(document.body, '[data-testid="ui-toast-viewport"]')
    await waitForText(document.body as unknown as HTMLElement, '批量删除完成')
    expect(mounted.container.textContent).not.toContain('Access User 09')

    nextPageButton.click()
    await nextTick()
    expect(mounted.container.textContent).not.toContain('Access User 01')

    await mounted.destroy()
  })

  it('toggles a role status directly from the left list and keeps menu cards badge-only', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/roles')

    await waitForSelector(mounted.container, '[data-testid="access-control-role-record-role-owner"] [role="switch"]')

    const roleSwitch = mounted.container.querySelector('[data-testid="access-control-role-record-role-owner"] [role="switch"]')
    if (!(roleSwitch instanceof HTMLButtonElement)) {
      throw new Error('Expected role status switch')
    }

    expect(roleSwitch.getAttribute('aria-checked')).toBe('true')
    roleSwitch.click()

    await waitForText(document.body as unknown as HTMLElement, '角色已停用')
    await waitForAttributeValue(mounted.container, '[data-testid="access-control-role-record-role-owner"] [role="switch"]', 'aria-checked', 'false')

    await mounted.destroy()

    const menusMounted = await mountRoutedApp('/workspaces/ws-local/access-control/menus')
    await waitForSelector(menusMounted.container, '[data-testid="access-control-menu-select"]')

    expect(menusMounted.container.querySelector('[data-testid="access-control-menu-select"] [role="switch"]')).toBeNull()

    await menusMounted.destroy()
  })

  it('toggles an org unit status directly from the left list and keeps resource cards badge-only', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/org')

    await waitForSelector(mounted.container, '[data-testid="access-control-org-unit-record-org-root"] [role="switch"]')

    const unitSwitch = mounted.container.querySelector('[data-testid="access-control-org-unit-record-org-root"] [role="switch"]')
    if (!(unitSwitch instanceof HTMLButtonElement)) {
      throw new Error('Expected org unit status switch')
    }

    expect(unitSwitch.getAttribute('aria-checked')).toBe('true')
    unitSwitch.click()

    await waitForText(document.body as unknown as HTMLElement, '部门已停用')
    await waitForAttributeValue(mounted.container, '[data-testid="access-control-org-unit-record-org-root"] [role="switch"]', 'aria-checked', 'false')

    await mounted.destroy()

    const resourcesMounted = await mountRoutedApp('/workspaces/ws-local/access-control/resources')
    await waitForSelector(resourcesMounted.container, '[data-testid="access-control-resource-select"]')

    expect(resourcesMounted.container.querySelector('[data-testid="access-control-resource-select"] [role="switch"]')).toBeNull()

    await resourcesMounted.destroy()
  })

  it('shows a toast instead of a page error when deleting the root org unit is blocked', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/org')

    await waitForSelector(mounted.container, '[data-testid="access-control-org-unit-record-org-root"]')

    const rootRecord = mounted.container.querySelector('[data-testid="access-control-org-unit-record-org-root"]')
    if (!(rootRecord instanceof HTMLElement)) {
      throw new Error('Expected root org unit record')
    }
    rootRecord.click()

    await waitForSelector(mounted.container, '[data-testid="access-control-org-unit-delete-button"]')

    const deleteButton = mounted.container.querySelector('[data-testid="access-control-org-unit-delete-button"]')
    if (!(deleteButton instanceof HTMLButtonElement)) {
      throw new Error('Expected root org unit delete button')
    }
    deleteButton.click()

    await waitForSelector(document.body, '[data-testid="ui-toast-viewport"]')
    await waitForText(document.body as unknown as HTMLElement, '根部门不可删除')
    expect(mounted.container.textContent).not.toContain('org-root cannot be deleted')

    await mounted.destroy()
  })

  it('supports bulk deleting org assignments from the assignments subsection', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/org')

    await waitForSelector(mounted.container, '[data-testid="ui-tabs-trigger-assignments"]')

    const assignmentsTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-assignments"]')
    if (!(assignmentsTab instanceof HTMLButtonElement)) {
      throw new Error('Expected assignments tab button')
    }
    assignmentsTab.click()

    await waitForSelector(mounted.container, '[data-testid="access-control-org-assignments-toolbar"]')

    const firstCheckbox = mounted.container.querySelector<HTMLInputElement>(
      '[data-testid="access-control-org-assignment-select-user-owner:org-root"] input[type="checkbox"]',
    )
    const secondCheckbox = mounted.container.querySelector<HTMLInputElement>(
      '[data-testid="access-control-org-assignment-select-user-operator:org-root"] input[type="checkbox"]',
    )
    if (!(firstCheckbox instanceof HTMLInputElement) || !(secondCheckbox instanceof HTMLInputElement)) {
      throw new Error('Expected assignment bulk-select checkboxes')
    }
    firstCheckbox.click()
    secondCheckbox.click()

    await waitForText(mounted.container, '已选 2 项')

    const bulkDeleteButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="access-control-org-assignments-bulk-delete-button"]')
    if (!(bulkDeleteButton instanceof HTMLButtonElement)) {
      throw new Error('Expected assignment bulk delete button')
    }
    bulkDeleteButton.click()

    await waitForSelector(document.body, '[data-testid="access-control-org-assignments-bulk-delete-confirm"]')

    const confirmButton = document.body.querySelector<HTMLButtonElement>('[data-testid="access-control-org-assignments-bulk-delete-confirm"]')
    if (!(confirmButton instanceof HTMLButtonElement)) {
      throw new Error('Expected assignment bulk delete confirm button')
    }
    confirmButton.click()

    await waitForText(document.body as unknown as HTMLElement, '批量删除完成')
    await waitForSelectorToDisappear(mounted.container, '[data-testid="access-control-org-assignment-select-user-owner:org-root"]')
    await waitForSelectorToDisappear(mounted.container, '[data-testid="access-control-org-assignment-select-user-operator:org-root"]')

    await mounted.destroy()
  })

  it('renders org units as a hierarchy tree and keeps descendants grouped under their parents', async () => {
    installWorkspaceApiFixture({ includeAccessOrgHierarchy: true })

    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/org')

    await waitForSelector(mounted.container, '[data-testid="ui-hierarchy-list"]')
    await waitForText(mounted.container, 'Engineering')
    await waitForText(mounted.container, 'Platform')

    const engineeringNode = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-org-unit-node-org-engineering"]')
    const platformNode = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-org-unit-node-org-platform"]')
    if (!(engineeringNode instanceof HTMLElement) || !(platformNode instanceof HTMLElement)) {
      throw new Error('Expected org hierarchy nodes')
    }

    expect(engineeringNode.getAttribute('data-depth')).toBe('1')
    expect(platformNode.getAttribute('data-depth')).toBe('2')
    expect(platformNode.textContent).toContain('Platform')

    const engineeringToggle = mounted.container.querySelector<HTMLElement>('[data-testid="ui-hierarchy-toggle-org-engineering"]')
    if (!(engineeringToggle instanceof HTMLElement)) {
      throw new Error('Expected org hierarchy toggle')
    }

    engineeringToggle.click()
    await waitForSelectorToDisappear(mounted.container, '[data-testid="access-control-org-unit-node-org-platform"]')

    engineeringToggle.click()
    await waitForSelector(mounted.container, '[data-testid="access-control-org-unit-node-org-platform"]')

    await mounted.destroy()
  })

  it('allows collapsing and expanding the root org unit branch', async () => {
    installWorkspaceApiFixture({ includeAccessOrgHierarchy: true })

    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/org')

    await waitForSelector(mounted.container, '[data-testid="access-control-org-unit-node-org-root"]')
    await waitForSelector(mounted.container, '[data-testid="access-control-org-unit-node-org-engineering"]')

    const rootToggle = mounted.container.querySelector<HTMLElement>('[data-testid="ui-hierarchy-toggle-org-root"]')
    if (!(rootToggle instanceof HTMLElement)) {
      throw new Error('Expected root org hierarchy toggle')
    }

    rootToggle.click()
    await waitForSelectorToDisappear(mounted.container, '[data-testid="access-control-org-unit-node-org-engineering"]')

    rootToggle.click()
    await waitForSelector(mounted.container, '[data-testid="access-control-org-unit-node-org-engineering"]')

    await mounted.destroy()
  })

  it('supports bulk deleting role bindings from the policies bindings subsection', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/policies')

    await waitForSelector(mounted.container, '[data-testid="ui-tabs-trigger-bindings"]')

    const bindingsTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-bindings"]')
    if (!(bindingsTab instanceof HTMLButtonElement)) {
      throw new Error('Expected bindings tab button')
    }
    bindingsTab.click()

    await waitForSelector(
      mounted.container,
      '[data-testid="access-control-policies-binding-select-binding-user-owner-role-owner"] input[type="checkbox"]',
    )

    const ownerCheckbox = mounted.container.querySelector<HTMLInputElement>(
      '[data-testid="access-control-policies-binding-select-binding-user-owner-role-owner"] input[type="checkbox"]',
    )
    const operatorCheckbox = mounted.container.querySelector<HTMLInputElement>(
      '[data-testid="access-control-policies-binding-select-binding-user-operator-role-operator"] input[type="checkbox"]',
    )
    if (!(ownerCheckbox instanceof HTMLInputElement) || !(operatorCheckbox instanceof HTMLInputElement)) {
      throw new Error('Expected binding bulk-select checkboxes')
    }
    ownerCheckbox.click()
    operatorCheckbox.click()

    await waitForText(mounted.container, '已选 2 项')
    await waitForSelector(mounted.container, '[data-testid="access-control-policies-bindings-bulk-delete-button"]')

    const bulkDeleteButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="access-control-policies-bindings-bulk-delete-button"]')
    if (!(bulkDeleteButton instanceof HTMLButtonElement)) {
      throw new Error('Expected bindings bulk delete button')
    }
    bulkDeleteButton.click()

    await waitForSelector(document.body, '[data-testid="access-control-policies-bindings-bulk-delete-confirm"]')

    const confirmButton = document.body.querySelector<HTMLButtonElement>('[data-testid="access-control-policies-bindings-bulk-delete-confirm"]')
    if (!(confirmButton instanceof HTMLButtonElement)) {
      throw new Error('Expected bindings bulk delete confirm button')
    }
    confirmButton.click()

    await waitForText(document.body as unknown as HTMLElement, '批量删除完成')
    await waitForSelectorToDisappear(mounted.container, '[data-testid="access-control-policies-binding-select-binding-user-owner-role-owner"]')
    await waitForSelectorToDisappear(mounted.container, '[data-testid="access-control-policies-binding-select-binding-user-operator-role-operator"]')

    await mounted.destroy()
  })

  it('groups permissions by capability module and only allows selecting leaf permissions', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/policies')

    await waitForSelector(mounted.container, '[data-testid="ui-hierarchy-list"]')
    await waitForText(mounted.container, 'workspace')
    await waitForText(mounted.container, 'access')
    await waitForText(mounted.container, 'runtime')

    const moduleNode = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-permission-module-access"]')
    const leafNode = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-permission-leaf-access.users.manage"]')
    if (!(moduleNode instanceof HTMLElement) || !(leafNode instanceof HTMLElement)) {
      throw new Error('Expected permission tree nodes')
    }

    const toggleButton = mounted.container.querySelector<HTMLElement>('[data-testid="ui-hierarchy-toggle-access"]')
    if (!(toggleButton instanceof HTMLElement)) {
      throw new Error('Expected permission tree toggle button')
    }

    toggleButton.click()
    await waitForSelectorToDisappear(mounted.container, '[data-testid="access-control-permission-leaf-access.users.manage"]')

    toggleButton.click()
    await waitForSelector(mounted.container, '[data-testid="access-control-permission-leaf-access.users.manage"]')

    moduleNode.click()
    await nextTick()
    expect(mounted.container.textContent).toContain('请选择权限')

    leafNode.click()
    await waitForText(mounted.container, 'access.users.manage')

    await mounted.destroy()
  })

  it('renders role permission bindings as grouped permission sections in the detail inspector', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/roles')

    await waitForSelector(mounted.container, '[data-testid="access-control-role-record-role-owner"]')

    const ownerRecord = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-role-record-role-owner"]')
    if (!(ownerRecord instanceof HTMLElement)) {
      throw new Error('Expected owner role record')
    }
    ownerRecord.click()

    await waitForSelector(mounted.container, '[data-testid="access-control-role-permissions-inspector"]')
    const inspectorTree = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-role-permissions-inspector"]')
    if (!(inspectorTree instanceof HTMLElement)) {
      throw new Error('Expected role permission inspector tree')
    }

    await waitForSelector(inspectorTree, '[data-testid="access-control-role-permission-section-access"]')

    const toggleButton = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-trigger-access"]')
    if (!(toggleButton instanceof HTMLElement)) {
      throw new Error('Expected role permission section toggle button')
    }

    const sectionBody = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-body-access"]')
    if (!(sectionBody instanceof HTMLElement)) {
      throw new Error('Expected role permission section body')
    }

    const initiallyExpanded = toggleButton.getAttribute('aria-expanded') === 'true'
    expect(sectionBody.style.display).toBe(initiallyExpanded ? '' : 'none')

    toggleButton.click()
    await nextTick()
    expect(toggleButton.getAttribute('aria-expanded')).toBe(initiallyExpanded ? 'false' : 'true')
    expect(sectionBody.style.display).toBe(initiallyExpanded ? 'none' : '')

    toggleButton.click()
    await nextTick()
    expect(toggleButton.getAttribute('aria-expanded')).toBe(initiallyExpanded ? 'true' : 'false')
    expect(sectionBody.style.display).toBe(initiallyExpanded ? '' : 'none')
    await waitForSelector(inspectorTree, '[data-testid="access-control-role-permission-row-access.users.manage"]')

    await mounted.destroy()
  })

  it('keeps role permission rows mounted when collapsing sections to avoid rebuild churn', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/roles')

    await waitForSelector(mounted.container, '[data-testid="access-control-role-record-role-owner"]')

    const ownerRecord = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-role-record-role-owner"]')
    if (!(ownerRecord instanceof HTMLElement)) {
      throw new Error('Expected owner role record')
    }
    ownerRecord.click()

    await waitForSelector(mounted.container, '[data-testid="access-control-role-permissions-inspector"]')
    const inspectorTree = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-role-permissions-inspector"]')
    if (!(inspectorTree instanceof HTMLElement)) {
      throw new Error('Expected role permission inspector tree')
    }

    await waitForSelector(inspectorTree, '[data-testid="access-control-role-permission-row-access.users.manage"]')

    const toggleButton = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-trigger-access"]')
    if (!(toggleButton instanceof HTMLElement)) {
      throw new Error('Expected role permission section toggle button')
    }

    const sectionBody = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-body-access"]')
    if (!(sectionBody instanceof HTMLElement)) {
      throw new Error('Expected role permission section body')
    }

    const leafRow = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-row-access.users.manage"]')
    if (!(leafRow instanceof HTMLElement)) {
      throw new Error('Expected role permission row')
    }

    const initiallyExpanded = toggleButton.getAttribute('aria-expanded') === 'true'
    if (!initiallyExpanded) {
      toggleButton.click()
      await nextTick()
    }

    toggleButton.click()
    await nextTick()

    expect(sectionBody.style.display).toBe('none')
    expect(inspectorTree.querySelector('[data-testid="access-control-role-permission-row-access.users.manage"]')).toBe(leafRow)

    await mounted.destroy()
  })

  it('allows editing permissions across multiple expanded role permission sections', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/roles')

    await waitForSelector(mounted.container, '[data-testid="access-control-role-record-role-owner"]')

    const ownerRecord = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-role-record-role-owner"]')
    if (!(ownerRecord instanceof HTMLElement)) {
      throw new Error('Expected owner role record')
    }
    ownerRecord.click()

    await waitForSelector(mounted.container, '[data-testid="access-control-role-permissions-inspector"]')
    const inspectorTree = mounted.container.querySelector<HTMLElement>('[data-testid="access-control-role-permissions-inspector"]')
    if (!(inspectorTree instanceof HTMLElement)) {
      throw new Error('Expected role permission inspector tree')
    }

    await waitForSelector(inspectorTree, '[data-testid="access-control-role-permission-trigger-access"]')
    await waitForSelector(inspectorTree, '[data-testid="access-control-role-permission-trigger-project"]')

    const accessToggle = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-trigger-access"]')
    if (!(accessToggle instanceof HTMLElement)) {
      throw new Error('Expected access module toggle button')
    }

    accessToggle.click()
    await nextTick()

    const projectToggle = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-trigger-project"]')
    if (!(projectToggle instanceof HTMLElement)) {
      throw new Error('Expected project module toggle button')
    }

    projectToggle.click()
    await nextTick()

    const accessBody = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-body-access"]')
    const projectBody = inspectorTree.querySelector<HTMLElement>('[data-testid="access-control-role-permission-body-project"]')
    if (!(accessBody instanceof HTMLElement) || !(projectBody instanceof HTMLElement)) {
      throw new Error('Expected role permission section bodies')
    }

    if (accessToggle.getAttribute('aria-expanded') !== 'true') {
      accessToggle.click()
      await nextTick()
    }
    if (projectToggle.getAttribute('aria-expanded') !== 'true') {
      projectToggle.click()
      await nextTick()
    }

    expect(accessBody.style.display).not.toBe('none')
    expect(projectBody.style.display).not.toBe('none')
    await waitForSelector(inspectorTree, '[data-testid="access-control-role-permission-row-access.users.manage"]')
    await waitForSelector(inspectorTree, '[data-testid="access-control-role-permission-row-project.view"]')

    await mounted.destroy()
  })

  it('renders menu management list as a hierarchy tree and supports branch collapse', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/access-control/menus')

    await waitForSelector(mounted.container, '[data-testid="access-control-menu-node-menu-workspace-access-control"]')
    await waitForSelector(mounted.container, '[data-testid="access-control-menu-node-menu-workspace-access-control-users"]')

    const toggleButton = mounted.container.querySelector<HTMLElement>('[data-testid="ui-hierarchy-toggle-menu-workspace-access-control"]')
    if (!(toggleButton instanceof HTMLElement)) {
      throw new Error('Expected menu hierarchy toggle button')
    }

    toggleButton.click()
    await waitForSelectorToDisappear(mounted.container, '[data-testid="access-control-menu-node-menu-workspace-access-control-users"]')

    toggleButton.click()
    await waitForSelector(mounted.container, '[data-testid="access-control-menu-node-menu-workspace-access-control-users"]')

    await mounted.destroy()
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
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).toContain('用户管理')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).not.toContain('基本资料')

    await mounted.destroy()
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

    await mounted.destroy()
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

    await mounted.destroy()
  })
})
