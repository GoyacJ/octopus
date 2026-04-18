// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import * as tauriClient from '@/tauri/client'
import ProjectsView from '@/views/workspace/ProjectsView.vue'
import { useShellStore } from '@/stores/shell'
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

async function mountProjectsView(
  props?: Record<string, unknown>,
  workspaceId = 'ws-local',
  projectId = 'proj-redesign',
) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  await router.push('/workspaces/ws-local/console/projects')
  await router.isReady()

  const app = createApp(ProjectsView, props)
  app.use(pinia)
  app.use(i18n)
  app.use(router)
  app.mount(container)

  const shellStore = useShellStore()
  await shellStore.bootstrap(workspaceId, projectId)
  await nextTick()

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
      throw new Error('Timed out waiting for projects management state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Workspace project management view', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
    await router.push('/workspaces/ws-local/console/projects')
    await router.isReady()
  })

  it('renders the project management view from workspace project data', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="workspace-projects-embedded"]') !== null)

    expect(mounted.container.querySelector('[data-testid="workspace-console-view"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-projects-embedded"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Workspace Governance')
    expect(mounted.container.querySelector('[data-testid="projects-name-input"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-description-input"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-preset-select"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-leader-select"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-inheritance-hint"]')?.textContent).toContain('继承工作区')
    expect(mounted.container.querySelector('[data-testid="projects-summary-models"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-summary-tools"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-default-model-select"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-total-tokens-input"]')).toBeNull()

    mounted.destroy()
  })

  it('renders a lightweight registry detail with capability summaries and an advanced settings entry', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="workspace-projects-embedded"]') !== null)

    expect(mounted.container.querySelector('[data-testid="projects-summary-models"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-summary-tools"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-summary-actors"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-summary-members"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-open-settings-button"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Claude Primary')

    mounted.destroy()
  })

  it('supports an embedded mode without rendering the standalone page shell', async () => {
    const mounted = await mountProjectsView({ embedded: true })

    await waitFor(() => mounted.container.querySelector('[data-testid="workspace-projects-embedded"]') !== null)

    expect(mounted.container.querySelector('[data-testid="workspace-projects-embedded"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-projects-view"]')).toBeNull()
    expect(mounted.container.textContent).toContain('Desktop Redesign')

    mounted.destroy()
  })

  it('uses the shared list row grammar for project selection inside the list-detail workspace', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-select-proj-redesign"]') !== null)

    const shell = mounted.container.querySelector<HTMLElement>('[data-testid="ui-list-detail-shell"]')
    const projectRow = mounted.container.querySelector<HTMLElement>('[data-testid="projects-select-proj-redesign"]')

    expect(shell).not.toBeNull()
    expect(shell?.className).toContain('gap-px')
    expect(projectRow).not.toBeNull()

    projectRow?.click()
    await nextTick()

    expect(projectRow?.getAttribute('data-ui-state')).toBe('active')
    expect(projectRow?.className).toContain('bg-accent')

    mounted.destroy()
  })

  it('creates a new project with minimum required fields only', async () => {
    vi.spyOn(tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> }, 'pickResourceDirectory')
      .mockResolvedValue('/workspace/projects/agent-studio/resources')

    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-create-header-button"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-create-header-button"]')?.click()
    await nextTick()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-name-input"]')
    const leaderSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="projects-leader-select"]')
    expect(nameInput).not.toBeNull()
    expect(leaderSelect).not.toBeNull()
    const leaderOptionLabels = Array.from(leaderSelect?.querySelectorAll('option') ?? []).map(option => option.textContent?.trim())
    expect(leaderOptionLabels).toContain('Architect Agent')
    expect(leaderOptionLabels).toContain('Coder Agent')
    expect(leaderOptionLabels).not.toContain('Finance Planner Template')

    nameInput!.value = 'Agent Studio'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    leaderSelect!.value = 'agent-coder'
    leaderSelect!.dispatchEvent(new Event('change', { bubbles: true }))

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-resource-directory-pick"]')?.click()
    await waitFor(() =>
      mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-resource-directory-path"]')?.value
        === '/workspace/projects/agent-studio/resources',
    )

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-create-button"]')?.click()

    await waitFor(() => workspaceStore.projects.some(project => project.name === 'Agent Studio'))

    const created = workspaceStore.projects.find(project => project.name === 'Agent Studio')
    expect(created).toBeTruthy()
    expect(created?.description).toBe('')
    expect(created?.leaderAgentId).toBe('agent-coder')
    expect(created?.assignments).toEqual({
      tools: {
        excludedSourceKeys: [],
      },
      agents: {
        excludedAgentIds: [],
        excludedTeamIds: [],
      },
    })
    expect(mounted.container.querySelector('[data-testid="projects-open-settings-button"]')).not.toBeNull()

    mounted.destroy()
  })

  it('updates preset summary and uses preset seeding without exposing advanced lists', async () => {
    vi.spyOn(tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> }, 'pickResourceDirectory')
      .mockResolvedValue('/workspace/projects/docs-workbench/resources')

    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-create-header-button"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-create-header-button"]')?.click()
    await nextTick()

    const presetSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="projects-preset-select"]')
    expect(presetSelect).not.toBeNull()

    presetSelect!.value = 'documentation'
    presetSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="projects-summary-models"]')?.textContent).toContain('默认 GPT-4o')
    expect(mounted.container.querySelector('[data-testid="projects-default-model-select"]')).toBeNull()

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-name-input"]')
    const leaderSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="projects-leader-select"]')
    nameInput!.value = 'Docs Workbench'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    leaderSelect!.value = 'agent-architect'
    leaderSelect!.dispatchEvent(new Event('change', { bubbles: true }))

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-resource-directory-pick"]')?.click()
    await waitFor(() =>
      mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-resource-directory-path"]')?.value
        === '/workspace/projects/docs-workbench/resources',
    )

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-create-button"]')?.click()

    await waitFor(() => workspaceStore.projects.some(project => project.name === 'Docs Workbench'))

    const created = workspaceStore.projects.find(project => project.name === 'Docs Workbench')
    expect(created?.leaderAgentId).toBe('agent-architect')
    expect(created?.assignments?.models?.configuredModelIds).toHaveLength(1)
    expect(created?.assignments?.models?.defaultConfiguredModelId).toBe(
      created?.assignments?.models?.configuredModelIds[0],
    )
    expect(created?.assignments?.tools).toEqual({
      excludedSourceKeys: [],
    })
    expect(created?.assignments?.agents).toEqual({
      excludedAgentIds: [],
      excludedTeamIds: [],
    })

    mounted.destroy()
  })

  it('opens project settings from the registry summary action', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-open-settings-button"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-open-settings-button"]')?.click()

    await waitFor(() => router.currentRoute.value.name === 'project-settings')
    expect(router.currentRoute.value.name).toBe('project-settings')
    expect(String(router.currentRoute.value.params.projectId)).toBe('proj-redesign')

    mounted.destroy()
  })

  it('archives and restores the selected project without changing the registry flow', async () => {
    const mounted = await mountProjectsView()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-archive-button"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-archive-button"]')?.click()

    await waitFor(() =>
      workspaceStore.projects.find(project => project.id === 'proj-redesign')?.status === 'archived',
    )

    mounted.container.querySelector<HTMLElement>('[data-testid="projects-select-proj-redesign"]')?.click()
    await waitFor(() => mounted.container.querySelector('[data-testid="projects-restore-button"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-restore-button"]')?.click()

    await waitFor(() =>
      workspaceStore.projects.find(project => project.id === 'proj-redesign')?.status === 'active',
    )

    mounted.destroy()
  })

  it('prevents archiving the last active project and surfaces an error', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const redesign = state.projects.find(project => project.id === 'proj-redesign')
        const governance = state.projects.find(project => project.id === 'proj-governance')

        if (!redesign || !governance) {
          throw new Error('Expected local workspace fixture projects')
        }

        governance.status = 'archived'
      },
    })

    const mounted = await mountProjectsView()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-archive-button"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-archive-button"]')?.click()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-error"]') !== null)

    expect(mounted.container.textContent).toContain(String(i18n.global.t('projects.errors.lastActiveProject')))

    mounted.destroy()
  })
})
