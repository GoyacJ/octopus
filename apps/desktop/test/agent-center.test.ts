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

function findButton(container: ParentNode, label: string) {
  return Array.from(container.querySelectorAll<HTMLButtonElement>('button'))
    .find(button => button.textContent?.includes(label))
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

async function waitForCondition(check: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!check()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for condition')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('workspace and project agents pages', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
  })

  it('renders workspace agent center tabs and resource tabs', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Architect Agent')

    expect(mounted.container.querySelector('[data-testid="workspace-console-view"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="workspace-console-tabs"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-embedded"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-tabs-shell"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Architect Agent')
    expect(mounted.container.textContent).toContain('Coder Agent')
    expect(mounted.container.textContent).toContain('数字团队')
    expect(mounted.container.textContent).toContain('内置工具')
    expect(mounted.container.textContent).toContain('技能')
    expect(mounted.container.textContent).toContain('MCP')
    expect(mounted.container.textContent).not.toContain('管理可被多个项目复用的工作区级数字员工。')
    expect(mounted.container.textContent).not.toContain('DIGITAL WORKFORCE')
    expect(mounted.container.querySelector('[data-testid="agent-center-hero"]')).toBeNull()

    const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(teamTab).not.toBeNull()
    teamTab?.click()
    await waitForCondition(() => router.currentRoute.value.query.tab === 'team')

    expect(router.currentRoute.value.query.tab).toBe('team')
    expect(mounted.container.textContent).toContain('Studio Direction Team')

    const skillTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-skill"]') as HTMLButtonElement | null
    expect(skillTab).not.toBeNull()
    skillTab?.click()
    await waitForText(mounted.container, 'help')
    expect(mounted.container.textContent).toContain('Architect Agent')
    expect(mounted.container.textContent).toContain('Studio Direction Team')
    expect(mounted.container.textContent).toContain('工作区')
    expect(mounted.container.textContent).toContain('Local Workspace')

    mounted.destroy()
  })

  it('renders project-scoped effective agents and teams from project assignments instead of project links', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project?.assignments?.agents) {
          throw new Error('Expected proj-redesign agent assignments')
        }

        project.assignments.agents.agentIds = ['agent-architect', 'agent-template-finance']
        project.assignments.agents.teamIds = ['team-studio', 'team-template-finance']
        state.projectAgentLinks['proj-redesign'] = []
        state.projectTeamLinks['proj-redesign'] = []
      },
    })

    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Redesign Copilot')
    await waitForText(mounted.container, 'Architect Agent')
    await waitForText(mounted.container, 'Finance Planner Template')

    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Redesign Copilot')
    expect(mounted.container.textContent).toContain('Architect Agent')
    expect(mounted.container.textContent).toContain('Finance Planner Template')
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-redesign"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-architect"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-template-finance"]')).toBeNull()
    expect(mounted.container.textContent).not.toContain('管理分配给当前项目的数字员工。')
    expect(mounted.container.textContent).not.toContain('DIGITAL WORKFORCE')
    expect(mounted.container.querySelector('[data-testid="agent-center-hero"]')).toBeNull()
    expect(mounted.container.textContent).not.toContain('接入工作区 Agent')

    const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(teamTab).not.toBeNull()
    teamTab?.click()
    await waitForCondition(() => router.currentRoute.value.query.tab === 'team')

    await waitForText(mounted.container, 'Finance Ops Template')
    expect(mounted.container.textContent).not.toContain('接入工作区 Team')
    expect(mounted.container.textContent).toContain('Studio Direction Team')
    expect(mounted.container.textContent).toContain('Redesign Tiger Team')
    expect(mounted.container.textContent).toContain('Finance Ops Template')
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-team-team-redesign"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-team-team-studio"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-team-team-template-finance"]')).toBeNull()

    mounted.destroy()
  })

  it('shows only effective project resources inside the project resource tabs', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Redesign Copilot')

    const skillTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-skill"]') as HTMLButtonElement | null
    expect(skillTab).not.toBeNull()
    skillTab?.click()
    await waitForText(mounted.container, 'redesign-review')

    expect(mounted.container.textContent).toContain('help')
    expect(mounted.container.textContent).toContain('redesign-review')
    expect(mounted.container.textContent).not.toContain('external-checks')
    expect(mounted.container.textContent).toContain('项目')
    expect(mounted.container.textContent).toContain('Desktop Redesign')

    const mcpTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-mcp"]') as HTMLButtonElement | null
    expect(mcpTab).not.toBeNull()
    mcpTab?.click()
    await waitForText(mounted.container, 'redesign-ops')

    expect(mounted.container.textContent).toContain('ops')
    expect(mounted.container.textContent).toContain('redesign-ops')
    expect(mounted.container.textContent).not.toContain('finance-ops')

    mounted.destroy()
  })

  it('keeps assigned workspace teams readonly inside the project scope', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents?tab=team')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Studio Direction Team')

    const removeButton = mounted.container.querySelector('[data-testid="agent-center-remove-team-team-studio"]') as HTMLButtonElement | null
    expect(removeButton).toBeNull()
    expect(mounted.container.textContent).toContain('Studio Direction Team')

    await router.push('/workspaces/ws-local/agents?tab=team')
    await waitForText(mounted.container, 'Studio Direction Team')

    mounted.destroy()
  })

  it('shows delete actions for workspace agent and team cards and keeps digital team relationships inside dialog', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Architect Agent')

    expect(mounted.container.textContent).not.toContain('编辑关系')
    expect(mounted.container.textContent).toContain('删除')

    const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(teamTab).not.toBeNull()
    teamTab?.click()
    await waitForCondition(() => router.currentRoute.value.query.tab === 'team')

    const teamOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-studio"]') as HTMLButtonElement | null
    expect(teamOpenButton).not.toBeNull()
    teamOpenButton?.click()
    await nextTick()

    expect(document.body.querySelector('[data-testid="agent-center-team-dialog"]')).not.toBeNull()
    expect(document.body.textContent).toContain('数字团队配置')
    expect(document.body.textContent).toContain('Studio Direction Team')
    expect(document.body.textContent).toContain('组织结构预览')

    mounted.destroy()
  })

  it('keeps import and batch export actions on the embedded workspace page', async () => {
    vi.spyOn(tauriClient, 'pickAgentBundleFolder').mockResolvedValue([
      {
        fileName: 'Imported Workspace Agent.md',
        contentType: 'text/markdown',
        byteSize: 42,
        dataBase64: btoa('# Imported Workspace Agent'),
        relativePath: 'Imported Workspace Agent/Imported Workspace Agent.md',
      },
    ])

    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Architect Agent')

    expect(mounted.container.querySelector('[data-testid="agent-center-embedded"]')).not.toBeNull()
    const importTrigger = mounted.container.querySelector('[data-testid="agent-center-import-agents-trigger"]') as HTMLButtonElement | null
    expect(importTrigger).not.toBeNull()

    const firstAgentCheckbox = mounted.container.querySelector('[data-testid="agent-center-select-agent-agent-architect"]') as HTMLLabelElement | null
    expect(firstAgentCheckbox).not.toBeNull()
    firstAgentCheckbox?.click()
    await nextTick()

    const exportTrigger = mounted.container.querySelector('[data-testid="agent-center-export-agents-trigger"]') as HTMLButtonElement | null
    expect(exportTrigger).not.toBeNull()
    expect(exportTrigger.disabled).toBe(false)

    mounted.destroy()
  })

  it('copies builtin agent templates into the workspace and keeps them out of export selection', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Finance Planner Template')

    expect(mounted.container.querySelector('[data-testid="agent-center-select-agent-agent-template-finance"]')).toBeNull()

    const copyButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-template-finance"]') as HTMLButtonElement | null
    expect(copyButton).not.toBeNull()
    copyButton?.click()

    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-workspace-finance-planner-template-copy"]') !== null,
    )

    mounted.destroy()
  })

  it('copies builtin digital team templates into the project scope', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project?.assignments?.agents) {
          throw new Error('Expected proj-redesign agent assignments')
        }

        project.assignments.agents.teamIds = ['team-studio', 'team-template-finance']
      },
    })

    await router.push('/workspaces/ws-local/projects/proj-redesign/agents?tab=team')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Finance Ops Template')

    expect(mounted.container.querySelector('[data-testid="agent-center-select-team-team-template-finance"]')).not.toBeNull()

    const copyButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-template-finance"]') as HTMLButtonElement | null
    expect(copyButton).not.toBeNull()
    copyButton?.click()

    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-team-team-project-finance-ops-template-copy"]') !== null,
    )

    mounted.destroy()
  })

  it('promotes project-owned agents and teams into the workspace without removing the project assets', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Redesign Copilot')

    const agentOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-redesign"]') as HTMLButtonElement | null
    expect(agentOpenButton).not.toBeNull()
    agentOpenButton?.click()
    await waitForText(document.body, '员工配置')

    const promoteAgentButton = document.body.querySelector('[data-testid="agent-center-promote-agent-button"]') as HTMLButtonElement | null
    expect(promoteAgentButton).not.toBeNull()
    promoteAgentButton?.click()

    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-redesign"]') !== null,
    )

    const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(teamTab).not.toBeNull()
    teamTab?.click()
    await waitForText(mounted.container, 'Redesign Tiger Team')

    const teamOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-redesign"]') as HTMLButtonElement | null
    expect(teamOpenButton).not.toBeNull()
    teamOpenButton?.click()
    await waitForText(document.body, '数字团队配置')

    const promoteTeamButton = document.body.querySelector('[data-testid="agent-center-promote-team-button"]') as HTMLButtonElement | null
    expect(promoteTeamButton).not.toBeNull()
    promoteTeamButton?.click()

    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-team-team-redesign"]') !== null,
    )

    await router.push('/workspaces/ws-local/agents')
    await waitForText(mounted.container, 'Redesign Copilot')
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-workspace-redesign-copilot-copy"]')).not.toBeNull()

    const workspaceTeamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(workspaceTeamTab).not.toBeNull()
    workspaceTeamTab?.click()
    await waitForText(mounted.container, 'Redesign Tiger Team')
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-team-team-workspace-redesign-tiger-team-copy"]')).not.toBeNull()

    mounted.destroy()
  })

  it('exports mixed agent and digital team selections in one batch payload', async () => {
    const saveSpy = vi.spyOn(tauriClient, 'saveAgentBundleExport')

    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Architect Agent')

    const agentCheckbox = mounted.container.querySelector('[data-testid="agent-center-select-agent-agent-architect"]') as HTMLLabelElement | null
    expect(agentCheckbox).not.toBeNull()
    agentCheckbox?.click()
    await nextTick()

    const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(teamTab).not.toBeNull()
    teamTab?.click()
    await waitForCondition(() => router.currentRoute.value.query.tab === 'team')
    await waitForText(mounted.container, 'Studio Direction Team')

    const teamCheckbox = mounted.container.querySelector('[data-testid="agent-center-select-team-team-studio"]') as HTMLLabelElement | null
    expect(teamCheckbox).not.toBeNull()
    teamCheckbox?.click()
    await nextTick()

    const exportTrigger = mounted.container.querySelector('[data-testid="agent-center-export-teams-trigger"]') as HTMLButtonElement | null
    expect(exportTrigger).not.toBeNull()
    exportTrigger?.click()
    await waitForCondition(() => document.body.querySelector('[data-testid="ui-dropdown-item-export-folder"]') !== null)

    const exportFolderButton = document.body.querySelector('[data-testid="ui-dropdown-item-export-folder"]') as HTMLElement | null
    expect(exportFolderButton).not.toBeNull()
    exportFolderButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))

    await waitForCondition(() => saveSpy.mock.calls.length > 0)

    expect(saveSpy).toHaveBeenLastCalledWith(
      expect.objectContaining({
        agentCount: 1,
        teamCount: 1,
      }),
      'folder',
    )

    mounted.destroy()
  })
})
