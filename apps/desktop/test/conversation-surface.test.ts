// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useRuntimeStore } from '@/stores/runtime'
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

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for conversation surface state')
    }

    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

describe('Conversation surfaces', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      preloadConversationMessages: true,
    })
    document.body.innerHTML = ''
  })

  it('renders runtime-backed messages and lets the user submit a new turn', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)
    expect(mounted.container.querySelector('[data-testid="conversation-tabs"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('请先查看当前桌面端实现状态')
    expect(mounted.container.textContent).toContain('建议先把 schema、共享 UI 和工作台布局拆开')

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    textarea.value = '继续推进真实 workspace API 收尾。'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const sendButton = Array.from(mounted.container.querySelectorAll('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('conversation.composer.send'))))
    sendButton?.click()

    await waitFor(() => runtime.activeMessages.some(message => message.content.includes('Completed request')))
    expect(mounted.container.textContent).toContain('Completed request')

    mounted.destroy()
  })

  it('renders the empty conversation state and creates a new routed conversation from it', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations')
    await router.isReady()

    const mounted = mountApp()

    await waitFor(() => mounted.container.textContent?.includes(String(i18n.global.t('conversation.empty.title'))) ?? false)

    expect(mounted.container.textContent).toContain(String(i18n.global.t('conversation.empty.title')))

    const createButton = Array.from(mounted.container.querySelectorAll('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('conversation.empty.create'))))
    createButton?.click()

    await waitFor(() => String(router.currentRoute.value.name) === 'project-conversation')
    expect(String(router.currentRoute.value.params.conversationId)).toMatch(/^conversation-/)

    mounted.destroy()
  })
})
