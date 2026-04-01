// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useWorkbenchStore } from '@/stores/workbench'

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

class MockFileReader {
  result: string | ArrayBuffer | null = null
  onload: null | (() => void) = null

  readAsDataURL(file: File) {
    this.result = `data:${file.type};base64,avatar-preview`
    this.onload?.()
  }
}

vi.stubGlobal('FileReader', MockFileReader)

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
    app,
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

describe('Merged agent center', () => {
  beforeEach(async () => {
    await router.push('/workspaces/ws-local/agents')
    await router.isReady()
    document.body.innerHTML = ''
    vi.spyOn(window, 'confirm').mockReturnValue(true)
  })

  it('renders compact tabs on top and supports icon/list view switching', async () => {
    const mounted = mountApp()

    await nextTick()

    expect(mounted.container.textContent).toContain('智能体')
    expect(mounted.container.querySelector('[data-testid="agent-center-toolbar"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-tab-agent"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-tab-team"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-view-icon"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-view-list"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-icon-view-agent"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-list-view-agent"]')).toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-view-list"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-list-view-agent"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-icon-view-agent"]')).toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-tab-team"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-list-view-team"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-item-team-team-studio"]')).not.toBeNull()

    mounted.destroy()
  })

  it('supports pagination and avatar upload for agents', async () => {
    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    await nextTick()

    for (let index = 0; index < 22; index += 1) {
      const agent = workbench.createAgent('workspace')
      workbench.updateAgent(agent.id, {
        name: `Mock Workspace Agent ${index + 1}`,
      })
    }

    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-pagination-agent"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-page-info-agent"]')?.textContent).toContain('1 / 2')
    expect(mounted.container.textContent).toContain('每页 20 项')
    expect(mounted.container.textContent).toContain('上一页')
    expect(mounted.container.textContent).toContain('下一页')
    expect(mounted.container.querySelector('[data-testid="agent-center-item-agent-agent-architect"]')).not.toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-page-next-agent"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-page-info-agent"]')?.textContent).toContain('2 / 2')
    expect(mounted.container.querySelector('[data-testid="agent-center-item-agent-agent-architect"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-page-info-team"]')).toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-item-agent-agent-mock-22"]')?.click()
    await nextTick()

    const avatarInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="agent-center-avatar-input"]')
    const file = new File(['avatar'], 'agent.png', { type: 'image/png' })
    Object.defineProperty(avatarInput, 'files', {
      configurable: true,
      value: [file],
    })
    avatarInput?.dispatchEvent(new Event('change'))
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-avatar-preview"]')).not.toBeNull()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-dialog-save"]')?.click()
    await nextTick()

    expect(workbench.agents.find((agent) => agent.id === 'agent-mock-22')?.avatar).toContain('data:image/png;base64')

    mounted.destroy()
  })

  it('keeps agent and team pagination independent and resets each on search changes', async () => {
    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    await nextTick()

    for (let index = 0; index < 22; index += 1) {
      const agent = workbench.createAgent('workspace')
      workbench.updateAgent(agent.id, {
        name: `Pagination Agent ${index + 1}`,
      })
    }

    for (let index = 0; index < 22; index += 1) {
      const team = workbench.createTeam('workspace')
      workbench.updateTeam(team.id, {
        name: `Pagination Team ${index + 1}`,
      })
    }

    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-page-next-agent"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-page-info-agent"]')?.textContent).toContain('2 / 2')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-tab-team"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-page-info-team"]')?.textContent).toContain('1 / 2')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-page-next-team"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-page-info-team"]')?.textContent).toContain('2 / 2')

    const searchInput = mounted.container.querySelector<HTMLInputElement>('[data-testid="agent-center-search"]')
    expect(searchInput).not.toBeNull()
    searchInput!.value = 'Pagination'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-page-info-team"]')?.textContent).toContain('1 / 2')

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-tab-agent"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-page-info-agent"]')?.textContent).toContain('1 / 2')

    mounted.destroy()
  })

  it('supports adding existing agents and rendering a flow canvas for team orchestration', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents?kind=team')
    await nextTick()

    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-item-team-team-redesign-copy"]')?.click()
    await nextTick()

    expect(mounted.container.querySelector('[data-testid="agent-center-member-picker"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-structure-canvas"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-flow-canvas"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="agent-center-flow-node-redesign-node-lead"]')).not.toBeNull()

    const memberSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="agent-center-member-picker"]')
    memberSelect!.value = 'agent-architect-copy-proj-redesign'
    memberSelect!.dispatchEvent(new Event('change'))
    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-member-add"]')?.click()
    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-dialog-save"]')?.click()
    await nextTick()

    const updatedTeam = workbench.teams.find((team) => team.id === 'team-redesign-copy')
    expect(updatedTeam?.members).toContain('agent-architect-copy-proj-redesign')
    expect(updatedTeam?.structureMode).toBe('flow')
    expect(updatedTeam?.structureNodes.some((node) => node.memberId === 'agent-architect-copy-proj-redesign')).toBe(true)

    mounted.destroy()
  })

  it('renders the origin column as plain table text in project list view', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/agents')
    await nextTick()

    const mounted = mountApp()

    await nextTick()

    mounted.container.querySelector<HTMLButtonElement>('[data-testid="agent-center-view-list"]')?.click()
    await nextTick()

    const originCell = mounted.container.querySelector<HTMLElement>('.table-origin')

    expect(originCell).not.toBeNull()
    expect(originCell?.textContent).toContain('工作区引用')
    expect(originCell?.className).toContain('table-origin')
    expect(originCell?.className).toContain('table-body-text')
    expect(originCell?.className).not.toContain('ui-badge')

    mounted.destroy()
  })
})
