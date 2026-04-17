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

  it('shows the leader name in the readonly team leader display for existing workspace teams', async () => {
    await router.push('/workspaces/ws-local/agents?tab=team')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Studio Direction Team')

    const teamOpenButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-studio"]') as HTMLButtonElement | null
    expect(teamOpenButton).not.toBeNull()
    teamOpenButton?.click()
    await waitForCondition(() =>
      document.body.querySelector('[data-testid="agent-center-team-dialog"]') !== null,
    )

    const leaderInput = document.body.querySelector('[data-testid="agent-center-team-leader-display"]') as HTMLInputElement | null
    expect(leaderInput).not.toBeNull()
    expect(leaderInput?.value).toBe('Architect Agent')

    mounted.destroy()
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

  it('keeps existing workspace agents content readonly but allows saving status changes with localized labels', async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()

    const mounted = mountApp()
    const agentStore = useAgentStore()
    const notificationStore = useNotificationStore()
    await waitForText(mounted.container, 'Architect Agent')

    const openButton = mounted.container.querySelector('[data-testid="agent-center-open-agent-agent-architect"]') as HTMLButtonElement | null
    expect(openButton).not.toBeNull()
    expect(openButton?.textContent).toContain('查看')
    expect(mounted.container.textContent).toContain('启用')
    openButton?.click()

    await waitForText(document.body, '员工配置')

    const nameInput = document.body.querySelector('input[placeholder="例如: 研发专家"]') as HTMLInputElement | null
    const statusSelect = document.body.querySelector('[data-testid="agent-center-agent-dialog"] select') as HTMLSelectElement | null
    expect(nameInput).not.toBeNull()
    expect(nameInput?.disabled).toBe(true)
    expect(statusSelect).not.toBeNull()
    expect(statusSelect?.disabled).toBe(false)
    expect(findButton(document.body, '保存配置')).toBeDefined()

    statusSelect!.value = 'archived'
    statusSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()

    findButton(document.body, '保存配置')?.click()

    await waitForCondition(() => agentStore.agents.find(agent => agent.id === 'agent-architect')?.status === 'archived')
    await waitForCondition(() =>
      notificationStore.notificationsState.some(notification =>
        notification.title.includes('保存完成')
        && (notification.body?.includes('Architect Agent') ?? false),
      ),
    )
    await waitForText(mounted.container, '禁用')

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

  it('keeps existing project teams content readonly but allows saving status changes', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents?tab=team')
    await router.isReady()

    const mounted = mountApp()
    const teamStore = useTeamStore()
    const notificationStore = useNotificationStore()
    await waitForText(mounted.container, 'Redesign Tiger Team')

    const openButton = mounted.container.querySelector('[data-testid="agent-center-open-team-team-redesign"]') as HTMLButtonElement | null
    expect(openButton).not.toBeNull()
    expect(openButton?.textContent).toContain('查看')
    openButton?.click()

    await waitForText(document.body, '数字团队配置')

    const nameInput = document.body.querySelector('input[placeholder="例如: 核心研发组"]') as HTMLInputElement | null
    const leaderDisplay = document.body.querySelector('[data-testid="agent-center-team-leader-display"]') as HTMLInputElement | null
    const statusSelect = document.body.querySelector('[data-testid="agent-center-team-dialog"] select') as HTMLSelectElement | null
    expect(nameInput).not.toBeNull()
    expect(nameInput?.disabled).toBe(true)
    expect(leaderDisplay).not.toBeNull()
    expect(leaderDisplay?.disabled).toBe(true)
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

  it('exports selected teams from the team tab in one batch payload', async () => {
    const saveSpy = vi.spyOn(tauriClient, 'saveAgentBundleExport')

    await router.push('/workspaces/ws-local/console/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Studio Direction Team')

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
        agentCount: 0,
        teamCount: 1,
      }),
      'folder',
    )

    mounted.destroy()
  })
})
