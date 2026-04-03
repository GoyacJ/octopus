// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { resolveMockField } from '@/i18n/copy'
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
    const detailSections = mounted.container.querySelectorAll('[data-testid^="conversation-detail-section-"]')
    const firstMessageRow = mounted.container.querySelector('[data-testid="conversation-message-bubble-msg-redesign-1"]')
    const secondMessageRow = mounted.container.querySelector('[data-testid="conversation-message-bubble-msg-redesign-2"]')
    const firstBubble = firstMessageRow?.querySelector('.message-bubble')
    const firstFooter = firstMessageRow?.querySelector('.message-footer')
    const secondProcessSummary = mounted.container.querySelector('[data-testid="conversation-message-process-summary-msg-redesign-2"]')
    const permissionTrigger = mounted.container.querySelector('[data-testid="composer-permission-trigger"]')
    const actorTrigger = mounted.container.querySelector('[data-testid="composer-actor-trigger"]')
    const modelTrigger = mounted.container.querySelector('[data-testid="composer-model-trigger"]')
    expect(messageStream).not.toBeNull()
    expect(scrollRegion).not.toBeNull()
    expect(scrollRegion?.className).toContain('scroll-y')
    expect(composerDock).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-detail-nav"]')).not.toBeNull()
    expect(mounted.container.querySelector('.conversation-hero')).toBeNull()
    expect(mounted.container.querySelector('.message-card')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-message-bubble-msg-redesign-1"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-message-bubble-msg-redesign-2"]')).not.toBeNull()
    expect(firstBubble?.querySelector('.message-actions')).toBeNull()
    expect(firstMessageRow?.querySelector('.message-actions')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-message-detail-toggle-msg-redesign-1"]')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-message-process-summary-msg-redesign-1"]')).toBeNull()
    expect(firstMessageRow?.querySelector('[data-testid="conversation-message-rollback-msg-redesign-1"]')).not.toBeNull()
    expect(firstFooter?.textContent).not.toContain('tokens')
    expect(firstFooter?.textContent).not.toContain('次工具调用')
    expect(secondProcessSummary).not.toBeNull()
    expect(secondProcessSummary?.textContent).toContain('思考过程')
    expect(secondMessageRow?.querySelector('[data-testid="conversation-message-rollback-msg-redesign-2"]')).toBeNull()
    expect(detailSections).toHaveLength(7)
    expect(mounted.container.querySelector('.detail-summary')).toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-detail-section-summary"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-detail-section-memories"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-detail-section-tools"]')).not.toBeNull()
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
    expect(router.currentRoute.value.name).toBe('conversation')
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
    expect(router.currentRoute.value.name).toBe('conversation')
    expect(
      mounted.container.querySelector('[data-testid="conversation-tab-active"]')?.textContent,
    ).toContain(resolveMockField('conversation', workbench.currentConversationId, 'title', workbench.activeConversation?.title ?? ''))

    mounted.destroy()
  })

  it('queues a message with the default actor while the current run is busy', async () => {
    const mounted = mountApp()
    const workbench = useWorkbenchStore()

    await nextTick()

    workbench.selectConversation('conv-redesign')
    if (workbench.activeRun) {
      workbench.activeRun.status = 'waiting_approval'
    }

    const textarea = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="conversation-composer-input"]')
    const modelTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="composer-model-trigger"]')
    const permissionTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="composer-permission-trigger"]')
    const actorTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="composer-actor-trigger"]')
    const resourceTrigger = mounted.container.querySelector<HTMLButtonElement>('[data-testid="composer-resource-trigger"]')
    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-composer-send"]')

    expect(textarea).not.toBeNull()
    expect(modelTrigger).not.toBeNull()
    expect(permissionTrigger).not.toBeNull()
    expect(actorTrigger).not.toBeNull()
    expect(resourceTrigger).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="composer-tool-select"]')).toBeNull()
    expect(sendButton).not.toBeNull()
    expect(actorTrigger?.textContent).toContain('默认智能体')

    resourceTrigger?.click()
    await nextTick()

    expect(document.body.querySelector('[data-testid="composer-resource-menu"]')).not.toBeNull()
    const uploadFileButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="resource-action-upload-file"]')
    expect(uploadFileButton).not.toBeNull()
    uploadFileButton?.click()
    await nextTick()

    workbench.sendMessage({
      content: '把发送框做成和设计图一致。',
      modelId: 'claude-sonnet',
      permissionMode: 'readonly',
      actorKind: undefined,
      actorId: undefined,
      resourceIds: workbench.projectResources
        .filter((resource) => resource.kind === 'file')
        .slice(0, 1)
        .map((resource) => resource.id),
      attachments: workbench.projectResources
        .filter((resource) => resource.kind === 'file')
        .slice(0, 1)
        .map((resource) => ({
          id: resource.id,
          name: resource.name,
          kind: 'file' as const,
        })),
    })
    await nextTick()

    expect(workbench.activeConversationQueue).toHaveLength(1)
    expect(workbench.activeConversationQueue[0]).toMatchObject({
      content: '把发送框做成和设计图一致。',
      modelId: 'claude-sonnet',
      permissionMode: 'readonly',
      requestedActorKind: undefined,
      requestedActorId: undefined,
      resolvedActorKind: 'team',
      resolvedActorId: 'team-redesign-copy',
    })
    expect(workbench.activeConversationQueue[0]?.attachments?.[0]?.kind).toBe('file')
    expect(workbench.activeConversationQueue[0]?.resourceIds?.length).toBeGreaterThan(0)
    expect(mounted.container.querySelector('[data-testid="conversation-queue-list"]')).not.toBeNull()
    expect(
      mounted.container.querySelector(`[data-testid="conversation-queue-item-${workbench.activeConversationQueue[0]?.id}"]`)?.textContent,
    ).toContain(`${resolveMockField('team', 'team-redesign-copy', 'name', 'team-redesign-copy')}:把发送框做成和设计图一致。`)

    const toggleButton = mounted.container.querySelector<HTMLButtonElement>(`[data-testid="conversation-queue-toggle-${workbench.activeConversationQueue[0]?.id}"]`)
    const removeButton = mounted.container.querySelector<HTMLButtonElement>(`[data-testid="conversation-queue-remove-${workbench.activeConversationQueue[0]?.id}"]`)
    expect(toggleButton).not.toBeNull()
    expect(removeButton).not.toBeNull()

    toggleButton?.click()
    await nextTick()

    expect(mounted.container.querySelector(`[data-testid="conversation-queue-item-${workbench.activeConversationQueue[0]?.id}"]`)?.className).toContain('expanded')

    removeButton?.click()
    await nextTick()

    expect(workbench.activeConversationQueue).toHaveLength(0)

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
    expect(mounted.container.querySelector(`[data-testid="conversation-message-bubble-${sentMessage?.id}"]`)).not.toBeNull()

    mounted.destroy()
  })
})
