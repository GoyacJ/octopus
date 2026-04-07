// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
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

async function waitForTextToDisappear(container: HTMLElement, value: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (container.textContent?.includes(value) ?? false) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for text to disappear: ${value}`)
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

async function waitForRouteName(name: string, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (router.currentRoute.value.name !== name) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error(`Timed out waiting for route: ${name}`)
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

describe('User center RBAC prototype', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    i18n.global.locale.value = 'zh-CN'
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders the owner-accessible user center tabs and user records from the real RBAC API', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/users')

    await waitForSelector(mounted.container, '[data-testid="user-center-tabs"]')
    await waitForText(mounted.container, 'Lin Zhou')

    expect(mounted.container.querySelector('[data-testid="user-center-tabs"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-profile"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-pet"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-users"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-roles"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-permissions"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-menu-workspace-user-center-menus"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('基本资料')
    expect(mounted.container.textContent).toContain('宠物')
    expect(mounted.container.textContent).toContain('成员管理')
    expect(mounted.container.textContent).toContain('角色管理')
    expect(mounted.container.textContent).toContain('权限管理')
    expect(mounted.container.textContent).toContain('导航管理')
    expect(mounted.container.textContent).toContain('Lobster Owner')
    expect(mounted.container.textContent).toContain('Lin Zhou')

    mounted.destroy()
  })

  it('renders permissions, roles, menus, pet, and profile pages through the new user center surfaces', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/users')

    await router.push('/workspaces/ws-local/user-center/pet')
    await waitForSelector(mounted.container, '[data-testid="user-center-pet-card"]')
    expect(mounted.container.textContent).toContain('宠物')
    expect((await findInput(mounted.container, '[data-testid="user-center-pet-display-name"]')).value).toBe('小章')
    expect((await findSelect(mounted.container, '[data-testid="user-center-pet-model-select"]')).value).toBe('openai-primary')
    expect((await findSelect(mounted.container, '[data-testid="user-center-pet-permission-select"]')).value).toBe('read-only')
    expect((mounted.container.querySelector('[data-testid="user-center-pet-greeting-input"]') as HTMLTextAreaElement | null)?.value).toBe('嗨！我是小章，今天也要加油哦！')
    expect((mounted.container.querySelector('[data-testid="user-center-pet-summary-input"]') as HTMLTextAreaElement | null)?.value).toBe('Octopus 首席吉祥物，负责卖萌和加油。')
    await waitForText(mounted.container, '"permissionMode": "read-only"')
    expect(mounted.container.querySelector('[data-testid="user-center-pet-runtime-preview"]')?.textContent).toContain('"configuredModelId": "openai-primary"')
    expect(mounted.container.querySelector('[data-testid="user-center-pet-runtime-preview"]')?.textContent).toContain('"permissionMode": "read-only"')
    expect(mounted.container.querySelector('[data-testid="user-center-pet-runtime-preview"]')?.textContent).toContain('"displayName": "小章"')

    await router.push('/workspaces/ws-local/user-center/permissions')
    await waitForText(mounted.container, 'Manage users')
    expect(mounted.container.textContent).toContain('Manage users')
    expect(mounted.container.textContent).toContain('Manage roles')

    await router.push('/workspaces/ws-local/user-center/roles')
    await waitForText(mounted.container, 'Owner')
    expect(mounted.container.textContent).toContain('Owner')
    expect(mounted.container.textContent).toContain('Operator')

    await router.push('/workspaces/ws-local/user-center/menus')
    await waitForText(mounted.container, '基本资料')
    expect(mounted.container.textContent).toContain('基本资料')
    expect(mounted.container.textContent).toContain('成员管理')

    await router.push('/workspaces/ws-local/user-center/profile')
    await waitForText(mounted.container, 'Lobster Owner')
    expect(mounted.container.textContent).toContain('Lobster Owner')
    expect(mounted.container.textContent).toContain('owner')
    expect(mounted.container.querySelector('[data-testid="profile-access-card"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="profile-access-roles"]')?.textContent).toContain('Owner')
    expect(mounted.container.querySelector('[data-testid="profile-access-permissions"]')?.textContent).toContain('Manage users')
    expect(mounted.container.querySelector('[data-testid="profile-access-permissions"]')?.textContent).toContain('Manage roles')
    expect(mounted.container.querySelector('[data-testid="profile-access-permissions"]')?.textContent).toContain('Manage tools')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).toContain('基本资料')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).toContain('成员管理')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).toContain('角色管理')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).toContain('权限管理')
    expect(mounted.container.querySelector('[data-testid="profile-access-menus"]')?.textContent).toContain('导航管理')
    expect(mounted.container.querySelector('[data-testid="user-runtime-editor"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="user-runtime-effective-preview"]')).not.toBeNull()

    mounted.destroy()
  })

  it('renders translated enum labels and placeholder copy across the user center module', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/users')

    await waitForText(mounted.container, 'Lin Zhou')
    expect(mounted.container.textContent).toContain('正常')
    expect(mounted.container.textContent).toContain('已设置')
    expect(mounted.container.textContent).toContain('停用')

    await router.push('/workspaces/ws-local/user-center/pet')
    await waitForText(mounted.container, '配置小章在当前工作区下的个人偏好')
    expect(mounted.container.textContent).toContain('默认模型')
    expect(mounted.container.textContent).toContain('问候语')
    expect(mounted.container.textContent).toContain('角色摘要')
    expect(mounted.container.textContent).toContain('权限模式')

    await router.push('/workspaces/ws-local/user-center/permissions')
    await waitForText(mounted.container, 'Manage users')
    expect(mounted.container.textContent).toContain('原子权限')
    expect(mounted.container.textContent).toContain('权限包')

    await router.push('/workspaces/ws-local/user-center/menus')
    await waitForText(mounted.container, '基本资料')
    expect(mounted.container.textContent).toContain('用户中心')

    await router.push('/workspaces/ws-local/user-center/recent-conversations')
    await waitForText(mounted.container, '最近会话')
    expect(mounted.container.textContent).toContain('全局最近会话功能开发中')

    await router.push('/workspaces/ws-local/user-center/todos')
    await waitForText(mounted.container, '待办事项')
    expect(mounted.container.textContent).toContain('全局待办事项功能开发中')

    mounted.destroy()
  })

  it('saves pet preferences into the current user runtime config', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/pet')

    await waitForSelector(mounted.container, '[data-testid="user-center-pet-card"]')

    updateInput(await findInput(mounted.container, '[data-testid="user-center-pet-display-name"]'), '章鱼助手')
    const greetingInput = mounted.container.querySelector('[data-testid="user-center-pet-greeting-input"]')
    if (!(greetingInput instanceof HTMLTextAreaElement)) {
      throw new Error('Expected pet greeting textarea')
    }
    greetingInput.value = '欢迎回来，我已经准备好了。'
    greetingInput.dispatchEvent(new Event('input', { bubbles: true }))
    const summaryInput = mounted.container.querySelector('[data-testid="user-center-pet-summary-input"]')
    if (!(summaryInput instanceof HTMLTextAreaElement)) {
      throw new Error('Expected pet summary textarea')
    }
    summaryInput.value = '负责陪伴与执行工作区任务的章鱼助手。'
    summaryInput.dispatchEvent(new Event('input', { bubbles: true }))
    updateSelect(await findSelect(mounted.container, '[data-testid="user-center-pet-model-select"]'), 'anthropic-alt')
    updateSelect(await findSelect(mounted.container, '[data-testid="user-center-pet-permission-select"]'), 'workspace-write')

    const saveButton = mounted.container.querySelector('[data-testid="user-center-pet-save"]')
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error('Expected pet save button')
    }
    saveButton.click()

    await waitForText(mounted.container, 'anthropic-alt')
    expect(mounted.container.querySelector('[data-testid="user-center-pet-runtime-preview"]')?.textContent).toContain('"configuredModelId": "anthropic-alt"')
    expect(mounted.container.querySelector('[data-testid="user-center-pet-runtime-preview"]')?.textContent).toContain('"permissionMode": "workspace-write"')
    expect(mounted.container.querySelector('[data-testid="user-center-pet-runtime-preview"]')?.textContent).toContain('"displayName": "章鱼助手"')
    expect(mounted.container.querySelector('[data-testid="user-center-pet-runtime-preview"]')?.textContent).toContain('"greeting": "欢迎回来，我已经准备好了。"')
    expect(mounted.container.querySelector('[data-testid="user-center-pet-runtime-preview"]')?.textContent).toContain('"summary": "负责陪伴与执行工作区任务的章鱼助手。"')

    await router.push('/workspaces/ws-local/user-center/profile')
    await waitForSelector(mounted.container, '[data-testid="user-runtime-effective-preview"]')
    expect(mounted.container.querySelector('[data-testid="user-runtime-effective-preview"]')?.textContent).toContain('"configuredModelId": "anthropic-alt"')
    expect(mounted.container.querySelector('[data-testid="user-runtime-effective-preview"]')?.textContent).toContain('"permissionMode": "workspace-write"')
    expect(mounted.container.querySelector('[data-testid="user-runtime-effective-preview"]')?.textContent).toContain('"displayName": "章鱼助手"')
    expect(mounted.container.querySelector('[data-testid="user-runtime-effective-preview"]')?.textContent).toContain('"greeting": "欢迎回来，我已经准备好了。"')
    expect(mounted.container.querySelector('[data-testid="user-runtime-effective-preview"]')?.textContent).toContain('"summary": "负责陪伴与执行工作区任务的章鱼助手。"')

    mounted.destroy()
  })

  it('navigates to roles, permissions, and menus from the profile access overview', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/profile')

    await waitForText(mounted.container, 'Lobster Owner')

    const rolesLink = mounted.container.querySelector('[data-testid="profile-access-roles-link"]')
    if (!(rolesLink instanceof HTMLButtonElement)) {
      throw new Error('Expected roles access link')
    }
    rolesLink.click()
    await waitForRouteName('workspace-user-center-roles')
    await waitForText(mounted.container, 'Operator')

    await router.push('/workspaces/ws-local/user-center/profile')
    await waitForText(mounted.container, 'Lobster Owner')

    const permissionsLink = mounted.container.querySelector('[data-testid="profile-access-permissions-link"]')
    if (!(permissionsLink instanceof HTMLButtonElement)) {
      throw new Error('Expected permissions access link')
    }
    permissionsLink.click()
    await waitForRouteName('workspace-user-center-permissions')
    await waitForText(mounted.container, 'Manage users')

    await router.push('/workspaces/ws-local/user-center/profile')
    await waitForText(mounted.container, 'Lobster Owner')

    const menusLink = mounted.container.querySelector('[data-testid="profile-access-menus-link"]')
    if (!(menusLink instanceof HTMLButtonElement)) {
      throw new Error('Expected menus access link')
    }
    menusLink.click()
    await waitForRouteName('workspace-user-center-menus')
    await waitForText(mounted.container, '基本资料')
    await waitForText(mounted.container, '成员管理')

    mounted.destroy()
  })

  it('supports updating the current user profile and keeps avatar fallback rendering when avatar is cleared', async () => {
    vi.spyOn(tauriClient, 'pickAvatarImage').mockResolvedValue({
      fileName: 'kai-avatar.png',
      contentType: 'image/png',
      dataBase64: 'a2Fp',
      byteSize: 3,
    })
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/profile')

    await waitForText(mounted.container, 'Lobster Owner')
    expect(mounted.container.querySelector('[data-testid="profile-avatar-file-label"]')?.textContent).toContain('当前头像')
    expect(mounted.container.textContent).not.toContain('未选择新的头像文件')

    const displayNameInput = await findInput(mounted.container, '[data-testid="profile-display-name-input"]')
    const usernameInput = await findInput(mounted.container, '[data-testid="profile-username-input"]')

    updateInput(displayNameInput, 'Kai Owner')
    updateInput(usernameInput, 'kai-owner')

    const pickAvatarButton = mounted.container.querySelector('[data-testid="profile-avatar-pick-button"]')
    if (!(pickAvatarButton instanceof HTMLButtonElement)) {
      throw new Error('Expected avatar pick button')
    }
    pickAvatarButton.click()

    await waitForText(mounted.container, 'kai-avatar.png')

    const saveButton = mounted.container.querySelector('[data-testid="profile-save-button"]')
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error('Expected profile save button')
    }
    saveButton.click()

    await waitForText(mounted.container, '个人资料已保存。')
    await waitForText(mounted.container, 'Kai Owner')
    await waitForText(mounted.container, 'kai-owner')
    const avatarImage = mounted.container.querySelector('[data-testid="profile-avatar-image"]')
    expect(avatarImage).not.toBeNull()

    const clearAvatarButton = mounted.container.querySelector('[data-testid="profile-avatar-clear-button"]')
    if (!(clearAvatarButton instanceof HTMLButtonElement)) {
      throw new Error('Expected avatar clear button')
    }
    clearAvatarButton.click()
    saveButton.click()
    await waitForText(mounted.container, '个人资料已保存。')

    const avatarFallback = mounted.container.querySelector('[data-testid="profile-avatar-fallback"]')
    expect(avatarFallback?.textContent).toContain('K')

    mounted.destroy()
  })

  it('supports changing the current user password and refreshes the password badge immediately', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/profile')

    await waitForText(mounted.container, 'Lobster Owner')

    const currentPasswordInput = await findInput(mounted.container, '[data-testid="profile-current-password-input"]')
    const newPasswordInput = await findInput(mounted.container, '[data-testid="profile-new-password-input"]')
    const confirmPasswordInput = await findInput(mounted.container, '[data-testid="profile-confirm-password-input"]')

    updateInput(currentPasswordInput, 'owner-owner')
    updateInput(newPasswordInput, 'owner-owner-2')
    updateInput(confirmPasswordInput, 'owner-owner-2')

    const updatePasswordButton = mounted.container.querySelector('[data-testid="profile-password-submit-button"]')
    if (!(updatePasswordButton instanceof HTMLButtonElement)) {
      throw new Error('Expected password update button')
    }
    updatePasswordButton.click()

    await waitForText(mounted.container, '密码已更新。')
    expect(currentPasswordInput.value).toBe('')
    expect(newPasswordInput.value).toBe('')
    expect(confirmPasswordInput.value).toBe('')
    expect(mounted.container.textContent).toContain('已设置')

    mounted.destroy()
  })

  it('shows server-side password change errors for invalid current password and short new password', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/profile')

    await waitForText(mounted.container, 'Lobster Owner')

    const currentPasswordInput = await findInput(mounted.container, '[data-testid="profile-current-password-input"]')
    const newPasswordInput = await findInput(mounted.container, '[data-testid="profile-new-password-input"]')
    const confirmPasswordInput = await findInput(mounted.container, '[data-testid="profile-confirm-password-input"]')
    const updatePasswordButton = mounted.container.querySelector('[data-testid="profile-password-submit-button"]')
    if (!(updatePasswordButton instanceof HTMLButtonElement)) {
      throw new Error('Expected password update button')
    }

    updateInput(currentPasswordInput, 'wrong-password')
    updateInput(newPasswordInput, 'owner-owner-2')
    updateInput(confirmPasswordInput, 'owner-owner-2')
    updatePasswordButton.click()
    await waitForText(mounted.container, 'Current password is incorrect.')

    updateInput(currentPasswordInput, 'owner-owner')
    updateInput(newPasswordInput, 'short')
    updateInput(confirmPasswordInput, 'short')
    updatePasswordButton.click()
    await waitForText(mounted.container, 'New password must be at least 8 characters.')

    mounted.destroy()
  })

  it('supports paginated member management with role select, project scope multi-select, default avatar and default password', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/users')

    await waitForText(mounted.container, 'Lin Zhou')
    expect(mounted.container.querySelector('[data-testid="users-list-pagination"]')).not.toBeNull()

    const newUserButton = mounted.container.querySelector('[data-testid="users-create-button"]')
    if (!(newUserButton instanceof HTMLButtonElement)) {
      throw new Error('Expected create member button')
    }
    newUserButton.click()

    const usernameInput = await findInput(mounted.container, '[data-testid="users-username-input"]')
    const displayNameInput = await findInput(mounted.container, '[data-testid="users-display-name-input"]')
    const roleSelect = await findSelect(mounted.container, '[data-testid="users-role-select"]')

    updateInput(usernameInput, 'member-alpha')
    updateInput(displayNameInput, 'Member Alpha')
    updateSelect(roleSelect, 'role-operator')

    const governanceScopeToggle = mounted.container.querySelector('[data-testid="users-project-scope-proj-governance"]')
    if (!(governanceScopeToggle instanceof HTMLElement)) {
      throw new Error('Expected project scope toggle')
    }
    governanceScopeToggle.click()

    const saveButton = mounted.container.querySelector('[data-testid="users-save-button"]')
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error('Expected save member button')
    }
    saveButton.click()

    await waitForText(mounted.container, 'Member Alpha')
    expect((await findSelect(mounted.container, '[data-testid="users-role-select"]')).value).toBe('role-operator')
    expect(mounted.container.textContent).toContain('需要重置')

    const detailFallback = mounted.container.querySelector('[data-testid="users-avatar-fallback"]')
    expect(detailFallback?.textContent).toContain('M')

    mounted.destroy()
  })

  it('supports updating member avatar and password, then deleting the member from the paginated list', async () => {
    vi.spyOn(tauriClient, 'pickAvatarImage').mockResolvedValue({
      fileName: 'member-beta.png',
      contentType: 'image/png',
      dataBase64: 'YmV0YQ==',
      byteSize: 4,
    })
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/users')

    await waitForText(mounted.container, 'Lin Zhou')

    const newUserButton = mounted.container.querySelector('[data-testid="users-create-button"]')
    if (!(newUserButton instanceof HTMLButtonElement)) {
      throw new Error('Expected create member button')
    }
    newUserButton.click()

    updateInput(await findInput(mounted.container, '[data-testid="users-username-input"]'), 'member-beta')
    updateInput(await findInput(mounted.container, '[data-testid="users-display-name-input"]'), 'Member Beta')
    updateSelect(await findSelect(mounted.container, '[data-testid="users-role-select"]'), 'role-owner')

    const saveButton = mounted.container.querySelector('[data-testid="users-save-button"]')
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error('Expected save member button')
    }
    saveButton.click()

    await waitForText(mounted.container, 'Member Beta')

    const pickAvatarButton = mounted.container.querySelector('[data-testid="users-avatar-pick-button"]')
    if (!(pickAvatarButton instanceof HTMLButtonElement)) {
      throw new Error('Expected member avatar pick button')
    }
    pickAvatarButton.click()
    await waitForText(mounted.container, 'member-beta.png')

    const customPasswordToggle = mounted.container.querySelector('[data-testid="users-password-mode-custom"]')
    if (!(customPasswordToggle instanceof HTMLElement)) {
      throw new Error('Expected custom password toggle')
    }
    customPasswordToggle.click()

    updateInput(await findInput(mounted.container, '[data-testid="users-password-input"]'), 'member-beta-1')
    updateInput(await findInput(mounted.container, '[data-testid="users-password-confirm-input"]'), 'member-beta-1')
    saveButton.click()

    await waitForSelector(mounted.container, '[data-testid="users-avatar-image"]')
    expect(mounted.container.textContent).toContain('已设置')

    const deleteTrigger = mounted.container.querySelector('[data-testid="users-delete-button-member-beta"]')
    if (!(deleteTrigger instanceof HTMLButtonElement)) {
      throw new Error('Expected delete member trigger')
    }
    deleteTrigger.click()

    await waitForSelector(document.body, '[data-testid="users-delete-confirm-button"]')
    const deleteConfirm = document.body.querySelector('[data-testid="users-delete-confirm-button"]')
    if (!(deleteConfirm instanceof HTMLButtonElement)) {
      throw new Error('Expected delete member confirm button')
    }
    deleteConfirm.click()

    await waitForText(mounted.container, 'Lin Zhou')
    await waitForTextToDisappear(mounted.container, 'Member Beta')
    expect(mounted.container.textContent).not.toContain('Member Beta')

    mounted.destroy()
  })

  it('supports paginated role management with permission and menu property lists plus role deletion', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/roles')

    await waitForText(mounted.container, 'Owner')
    expect(mounted.container.querySelector('[data-testid="roles-list-pagination"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Manage users')
    expect(mounted.container.textContent).toContain('基本资料')
    const appMenuGroup = mounted.container.querySelector('[data-testid="roles-menu-group-app"]')
    const workspaceMenuGroup = mounted.container.querySelector('[data-testid="roles-menu-group-workspace"]')
    const projectMenuGroup = mounted.container.querySelector('[data-testid="roles-menu-group-project"]')
    const userCenterMenuGroup = mounted.container.querySelector('[data-testid="roles-menu-group-user-center"]')
    expect(appMenuGroup).not.toBeNull()
    expect(workspaceMenuGroup).not.toBeNull()
    expect(projectMenuGroup).not.toBeNull()
    expect(userCenterMenuGroup).not.toBeNull()
    expect(appMenuGroup?.textContent).toContain('应用')
    expect(workspaceMenuGroup?.textContent).toContain('工作区')
    expect(projectMenuGroup?.textContent).toContain('项目')
    expect(userCenterMenuGroup?.textContent).toContain('用户中心')
    expect(appMenuGroup?.className).not.toContain('border')

    const userCenterMenuTrigger = mounted.container.querySelector('[data-testid="ui-accordion-trigger-roles-menu-branch-user-center"]')
    if (!(userCenterMenuTrigger instanceof HTMLButtonElement)) {
      throw new Error('Expected role user-center menu trigger')
    }
    if (!mounted.container.querySelector('[data-testid="roles-menu-menu-workspace-user-center-profile"]')) {
      userCenterMenuTrigger.click()
    }
    await waitForSelector(mounted.container, '[data-testid="roles-menu-menu-workspace-user-center-profile"]')

    const createRoleButton = mounted.container.querySelector('[data-testid="roles-create-button"]')
    if (!(createRoleButton instanceof HTMLButtonElement)) {
      throw new Error('Expected create role button')
    }
    createRoleButton.click()

    updateInput(await findInput(mounted.container, '[data-testid="roles-name-input"]'), 'Auditor')
    updateInput(await findInput(mounted.container, '[data-testid="roles-code-input"]'), 'auditor')

    const permissionToggle = mounted.container.querySelector('[data-testid="roles-permission-perm-manage-users"]')
    if (!(permissionToggle instanceof HTMLElement)) {
      throw new Error('Expected role permission toggle')
    }
    permissionToggle.click()

    const menuToggle = mounted.container.querySelector('[data-testid="roles-menu-menu-workspace-user-center-profile"]')
    if (!(menuToggle instanceof HTMLElement)) {
      throw new Error('Expected role menu toggle')
    }
    menuToggle.click()

    const saveRoleButton = mounted.container.querySelector('[data-testid="roles-save-button"]')
    if (!(saveRoleButton instanceof HTMLButtonElement)) {
      throw new Error('Expected save role button')
    }
    saveRoleButton.click()

    await waitForText(mounted.container, 'Auditor')
    expect(mounted.container.textContent).toContain('角色信息已保存。')

    const deleteRoleButton = mounted.container.querySelector('[data-testid="roles-delete-button-auditor"]')
    if (!(deleteRoleButton instanceof HTMLButtonElement)) {
      throw new Error('Expected delete role button')
    }
    deleteRoleButton.click()

    await waitForSelector(document.body, '[data-testid="roles-delete-confirm-button"]')
    const deleteConfirm = document.body.querySelector('[data-testid="roles-delete-confirm-button"]')
    if (!(deleteConfirm instanceof HTMLButtonElement)) {
      throw new Error('Expected delete role confirm button')
    }
    deleteConfirm.click()

    await waitForTextToDisappear(mounted.container, 'Auditor')
    expect(mounted.container.textContent).not.toContain('Auditor')

    mounted.destroy()
  })

  it('supports permission management with create, target binding, bundle composition, and deletion', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/permissions')

    await waitForText(mounted.container, 'Manage users')
    expect(mounted.container.querySelector('[data-testid="permissions-list-pagination"]')).not.toBeNull()

    const createPermissionButton = mounted.container.querySelector('[data-testid="permissions-create-button"]')
    if (!(createPermissionButton instanceof HTMLButtonElement)) {
      throw new Error('Expected create permission button')
    }
    createPermissionButton.click()

    updateInput(await findInput(mounted.container, '[data-testid="permissions-name-input"]'), 'Manage launch runbooks')
    updateInput(await findInput(mounted.container, '[data-testid="permissions-code-input"]'), 'workspace.resources')
    updateSelect(await findSelect(mounted.container, '[data-testid="permissions-target-type-select"]'), 'resource')
    await nextTick()

    const resourceToggle = mounted.container.querySelector('[data-testid="permissions-target-ws-local-res-workspace-1"]')
    if (!(resourceToggle instanceof HTMLElement)) {
      throw new Error('Expected resource target toggle')
    }
    resourceToggle.click()

    const savePermissionButton = mounted.container.querySelector('[data-testid="permissions-save-button"]')
    if (!(savePermissionButton instanceof HTMLButtonElement)) {
      throw new Error('Expected save permission button')
    }
    savePermissionButton.click()

    await waitForText(mounted.container, 'Manage launch runbooks')
    expect(mounted.container.textContent).toContain('权限信息已保存。')

    createPermissionButton.click()
    updateInput(await findInput(mounted.container, '[data-testid="permissions-name-input"]'), 'Workspace operations bundle')
    updateInput(await findInput(mounted.container, '[data-testid="permissions-code-input"]'), 'workspace.bundle.ops')
    updateSelect(await findSelect(mounted.container, '[data-testid="permissions-kind-select"]'), 'bundle')
    await nextTick()

    const memberToggle = mounted.container.querySelector('[data-testid="permissions-member-perm-manage-users"]')
    if (!(memberToggle instanceof HTMLElement)) {
      throw new Error('Expected permission member toggle')
    }
    memberToggle.click()
    savePermissionButton.click()

    await waitForText(mounted.container, 'Workspace operations bundle')
    expect(mounted.container.textContent).toContain('包含 1 个成员权限')

    const deletePermissionButton = mounted.container.querySelector('[data-testid="permissions-delete-button-workspace.bundle.ops"]')
    if (!(deletePermissionButton instanceof HTMLButtonElement)) {
      throw new Error('Expected delete permission button')
    }
    deletePermissionButton.click()

    await waitForSelector(document.body, '[data-testid="permissions-delete-confirm-button"]')
    const deleteConfirm = document.body.querySelector('[data-testid="permissions-delete-confirm-button"]')
    if (!(deleteConfirm instanceof HTMLButtonElement)) {
      throw new Error('Expected delete permission confirm button')
    }
    deleteConfirm.click()

    await waitForTextToDisappear(mounted.container, 'Workspace operations bundle')
    expect(mounted.container.textContent).not.toContain('Workspace operations bundle')

    mounted.destroy()
  })

  it('renders menu management as the same collapsible tree structure', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/user-center/menus')

    await waitForText(mounted.container, '基本资料')

    const appMenuGroup = mounted.container.querySelector('[data-testid="menus-tree-group-app"]')
    const workspaceMenuGroup = mounted.container.querySelector('[data-testid="menus-tree-group-workspace"]')
    const projectMenuGroup = mounted.container.querySelector('[data-testid="menus-tree-group-project"]')
    const userCenterMenuGroup = mounted.container.querySelector('[data-testid="menus-tree-group-user-center"]')
    expect(appMenuGroup).not.toBeNull()
    expect(workspaceMenuGroup).not.toBeNull()
    expect(projectMenuGroup).not.toBeNull()
    expect(userCenterMenuGroup).not.toBeNull()
    expect(appMenuGroup?.className).not.toContain('border')

    const userCenterMenuTrigger = mounted.container.querySelector('[data-testid="ui-accordion-trigger-menus-tree-branch-user-center"]')
    if (!(userCenterMenuTrigger instanceof HTMLButtonElement)) {
      throw new Error('Expected menu management user-center trigger')
    }
    userCenterMenuTrigger.click()
    await waitForSelectorToDisappear(mounted.container, '[data-testid="menus-tree-menu-menu-workspace-user-center-profile"]')
    const reopenedMenuTrigger = mounted.container.querySelector('[data-testid="ui-accordion-trigger-menus-tree-branch-user-center"]')
    if (!(reopenedMenuTrigger instanceof HTMLButtonElement)) {
      throw new Error('Expected menu management user-center trigger after collapse')
    }
    reopenedMenuTrigger.click()
    await waitForSelector(mounted.container, '[data-testid="menus-tree-menu-menu-workspace-user-center-profile"]')

    const usersMenu = mounted.container.querySelector('[data-testid="menus-tree-menu-menu-workspace-user-center-users"]')
    if (!(usersMenu instanceof HTMLButtonElement)) {
      throw new Error('Expected users menu tree item')
    }
    usersMenu.click()

    expect((await findInput(mounted.container, '[data-testid="menus-label-input"]')).value).toBe('Users')

    mounted.destroy()
  })
})
