// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useTeamStore } from '@/stores/team'
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

  it('renders document sections instead of tabs and keeps runtime inputs inside dialogs', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    expect(mounted.container.querySelector('[data-testid="project-settings-overview-section"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-grants-section"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-runtime-section"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-members-section"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="ui-tabs-trigger-basics"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-settings-name-input"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="project-runtime-total-tokens-input"]')).toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-runtime-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-runtime-models-dialog"]') !== null)

    expect(document.body.querySelector('[data-testid="project-runtime-total-tokens-input"]')).not.toBeNull()

    mounted.destroy()
  })

  it('keeps grant dialogs inside the shared scrollable dialog shell', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-models-dialog"]') !== null)

    const dialogContent = document.body.querySelector<HTMLElement>('[data-testid="project-settings-grants-models-dialog"]')
    const dialogBody = document.body.querySelector<HTMLElement>('[data-testid="ui-dialog-body"]')

    expect(dialogContent).not.toBeNull()
    expect(dialogContent?.className).toContain('max-h-[calc(100dvh-2rem)]')
    expect(dialogBody).not.toBeNull()
    expect(dialogBody?.className).toContain('overflow-y-auto')

    mounted.destroy()
  })

  it('supports select all and clear all actions across grant dialogs', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()
    const catalogStore = useCatalogStore()
    const agentStore = useAgentStore()
    const teamStore = useTeamStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    const workspaceModelCount = catalogStore.configuredModelOptions.length
    const workspaceToolCount = catalogStore.managementProjection.assets.filter(entry => entry.enabled).length
    const workspaceAgentCount = agentStore.workspaceOwnedAgents.length + agentStore.builtinTemplateAgents.length
    const workspaceTeamCount = teamStore.workspaceOwnedTeams.length + teamStore.builtinTemplateTeams.length

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-models-dialog"]') !== null)

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
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-models-dialog"]') === null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-tools"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-tools-dialog"]') !== null)

    const selectAllToolsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-select-all"]')
    const clearAllToolsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-clear-all"]')
    const saveGrantToolsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-save-button"]')

    expect(selectAllToolsButton).not.toBeNull()
    expect(clearAllToolsButton).not.toBeNull()
    expect(saveGrantToolsButton).not.toBeNull()

    selectAllToolsButton?.click()
    await nextTick()
    saveGrantToolsButton?.click()

    await waitFor(() => {
      const project = workspaceStore.projects.find(item => item.id === 'proj-redesign')
      return project?.assignments?.tools?.sourceKeys.length === workspaceToolCount
    })

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-tools"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-tools-dialog"]') !== null)

    document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-clear-all"]')?.click()
    await nextTick()
    document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-save-button"]')?.click()

    await waitFor(() => {
      const project = workspaceStore.projects.find(item => item.id === 'proj-redesign')
      return project?.assignments?.tools == null
    })

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-actors"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-actors-dialog"]') !== null)

    const selectAllAgentsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-agents-select-all"]')
    const clearAllAgentsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-agents-clear-all"]')
    const selectAllTeamsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-teams-select-all"]')
    const clearAllTeamsButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-teams-clear-all"]')

    expect(selectAllAgentsButton).not.toBeNull()
    expect(clearAllAgentsButton).not.toBeNull()
    expect(selectAllTeamsButton).not.toBeNull()
    expect(clearAllTeamsButton).not.toBeNull()

    selectAllAgentsButton?.click()
    selectAllTeamsButton?.click()
    await nextTick()

    const agentCheckboxes = Array.from(
      document.body.querySelectorAll<HTMLInputElement>('[data-testid^="project-grant-agent-option-"] input[type="checkbox"]'),
    )
    const teamCheckboxes = Array.from(
      document.body.querySelectorAll<HTMLInputElement>('[data-testid^="project-grant-team-option-"] input[type="checkbox"]'),
    )

    expect(agentCheckboxes).toHaveLength(workspaceAgentCount)
    expect(teamCheckboxes).toHaveLength(workspaceTeamCount)
    expect(agentCheckboxes.every(input => input.checked)).toBe(true)
    expect(teamCheckboxes.every(input => input.checked)).toBe(true)

    clearAllAgentsButton?.click()
    clearAllTeamsButton?.click()
    await nextTick()

    expect(agentCheckboxes.every(input => !input.checked)).toBe(true)
    expect(teamCheckboxes.every(input => !input.checked)).toBe(true)

    mounted.destroy()
  })

  it('shows grant and runtime summaries separately on the document page', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    const grantsSection = mounted.container.querySelector('[data-testid="project-settings-grants-section"]')
    const runtimeSection = mounted.container.querySelector('[data-testid="project-settings-runtime-section"]')

    expect(grantsSection?.textContent).toContain('已授予 2 个，默认 Claude Primary')
    expect(runtimeSection?.textContent).toContain('已授予 2 个，启用 1 个，默认 Claude Primary')
    expect(runtimeSection?.textContent).toContain('已启用 1 个工具')

    mounted.destroy()
  })

  it('saves project grants and runtime actor refinement through separate flows', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-actors"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-actors-dialog"]') !== null)

    const grantBuiltinAgent = document.body.querySelector<HTMLLabelElement>('[data-testid="project-grant-agent-option-agent-template-finance"]')
    const grantBuiltinTeam = document.body.querySelector<HTMLLabelElement>('[data-testid="project-grant-team-option-team-template-finance"]')
    const grantSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-actors-save-button"]')

    expect(grantBuiltinAgent).not.toBeNull()
    expect(grantBuiltinTeam).not.toBeNull()
    expect(grantSaveButton).not.toBeNull()

    grantBuiltinAgent?.click()
    grantBuiltinTeam?.click()
    await nextTick()
    grantSaveButton?.click()

    await waitFor(() => {
      const project = workspaceStore.projects.find(item => item.id === 'proj-redesign')
      const assignments = project?.assignments?.agents
      return Boolean(
        assignments?.agentIds.includes('agent-template-finance')
        && assignments?.teamIds.includes('team-template-finance'),
      )
    })

    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.enabledAgentIds).toEqual(['agent-architect'])
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.enabledTeamIds).toEqual(['team-studio'])

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-runtime-actors"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-runtime-actors-dialog"]') !== null)

    const runtimeBuiltinAgent = document.body.querySelector<HTMLLabelElement>('[data-testid="project-runtime-agent-option-agent-template-finance"]')
    const runtimeBuiltinTeam = document.body.querySelector<HTMLLabelElement>('[data-testid="project-runtime-team-option-team-template-finance"]')
    const runtimeSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-actors-save-button"]')

    expect(runtimeBuiltinAgent).not.toBeNull()
    expect(runtimeBuiltinTeam).not.toBeNull()
    expect(runtimeSaveButton).not.toBeNull()

    runtimeBuiltinAgent?.click()
    runtimeBuiltinTeam?.click()
    await nextTick()
    runtimeSaveButton?.click()

    await waitFor(() => {
      const settings = workspaceStore.getProjectSettings('proj-redesign').agents
      return Boolean(
        settings?.enabledAgentIds.includes('agent-template-finance')
        && settings?.enabledTeamIds.includes('team-template-finance'),
      )
    })

    const project = workspaceStore.projects.find(item => item.id === 'proj-redesign')
    expect(project?.assignments?.agents?.agentIds).toEqual(['agent-architect', 'agent-template-finance'])
    expect(project?.assignments?.agents?.teamIds).toEqual(['team-studio', 'team-template-finance'])
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.enabledAgentIds).toEqual([
      'agent-architect',
      'agent-template-finance',
    ])
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.enabledTeamIds).toEqual([
      'team-studio',
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

  it('saves runtime model quota from the runtime dialog only', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-runtime-models"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-runtime-models-dialog"]') !== null)

    const quotaInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-runtime-total-tokens-input"]')
    const saveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-models-save-button"]')

    expect(quotaInput).not.toBeNull()
    expect(saveButton).not.toBeNull()

    quotaInput!.value = '500000'
    quotaInput!.dispatchEvent(new Event('input', { bubbles: true }))
    saveButton?.click()

    await waitFor(() => workspaceStore.getProjectSettings('proj-redesign').models?.totalTokens === 500000)

    expect(workspaceStore.projects.find(item => item.id === 'proj-redesign')?.assignments?.models?.configuredModelIds).toEqual([
      'anthropic-primary',
      'anthropic-alt',
    ])
    expect(workspaceStore.getProjectSettings('proj-redesign').models?.allowedConfiguredModelIds).toEqual(['anthropic-primary'])

    mounted.destroy()
  })
})
