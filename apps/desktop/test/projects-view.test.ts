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

  const app = createApp(ProjectsView, props)
  app.use(pinia)
  app.use(i18n)
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
    expect(mounted.container.querySelector('[data-testid="projects-default-model-select"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.tools.groups.builtin')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.tools.groups.skill')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projectSettings.tools.groups.mcp')))

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

  it('creates a new active project and persists workspace assignments', async () => {
    vi.spyOn(tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> }, 'pickResourceDirectory')
      .mockResolvedValue('/workspace/projects/agent-studio/resources')

    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-create-button"]') !== null)

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-name-input"]')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="projects-description-input"]')
    const defaultModelSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="projects-default-model-select"]')
    expect(nameInput).not.toBeNull()
    expect(descriptionInput).not.toBeNull()
    expect(defaultModelSelect).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="projects-resource-directory-pick"]')).not.toBeNull()

    nameInput!.value = 'Agent Studio'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Project management workspace surface.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))
    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-resource-directory-pick"]')?.click()
    await waitFor(() =>
      mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-resource-directory-path"]')?.value
        === '/workspace/projects/agent-studio/resources',
    )

    mounted.container.querySelector<HTMLElement>('[aria-label="Claude Primary"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="bash"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="help"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="ops"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="Architect Agent"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLElement>('[aria-label="Studio Direction Team"]')?.click()
    await nextTick()

    defaultModelSelect!.value = 'anthropic-primary'
    defaultModelSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-create-button"]')?.click()

    await waitFor(() => mounted.container.textContent?.includes('Agent Studio') ?? false)
    const created = workspaceStore.projects.find(project => project.name === 'Agent Studio')
    expect(created).toBeTruthy()
    expect(created?.assignments?.models?.configuredModelIds).toEqual(['anthropic-primary'])
    expect(created?.assignments?.models?.defaultConfiguredModelId).toBe('anthropic-primary')
    expect(created?.assignments?.tools?.sourceKeys).toEqual([
      'builtin:bash',
      'skill:data/skills/help/SKILL.md',
      'mcp:ops',
    ])
    expect(created?.assignments?.agents?.agentIds).toEqual(['agent-architect'])
    expect(created?.assignments?.agents?.teamIds).toEqual(['team-studio'])

    mounted.destroy()
  })

  it('requires selecting a resource directory when creating a project', async () => {
    vi.spyOn(tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> }, 'pickResourceDirectory')
      .mockResolvedValue('/workspace/projects/agent-studio/resources')

    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-resource-directory-pick"]') !== null)

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-name-input"]')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="projects-description-input"]')
    const resourceDirectoryPath = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-resource-directory-path"]')
    const pickButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-resource-directory-pick"]')

    expect(resourceDirectoryPath).not.toBeNull()
    expect(pickButton).not.toBeNull()

    nameInput!.value = 'Agent Studio'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Project management workspace surface.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))

    pickButton!.click()
    await waitFor(() => resourceDirectoryPath!.value === '/workspace/projects/agent-studio/resources')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-create-button"]')?.click()

    await waitFor(() => workspaceStore.projects.some(project => project.name === 'Agent Studio'))

    const created = workspaceStore.projects.find(project => project.name === 'Agent Studio')
    expect(created?.resourceDirectory).toBe('/workspace/projects/agent-studio/resources')

    mounted.destroy()
  })

  it('creates a project with a remotely selected resource directory', async () => {
    const pickResourceDirectorySpy = vi.spyOn(
      tauriClient as unknown as { pickResourceDirectory: () => Promise<string | null> },
      'pickResourceDirectory',
    ).mockResolvedValue('/local/path/should/not/be/used')

    const mounted = await mountProjectsView(undefined, 'ws-enterprise', 'proj-launch')
    const shellStore = useShellStore()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-resource-directory-pick"]') !== null)

    expect(shellStore.activeWorkspaceConnection?.workspaceId).toBe('ws-enterprise')
    expect(shellStore.activeWorkspaceConnection?.transportSecurity).toBe('trusted')

    const nameInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-name-input"]')
    const descriptionInput = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="projects-description-input"]')
    const resourceDirectoryPath = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-resource-directory-path"]')

    nameInput!.value = 'Remote Studio'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    descriptionInput!.value = 'Remote project resource directory selection.'
    descriptionInput!.dispatchEvent(new Event('input', { bubbles: true }))

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-resource-directory-pick"]')?.click()

    await waitFor(() => document.body.querySelector('[data-testid="remote-resource-directory-dialog"]') !== null)

    const waitForDialogPath = async (path: string) => {
      await waitFor(() =>
        document.body.querySelector('[data-testid="remote-resource-directory-dialog"]')?.textContent?.includes(path) ?? false,
      )
    }

    const clickDialogEntry = async (label: string, expectedPath: string) => {
      await waitFor(() => {
        const dialog = document.body.querySelector('[data-testid="remote-resource-directory-dialog"]')
        return Array.from(dialog?.querySelectorAll('button') ?? [])
          .some(item => item.textContent?.includes(label))
      })

      const dialog = document.body.querySelector('[data-testid="remote-resource-directory-dialog"]')
      const button = Array.from(dialog?.querySelectorAll('button') ?? [])
        .find(item => item.textContent?.includes(label))
      button?.click()
      await waitForDialogPath(expectedPath)
    }

    await waitForDialogPath('/remote')
    await clickDialogEntry('projects', '/remote/projects')
    await clickDialogEntry('launch-readiness', '/remote/projects/launch-readiness')
    await clickDialogEntry('resources', '/remote/projects/launch-readiness/resources')

    Array.from(document.body.querySelectorAll('button'))
      .find(button => button.textContent?.includes(String(i18n.global.t('resources.remoteBrowser.actions.chooseCurrent'))))
      ?.click()

    await waitFor(() =>
      resourceDirectoryPath?.value === '/remote/projects/launch-readiness/resources',
    )

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-create-button"]')?.click()

    await waitFor(() => workspaceStore.projects.some(project => project.name === 'Remote Studio'))

    const created = workspaceStore.projects.find(project => project.name === 'Remote Studio')
    expect(created?.resourceDirectory).toBe('/remote/projects/launch-readiness/resources')
    expect(pickResourceDirectorySpy).not.toHaveBeenCalled()

    mounted.destroy()
  })

  it('selects the edited project as the active project scope for downstream project surfaces', async () => {
    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-select-proj-governance"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-governance"]')?.click()
    await waitFor(() => workspaceStore.currentProjectId === 'proj-governance')

    expect(workspaceStore.currentProjectId).toBe('proj-governance')

    mounted.destroy()
  })

  it('shows and saves token quota fields in the selected project detail', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const projectSource = state.runtimeProjectConfigs['proj-redesign']?.sources.find(source => source.scope === 'project')
        const projectDocument = (projectSource?.document ?? {}) as Record<string, any>
        projectSource!.document = projectDocument
        const projectSettings = (projectDocument.projectSettings ??= {}) as Record<string, any>
        const models = (projectSettings.models ??= {}) as Record<string, any>

        models.totalTokens = 500000
        ;(state.dashboards['proj-redesign'] as Record<string, any>).usedTokens = 125000
      },
    })

    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-select-proj-redesign"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-redesign"]')?.click()
    await waitFor(() => mounted.container.querySelector('[data-testid="projects-total-tokens-input"]') !== null)

    const totalTokensInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-total-tokens-input"]')
    const saveButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-save-button"]')

    await waitFor(() =>
      mounted.container.querySelector<HTMLInputElement>('[data-testid="projects-total-tokens-input"]')?.value === '500000',
    )

    expect(totalTokensInput).not.toBeNull()
    expect(saveButton).not.toBeNull()
    expect(totalTokensInput?.value).toBe('500000')
    expect(mounted.container.querySelector('[data-testid="projects-used-tokens-value"]')?.textContent).toContain('125,000')

    totalTokensInput!.value = '750000'
    totalTokensInput!.dispatchEvent(new Event('input', { bubbles: true }))
    totalTokensInput!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    saveButton?.click()

    await waitFor(() => workspaceStore.getProjectSettings('proj-redesign').models?.totalTokens === 750000)

    expect(mounted.container.querySelector('[data-testid="projects-used-tokens-value"]')?.textContent).toContain('125,000')

    mounted.destroy()
  })

  it('archives the current project, hides it from the sidebar tree, and switches to the next active project', async () => {
    const mounted = mountApp()
    const workspaceStore = useWorkspaceStore()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-select-proj-redesign"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-redesign"]')?.click()
    await waitFor(() => workspaceStore.currentProjectId === 'proj-redesign')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-archive-button"]')?.click()

    await waitFor(() =>
      workspaceStore.currentProjectId === 'proj-governance'
      && mounted.container.textContent?.includes('Workspace Governance')
      && mounted.container.textContent?.includes(String(i18n.global.t('projects.status.archived'))),
    )

    expect(workspaceStore.currentProjectId).toBe('proj-governance')
    expect(mounted.container.querySelector('[data-testid="sidebar-project-proj-redesign"]')).toBeNull()
    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projects.status.archived')))

    mounted.destroy()
  })

  it('prevents archiving the last active project and surfaces an error', async () => {
    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-select-proj-governance"]') !== null)

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-redesign"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-archive-button"]')?.click()
    await waitFor(() =>
      mounted.container.textContent?.includes(String(i18n.global.t('projects.status.archived'))) ?? false,
    )

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-select-proj-governance"]')?.click()
    await nextTick()
    mounted.container.querySelector<HTMLButtonElement>('[data-testid="projects-archive-button"]')?.click()

    await waitFor(() => mounted.container.querySelector('[data-testid="projects-error"]') !== null)
    expect(mounted.container.textContent).toContain(String(i18n.global.t('projects.errors.lastActiveProject')))

    mounted.destroy()
  })
})
