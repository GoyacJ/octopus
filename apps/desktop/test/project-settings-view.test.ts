// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useInboxStore } from '@/stores/inbox'
import { useNotificationStore } from '@/stores/notifications'
import { useTeamStore } from '@/stores/team'
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

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for project settings state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Project settings view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders document sections and keeps project capability inputs inside unified capability dialogs', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    expect(mounted.container.querySelector('[data-testid="project-settings-overview-section"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-capabilities-section"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-grants-section"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-runtime-section"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-members-section"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-basics"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-name-input"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-resource-directory-path"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-runtime-total-tokens-input"]')).toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-models-dialog"]') !== null)

    expect(document.body.querySelector('[data-testid="project-runtime-total-tokens-input"]')).toBeNull()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-project"]')?.click()
    await nextTick()

    expect(document.body.querySelector('[data-testid="project-runtime-total-tokens-input"]')).not.toBeNull()

    mounted.destroy()
  })

  it('saves project metadata from project settings instead of the workspace registry', async () => {
    vi.spyOn(tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> }, 'pickResourceDirectory')
      .mockResolvedValue('data/projects/proj-redesign-v2/resources')

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const notificationStore = useNotificationStore()
    const workspaceStore = useWorkspaceStore()
    const updateProjectSpy = vi.spyOn(workspaceStore, 'updateProject')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')
    await waitForSelector(mounted.container, '[data-testid="project-settings-name-input"]')

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="project-settings-name-input"]')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="project-settings-description-input"]')
    const resourceDirectoryInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="project-settings-resource-directory-path"]')
    const managerSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="project-settings-manager-select"]')
    const presetSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="project-settings-preset-select"]')
    const saveButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-save-button"]')

    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()
    expect(resourceDirectoryInput).not.toBeNull()
    expect(managerSelect).not.toBeNull()
    expect(presetSelect).not.toBeNull()
    expect(saveButton).not.toBeNull()

    nameInput!.value = 'Desktop Redesign v2'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Refined ownership and runtime boundaries.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))
    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-resource-directory-pick"]')?.click()
    await waitFor(() =>
      mounted.container.querySelector<HTMLInputElement>('[data-testid="project-settings-resource-directory-path"]')?.value
        === 'data/projects/proj-redesign-v2/resources',
    )
    managerSelect!.value = 'user-operator'
    managerSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    presetSelect!.value = 'documentation'
    presetSelect!.dispatchEvent(new Event('change', { bubbles: true }))

    saveButton?.click()

    await waitFor(() => {
      const project = workspaceStore.projects.find(item => item.id === 'proj-redesign')
      return project?.name === 'Desktop Redesign v2'
        && project?.description === 'Refined ownership and runtime boundaries.'
        && project?.resourceDirectory === 'data/projects/proj-redesign-v2/resources'
        && project?.managerUserId === 'user-operator'
        && project?.presetCode === 'documentation'
    })
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.source === 'workspace-project-governance'
        && notification.routeTo === '/workspaces/ws-local/projects/proj-redesign/settings'
        && notification.body?.includes('Desktop Redesign v2'),
      ),
    )

    expect(updateProjectSpy).toHaveBeenCalledWith(
      'proj-redesign',
      expect.objectContaining({
        name: 'Desktop Redesign v2',
        description: 'Refined ownership and runtime boundaries.',
        resourceDirectory: 'data/projects/proj-redesign-v2/resources',
        managerUserId: 'user-operator',
        presetCode: 'documentation',
      }),
    )
    expect(notificationStore.notificationsState.some(notification =>
      notification.source === 'workspace-project-governance'
      && notification.routeTo === '/workspaces/ws-local/projects/proj-redesign/settings'
      && notification.actionLabel
      && notification.body?.includes('Desktop Redesign v2'),
    )).toBe(true)

    mounted.destroy()
  })

  it('keeps unified capability dialogs inside the shared scrollable dialog shell', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-models-dialog"]') !== null)

    const dialogContent = document.body.querySelector<HTMLElement>('[data-testid="project-settings-models-dialog"]')
    const dialogBody = document.body.querySelector<HTMLElement>('[data-testid="ui-dialog-body"]')

    expect(dialogContent).not.toBeNull()
    expect(dialogContent?.className).toContain('max-h-[calc(100dvh-2rem)]')
    expect(dialogBody).not.toBeNull()
    expect(dialogBody?.className).toContain('overflow-y-auto')

    mounted.destroy()
  })

  it('supports select all and clear all actions across workspace capability baselines', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()
    const catalogStore = useCatalogStore()
    const agentStore = useAgentStore()
    const teamStore = useTeamStore()
    const updateProjectSpy = vi.spyOn(workspaceStore, 'updateProject')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    const workspaceModelCount = catalogStore.configuredModelOptions.length
    const workspaceBuiltinToolSourceKeys = catalogStore.managementProjection.assets
      .filter(entry =>
        entry.enabled
        && entry.kind === 'builtin'
        && !(entry.ownerScope === 'project' && entry.ownerId === 'proj-redesign'),
      )
      .map(entry => entry.sourceKey)
    const workspaceAgentCount = agentStore.workspaceAgents.filter(agent => agent.status === 'active').length
    const workspaceTeamCount = teamStore.workspaceTeams.filter(team => team.status === 'active').length

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-models-dialog"]') !== null)

    const selectAllModelsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-models-select-all"]')
    const clearAllModelsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-models-clear-all"]')

    expect(selectAllModelsButton).not.toBeNull()
    expect(clearAllModelsButton).not.toBeNull()

    selectAllModelsButton?.click()
    await nextTick()

    const modelCheckboxes = Array.from(
      document.body.querySelectorAll<HTMLInputElement>('[data-testid^="project-grant-model-option-"] input[type="checkbox"]'),
    )

    expect(modelCheckboxes).toHaveLength(workspaceModelCount)
    expect(modelCheckboxes.every(input => input.checked)).toBe(true)

    clearAllModelsButton?.click()
    await nextTick()

    expect(modelCheckboxes.every(input => !input.checked)).toBe(true)

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-dialog-close"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-models-dialog"]') === null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-tools"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-tools-dialog"]') !== null)

    expect(document.body.querySelector('[data-testid="project-settings-tools-scope-tabs"]')).not.toBeNull()
    const grantToolSearch = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-grants-tools-search"]')
    const selectAllToolsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-select-all"]')
    const clearAllToolsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-clear-all"]')
    const saveGrantToolsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-save-button"]')

    expect(grantToolSearch).not.toBeNull()
    expect(selectAllToolsButton).not.toBeNull()
    expect(clearAllToolsButton).not.toBeNull()
    expect(saveGrantToolsButton).not.toBeNull()

    grantToolSearch!.value = 'bash'
    grantToolSearch!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(document.body.querySelectorAll('[data-testid^="project-grant-tool-option-"]')).toHaveLength(1)
    expect(document.body.textContent).toContain('bash')

    grantToolSearch!.value = ''
    grantToolSearch!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    selectAllToolsButton?.click()
    await nextTick()

    const builtinToolCheckboxes = Array.from(
      document.body.querySelectorAll<HTMLInputElement>('[data-testid^="project-grant-tool-option-"] input[type="checkbox"]'),
    )
    expect(builtinToolCheckboxes).toHaveLength(workspaceBuiltinToolSourceKeys.length)
    expect(builtinToolCheckboxes.every(input => input.checked)).toBe(true)

    const updateProjectCallsBeforeSelectAllTools = updateProjectSpy.mock.calls.length
    saveGrantToolsButton?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-tools-dialog"]') === null)
    expect(updateProjectSpy.mock.calls).toHaveLength(updateProjectCallsBeforeSelectAllTools)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-tools"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-tools-dialog"]') !== null)

    const reopenedGrantToolsDialog = document.body.querySelector<HTMLElement>('[data-testid="project-settings-tools-dialog"]')
    const reopenedToolCheckboxes = Array.from(
      reopenedGrantToolsDialog?.querySelectorAll<HTMLInputElement>('[data-testid^="project-grant-tool-option-"] input[type="checkbox"]') ?? [],
    )
    expect(reopenedToolCheckboxes.every(input => input.checked)).toBe(true)

    document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-clear-all"]')?.click()
    await nextTick()
    const updateProjectCallsBeforeClearAllTools = updateProjectSpy.mock.calls.length
    document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-save-button"]')?.click()

    await waitFor(() => document.body.querySelector('[data-testid="project-settings-tools-dialog"]') === null)
    expect(updateProjectSpy.mock.calls).toHaveLength(updateProjectCallsBeforeClearAllTools)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-tools"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-tools-dialog"]') !== null)

    const clearedGrantToolsDialog = document.body.querySelector<HTMLElement>('[data-testid="project-settings-tools-dialog"]')
    const clearedToolCheckboxes = Array.from(
      clearedGrantToolsDialog?.querySelectorAll<HTMLInputElement>('[data-testid^="project-grant-tool-option-"] input[type="checkbox"]') ?? [],
    )
    expect(clearedToolCheckboxes.every(input => !input.checked)).toBe(true)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-agents"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-actors-dialog"]') !== null)

    expect(document.body.querySelector('[data-testid="project-settings-actors-scope-tabs"]')).not.toBeNull()
    const grantActorSearch = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-grants-actors-search"]')
    const selectAllAgentsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-agents-select-all"]')
    const clearAllAgentsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-agents-clear-all"]')

    expect(grantActorSearch).not.toBeNull()
    expect(selectAllAgentsButton).not.toBeNull()
    expect(clearAllAgentsButton).not.toBeNull()

    grantActorSearch!.value = 'finance'
    grantActorSearch!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(document.body.querySelectorAll('[data-testid^="project-grant-agent-option-"]')).toHaveLength(1)
    expect(document.body.textContent).toContain('Finance Planner Template')

    clearAllAgentsButton?.click()
    await nextTick()

    expect(document.body.textContent).toContain('当前 Leader 必须保持可授予且未被禁用，请先选择新的 Leader。')
    expect(
      document.body
        .querySelector<HTMLInputElement>('[data-testid="project-grant-agent-option-agent-architect"] input[type="checkbox"]')
        ?.checked,
    ).toBe(true)

    grantActorSearch!.value = ''
    grantActorSearch!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    selectAllAgentsButton?.click()
    await nextTick()

    const agentCheckboxes = Array.from(
      document.body.querySelectorAll<HTMLInputElement>('[data-testid^="project-grant-agent-option-"] input[type="checkbox"]'),
    )

    expect(agentCheckboxes).toHaveLength(workspaceAgentCount)
    expect(agentCheckboxes.every(input => input.checked)).toBe(true)

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-teams"]')?.click()
    await nextTick()

    const selectAllTeamsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-teams-select-all"]')
    const clearAllTeamsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-teams-clear-all"]')

    expect(selectAllTeamsButton).not.toBeNull()
    expect(clearAllTeamsButton).not.toBeNull()

    grantActorSearch!.value = 'finance'
    grantActorSearch!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(document.body.querySelectorAll('[data-testid^="project-grant-team-option-"]')).toHaveLength(1)
    expect(document.body.textContent).toContain('Finance Ops Template')

    grantActorSearch!.value = ''
    grantActorSearch!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    selectAllTeamsButton?.click()
    await nextTick()

    const teamCheckboxes = Array.from(
      document.body.querySelectorAll<HTMLInputElement>('[data-testid^="project-grant-team-option-"] input[type="checkbox"]'),
    )

    expect(teamCheckboxes).toHaveLength(workspaceTeamCount)
    expect(teamCheckboxes.every(input => input.checked)).toBe(true)

    clearAllTeamsButton?.click()
    await nextTick()

    expect(teamCheckboxes.every(input => !input.checked)).toBe(true)

    mounted.destroy()
  })

  it('shows capability inheritance and project-disabled summaries in a single capability section', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    const capabilitySection = mounted.container.querySelector('[data-testid="project-settings-capabilities-section"]')
    const modelsCard = mounted.container.querySelector('[data-testid="project-settings-capability-models-card"]')
    const toolsCard = mounted.container.querySelector('[data-testid="project-settings-capability-tools-card"]')
    const agentsCard = mounted.container.querySelector('[data-testid="project-settings-capability-agents-card"]')
    const teamsCard = mounted.container.querySelector('[data-testid="project-settings-capability-teams-card"]')

    expect(capabilitySection).not.toBeNull()
    expect(modelsCard?.textContent).toContain('工作区已授予 2 个')
    expect(modelsCard?.textContent).toContain('项目已启用 1 个')
    expect(modelsCard?.textContent).toContain('项目已禁用 1 个')
    expect(modelsCard?.textContent).toContain('默认 Claude Primary')
    expect(toolsCard?.textContent).toContain('项目自有')
    expect(agentsCard?.textContent).toContain('项目已禁用')
    expect(teamsCard?.textContent).toContain('项目已禁用')

    mounted.destroy()
  })

  it('persists actor workspace baselines and project refinements through delta-only settings saves', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()
    const updateProjectSpy = vi.spyOn(workspaceStore, 'updateProject')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-agents"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-actors-dialog"]') !== null)

    const grantBuiltinAgent = document.body.querySelector<HTMLInputElement>('[data-testid="project-grant-agent-option-agent-template-finance"] input[type="checkbox"]')
    const grantSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-actors-save-button"]')

    expect(grantBuiltinAgent).not.toBeNull()
    expect(grantSaveButton).not.toBeNull()

    grantBuiltinAgent?.click()
    await nextTick()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-teams"]')?.click()
    await nextTick()

    const grantBuiltinTeam = document.body.querySelector<HTMLInputElement>('[data-testid="project-grant-team-option-team-template-finance"] input[type="checkbox"]')
    expect(grantBuiltinTeam).not.toBeNull()

    grantBuiltinTeam?.click()
    await nextTick()
    const updateProjectCallsBeforeGrantSave = updateProjectSpy.mock.calls.length
    grantSaveButton?.click()

    await waitFor(() => {
      mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-agents"]')?.click()
      return document.body
        .querySelector<HTMLInputElement>('[data-testid="project-grant-agent-option-agent-template-finance"] input[type="checkbox"]')
        ?.checked === true
    })
    expect(updateProjectSpy.mock.calls).toHaveLength(updateProjectCallsBeforeGrantSave)

    const reopenedGrantActorsDialog = document.body.querySelector<HTMLElement>('[data-testid="project-settings-actors-dialog"]')
    expect(
      reopenedGrantActorsDialog
        .querySelector<HTMLInputElement>('[data-testid="project-grant-agent-option-agent-template-finance"] input[type="checkbox"]')
        ?.checked,
    ).toBe(true)
    reopenedGrantActorsDialog
      ?.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-teams"]')
      ?.click()
    await nextTick()
    expect(
      reopenedGrantActorsDialog
        .querySelector<HTMLInputElement>('[data-testid="project-grant-team-option-team-template-finance"] input[type="checkbox"]')
        ?.checked,
    ).toBe(true)
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.disabledAgentIds).toEqual([
      'agent-coder',
      'agent-redesign',
    ])
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.disabledTeamIds).toEqual(['team-redesign'])

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-dialog-close"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-actors-dialog"]') === null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-agents"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-actors-dialog"]') !== null)
    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-project"]')?.click()
    await nextTick()

    const runtimeBuiltinAgent = document.body.querySelector<HTMLInputElement>('[data-testid="project-runtime-agent-option-agent-template-finance"] input[type="checkbox"]')
    const runtimeSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-actors-save-button"]')

    expect(runtimeBuiltinAgent).not.toBeNull()
    expect(runtimeSaveButton).not.toBeNull()

    runtimeBuiltinAgent?.click()
    await nextTick()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-teams"]')?.click()
    await nextTick()

    const runtimeBuiltinTeam = document.body.querySelector<HTMLInputElement>('[data-testid="project-runtime-team-option-team-template-finance"] input[type="checkbox"]')
    expect(runtimeBuiltinTeam).not.toBeNull()

    runtimeBuiltinTeam?.click()
    await nextTick()
    runtimeSaveButton?.click()

    await waitFor(() => {
      const settings = workspaceStore.getProjectSettings('proj-redesign').agents
      return Boolean(
        settings?.disabledAgentIds.includes('agent-template-finance')
        && settings?.disabledTeamIds.includes('team-template-finance'),
      )
    })

    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.disabledAgentIds).toEqual([
      'agent-coder',
      'agent-redesign',
      'agent-template-finance',
    ])
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.disabledTeamIds).toEqual([
      'team-redesign',
      'team-template-finance',
    ])

    mounted.destroy()
  })

  it('reads project members from project governance fields inside the member dialog', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        ;(project as any).ownerUserId = 'user-owner'
        ;(project as any).memberUserIds = ['user-owner']
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    expect(mounted.container.textContent).toContain('1 人，其中 1 人可编辑')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-members"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-members-dialog"]') !== null)

    expect(
      document.body
        .querySelector<HTMLInputElement>('[data-testid="project-member-option-user-owner"] input[type="checkbox"]')
        ?.checked,
    ).toBe(true)
    expect(
      document.body
        .querySelector<HTMLInputElement>('[data-testid="project-member-option-user-operator"] input[type="checkbox"]')
        ?.checked,
    ).toBe(false)

    mounted.destroy()
  })

  it('archives and restores the project from the settings lifecycle section', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const notificationStore = useNotificationStore()
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    expect(mounted.container.querySelector('[data-testid="project-settings-lifecycle-section"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-archive-button"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-restore-button"]')).toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-archive-button"]')?.click()

    await waitFor(() =>
      workspaceStore.projects.find(project => project.id === 'proj-redesign')?.status === 'archived',
    )
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.source === 'workspace-project-governance'
        && notification.routeTo === '/workspaces/ws-local/projects/proj-redesign/settings'
        && notification.body?.includes('Desktop Redesign'),
      ),
    )

    expect(mounted.container.querySelector('[data-testid="project-settings-archive-button"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-restore-button"]')).not.toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-restore-button"]')?.click()

    await waitFor(() =>
      workspaceStore.projects.find(project => project.id === 'proj-redesign')?.status === 'active',
    )

    expect(mounted.container.querySelector('[data-testid="project-settings-archive-button"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-restore-button"]')).toBeNull()
    expect(notificationStore.notificationsState.filter(notification =>
      notification.source === 'workspace-project-governance'
      && notification.routeTo === '/workspaces/ws-local/projects/proj-redesign/settings'
      && notification.body?.includes('Desktop Redesign'),
    )).toHaveLength(2)

    mounted.destroy()
  })

  it('creates deletion requests from the settings lifecycle section once the project is archived', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const governance = state.projects.find(project => project.id === 'proj-governance')
        if (!governance) {
          throw new Error('Expected governance project in local workspace fixture')
        }

        governance.status = 'archived'
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-governance/settings')
    const inboxStore = useInboxStore()
    const notificationStore = useNotificationStore()
    const workspaceStore = useWorkspaceStore()
    const inboxBootstrapSpy = vi.spyOn(inboxStore, 'bootstrap')
    const createDeletionRequestSpy = vi.spyOn(workspaceStore, 'createProjectDeletionRequest')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')
    await waitForSelector(mounted.container, '[data-testid="project-settings-request-delete-button"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-request-delete-button"]')?.click()

    await waitFor(() =>
      workspaceStore.getProjectDeletionRequests('proj-governance').some(request => request.status === 'pending'),
    )
    await waitFor(() =>
      inboxBootstrapSpy.mock.calls.some(call => call[0] === undefined && call[1] === true),
    )
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.source === 'workspace-project-governance'
        && notification.routeTo === '/workspaces/ws-local/projects/proj-governance/settings?review=deletion-request'
        && notification.body?.includes('Workspace Governance'),
      ),
    )

    expect(createDeletionRequestSpy).toHaveBeenCalledWith('proj-governance', {})
    expect(mounted.container.querySelector('[data-testid="project-settings-request-delete-button"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-delete-request-status"]')).not.toBeNull()
    expect(notificationStore.notificationsState.some(notification =>
      notification.source === 'workspace-project-governance'
      && notification.routeTo === '/workspaces/ws-local/projects/proj-governance/settings?review=deletion-request'
      && notification.actionLabel
      && notification.body?.includes('Workspace Governance'),
    )).toBe(true)

    mounted.destroy()
  })

  it('supports deletion review deep-links and final delete directly from project settings', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const governance = state.projects.find(project => project.id === 'proj-governance')
        if (!governance) {
          throw new Error('Expected governance project in local workspace fixture')
        }

        governance.status = 'archived'
        state.projectDeletionRequests = [{
          id: 'delete-req-pending',
          workspaceId: state.workspace.id,
          projectId: governance.id,
          requestedByUserId: 'user-owner',
          reviewedByUserId: undefined,
          status: 'pending',
          reviewComment: undefined,
          createdAt: 1,
          updatedAt: 1,
          reviewedAt: undefined,
        }]
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-governance/settings?review=deletion-request')
    const inboxStore = useInboxStore()
    const notificationStore = useNotificationStore()
    const workspaceStore = useWorkspaceStore()
    const inboxBootstrapSpy = vi.spyOn(inboxStore, 'bootstrap')
    const deleteProjectSpy = vi.spyOn(workspaceStore, 'deleteProject')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')
    await waitForSelector(mounted.container, '[data-testid="project-settings-lifecycle-review-callout"]')
    await waitForSelector(mounted.container, '[data-testid="project-settings-delete-request-approve-button"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-delete-request-approve-button"]')?.click()

    await waitFor(() => mounted.container.querySelector('[data-testid="project-settings-delete-project-button"]') !== null)
    await waitFor(() =>
      inboxBootstrapSpy.mock.calls.some(call => call[0] === undefined && call[1] === true),
    )
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.source === 'workspace-project-governance'
        && notification.routeTo === '/workspaces/ws-local/projects/proj-governance/settings?review=deletion-request'
        && notification.body?.includes('Workspace Governance'),
      ),
    )

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-delete-project-button"]')?.click()

    await waitFor(() =>
      workspaceStore.projects.every(project => project.id !== 'proj-governance'),
    )
    await waitFor(() => router.currentRoute.value.name === 'workspace-console-projects')
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.source === 'workspace-project-governance'
        && notification.routeTo === '/workspaces/ws-local/console/projects'
        && notification.body?.includes('Workspace Governance'),
      ),
    )

    expect(deleteProjectSpy).toHaveBeenCalledWith('proj-governance')
    expect(notificationStore.notificationsState.some(notification =>
      notification.source === 'workspace-project-governance'
      && notification.routeTo === '/workspaces/ws-local/console/projects'
      && notification.actionLabel
      && notification.body?.includes('Workspace Governance'),
    )).toBe(true)

    mounted.destroy()
  })

  it('supports rejecting deletion requests from the project settings review state', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const governance = state.projects.find(project => project.id === 'proj-governance')
        if (!governance) {
          throw new Error('Expected governance project in local workspace fixture')
        }

        governance.status = 'archived'
        state.projectDeletionRequests = [{
          id: 'delete-req-pending',
          workspaceId: state.workspace.id,
          projectId: governance.id,
          requestedByUserId: 'user-owner',
          reviewedByUserId: undefined,
          status: 'pending',
          reviewComment: undefined,
          createdAt: 1,
          updatedAt: 1,
          reviewedAt: undefined,
        }]
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-governance/settings?review=deletion-request')
    const inboxStore = useInboxStore()
    const notificationStore = useNotificationStore()
    const workspaceStore = useWorkspaceStore()
    const inboxBootstrapSpy = vi.spyOn(inboxStore, 'bootstrap')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')
    await waitForSelector(mounted.container, '[data-testid="project-settings-delete-request-reject-button"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-delete-request-reject-button"]')?.click()

    await waitFor(() =>
      workspaceStore.getProjectDeletionRequests('proj-governance')[0]?.status === 'rejected',
    )
    await waitFor(() =>
      inboxBootstrapSpy.mock.calls.some(call => call[0] === undefined && call[1] === true),
    )
    await waitFor(() =>
      notificationStore.notificationsState.some(notification =>
        notification.source === 'workspace-project-governance'
        && notification.routeTo === '/workspaces/ws-local/projects/proj-governance/settings'
        && notification.body?.includes('Workspace Governance'),
      ),
    )

    expect(notificationStore.notificationsState.some(notification =>
      notification.source === 'workspace-project-governance'
      && notification.routeTo === '/workspaces/ws-local/projects/proj-governance/settings'
      && notification.actionLabel
      && notification.body?.includes('Workspace Governance'),
    )).toBe(true)

    mounted.destroy()
  })

  it('saves project model quota from the unified capability dialog only', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-models-dialog"]') !== null)
    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-project"]')?.click()
    await nextTick()

    const quotaInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-runtime-total-tokens-input"]')
    const saveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-models-save-button"]')

    expect(quotaInput).not.toBeNull()
    expect(saveButton).not.toBeNull()

    quotaInput!.value = '500000'
    quotaInput!.dispatchEvent(new Event('input', { bubbles: true }))
    saveButton?.click()

    await waitFor(() => workspaceStore.getProjectSettings('proj-redesign').models?.totalTokens === 500000)

    expect(workspaceStore.projects.find(item => item.id === 'proj-redesign')?.assignments).toBeUndefined()
    expect(workspaceStore.getProjectSettings('proj-redesign').models?.allowedConfiguredModelIds).toEqual(['anthropic-primary'])

    mounted.destroy()
  })

  it('saves workspace model baseline selection without routing through project metadata updates', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()
    const updateProjectSpy = vi.spyOn(workspaceStore, 'updateProject')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-models-dialog"]') !== null)

    const secondaryModel = document.body.querySelector<HTMLInputElement>('[data-testid="project-grant-model-option-anthropic-alt"] input[type="checkbox"]')
    const saveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-models-save-button"]')

    expect(secondaryModel).not.toBeNull()
    expect(saveButton).not.toBeNull()

    secondaryModel?.click()
    await nextTick()

    const updateProjectCallsBeforeSave = updateProjectSpy.mock.calls.length
    saveButton?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-models-dialog"]') === null)
    expect(updateProjectSpy.mock.calls).toHaveLength(updateProjectCallsBeforeSave)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-edit-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-models-dialog"]') !== null)
    const reopenedGrantModelsDialog = document.body.querySelector<HTMLElement>('[data-testid="project-settings-models-dialog"]')
    expect(
      reopenedGrantModelsDialog
        .querySelector<HTMLInputElement>('[data-testid="project-grant-model-option-anthropic-primary"] input[type="checkbox"]')
        ?.checked,
    ).toBe(true)
    expect(
      reopenedGrantModelsDialog
        .querySelector<HTMLInputElement>('[data-testid="project-grant-model-option-anthropic-alt"] input[type="checkbox"]')
        ?.checked,
    ).toBe(false)

    mounted.destroy()
  })
})
