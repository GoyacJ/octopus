// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApp, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
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
