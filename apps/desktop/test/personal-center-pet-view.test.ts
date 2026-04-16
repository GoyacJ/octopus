// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import type { AgentRecord } from '@octopus/schema'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { createAppRouter } from '@/router'
import { installWorkspaceApiFixture } from './support/workspace-fixture'
import {
  createSessionDetail,
  type RuntimeSessionState,
} from './support/workspace-fixture-runtime'

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
  const router = createAppRouter()

  const app = createApp(App)
  app.use(pinia)
  app.use(i18n)
  app.use(router)
  app.mount(container)

  return {
    app,
    container,
    router,
    async destroy() {
      app.unmount()
      await nextTick()
      await new Promise(resolve => window.setTimeout(resolve, 0))
      container.remove()
      await nextTick()
    },
  }
}

async function mountRoutedApp(path: string) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const mounted = mountApp(pinia)
  await mounted.router.push(path)
  await mounted.router.isReady()
  await nextTick()
  await new Promise(resolve => window.setTimeout(resolve, 0))
  await new Promise(resolve => window.setTimeout(resolve, 0))
  return mounted
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

async function findInput(container: HTMLElement, selector: string) {
  await waitForSelector(container, selector)
  const input = container.querySelector(selector)
  if (!(input instanceof HTMLInputElement)) {
    throw new Error(`Expected input for selector: ${selector}`)
  }
  return input
}

function updateInput(input: HTMLInputElement, value: string) {
  input.value = value
  input.dispatchEvent(new Event('input', { bubbles: true }))
  input.dispatchEvent(new Event('change', { bubbles: true }))
}

function findPageHeaderDescription(container: HTMLElement, title: string) {
  const heading = Array.from(container.querySelectorAll('h1, h2')).find(node => node.textContent?.trim() === title)
  if (!heading?.parentElement) {
    throw new Error(`Expected page heading: ${title}`)
  }
  const paragraphs = heading.parentElement.querySelectorAll('p')
  return paragraphs.item(paragraphs.length - 1)?.textContent?.trim() ?? ''
}

describe('personal center pet experience', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    window.localStorage.clear()
    i18n.global.locale.value = 'zh-CN'
    document.body.innerHTML = ''
  })

  it('renders pet dashboard stats and persists reminder preferences from the personal center route', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const petConfig = {
          configuredModelId: 'anthropic-alt',
          permissionMode: 'workspace-write',
          displayName: '章鱼管家',
          greeting: '欢迎回来，我已经准备好了。',
          summary: '负责陪伴与执行工作区任务的章鱼助手。',
          reminderTtlMinutes: 180,
          quietHours: {
            enabled: false,
            start: '22:00',
            end: '07:30',
          },
        }

        state.petProfile.displayName = petConfig.displayName
        state.petProfile.summary = petConfig.summary
        state.petProfile.greeting = petConfig.greeting
        state.petProfile.mood = 'happy'
        state.workspacePetPresence.lastInteractionAt = 1_712_000_000_000
        state.runtimeUserConfig.effectiveConfig = {
          ...(state.runtimeUserConfig.effectiveConfig as Record<string, unknown>),
          pet: petConfig,
        }

        const userSource = state.runtimeUserConfig.sources.find(source => source.scope === 'user' && source.ownerId === 'user-owner')
        if (userSource) {
          userSource.document = {
            ...((userSource.document ?? {}) as Record<string, unknown>),
            pet: petConfig,
          }
        }

        const homeDetail = createSessionDetail('conv-pet-home', '', 'Pet Home', 'pet')
        const homeRuntimeState: RuntimeSessionState = {
          detail: homeDetail,
          events: [],
          nextSequence: 1,
        }
        state.runtimeSessions.set(homeDetail.summary.id, homeRuntimeState)
        state.workspacePetBinding = {
          petId: state.petProfile.id,
          workspaceId: state.workspace.id,
          ownerUserId: state.petProfile.ownerUserId,
          contextScope: 'home',
          conversationId: homeDetail.summary.conversationId,
          sessionId: homeDetail.summary.id,
          updatedAt: 1_712_000_000_000,
        }

        const projectDetail = createSessionDetail('conv-pet-project', 'proj-redesign', 'Pet Project', 'pet')
        const projectRuntimeState: RuntimeSessionState = {
          detail: projectDetail,
          events: [],
          nextSequence: 1,
        }
        state.runtimeSessions.set(projectDetail.summary.id, projectRuntimeState)

        state.workspaceKnowledge = [
          ...state.workspaceKnowledge,
          {
            ...state.workspaceKnowledge[0],
            id: `${state.workspace.id}-knowledge-personal-pet`,
            title: '宠物陪伴偏好',
            scope: 'personal',
            visibility: 'private',
            ownerUserId: 'user-owner',
            updatedAt: 130,
          },
        ]

        state.workspaceResources = [
          ...state.workspaceResources,
          {
            ...state.workspaceResources[0],
            id: `${state.workspace.id}-res-personal-pet`,
            name: '宠物提醒素材',
            scope: 'personal',
            visibility: 'private',
            ownerUserId: 'user-owner',
            updatedAt: 131,
          },
        ]

        state.inboxItems = [
          ...state.inboxItems,
          {
            ...state.inboxItems[0],
            id: 'inbox-pet-reminder',
            title: '宠物回访提醒',
            projectId: undefined,
            createdAt: 132,
          },
        ]
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/personal-center/pet')

    await waitForSelector(mounted.container, '[data-testid="personal-center-pet-stats-panel"]')
    await waitForSelector(mounted.container, '[data-testid="personal-center-pet-preferences-panel"]')

    expect(findPageHeaderDescription(mounted.container, '个人中心')).toBe('维护当前账号的资料、密码与宠物偏好。')
    expect(mounted.container.querySelector('[data-testid="personal-center-pet-species-summary"]')?.textContent).toContain('octopus')
    expect(mounted.container.querySelector('[data-testid="personal-center-pet-model-summary"]')?.textContent).toContain('Claude Alt')
    expect(mounted.container.querySelector('[data-testid="personal-center-pet-permission-summary"]')?.textContent).toContain('工作区可写')
    expect(mounted.container.querySelector('[data-testid="personal-center-pet-metric-memory"]')?.textContent).toContain('1')
    expect(mounted.container.querySelector('[data-testid="personal-center-pet-metric-knowledge"]')?.textContent).toContain('2')
    expect(mounted.container.querySelector('[data-testid="personal-center-pet-metric-resource"]')?.textContent).toContain('3')
    expect(mounted.container.querySelector('[data-testid="personal-center-pet-metric-reminder"]')?.textContent).toContain('3')
    expect(mounted.container.querySelector('[data-testid="personal-center-pet-metric-activity"]')?.textContent).toContain('1')

    updateInput(await findInput(mounted.container, '[data-testid="personal-center-pet-reminder-ttl"]'), '240')
    updateInput(await findInput(mounted.container, '[data-testid="personal-center-pet-quiet-hours-start"]'), '23:00')
    updateInput(await findInput(mounted.container, '[data-testid="personal-center-pet-quiet-hours-end"]'), '06:30')

    const quietHoursSwitch = mounted.container.querySelector('[data-testid="personal-center-pet-quiet-hours-enabled"] button[role="switch"]')
    if (!(quietHoursSwitch instanceof HTMLButtonElement)) {
      throw new Error('Expected quiet hours switch button')
    }
    quietHoursSwitch.click()

    const saveButton = mounted.container.querySelector('[data-testid="personal-center-pet-save"]')
    if (!(saveButton instanceof HTMLButtonElement)) {
      throw new Error('Expected pet save button')
    }
    saveButton.click()

    await waitForText(mounted.container, '"reminderTtlMinutes": 240')
    await waitForText(mounted.container, '"enabled": true')
    await waitForText(mounted.container, '"start": "23:00"')
    await waitForText(mounted.container, '"end": "06:30"')

    await mounted.destroy()
  })

  it('keeps leaked pet records out of the generic workspace agent center list', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const leakedPetRecord: AgentRecord = {
          ...state.agents[0]!,
          id: state.petProfile.id,
          name: '个人宠物记录',
          description: 'Should not appear in the generic agent center list.',
          prompt: 'hidden pet asset',
          updatedAt: 188,
        }

        state.agents = [...state.agents, leakedPetRecord]
      },
    })

    const mounted = await mountRoutedApp('/workspaces/ws-local/agents')

    await waitForText(mounted.container, 'Architect Agent')
    expect(mounted.container.textContent).not.toContain('个人宠物记录')

    await mounted.destroy()
  })
})
