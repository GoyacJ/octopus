// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { resolveMockField } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
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

describe('Conversation surfaces', () => {
  beforeEach(async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()
    document.body.innerHTML = ''
    window.localStorage.clear()
  })

  it('renders conversation tabs on conversation pages but not on overview pages', async () => {
    const mounted = mountApp()

    await nextTick()
    expect(mounted.container.querySelector('[data-testid="conversation-tabs"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-tabs-panel"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-chat-layout"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-detail-panel"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-composer"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-tabs-divider"]')).not.toBeNull()

    await router.push('/workspaces/ws-local/overview?project=proj-redesign')
    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(mounted.container.querySelector('[data-testid="conversation-tabs"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-detail-panel"]')).toBeNull()

    mounted.destroy()
  })

  it('renders bubble messages and the new conversation detail sections', async () => {
    const mounted = mountApp()

    await nextTick()

    const messageStream = mounted.container.querySelector('.message-stream')
    const scrollRegion = mounted.container.querySelector('[data-testid="conversation-message-scroll"]')
    const composerDock = mounted.container.querySelector('[data-testid="conversation-composer-dock"]')
    const detailNavButtons = mounted.container.querySelectorAll('[data-testid="conversation-detail-panel"] nav button')
    const permissionTrigger = mounted.container.querySelector('[data-testid="composer-permission-trigger"]')
    const actorTrigger = mounted.container.querySelector('[data-testid="composer-actor-trigger"]')
    const modelTrigger = mounted.container.querySelector('[data-testid="composer-model-trigger"]')
    expect(messageStream).not.toBeNull()
    expect(scrollRegion).not.toBeNull()
    expect(scrollRegion?.className).toContain('message-stream')
    expect(scrollRegion?.className).toContain('overflow-y-auto')
    expect(composerDock).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-detail-panel"]')).not.toBeNull()
    expect(mounted.container.querySelector('.conversation-hero')).toBeNull()
    expect(mounted.container.querySelector('.message-card')).toBeNull()
    expect(mounted.container.textContent).toContain('请先查看当前桌面端实现状态')
    expect(mounted.container.textContent).toContain('建议先把 schema、共享 UI 和工作台布局拆开')
    expect(mounted.container.textContent).toContain('Thinking...')
    expect(mounted.container.textContent).not.toContain('思考过程')
    expect(detailNavButtons.length).toBeGreaterThanOrEqual(7)
    expect(mounted.container.querySelector('.detail-summary')).toBeNull()
    expect(permissionTrigger?.textContent).toContain('自动')
    expect(actorTrigger?.textContent).toContain('默认智能体')
    expect(modelTrigger?.textContent).toContain('GPT-4o')

    mounted.destroy()
  })

  it('creates and deletes conversations from the conversation tab strip', async () => {
    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    await nextTick()

    const beforeCount = workbench.projectConversations.length
    const createButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-tab-create"]')
    expect(createButton).not.toBeNull()

    createButton?.click()
    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(workbench.projectConversations.length).toBe(beforeCount + 1)
    expect(router.currentRoute.value.name).toBe('project-conversation')
    expect(router.currentRoute.value.params.conversationId).toBe(workbench.currentConversationId)

    const activeConversationId = workbench.currentConversationId
    const deleteButton = mounted.container.querySelector<HTMLButtonElement>(`[data-testid="conversation-tab-delete-${activeConversationId}"]`)
    expect(deleteButton).not.toBeNull()

    deleteButton?.click()
    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(workbench.projectConversations.length).toBe(beforeCount)
    expect(workbench.projectConversations.some((item) => item.id === activeConversationId)).toBe(false)

    mounted.destroy()
  })

  it('collapses the conversation detail pane into a narrow rail and keeps section switching usable', async () => {
    const mounted = mountApp()
    const shell = useShellStore()

    try {
      await nextTick()

      shell.setRightSidebarCollapsed(true)
      await nextTick()
      await new Promise((resolve) => window.setTimeout(resolve, 0))

      expect(mounted.container.querySelector('[data-testid="conversation-detail-panel"]')).toBeNull()
      expect(mounted.container.querySelector('[data-testid="conversation-detail-rail"]')).not.toBeNull()
      expect(mounted.container.querySelector('[data-testid="conversation-detail-rail-nav"]')).not.toBeNull()

      const timelineButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-detail-rail-section-timeline"]')
      expect(timelineButton).not.toBeNull()

      timelineButton?.click()
      await nextTick()

      expect(shell.detailFocus).toBe('timeline')
      expect(shell.rightSidebarCollapsed).toBe(false)
      expect(mounted.container.querySelector('[data-testid="conversation-detail-panel"]')).not.toBeNull()
    }
    finally {
      mounted.destroy()
    }
  })

  it('routes to the empty conversation landing page when the last conversation is deleted', async () => {
    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    try {
      await nextTick()

      await router.push('/workspaces/ws-local/projects/proj-governance/conversations/conv-governance')
      await nextTick()
      await new Promise((resolve) => window.setTimeout(resolve, 0))

      const deleteButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-tab-delete-conv-governance"]')
      expect(deleteButton).not.toBeNull()

      deleteButton?.click()
      await nextTick()
      await new Promise((resolve) => window.setTimeout(resolve, 0))

      expect(workbench.projectConversations).toHaveLength(0)
      expect(router.currentRoute.value.name).toBe('project-conversations')
      expect(mounted.container.querySelector('[data-testid="conversation-empty-state"]')).not.toBeNull()
      expect(mounted.container.querySelector('[data-testid="conversation-detail-panel"]')).toBeNull()
      expect(mounted.container.querySelector('[data-testid="conversation-detail-rail"]')).toBeNull()
    }
    finally {
      mounted.destroy()
    }
  })

  it('creates the first conversation from the empty conversation landing page', async () => {
    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    await nextTick()

    workbench.selectProject('proj-governance')
    workbench.removeConversation('conv-governance')
    await router.push('/workspaces/ws-local/projects/proj-governance/conversations')
    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(mounted.container.querySelector('[data-testid="conversation-empty-state"]')).not.toBeNull()

    const createButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-empty-create"]')
    expect(createButton).not.toBeNull()

    createButton?.click()
    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(workbench.projectConversations).toHaveLength(1)
    expect(router.currentRoute.value.name).toBe('project-conversation')
    expect(
      mounted.container.querySelector('[data-testid="conversation-tab-active"]')?.textContent,
    ).toContain(resolveMockField('conversation', workbench.currentConversationId, 'title', workbench.activeConversation?.title ?? ''))

    mounted.destroy()
  })

  it('queues a message with the default actor while the current run is busy', async () => {
    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    if (runtime.activeRun) {
      runtime.activeRun.status = 'waiting_approval'
    }

    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await nextTick()

    const textarea = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="conversation-runtime-composer-input"]')
    const modelTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="composer-model-trigger"]')
    const permissionTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="composer-permission-trigger"]')
    const actorTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="composer-actor-trigger"]')
    const resourceTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="composer-resource-trigger"]')
    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-runtime-send"]')

    expect(textarea).not.toBeNull()
    expect(modelTrigger).not.toBeNull()
    expect(permissionTrigger).not.toBeNull()
    expect(actorTrigger).not.toBeNull()
    expect(resourceTrigger).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="composer-tool-select"]')).toBeNull()
    expect(sendButton).not.toBeNull()
    expect(actorTrigger?.textContent).toContain('默认智能体')
    expect(modelTrigger?.textContent).toContain('GPT-4o')
    expect(permissionTrigger?.textContent).toContain('自动')

    await runtime.submitTurn({
      content: '把发送框做成和设计图一致。',
      modelId: 'gpt-4o',
      permissionMode: 'auto',
      actorLabel: '默认智能体 · 文案编排小队',
    })
    await nextTick()

    expect(runtime.activeQueue).toHaveLength(1)
    expect(runtime.activeQueue[0]).toMatchObject({
      content: '把发送框做成和设计图一致。',
      modelId: 'gpt-4o',
      permissionMode: 'auto',
      actorLabel: '默认智能体 · 文案编排小队',
    })
    expect(mounted.container.querySelector('[data-testid="conversation-queue-list"]')).not.toBeNull()
    expect(
      mounted.container.querySelector(`[data-testid="conversation-queue-item-${runtime.activeQueue[0]?.id}"]`)?.textContent,
    ).toContain('默认智能体 · 文案编排小队:')
    expect(
      mounted.container.querySelector(`[data-testid="conversation-queue-item-${runtime.activeQueue[0]?.id}"]`)?.textContent,
    ).toContain('把发送框做成和设计图一致。')

    const toggleButton = mounted.container.querySelector<HTMLButtonElement>(`[data-testid="conversation-queue-toggle-${runtime.activeQueue[0]?.id}"]`)
    const removeButton = mounted.container.querySelector<HTMLButtonElement>(`[data-testid="conversation-queue-remove-${runtime.activeQueue[0]?.id}"]`)
    expect(toggleButton).not.toBeNull()
    expect(removeButton).not.toBeNull()

    toggleButton?.click()
    await nextTick()

    expect(mounted.container.querySelector(`[data-testid="conversation-queue-item-${runtime.activeQueue[0]?.id}"]`)?.className).toContain('expanded')

    removeButton?.click()
    await nextTick()

    expect(runtime.activeQueue).toHaveLength(0)

    mounted.destroy()
  })

  it('sends immediately with an explicit actor after the current run completes', async () => {
    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    workbench.completeActiveRun('completed')
    await nextTick()

    workbench.sendMessage({
      content: '继续把消息展示切换成气泡布局。',
      modelId: 'claude-sonnet',
      permissionMode: 'readonly',
      actorKind: 'team',
      actorId: 'team-redesign-copy',
      resourceIds: [],
      attachments: [],
    })
    await nextTick()

    const sentMessage = workbench.conversationMessages.find((message) => message.content === '继续把消息展示切换成气泡布局。')
    expect(sentMessage).toMatchObject({
      senderType: 'user',
      content: '继续把消息展示切换成气泡布局。',
      modelId: 'claude-sonnet',
      permissionMode: 'readonly',
      actorKind: 'team',
      actorId: 'team-redesign-copy',
    })
    expect(mounted.container.textContent).toContain('继续把消息展示切换成气泡布局。')

    mounted.destroy()
  })

  it('routes the composer through runtime messages for the active conversation', async () => {
    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    const textarea = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="conversation-runtime-composer-input"]')
    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-runtime-send"]')
    expect(textarea).not.toBeNull()
    expect(sendButton).not.toBeNull()

    textarea!.value = '请总结当前 desktop runtime 集成进展。'
    textarea!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    sendButton?.click()

    await new Promise((resolve) => window.setTimeout(resolve, 160))
    await nextTick()

    expect(runtime.activeConversationId).toBe('conv-redesign')
    expect(runtime.activeMessages.some((message) => message.content === '请总结当前 desktop runtime 集成进展。')).toBe(true)
    expect(mounted.container.textContent).toContain('请总结当前 desktop runtime 集成进展。')

    runtime.dispose()
    mounted.destroy()
  })
})
