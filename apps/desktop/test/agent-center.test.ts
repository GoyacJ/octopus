// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
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

    expect(mounted.container.querySelector('[data-testid="agent-center-view"]')).not.toBeNull()
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

    mounted.destroy()
  })

  it('renders project-scoped agents and teams together with linked workspace entries', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Redesign Copilot')
    await waitForText(mounted.container, 'Architect Agent')

    expect(mounted.container.textContent).toContain('Desktop Redesign')
    expect(mounted.container.textContent).toContain('Redesign Copilot')
    expect(mounted.container.textContent).toContain('Architect Agent')
    expect(mounted.container.textContent).not.toContain('管理分配给当前项目的数字员工。')
    expect(mounted.container.textContent).not.toContain('DIGITAL WORKFORCE')
    expect(mounted.container.querySelector('[data-testid="agent-center-hero"]')).toBeNull()
    expect(mounted.container.textContent).not.toContain('接入工作区 Agent')

    const teamTab = mounted.container.querySelector('[data-testid="ui-tabs-trigger-team"]') as HTMLButtonElement | null
    expect(teamTab).not.toBeNull()
    teamTab?.click()
    await waitForCondition(() => router.currentRoute.value.query.tab === 'team')

    expect(mounted.container.textContent).not.toContain('接入工作区 Team')
    expect(mounted.container.textContent).toContain('Studio Direction Team')
    expect(mounted.container.textContent).toContain('Redesign Tiger Team')

    mounted.destroy()
  })

  it('unlinks a linked project team without deleting the workspace digital team', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents?tab=team')
    await router.isReady()

    const mounted = mountApp()
    await waitForText(mounted.container, 'Studio Direction Team')

    const removeButton = mounted.container.querySelector('[data-testid="agent-center-remove-team-team-studio"]') as HTMLButtonElement | null
    expect(removeButton).not.toBeNull()
    removeButton?.click()
    await waitForText(document.body, '确认删除')

    const confirmButton = Array.from(document.body.querySelectorAll<HTMLButtonElement>('button'))
      .find(button => button.textContent?.includes('确认删除'))
    expect(confirmButton).not.toBeNull()
    confirmButton?.click()

    await waitForCondition(() => !mounted.container.textContent?.includes('Studio Direction Team'))

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
})
