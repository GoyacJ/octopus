// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useAgentStore } from '@/stores/agent'
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
    const tabsShell = mounted.container.querySelector<HTMLElement>('[data-testid="agent-center-tabs-shell"]')
    expect(tabsShell).not.toBeNull()
    expect(tabsShell?.className).toContain('border-b')
    expect(tabsShell?.className).toContain('border-border')
    expect(tabsShell?.className).not.toContain('bg-subtle')
    expect(tabsShell?.className).not.toContain('rounded')
    expect(mounted.container.textContent).toContain('Architect Agent')
    expect(mounted.container.textContent).toContain('Coder Agent')
    expect(mounted.container.textContent).toContain('Finance Planner Template')
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
    expect(mounted.container.textContent).toContain('Finance Ops Template')

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

  it('keeps agent and team cards on neutral hover states instead of accent border flashes', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Architect Agent')

    const agentOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-architect"]') as HTMLButtonElement | null
    const agentCard = agentOpenButton?.closest<HTMLElement>('[role="button"]')
    expect(agentCard).not.toBeNull()
    expect(agentCard?.className).toContain('hover:border-border-strong')
    expect(agentCard?.className).toContain('hover:bg-subtle')
    expect(agentCard?.className).not.toContain('hover:border-primary/30')

    const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(teamTab).not.toBeNull()
    teamTab?.click()
    await waitForCondition(() => router.currentRoute.value.query.tab === 'team')
    await waitForText(mounted.container, 'Studio Direction Team')

    const teamOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-studio"]') as HTMLButtonElement | null
    const teamCard = teamOpenButton?.closest<HTMLElement>('[role="button"]')
    expect(teamCard).not.toBeNull()
    expect(teamCard?.className).toContain('hover:border-border-strong')
    expect(teamCard?.className).toContain('hover:bg-subtle')
    expect(teamCard?.className).not.toContain('hover:border-primary/30')

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

  it('lets project-owned agents and teams stay editable, copyable, and promotable inside the project scope', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    try {
      await waitForText(mounted.container, 'Redesign Copilot')

      const projectAgentOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-redesign"]') as HTMLButtonElement | null
      expect(projectAgentOpenButton).not.toBeNull()
      projectAgentOpenButton?.click()

      await waitForText(document.body, '员工配置')
      const projectAgentNameInput = document.body.querySelector('input[placeholder="例如: 研发专家"]') as HTMLInputElement | null
      const projectAgentStatusSelect = document.body.querySelector('[data-testid="agent-center-agent-dialog"] select') as HTMLSelectElement | null
      expect(projectAgentNameInput).not.toBeNull()
      expect(projectAgentNameInput?.disabled).toBe(false)
      expect(projectAgentStatusSelect).not.toBeNull()
      expect(projectAgentStatusSelect?.disabled).toBe(false)
      expect(findButton(document.body, '保存配置')).toBeDefined()
      expect(document.body.querySelector('[data-testid="agent-center-copy-agent-button"]')).not.toBeNull()
      expect(document.body.querySelector('[data-testid="agent-center-promote-agent-button"]')).not.toBeNull()

      const closeAgentDialogButton = document.body.querySelector('[data-testid="ui-dialog-close"]') as HTMLButtonElement | null
      expect(closeAgentDialogButton).not.toBeNull()
      closeAgentDialogButton?.click()
      await waitForCondition(() => document.body.querySelector('[data-testid="agent-center-agent-dialog"]') === null)

      const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
      expect(teamTab).not.toBeNull()
      teamTab?.click()
      await waitForCondition(() => router.currentRoute.value.query.tab === 'team')
      await waitForText(mounted.container, 'Redesign Tiger Team')

      const projectTeamOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-redesign"]') as HTMLButtonElement | null
      expect(projectTeamOpenButton).not.toBeNull()
      projectTeamOpenButton?.click()

      await waitForText(document.body, '数字团队配置')
      const projectTeamNameInput = document.body.querySelector('input[placeholder="例如: 核心研发组"]') as HTMLInputElement | null
      expect(projectTeamNameInput).not.toBeNull()
      expect(projectTeamNameInput?.disabled).toBe(false)
      expect(findButton(document.body, '保存配置')).toBeDefined()
      expect(document.body.querySelector('[data-testid="agent-center-copy-team-button"]')).not.toBeNull()
      expect(document.body.querySelector('[data-testid="agent-center-promote-team-button"]')).not.toBeNull()
    } finally {
      mounted.destroy()
    }
  })

  it('keeps assigned workspace agents and teams readonly in project scope but allows copying them into project assets', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    try {
      await waitForText(mounted.container, 'Architect Agent')

      const workspaceAgentOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-architect"]') as HTMLButtonElement | null
      expect(workspaceAgentOpenButton).not.toBeNull()
      workspaceAgentOpenButton?.click()

      await waitForText(document.body, '员工配置')
      const workspaceAgentNameInput = document.body.querySelector('input[placeholder="例如: 研发专家"]') as HTMLInputElement | null
      const workspaceAgentStatusSelect = document.body.querySelector('[data-testid="agent-center-agent-dialog"] select') as HTMLSelectElement | null
      expect(workspaceAgentNameInput).not.toBeNull()
      expect(workspaceAgentNameInput?.disabled).toBe(true)
      expect(workspaceAgentStatusSelect).not.toBeNull()
      expect(workspaceAgentStatusSelect?.disabled).toBe(true)
      expect(findButton(document.body, '保存配置')).toBeUndefined()

      const workspaceAgentCopyButton = document.body.querySelector('[data-testid="agent-center-copy-agent-button"]') as HTMLButtonElement | null
      expect(workspaceAgentCopyButton).not.toBeNull()
      workspaceAgentCopyButton?.click()

      await waitForCondition(() =>
        mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-project-architect-agent-copy"]') !== null,
      )

      const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
      expect(teamTab).not.toBeNull()
      teamTab?.click()
      await waitForCondition(() => router.currentRoute.value.query.tab === 'team')
      await waitForText(mounted.container, 'Studio Direction Team')

      const workspaceTeamOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-studio"]') as HTMLButtonElement | null
      expect(workspaceTeamOpenButton).not.toBeNull()
      workspaceTeamOpenButton?.click()

      await waitForText(document.body, '数字团队配置')
      const workspaceTeamNameInput = document.body.querySelector('input[placeholder="例如: 核心研发组"]') as HTMLInputElement | null
      const workspaceTeamStatusSelect = document.body.querySelector('[data-testid="agent-center-team-dialog"] select') as HTMLSelectElement | null
      expect(workspaceTeamNameInput).not.toBeNull()
      expect(workspaceTeamNameInput?.disabled).toBe(true)
      expect(workspaceTeamStatusSelect).not.toBeNull()
      expect(workspaceTeamStatusSelect?.disabled).toBe(true)
      expect(findButton(document.body, '保存配置')).toBeUndefined()

      const workspaceTeamCopyButton = document.body.querySelector('[data-testid="agent-center-copy-team-button"]') as HTMLButtonElement | null
      expect(workspaceTeamCopyButton).not.toBeNull()
      workspaceTeamCopyButton?.click()

      await waitForCondition(() =>
        mounted.container.querySelector('[data-testid="agent-center-remove-team-team-project-studio-direction-team-copy"]') !== null,
      )
    } finally {
      mounted.destroy()
    }
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

  it('keeps existing workspace teams editable and copyable from the workspace scope', async () => {
    await router.push('/workspaces/ws-local/agents?tab=team')
    await router.isReady()

    const mounted = mountApp()
    try {
      const teamStore = useTeamStore()
      const notificationStore = useNotificationStore()
      await waitForText(mounted.container, 'Studio Direction Team')

      const teamOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-studio"]') as HTMLButtonElement | null
      expect(teamOpenButton).not.toBeNull()
      expect(teamOpenButton?.textContent).toContain('编辑')
      teamOpenButton?.click()
      await waitForCondition(() =>
        document.body.querySelector('[data-testid="agent-center-team-dialog"]') !== null,
      )

      const nameInput = document.body.querySelector('input[placeholder="例如: 核心研发组"]') as HTMLInputElement | null
      const leaderInput = document.body.querySelector('[data-testid="agent-center-team-leader-display"]') as HTMLInputElement | null
      const statusSelect = document.body.querySelector('[data-testid="agent-center-team-dialog"] select') as HTMLSelectElement | null
      const copyButton = document.body.querySelector('[data-testid="agent-center-copy-team-button"]') as HTMLButtonElement | null
      expect(nameInput).not.toBeNull()
      expect(nameInput?.disabled).toBe(false)
      expect(leaderInput).toBeNull()
      expect(statusSelect).not.toBeNull()
      expect(statusSelect?.disabled).toBe(false)
      expect(copyButton).not.toBeNull()
      expect(findButton(document.body, '保存配置')).toBeDefined()

      nameInput!.value = 'Studio Direction Team Revised'
      nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
      await nextTick()

      findButton(document.body, '保存配置')?.click()

      await waitForCondition(() => teamStore.teams.find(team => team.id === 'team-studio')?.name === 'Studio Direction Team Revised')
      await waitForCondition(() =>
        notificationStore.notificationsState.some(notification =>
          notification.title.includes('保存完成')
          && (notification.body?.includes('Studio Direction Team Revised') ?? false),
        ),
      )
    } finally {
      mounted.destroy()
    }
  })

  it('does not keep the save action when switching from editable agent to builtin agent', async () => {
    await router.push('/workspaces/ws-local/console/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Architect Agent')

    const editableOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-architect"]') as HTMLButtonElement | null
    expect(editableOpenButton).not.toBeNull()
    editableOpenButton?.click()
    await waitForText(document.body, '员工配置')
    expect(findButton(document.body, '保存配置')).toBeDefined()

    const closeButton = document.body.querySelector('[data-testid="ui-dialog-close"]') as HTMLButtonElement | null
    expect(closeButton).not.toBeNull()
    closeButton?.click()
    await waitForCondition(() => document.body.querySelector('[data-testid="agent-center-agent-dialog"]') === null)

    const builtinOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-template-finance"]') as HTMLButtonElement | null
    expect(builtinOpenButton).not.toBeNull()
    builtinOpenButton?.click()
    await waitForText(document.body, '员工配置')

    const builtinStatusSelect = document.body.querySelector('[data-testid="agent-center-agent-dialog"] select') as HTMLSelectElement | null
    expect(builtinStatusSelect).not.toBeNull()
    expect(builtinStatusSelect?.disabled).toBe(true)
    expect(findButton(document.body, '保存配置')).toBeUndefined()

    mounted.destroy()
  })

  it('keeps existing workspace agents editable and copyable with localized status labels', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    try {
      const agentStore = useAgentStore()
      const notificationStore = useNotificationStore()
      await waitForText(mounted.container, 'Architect Agent')

      const openButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-architect"]') as HTMLButtonElement | null
      expect(openButton).not.toBeNull()
      expect(openButton?.textContent).toContain('编辑')
      expect(mounted.container.textContent).toContain('启用')
      openButton?.click()

      await waitForText(document.body, '员工配置')

      const nameInput = document.body.querySelector('input[placeholder="例如: 研发专家"]') as HTMLInputElement | null
      const statusSelect = document.body.querySelector('[data-testid="agent-center-agent-dialog"] select') as HTMLSelectElement | null
      const copyButton = document.body.querySelector('[data-testid="agent-center-copy-agent-button"]') as HTMLButtonElement | null
      expect(nameInput).not.toBeNull()
      expect(nameInput?.disabled).toBe(false)
      expect(statusSelect).not.toBeNull()
      expect(statusSelect?.disabled).toBe(false)
      expect(copyButton).not.toBeNull()
      expect(findButton(document.body, '保存配置')).toBeDefined()

      nameInput!.value = 'Architect Agent Revised'
      nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
      await nextTick()

      statusSelect!.value = 'archived'
      statusSelect!.dispatchEvent(new Event('change', { bubbles: true }))
      await nextTick()

      findButton(document.body, '保存配置')?.click()

      await waitForCondition(() => agentStore.agents.find(agent => agent.id === 'agent-architect')?.status === 'archived')
      await waitForCondition(() => agentStore.agents.find(agent => agent.id === 'agent-architect')?.name === 'Architect Agent Revised')
      await waitForCondition(() =>
        notificationStore.notificationsState.some(notification =>
          notification.title.includes('保存完成')
          && (notification.body?.includes('Architect Agent Revised') ?? false),
        ),
      )
      await waitForText(mounted.container, 'Architect Agent Revised')
      await waitForText(mounted.container, '禁用')
    } finally {
      mounted.destroy()
    }
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

    await router.push('/workspaces/ws-local/console/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-select-agent-agent-architect"]') !== null,
    )

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

  it('opens the bulk export menu even when nothing is selected', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Architect Agent')

    const exportTrigger = mounted.container.querySelector('[data-testid="agent-center-export-agents-trigger"]') as HTMLButtonElement | null
    expect(exportTrigger).not.toBeNull()
    expect(exportTrigger?.disabled).toBe(false)
    exportTrigger?.click()

    await waitForCondition(() => document.body.querySelector('[data-testid="ui-dropdown-item-export-empty"]') !== null)
    expect(document.body.textContent).toContain('请先选择要导出的数字员工')

    mounted.destroy()
  })

  it('opens builtin agent templates readonly and copies only on explicit action', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Finance Planner Template')

    expect(mounted.container.querySelector('[data-testid="agent-center-select-agent-agent-template-finance"]')).toBeNull()

    const openButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-template-finance"]') as HTMLButtonElement | null
    expect(openButton).not.toBeNull()
    openButton?.click()

    await waitForText(document.body, '员工配置')
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-workspace-finance-planner-template-copy"]')).toBeNull()
    expect(findButton(document.body, '保存配置')).toBeUndefined()

    const copyButton = document.body.querySelector('[data-testid="agent-center-copy-agent-button"]') as HTMLButtonElement | null
    expect(copyButton).not.toBeNull()
    copyButton?.click()

    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-workspace-finance-planner-template-copy"]') !== null,
    )

    mounted.destroy()
  })

  it('opens builtin digital team templates readonly, shows leader name, and copies on demand', async () => {
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

    const openButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-template-finance"]') as HTMLButtonElement | null
    expect(openButton).not.toBeNull()
    openButton?.click()

    await waitForText(document.body, '数字团队配置')
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-team-team-project-finance-ops-template-copy"]')).toBeNull()
    const builtinTeamStatusSelect = document.body.querySelector('[data-testid="agent-center-team-dialog"] select') as HTMLSelectElement | null
    expect(builtinTeamStatusSelect).not.toBeNull()
    expect(builtinTeamStatusSelect?.disabled).toBe(true)
    expect(findButton(document.body, '保存配置')).toBeUndefined()

    const leaderDisplay = document.body.querySelector('[data-testid="agent-center-team-leader-display"]') as HTMLInputElement | null
    expect(leaderDisplay).not.toBeNull()
    expect(leaderDisplay?.value).toBe('Finance Planner Template')

    const copyButton = document.body.querySelector('[data-testid="agent-center-copy-team-button"]') as HTMLButtonElement | null
    expect(copyButton).not.toBeNull()
    copyButton?.click()

    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-team-team-project-finance-ops-template-copy"]') !== null,
    )

    mounted.destroy()
  })

  it('keeps existing project teams editable and allows saving status changes', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents?tab=team')
    await router.isReady()

    const mounted = mountApp()
    const teamStore = useTeamStore()
    const notificationStore = useNotificationStore()
    await waitForText(mounted.container, 'Redesign Tiger Team')

    const openButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-redesign"]') as HTMLButtonElement | null
    expect(openButton).not.toBeNull()
    expect(openButton?.textContent).toContain('编辑')
    openButton?.click()

    await waitForText(document.body, '数字团队配置')

    const nameInput = document.body.querySelector('input[placeholder="例如: 核心研发组"]') as HTMLInputElement | null
    const leaderDisplay = document.body.querySelector('[data-testid="agent-center-team-leader-display"]') as HTMLInputElement | null
    const statusSelect = document.body.querySelector('[data-testid="agent-center-team-dialog"] select') as HTMLSelectElement | null
    expect(nameInput).not.toBeNull()
    expect(nameInput?.disabled).toBe(false)
    expect(leaderDisplay).toBeNull()
    expect(statusSelect).not.toBeNull()
    expect(statusSelect?.disabled).toBe(false)
    expect(findButton(document.body, '保存配置')).toBeDefined()

    statusSelect!.value = 'archived'
    statusSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    findButton(document.body, '保存配置')?.click()

    await waitForCondition(() => teamStore.teams.find(team => team.id === 'team-redesign')?.status === 'archived')
    await waitForCondition(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title.includes('保存完成')
        && (notification.body?.includes('Redesign Tiger Team') ?? false),
      ),
    )

    mounted.destroy()
  })

  it('renders team avatars in card view', async () => {
    await router.push('/workspaces/ws-local/agents?tab=team')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Studio Direction Team')

    const cardAvatar = mounted.container.querySelector('[data-testid="agent-center-team-card-avatar-team-studio"]') as HTMLImageElement | null
    expect(cardAvatar).not.toBeNull()
    expect(cardAvatar?.getAttribute('src')).toContain('data:image/png;base64')

    mounted.destroy()
  })

  it('shows 20 agents per page instead of 6', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        for (let index = 0; index < 19; index += 1) {
          state.agents.push({
            id: `agent-bulk-${index + 1}`,
            workspaceId: state.workspace.id,
            scope: 'workspace',
            name: `Bulk Agent ${index + 1}`,
            avatarPath: `data/blobs/avatars/agent-bulk-${index + 1}.png`,
            avatar: 'data:image/png;base64,iVBORw0KGgo=',
            personality: 'Bulk fixture agent',
            tags: ['bulk'],
            prompt: `Handle bulk task ${index + 1}.`,
            builtinToolKeys: ['bash'],
            skillIds: [],
            mcpServerNames: [],
            description: `Bulk agent ${index + 1}`,
            status: 'active',
            updatedAt: 50 - index,
          })
        }
      },
    })

    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Bulk Agent 1')
    await waitForText(mounted.container, 'Bulk Agent 18')

    expect(mounted.container.textContent).toContain('1 / 2')
    expect(mounted.container.textContent).toContain('Bulk Agent 18')
    expect(mounted.container.textContent).not.toContain('Bulk Agent 19')

    const nextPageButton = mounted.container.querySelector('[data-testid="ui-pagination-next"]') as HTMLButtonElement | null
    expect(nextPageButton).not.toBeNull()
    nextPageButton?.click()

    await waitForText(mounted.container, 'Bulk Agent 19')
    expect(mounted.container.textContent).toContain('2 / 2')
    expect(mounted.container.textContent).toContain('Finance Planner Template')

    mounted.destroy()
  })

  it('shows success notifications after saving and deleting a workspace agent', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    const agentStore = useAgentStore()
    const notificationStore = useNotificationStore()
    await waitForText(mounted.container, 'Architect Agent')

    findButton(mounted.container, '新建数字员工')?.click()
    await waitForText(document.body, '员工配置')

    const nameInput = document.body.querySelector('input[placeholder="例如: 研发专家"]') as HTMLInputElement | null
    expect(nameInput).not.toBeNull()
    nameInput!.value = 'Notification Agent'
    nameInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const saveButton = findButton(document.body, '保存配置')
    expect(saveButton).not.toBeNull()
    saveButton?.click()

    await waitForText(mounted.container, 'Notification Agent')
    await waitForCondition(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title.includes('保存完成')
        && (notification.body?.includes('Notification Agent') ?? false),
      ),
    )

    const createdAgent = agentStore.agents.find(agent => agent.name === 'Notification Agent')
    expect(createdAgent).not.toBeUndefined()

    const removeButton = mounted.container.querySelector(`[data-testid="agent-center-remove-agent-${createdAgent!.id}"]`) as HTMLButtonElement | null
    expect(removeButton).not.toBeNull()
    removeButton?.click()

    await waitForText(document.body, '确认删除')
    findButton(document.body, '确认删除')?.click()

    await waitForCondition(() =>
      mounted.container.querySelector(`[data-testid="agent-center-remove-agent-${createdAgent!.id}"]`) === null,
    )
    await waitForCondition(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title.includes('删除完成')
        && (notification.body?.includes('Notification Agent') ?? false),
      ),
    )

    mounted.destroy()
  })

  it('promotes project-owned agents and teams into the workspace without removing the project assets', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    const agentStore = useAgentStore()
    const teamStore = useTeamStore()
    await waitForText(mounted.container, 'Redesign Copilot')

    const agentPromotion = await agentStore.copyToWorkspace('agent-redesign')
    const teamPromotion = await teamStore.copyToWorkspace('team-redesign')
    expect(agentPromotion.agentCount).toBeGreaterThan(0)
    expect(teamPromotion.teamCount).toBeGreaterThan(0)

    await router.push('/workspaces/ws-local/console/agents')
    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-workspace-redesign-copilot-copy"]') !== null,
    )

    const workspaceTeamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(workspaceTeamTab).not.toBeNull()
    workspaceTeamTab?.click()
    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-team-team-workspace-redesign-tiger-team-copy"]') !== null,
    )

    mounted.destroy()
  })

  it('imports bundles into project scope as project-owned assets instead of workspace assets', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    const agentStore = useAgentStore()
    await waitForText(mounted.container, 'Redesign Copilot')

    const imported = await agentStore.importBundle(
      {
        files: [
          {
            relativePath: 'agent-bundle/manifest.json',
            name: 'manifest.json',
            content: btoa(JSON.stringify({ version: 2 })),
            contentType: 'application/json',
          },
        ],
      },
      'proj-redesign',
    )

    expect(imported.agentCount).toBeGreaterThan(0)
    expect(imported.teamCount).toBeGreaterThan(0)

    await waitForCondition(() =>
      mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-imported-project"]') !== null,
    )
    expect(mounted.container.textContent).toContain('Imported Project Agent')
    expect(mounted.container.querySelector('[data-testid="agent-center-remove-agent-agent-imported-project"]')).not.toBeNull()

    const importedAgentOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-imported-project"]') as HTMLButtonElement | null
    expect(importedAgentOpenButton).not.toBeNull()
    importedAgentOpenButton?.click()

    await waitForText(document.body, '员工配置')
    const importedAgentNameInput = document.body.querySelector('input[placeholder="例如: 研发专家"]') as HTMLInputElement | null
    expect(importedAgentNameInput?.disabled).toBe(false)
    expect(document.body.querySelector('[data-testid="agent-center-promote-agent-button"]')).not.toBeNull()

    const closeAgentDialogButton = document.body.querySelector('[data-testid="ui-dialog-close"]') as HTMLButtonElement | null
    closeAgentDialogButton?.click()
    await waitForCondition(() => document.body.querySelector('[data-testid="agent-center-agent-dialog"]') === null)

    const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(teamTab).not.toBeNull()
    teamTab?.click()
    await waitForCondition(() => router.currentRoute.value.query.tab === 'team')
    await waitForText(mounted.container, 'Imported Project Team')

    expect(mounted.container.querySelector('[data-testid="agent-center-remove-team-team-imported-project"]')).not.toBeNull()

    await router.push('/workspaces/ws-local/agents')
    await waitForText(mounted.container, 'Architect Agent')
    expect(mounted.container.textContent).not.toContain('Imported Project Agent')
    expect(mounted.container.textContent).not.toContain('Imported Project Team')

    mounted.destroy()
  })

  it('exports only effective project assets from live inheritance and keeps team member agents bundled', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project) {
          throw new Error('Expected proj-redesign project fixture')
        }

        project.assignments = {
          ...(project.assignments ?? {}),
          agents: {
            excludedAgentIds: ['agent-coder'],
            excludedTeamIds: ['team-template-finance'],
          },
        }
        project.linkedWorkspaceAssets.agentIds = []
        state.projectAgentLinks['proj-redesign'] = []
        state.projectTeamLinks['proj-redesign'] = []
      },
    })

    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    const agentStore = useAgentStore()
    await waitForText(mounted.container, 'Architect Agent')

    const allowedBuiltinAgentExport = await agentStore.exportBundle(
      {
        mode: 'single',
        agentIds: ['agent-template-finance'],
        teamIds: [],
      },
      'proj-redesign',
    )
    expect(allowedBuiltinAgentExport.agentCount).toBe(1)
    expect(allowedBuiltinAgentExport.teamCount).toBe(0)

    const blockedWorkspaceAgentExport = await agentStore.exportBundle(
      {
        mode: 'single',
        agentIds: ['agent-coder'],
        teamIds: [],
      },
      'proj-redesign',
    )
    expect(blockedWorkspaceAgentExport.agentCount).toBe(0)
    expect(blockedWorkspaceAgentExport.teamCount).toBe(0)
    expect(blockedWorkspaceAgentExport.fileCount).toBe(0)

    const allowedWorkspaceTeamExport = await agentStore.exportBundle(
      {
        mode: 'single',
        agentIds: [],
        teamIds: ['team-studio'],
      },
      'proj-redesign',
    )
    expect(allowedWorkspaceTeamExport.teamCount).toBe(1)
    expect(allowedWorkspaceTeamExport.agentCount).toBe(2)

    mounted.destroy()
  })

  it('exports selected teams in one batch payload and includes member agents', async () => {
    await router.push('/workspaces/ws-local/console/agents')
    await router.isReady()

    const mounted = mountApp()
    const agentStore = useAgentStore()
    const workspaceStore = useWorkspaceStore()
    await workspaceStore.ensureWorkspaceBootstrap('conn-local')

    const exported = await agentStore.exportBundle({
      mode: 'batch',
      agentIds: [],
      teamIds: ['team-studio'],
    })

    expect(exported.agentCount).toBe(2)
    expect(exported.teamCount).toBe(1)

    mounted.destroy()
  })
})
