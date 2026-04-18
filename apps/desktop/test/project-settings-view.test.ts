// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useAgentStore } from '@/stores/agent'
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

  it('edits the project leader from overview and persists the selected workspace agent', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        project.leaderAgentId = 'agent-architect'
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    expect(mounted.container.querySelector('[data-testid="project-settings-overview-leader-card"]')?.textContent).toContain('Architect Agent')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-overview"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-overview-dialog"]') !== null)

    const leaderSelect = document.body.querySelector<HTMLSelectElement>('[data-testid="project-settings-overview-leader-select"]')
    const leaderHint = document.body.querySelector<HTMLElement>('[data-testid="project-settings-overview-leader-hint"]')
    const saveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-overview-save-button"]')

    expect(leaderSelect).not.toBeNull()
    expect(leaderHint?.textContent).toContain('Leader')
    expect(saveButton).not.toBeNull()

    const leaderOptionLabels = Array.from(leaderSelect?.querySelectorAll('option') ?? []).map(option => option.textContent?.trim())
    expect(leaderOptionLabels).toContain('Architect Agent')
    expect(leaderOptionLabels).toContain('Coder Agent')
    expect(leaderOptionLabels).not.toContain('Finance Planner Template')

    leaderSelect!.value = 'agent-coder'
    leaderSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    saveButton?.click()

    await waitFor(() =>
      workspaceStore.projects.find(item => item.id === 'proj-redesign')?.leaderAgentId === 'agent-coder',
    )

    expect(workspaceStore.projects.find(item => item.id === 'proj-redesign')?.leaderAgentId).toBe('agent-coder')

    mounted.destroy()
  })

  it('supports tabbed tool dialogs with search and tab-scoped bulk actions', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        project.assignments = {
          ...(project.assignments ?? {}),
          tools: {
            excludedSourceKeys: [],
          },
        }

        const projectConfig = state.runtimeProjectConfigs['proj-redesign']
        const projectSource = projectConfig?.sources.find(source => source.scope === 'project')
        if (!projectSource) {
          throw new Error('Expected proj-redesign runtime project source')
        }

        ;(projectSource as any).document = {
          ...((projectSource as any).document ?? {}),
          projectSettings: {
            ...((projectSource as any).document?.projectSettings ?? {}),
            tools: {
              disabledSourceKeys: [],
              overrides: {},
            },
          },
        }
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-tools"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-tools-dialog"]') !== null)

    const grantSearchInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-grant-tools-search-input"]')
    const grantClearButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grant-tools-clear-visible-button"]')
    const grantSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-tools-save-button"]')

    expect(document.body.querySelector('[data-testid="ui-tabs-trigger-builtin"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="ui-tabs-trigger-skill"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="ui-tabs-trigger-mcp"]')).not.toBeNull()
    expect(grantSearchInput).not.toBeNull()
    expect(grantClearButton).not.toBeNull()
    expect(grantSaveButton).not.toBeNull()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-skill"]')?.click()
    await nextTick()

    grantSearchInput!.value = 'external-checks'
    grantSearchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(
      document.body.querySelector('[data-testid="project-grant-tool-option-skill:.codex/skills/external-checks/SKILL.md"]'),
    ).not.toBeNull()
    expect(
      document.body.querySelector('[data-testid="project-grant-tool-option-skill:.claude/skills/external-help/SKILL.md"]'),
    ).toBeNull()

    grantClearButton?.click()
    await nextTick()

    grantSaveButton?.click()

    await waitFor(() => {
      const project = workspaceStore.projects.find(item => item.id === 'proj-redesign')
      return project?.assignments?.tools?.excludedSourceKeys?.includes('skill:.codex/skills/external-checks/SKILL.md') === true
    })

    expect(workspaceStore.projects.find(item => item.id === 'proj-redesign')?.assignments?.tools?.excludedSourceKeys).toEqual([
      'skill:.codex/skills/external-checks/SKILL.md',
    ])

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-runtime-tools"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-runtime-tools-dialog"]') !== null)

    const runtimeSearchInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-runtime-tools-search-input"]')
    const runtimeClearButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-tools-clear-visible-button"]')
    const runtimeSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-tools-save-button"]')

    expect(runtimeSearchInput).not.toBeNull()
    expect(runtimeClearButton).not.toBeNull()
    expect(runtimeSaveButton).not.toBeNull()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-skill"]')?.click()
    await nextTick()
    runtimeClearButton?.click()
    await nextTick()

    runtimeSaveButton?.click()

    await waitFor(() =>
      workspaceStore.getProjectSettings('proj-redesign').tools?.disabledSourceKeys?.length === 4,
    )

    expect(workspaceStore.getProjectSettings('proj-redesign').tools?.disabledSourceKeys).toEqual([
      'skill:data/skills/help/SKILL.md',
      'skill:.claude/skills/external-help/SKILL.md',
      'skill:builtin-assets/skills/financial-calculator/SKILL.md',
      'skill:data/projects/proj-redesign/skills/redesign-review/SKILL.md',
    ])

    mounted.destroy()
  })

  it('saves project grants and runtime actor refinement through separate flows', async () => {
    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-actors"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-actors-dialog"]') !== null)

    const grantSearchInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-grant-actors-search-input"]')
    const grantSelectAllButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grant-actors-select-all-visible-button"]')
    const grantSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-actors-save-button"]')

    expect(document.body.querySelector('[data-testid="ui-tabs-trigger-agents"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="ui-tabs-trigger-teams"]')).not.toBeNull()
    expect(grantSearchInput).not.toBeNull()
    expect(grantSelectAllButton).not.toBeNull()
    expect(grantSaveButton).not.toBeNull()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-teams"]')?.click()
    await nextTick()

    grantSearchInput!.value = 'Finance Ops Template'
    grantSearchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    grantSelectAllButton?.click()
    await nextTick()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-agents"]')?.click()
    await nextTick()

    grantSearchInput!.value = 'Finance Planner Template'
    grantSearchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    grantSelectAllButton?.click()
    await nextTick()
    grantSaveButton?.click()

    await waitFor(() => {
      const project = workspaceStore.projects.find(item => item.id === 'proj-redesign')
      const assignments = project?.assignments?.agents
      return Boolean(
        assignments?.excludedAgentIds
        && assignments.excludedTeamIds,
      )
    })

    const projectAfterGrant = workspaceStore.projects.find(item => item.id === 'proj-redesign')
    expect(projectAfterGrant?.assignments?.agents?.excludedAgentIds).toEqual(['agent-coder'])
    expect(projectAfterGrant?.assignments?.agents?.excludedTeamIds).toEqual([])

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-runtime-actors"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-runtime-actors-dialog"]') !== null)

    const runtimeSearchInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-runtime-actors-search-input"]')
    const runtimeSelectAllButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-actors-select-all-visible-button"]')
    const runtimeSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-actors-save-button"]')

    expect(document.body.querySelector('[data-testid="ui-tabs-trigger-agents"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="ui-tabs-trigger-teams"]')).not.toBeNull()
    expect(runtimeSearchInput).not.toBeNull()
    expect(runtimeSelectAllButton).not.toBeNull()
    expect(runtimeSaveButton).not.toBeNull()

    runtimeSearchInput!.value = 'Finance Planner Template'
    runtimeSearchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    runtimeSelectAllButton?.click()
    await nextTick()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-teams"]')?.click()
    await nextTick()

    runtimeSearchInput!.value = 'Finance Ops Template'
    runtimeSearchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    runtimeSelectAllButton?.click()
    await nextTick()
    runtimeSaveButton?.click()

    await waitFor(() => {
      const settings = workspaceStore.getProjectSettings('proj-redesign').agents
      return Boolean(
        settings?.disabledAgentIds
        && settings?.disabledTeamIds,
      )
    })

    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.disabledAgentIds).toEqual([])
    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.disabledTeamIds).toEqual([])

    mounted.destroy()
  })

  it('keeps the current leader granted and enabled inside actor dialogs', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign fixture project')
        }

        project.leaderAgentId = 'agent-architect'
        project.assignments = {
          ...(project.assignments ?? {}),
          agents: {
            excludedAgentIds: [],
            excludedTeamIds: [],
          },
        }

        const projectConfig = state.runtimeProjectConfigs['proj-redesign']
        const projectSource = projectConfig?.sources.find(source => source.scope === 'project')
        if (!projectSource) {
          throw new Error('Expected proj-redesign runtime project source')
        }

        ;(projectSource as any).document = {
          ...((projectSource as any).document ?? {}),
          projectSettings: {
            ...((projectSource as any).document?.projectSettings ?? {}),
            agents: {
              disabledAgentIds: [],
              disabledTeamIds: [],
            },
          },
        }
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-actors"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-actors-dialog"]') !== null)

    const grantSearchInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-grant-actors-search-input"]')
    const grantClearButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grant-actors-clear-visible-button"]')
    const grantSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-actors-save-button"]')

    grantSearchInput!.value = 'Architect Agent'
    grantSearchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(document.body.querySelector('[data-testid="project-grant-agent-leader-badge-agent-architect"]')).not.toBeNull()

    grantClearButton?.click()
    await nextTick()
    grantSaveButton?.click()

    await waitFor(() => Array.isArray(workspaceStore.projects.find(item => item.id === 'proj-redesign')?.assignments?.agents?.excludedAgentIds))

    expect(workspaceStore.projects.find(item => item.id === 'proj-redesign')?.assignments?.agents?.excludedAgentIds).not.toContain('agent-architect')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-runtime-actors"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-runtime-actors-dialog"]') !== null)

    const runtimeSearchInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-runtime-actors-search-input"]')
    const runtimeClearButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-actors-clear-visible-button"]')
    const runtimeSaveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-runtime-actors-save-button"]')

    runtimeSearchInput!.value = 'Architect Agent'
    runtimeSearchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(document.body.querySelector('[data-testid="project-runtime-agent-leader-badge-agent-architect"]')).not.toBeNull()

    runtimeClearButton?.click()
    await nextTick()
    runtimeSaveButton?.click()

    await waitFor(() => Array.isArray(workspaceStore.getProjectSettings('proj-redesign').agents?.disabledAgentIds))

    expect(workspaceStore.getProjectSettings('proj-redesign').agents?.disabledAgentIds).not.toContain('agent-architect')

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

  it('shows promoted project assets as selectable workspace candidates in other project settings', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-governance')
        if (!project) {
          throw new Error('Expected proj-governance fixture project')
        }

        project.assignments = {
          ...(project.assignments ?? {}),
          agents: {
            agentIds: [],
            teamIds: [],
          },
        }
      },
    })

    const promotedAssetsApp = await mountRoutedApp('/workspaces/ws-local/projects/proj-redesign/agents')
    const agentStore = useAgentStore()
    const teamStore = useTeamStore()

    await waitFor(() => agentStore.projectOwnedAgents.some(agent => agent.id === 'agent-redesign'))
    await waitFor(() => teamStore.projectOwnedTeams.some(team => team.id === 'team-redesign'))

    await agentStore.copyToWorkspace('agent-redesign')
    await teamStore.copyToWorkspace('team-redesign')

    promotedAssetsApp.destroy()

    const mounted = await mountRoutedApp('/workspaces/ws-local/projects/proj-governance/settings')
    const workspaceStore = useWorkspaceStore()

    await waitForSelector(mounted.container, '[data-testid="project-settings-view"]')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="project-settings-open-grants-actors"]')?.click()
    await waitFor(() => document.body.querySelector('[data-testid="project-settings-grants-actors-dialog"]') !== null)

    const searchInput = document.body.querySelector<HTMLInputElement>('[data-testid="project-settings-grant-actors-search-input"]')
    const selectAllButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grant-actors-select-all-visible-button"]')
    const saveButton = document.body.querySelector<HTMLButtonElement>('[data-testid="project-settings-grants-actors-save-button"]')
    expect(searchInput).not.toBeNull()
    expect(selectAllButton).not.toBeNull()
    expect(saveButton).not.toBeNull()

    searchInput!.value = 'Redesign Copilot'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(document.body.querySelector('[data-testid="project-grant-agent-option-agent-workspace-redesign-copilot-copy"]')).not.toBeNull()
    selectAllButton?.click()
    await nextTick()

    document.body.querySelector<HTMLButtonElement>('[data-testid="ui-tabs-trigger-teams"]')?.click()
    await nextTick()

    searchInput!.value = 'Redesign Tiger Team'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(document.body.querySelector('[data-testid="project-grant-team-option-team-workspace-redesign-tiger-team-copy"]')).not.toBeNull()
    selectAllButton?.click()
    await nextTick()

    saveButton?.click()

    await waitFor(() => {
      const project = workspaceStore.projects.find(item => item.id === 'proj-governance')
      const assignments = project?.assignments?.agents
      return Boolean(
        assignments?.excludedAgentIds
        && assignments?.excludedTeamIds
        && !assignments.excludedAgentIds.includes('agent-workspace-redesign-copilot-copy')
        && !assignments.excludedTeamIds.includes('team-workspace-redesign-tiger-team-copy'),
      )
    })

    const projectAssignments = workspaceStore.projects.find(item => item.id === 'proj-governance')?.assignments?.agents
    expect(projectAssignments?.excludedAgentIds).not.toContain('agent-workspace-redesign-copilot-copy')
    expect(projectAssignments?.excludedTeamIds).not.toContain('team-workspace-redesign-tiger-team-copy')

    mounted.destroy()
  })
})
