// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

const { bootstrapShellHostMock, savePreferencesMock } = vi.hoisted(() => ({
  bootstrapShellHostMock: vi.fn(),
  savePreferencesMock: vi.fn(),
}))

vi.mock('@/tauri/client', () => ({
  bootstrapShellHost: bootstrapShellHostMock,
  savePreferences: savePreferencesMock,
}))

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'

import type { ShellBootstrap } from '@octopus/schema'

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

function createBootstrap(backendReady: boolean): ShellBootstrap {
  return {
    hostState: {
      platform: 'tauri',
      mode: 'local',
      appVersion: '0.1.0-test',
      cargoWorkspace: true,
      shell: 'tauri2',
    },
    preferences: {
      theme: 'system',
      locale: 'zh-CN',
      compactSidebar: false,
      leftSidebarCollapsed: false,
      rightSidebarCollapsed: false,
      defaultWorkspaceId: 'ws-local',
      lastVisitedRoute: '/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign',
    },
    connections: [],
    backend: {
      baseUrl: 'http://127.0.0.1:43127',
      authToken: 'desktop-test-token',
      state: backendReady ? 'ready' : 'unavailable',
      transport: 'http',
    },
  }
}

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

async function flushUi() {
  await nextTick()
  await new Promise((resolve) => window.setTimeout(resolve, 0))
  await nextTick()
}

describe('App mock-first shell bootstrap', () => {
  beforeEach(async () => {
    bootstrapShellHostMock.mockReset()
    savePreferencesMock.mockReset()
    savePreferencesMock.mockImplementation(async (preferences) => preferences)
    window.localStorage.clear()
    document.body.innerHTML = ''
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()
  })

  it('renders the routed content even when the injected backend is marked unavailable', async () => {
    bootstrapShellHostMock.mockResolvedValue(createBootstrap(false))

    const mounted = mountApp()

    await flushUi()

    expect(mounted.container.querySelector('[data-testid="desktop-backend-guard"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-chat-layout"]')).not.toBeNull()

    mounted.destroy()
  })

  it('does not expose backend recovery actions in mock-first mode', async () => {
    bootstrapShellHostMock.mockResolvedValue(createBootstrap(false))

    const mounted = mountApp()

    await flushUi()

    expect(mounted.container.querySelector('[data-testid="desktop-backend-retry"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="desktop-backend-restart"]')).toBeNull()

    mounted.destroy()
  })
})
