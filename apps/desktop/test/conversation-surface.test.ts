// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useKnowledgeStore } from '@/stores/knowledge'
import type { WorkspaceClient } from '@/tauri/workspace-client'
import * as tauriClient from '@/tauri/client'
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

function configureWorkspaceClient(
  transform: (client: WorkspaceClient) => WorkspaceClient,
) {
  const createWorkspaceClientMock = vi.mocked(tauriClient.createWorkspaceClient)
  const baseImplementation = createWorkspaceClientMock.getMockImplementation()
  expect(baseImplementation).toBeTypeOf('function')

  createWorkspaceClientMock.mockImplementation((context) =>
    transform(baseImplementation!(context) as WorkspaceClient) as ReturnType<typeof tauriClient.createWorkspaceClient>,
  )
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
    expect(mounted.container.querySelector('[data-testid="conversation-message-list"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-context-pane"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-composer"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-model-select"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-permission-select"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-actor-select"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('请先查看当前桌面端实现状态')
    expect(mounted.container.textContent).toContain('建议先把 schema、共享 UI 和工作台布局拆开')

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    textarea.value = '继续推进真实 workspace API 收尾。'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()
    expect(sendButton?.disabled).toBe(false)
    sendButton?.click()

    await waitFor(() => runtime.activeMessages.some(message => message.content === '继续推进真实 workspace API 收尾。'))
    expect(mounted.container.textContent).toContain('继续推进真实 workspace API 收尾。')

    await waitFor(() => runtime.activeMessages.some(message => message.content.includes('Completed request')))
    expect(mounted.container.textContent).toContain('Completed request')

    mounted.destroy()
  })

  it('does not fetch admin access-control collections when opening a conversation', async () => {
    let accessUsersCalls = 0
    let accessRolesCalls = 0
    let accessPermissionCalls = 0

    configureWorkspaceClient(client => ({
      ...client,
      accessControl: {
        ...client.accessControl,
        async listUsers() {
          accessUsersCalls += 1
          return await client.accessControl.listUsers()
        },
        async listRoles() {
          accessRolesCalls += 1
          return await client.accessControl.listRoles()
        },
        async listPermissionDefinitions() {
          accessPermissionCalls += 1
          return await client.accessControl.listPermissionDefinitions()
        },
      },
    }))

    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    expect(accessUsersCalls).toBe(0)
    expect(accessRolesCalls).toBe(0)
    expect(accessPermissionCalls).toBe(0)

    mounted.destroy()
  })

  it('renders runtime actor and artifact labels from real entity records', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    expect(mounted.container.textContent).toContain('Studio Direction Team')
    expect(mounted.container.textContent).toContain('Team')
    expect(mounted.container.textContent).toContain('Runtime Delivery Summary')
    expect(mounted.container.textContent).toContain('v3')
    expect(mounted.container.querySelectorAll('[data-testid="conversation-avatar-image"]').length).toBeGreaterThan(0)

    mounted.destroy()
  })

  it('shows the user message immediately while the assistant response is still pending', async () => {
    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async submitUserTurn(sessionId, input, idempotencyKey) {
          await new Promise(resolve => window.setTimeout(resolve, 120))
          return client.runtime.submitUserTurn(sessionId, input, idempotencyKey)
        },
      },
    }))

    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    textarea.value = '先立即显示这条用户消息。'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()
    sendButton?.click()

    await nextTick()
    expect(textarea.value).toBe('')
    await waitFor(() => runtime.activeMessages.some(message => message.content === '先立即显示这条用户消息。'))
    expect(mounted.container.textContent).toContain('先立即显示这条用户消息。')
    await waitFor(() => runtime.activeMessages.some(message => message.content === 'Thinking…'))
    expect(mounted.container.textContent).toContain('Thinking…')
    expect(runtime.activeMessages.some(message => message.content.includes('Completed request'))).toBe(false)

    await waitFor(() => runtime.activeMessages.some(message => message.content.includes('Completed request')))

    mounted.destroy()
  })

  it('shows live process text on the assistant placeholder before the final result arrives', async () => {
    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async submitUserTurn(sessionId, input, idempotencyKey) {
          await new Promise(resolve => window.setTimeout(resolve, 120))
          return client.runtime.submitUserTurn(sessionId, input, idempotencyKey)
        },
      },
    }))

    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    textarea.value = '先显示实时处理过程。'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()
    sendButton?.click()

    await waitFor(() => runtime.activeMessages.some(message => message.content === 'Thinking…'))
    const processToggle = Array.from(mounted.container.querySelectorAll('button'))
      .filter(button => button.textContent?.includes('Thinking') || button.textContent?.includes('Processing'))
      .at(-1)
    processToggle?.click()
    await nextTick()
    expect(mounted.container.textContent).toContain('Preparing the assistant response.')
    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-inline-tool-calls"]') !== null)
    expect(mounted.container.textContent).toContain('Used Workspace API')
    expect(mounted.container.textContent).toContain('Called 1 time')

    await waitFor(() => runtime.activeMessages.some(message => message.content.includes('Completed request')))

    mounted.destroy()
  })

  it('renders inline approval actions on the assistant placeholder and resolves them', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    textarea.value = 'Run pwd in the workspace terminal.'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()
    sendButton?.click()

    await waitFor(() => runtime.activeMessages.some(message => !!message.approval))
    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-inline-approval"]') !== null)
    expect(mounted.container.textContent).toContain('Approve workspace command execution')
    expect(mounted.container.textContent).toContain('Run pwd in the workspace terminal.')
    expect(mounted.container.querySelector('[data-testid="conversation-inline-approve"]')).not.toBeNull()
    expect(mounted.container.querySelector('[data-testid="conversation-inline-reject"]')).not.toBeNull()

    const approveButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-inline-approve"]')
    approveButton?.click()

    await waitFor(() => runtime.pendingApproval === null)
    await waitFor(() => runtime.activeMessages.some(message => message.content === 'Command approved and execution completed.'))
    expect(mounted.container.querySelector('[data-testid="conversation-inline-approval"]')).toBeNull()

    mounted.destroy()
  })

  it('keeps approval actions only on the inline assistant card', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    textarea.value = 'Run pwd in the workspace terminal.'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()
    sendButton?.click()

    await waitFor(() => runtime.pendingApproval !== null)
    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-inline-approval"]') !== null)
    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-runtime-mediation"]') !== null)

    expect(mounted.container.querySelectorAll('[data-testid="conversation-inline-approve"]')).toHaveLength(1)
    expect(mounted.container.querySelectorAll('[data-testid="conversation-inline-reject"]')).toHaveLength(1)
    expect(
      mounted.container
        .querySelector('[data-testid="conversation-runtime-mediation"]')
        ?.querySelectorAll('button'),
    ).toHaveLength(0)

    mounted.destroy()
  })

  it('seeds the permission selector from the effective project runtime config default mode', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      preloadConversationMessages: true,
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const projectConfig = state.runtimeProjectConfigs['proj-redesign']
        if (!projectConfig) {
          throw new Error('Expected proj-redesign runtime config')
        }

        state.runtimeProjectConfigs['proj-redesign'] = {
          ...projectConfig,
          effectiveConfig: {
            ...(projectConfig.effectiveConfig as Record<string, any>),
            permissions: {
              defaultMode: 'danger-full-access',
              maxMode: 'danger-full-access',
            },
          },
          sources: projectConfig.sources.map((source) => (
            source.scope === 'project'
              ? {
                  ...source,
                  document: {
                    ...((source.document as Record<string, any>) ?? {}),
                    permissions: {
                      defaultMode: 'danger-full-access',
                      maxMode: 'danger-full-access',
                    },
                  },
                }
              : source
          )),
        }
      },
    })

    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)
    await waitFor(() =>
      (mounted.container.querySelector('[data-testid="conversation-permission-select"]') as HTMLSelectElement | null)?.value === 'danger-full-access',
    )

    const permissionSelect = mounted.container.querySelector('[data-testid="conversation-permission-select"]') as HTMLSelectElement
    expect(permissionSelect.value).toBe('danger-full-access')

    mounted.destroy()
  })

  it('queues follow-up turns above the composer while a run is pending and lets the user remove them', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()

    textarea.value = 'Run pwd in the workspace terminal.'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    sendButton?.click()

    await waitFor(() => runtime.pendingApproval !== null)
    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-inline-approval"]') !== null)

    textarea.value = '把这条加入队列。'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    sendButton?.click()

    await waitFor(() => runtime.activeQueue.length === 1)
    expect(runtime.activeQueue[0]?.content).toBe('把这条加入队列。')

    const queueList = mounted.container.querySelector('[data-testid="conversation-queue-list"]')
    const composer = mounted.container.querySelector('[data-testid="conversation-composer"]')
    expect(queueList).not.toBeNull()
    expect(composer).not.toBeNull()
    expect(queueList?.compareDocumentPosition(composer as Node) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(mounted.container.textContent).toContain('把这条加入队列。')

    const removeButton = mounted.container.querySelector<HTMLButtonElement>(`[data-testid="conversation-queue-remove-${runtime.activeQueue[0]?.id}"]`)
    expect(removeButton).not.toBeNull()
    removeButton?.click()

    await waitFor(() => runtime.activeQueue.length === 0)
    expect(mounted.container.querySelector('[data-testid="conversation-queue-list"]')).toBeNull()

    mounted.destroy()
  })

  it('keeps the draft and shows the runtime error when submission fails', async () => {
    configureWorkspaceClient((client) => ({
      ...client,
      runtime: {
        ...client.runtime,
        async submitUserTurn() {
          throw new Error('missing configured credential env var `ANTHROPIC_API_KEY` for provider `anthropic`')
        },
      },
    }))

    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    textarea.value = '继续推进真实 workspace API 收尾。'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()
    sendButton?.click()

    await waitFor(() => runtime.error.includes('missing configured credential env var'))
    expect(textarea.value).toBe('继续推进真实 workspace API 收尾。')
    expect(mounted.container.textContent).toContain('missing configured credential env var `ANTHROPIC_API_KEY` for provider `anthropic`')
    expect(mounted.container.querySelector('[role="alert"]')?.textContent).toContain('missing configured credential env var')

    mounted.destroy()
  })

  it('renders runtime-backed summary, memories, and tools in the conversation context pane', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign?mode=context')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    expect(mounted.container.textContent).toContain(String(i18n.global.t('conversation.detail.summary.tokenUsage')))
    expect(mounted.container.textContent).toContain('1000')

    Array.from(mounted.container.querySelectorAll('button')).find(button => button.textContent?.includes('记忆'))?.click()
    await waitFor(() => mounted.container.textContent?.includes('建议先把 schema、共享 UI 和工作台布局拆开') ?? false)
    expect(mounted.container.textContent).toContain('Studio Direction Team')

    Array.from(mounted.container.querySelectorAll('button')).find(button => button.textContent?.includes('工具'))?.click()
    await waitFor(() => mounted.container.textContent?.includes('Workspace API') ?? false)
    expect(mounted.container.textContent).toContain('workspace-api')

    mounted.destroy()
  })

  it('renders deliverable metadata and preview chrome in the conversation context pane', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign?mode=deliverable&deliverable=artifact-run-conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)
    await waitFor(() => mounted.container.querySelector('[data-testid="deliverable-preview-panel"]') !== null)

    expect(mounted.container.querySelector('[data-testid="deliverable-version-list"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Runtime Delivery Summary')
    expect(mounted.container.textContent).toContain('artifact-run-conv-redesign')
    expect(mounted.container.textContent).toContain(String(i18n.global.t('conversation.detail.deliverables.previewTitle')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('conversation.detail.deliverables.contentType')))
    expect(mounted.container.textContent).toContain('Runtime Delivery Summary.md')
    expect(mounted.container.textContent).toContain('text/markdown')

    mounted.destroy()
  })

  it('renders the selected deliverable as a preview surface with version history', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign?mode=deliverable&deliverable=artifact-run-conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)
    await waitFor(() => mounted.container.querySelector('[data-testid="deliverable-preview-panel"]') !== null)

    expect(mounted.container.querySelector('[data-testid="deliverable-version-list"]')).not.toBeNull()
    expect(mounted.container.textContent).toContain('Version 3 content for artifact-run-conv-redesign.')
    expect(mounted.container.textContent).toContain('Runtime Delivery Summary v2')

    mounted.destroy()
  })

  it('switches deliverable versions in place and syncs the route query', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign?mode=deliverable&deliverable=artifact-run-conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)
    await waitFor(() => mounted.container.querySelector('[data-testid="deliverable-version-2"]') !== null)

    const versionButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="deliverable-version-2"]')
    expect(versionButton).not.toBeNull()
    versionButton?.click()

    await waitFor(() => router.currentRoute.value.query.version === '2')
    await waitFor(() => mounted.container.textContent?.includes('Version 2 content for artifact-run-conv-redesign.') ?? false)
    expect(mounted.container.textContent).not.toContain('Version 3 content for artifact-run-conv-redesign.')

    mounted.destroy()
  })

  it('allows inline deliverable editing and saves a new version', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign?mode=deliverable&deliverable=artifact-run-conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)
    await waitFor(() => mounted.container.querySelector('[data-testid="deliverable-edit-button"]') !== null)

    const editButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="deliverable-edit-button"]')
    expect(editButton).not.toBeNull()
    editButton?.click()

    await waitFor(() => mounted.container.querySelector('[data-testid="deliverable-editor"]') !== null)

    const editor = mounted.container.querySelector<HTMLTextAreaElement>('[data-testid="deliverable-editor"]')
    expect(editor).not.toBeNull()
    editor!.value = '# Runtime Delivery Summary\n\nTask 5 saved version content.'
    editor!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const saveButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="deliverable-save-version"]')
    expect(saveButton).not.toBeNull()
    saveButton?.click()

    await waitFor(() => router.currentRoute.value.query.version === '4')
    await waitFor(() => mounted.container.textContent?.includes('Task 5 saved version content.') ?? false)
    expect(mounted.container.textContent).toContain('v4')

    mounted.destroy()
  })

  it('promotes the selected deliverable and forks it into a new conversation from the context pane', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign?mode=deliverable&deliverable=artifact-run-conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()
    const knowledgeStore = useKnowledgeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)
    await waitFor(() => mounted.container.querySelector('[data-testid="deliverable-preview-panel"]') !== null)

    const promoteButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-deliverable-promote"]')
    expect(promoteButton).not.toBeNull()
    promoteButton?.click()

    await waitFor(() =>
      knowledgeStore.activeProjectKnowledge.some(entry => entry.sourceRef === 'artifact-run-conv-redesign'),
    )
    expect(mounted.container.textContent).toContain(String(i18n.global.t('deliverables.status.promoted')))

    const forkButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-deliverable-fork"]')
    expect(forkButton).not.toBeNull()
    forkButton?.click()

    await waitFor(() =>
      typeof router.currentRoute.value.params.conversationId === 'string'
      && router.currentRoute.value.params.conversationId.startsWith('conv-fork-artifact-run-conv-redesign-'),
    )
    expect(router.currentRoute.value.fullPath).toContain('/conversations/conv-fork-artifact-run-conv-redesign-')

    mounted.destroy()
  })

  it('scopes the composer model and actor selectors to the project settings assignments', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      preloadConversationMessages: true,
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
        const projectConfig = state.runtimeProjectConfigs['proj-redesign']
        const projectSource = projectConfig?.sources.find(source => source.scope === 'project')
        if (
          projectSource
          && projectSource.document
          && typeof projectSource.document === 'object'
          && !Array.isArray(projectSource.document)
        ) {
          ;(projectSource.document as Record<string, any>).projectSettings = {
            ...((projectSource.document as Record<string, any>).projectSettings ?? {}),
            agents: {
              enabledAgentIds: ['agent-architect', 'agent-template-finance'],
              enabledTeamIds: ['team-studio', 'team-template-finance'],
            },
          }
        }
        if (projectConfig) {
          const effectiveConfig = projectConfig.effectiveConfig as Record<string, any>
          state.runtimeProjectConfigs['proj-redesign'] = {
            ...projectConfig,
            effectiveConfig: {
              ...effectiveConfig,
              projectSettings: {
                ...(effectiveConfig.projectSettings ?? {}),
                agents: {
                  enabledAgentIds: ['agent-architect', 'agent-template-finance'],
                  enabledTeamIds: ['team-studio', 'team-template-finance'],
                },
              },
            },
          }
        }
        state.projectAgentLinks['proj-redesign'] = []
        state.projectTeamLinks['proj-redesign'] = []
      },
    })

    configureWorkspaceClient((client) => ({
      ...client,
      catalog: {
        ...client.catalog,
        async getSnapshot() {
          const snapshot = await client.catalog.getSnapshot()
          return {
            ...snapshot,
            configuredModels: [
              {
                configuredModelId: 'anthropic-primary',
                name: 'Claude Primary',
                providerId: 'anthropic',
                modelId: 'claude-sonnet-4-5',
                credentialRef: 'env:ANTHROPIC_API_KEY',
                tokenUsage: {
                  usedTokens: 0,
                  exhausted: false,
                },
                enabled: true,
                source: 'workspace',
              },
              {
                configuredModelId: 'anthropic-alt',
                name: 'Claude Alt',
                providerId: 'anthropic',
                modelId: 'claude-sonnet-4-5',
                credentialRef: 'env:ANTHROPIC_ALT_API_KEY',
                tokenUsage: {
                  usedTokens: 0,
                  exhausted: false,
                },
                enabled: true,
                source: 'workspace',
              },
            ],
            defaultSelections: {
              conversation: {
                configuredModelId: 'anthropic-primary',
                providerId: 'anthropic',
                modelId: 'claude-sonnet-4-5',
                surface: 'conversation',
              },
            },
          } as any
        },
      },
    }))
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    const modelSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="conversation-model-select"]')
    expect(modelSelect).not.toBeNull()
    const modelOptionLabels = Array.from(modelSelect?.querySelectorAll('option') ?? []).map(option => option.textContent?.trim())
    expect(modelOptionLabels).toContain('Claude Primary')
    expect(modelOptionLabels).not.toContain('Claude Alt')

    const actorSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="conversation-actor-select"]')
    expect(actorSelect).not.toBeNull()
    const actorOptionLabels = Array.from(actorSelect?.querySelectorAll('option') ?? []).map(option => option.textContent?.trim())
    expect(actorOptionLabels).toContain('Architect Agent')
    expect(actorOptionLabels).toContain('Redesign Copilot')
    expect(actorOptionLabels).toContain('Finance Planner Template')
    expect(actorOptionLabels).toContain('Studio Direction Team')
    expect(actorOptionLabels).toContain('Redesign Tiger Team')
    expect(actorOptionLabels).toContain('Finance Ops Template')
    expect(actorOptionLabels).not.toContain('Coder Agent')

    mounted.destroy()
  })

  it('falls back to assigned seeded models when project settings are absent', async () => {
    vi.restoreAllMocks()
    installWorkspaceApiFixture({
      preloadConversationMessages: true,
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }

        const project = state.projects.find(item => item.id === 'proj-redesign')
        if (!project?.assignments?.models) {
          throw new Error('Expected proj-redesign model assignments')
        }

        project.assignments.models.configuredModelIds = ['claude-sonnet-4-5']
        project.assignments.models.defaultConfiguredModelId = 'claude-sonnet-4-5'

        state.catalog.configuredModels = [
          {
            configuredModelId: 'claude-sonnet-4-5',
            name: 'Claude Sonnet 4.5',
            providerId: 'anthropic',
            modelId: 'claude-sonnet-4-5',
            credentialRef: 'env:ANTHROPIC_API_KEY',
            tokenUsage: {
              usedTokens: 0,
              exhausted: false,
            },
            enabled: true,
            source: 'seeded',
            status: 'unconfigured',
            configured: false,
          },
        ]
        state.catalog.defaultSelections = {
          conversation: {
            configuredModelId: 'claude-sonnet-4-5',
            providerId: 'anthropic',
            modelId: 'claude-sonnet-4-5',
            surface: 'conversation',
          },
        }

        const projectConfig = state.runtimeProjectConfigs['proj-redesign']
        if (!projectConfig) {
          throw new Error('Expected proj-redesign runtime config')
        }

        state.runtimeProjectConfigs['proj-redesign'] = {
          ...projectConfig,
          effectiveConfig: {
            approvals: {
              defaultMode: 'manual',
            },
          },
          sources: projectConfig.sources.map(source =>
            source.scope === 'project'
              ? {
                  ...source,
                  document: {
                    approvals: {
                      defaultMode: 'manual',
                    },
                  },
                }
              : source
          ),
        }
      },
    })

    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)
    await waitFor(() => {
      const modelSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="conversation-model-select"]')
      const modelOptionLabels = Array.from(modelSelect?.querySelectorAll('option') ?? []).map(option => option.textContent?.trim())
      return modelOptionLabels.includes('Claude Sonnet 4.5')
    })

    const modelSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="conversation-model-select"]')
    expect(modelSelect).not.toBeNull()
    const modelOptionLabels = Array.from(modelSelect?.querySelectorAll('option') ?? []).map(option => option.textContent?.trim())
    expect(modelOptionLabels).toContain('Claude Sonnet 4.5')
    expect(modelSelect?.disabled).toBe(false)

    const textarea = mounted.container.querySelector('textarea') as HTMLTextAreaElement
    textarea.value = '继续推进真实 workspace API 收尾。'
    textarea.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()

    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()
    expect(sendButton?.disabled).toBe(false)

    mounted.destroy()
  })

  it('keeps configured workspace agents and teams visible in the composer even without project link records', async () => {
    configureWorkspaceClient((client) => ({
      ...client,
      agents: {
        ...client.agents,
        async listProjectLinks() {
          return []
        },
      },
      teams: {
        ...client.teams,
        async listProjectLinks() {
          return []
        },
      },
    }))

    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign')
    await router.isReady()

    const mounted = mountApp()

    await waitFor(() => {
      const actorSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="conversation-actor-select"]')
      const actorOptionLabels = Array.from(actorSelect?.querySelectorAll('option') ?? []).map(option => option.textContent?.trim())
      return actorOptionLabels.includes('Architect Agent') && actorOptionLabels.includes('Studio Direction Team')
    })

    const actorSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="conversation-actor-select"]')
    expect(actorSelect).not.toBeNull()
    const actorOptionLabels = Array.from(actorSelect?.querySelectorAll('option') ?? []).map(option => option.textContent?.trim())
    expect(actorOptionLabels).toContain('Architect Agent')
    expect(actorOptionLabels).toContain('Studio Direction Team')

    mounted.destroy()
  })

  it('keeps composer model and actor selects empty and disabled when the project has no assignments', async () => {
    await router.push('/workspaces/ws-local/projects/proj-governance/conversations/conv-governance')
    await router.isReady()

    const mounted = mountApp()

    await waitFor(() => mounted.container.querySelector('[data-testid="conversation-composer"]') !== null)

    const modelSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="conversation-model-select"]')
    expect(modelSelect).not.toBeNull()
    expect(modelSelect?.disabled).toBe(true)
    expect(modelSelect?.querySelectorAll('option')).toHaveLength(0)

    const actorSelect = mounted.container.querySelector<HTMLSelectElement>('[data-testid="conversation-actor-select"]')
    expect(actorSelect).not.toBeNull()
    expect(actorSelect?.disabled).toBe(true)
    expect(actorSelect?.querySelectorAll('option')).toHaveLength(0)

    const sendButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-send-button"]')
    expect(sendButton).not.toBeNull()
    expect(sendButton?.disabled).toBe(true)

    mounted.destroy()
  })

  it('renders conversation-linked resources in the resource detail pane', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations/conv-redesign?mode=context')
    await router.isReady()

    const mounted = mountApp()
    const runtime = useRuntimeStore()

    await waitFor(() => runtime.activeMessages.length >= 3)

    expect(mounted.container.textContent).toContain('Desktop Redesign API')
    expect(mounted.container.textContent).toContain(String(i18n.global.t('enum.projectResourceOrigin.generated')))
    expect(mounted.container.textContent).toContain('artifact-run-conv-redesign')

    const resourceFilter = mounted.container.querySelector(`input[placeholder="${String(i18n.global.t('conversation.detail.resources.filterPlaceholder'))}"]`) as HTMLInputElement
    resourceFilter.value = 'API'
    resourceFilter.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    expect(mounted.container.textContent).toContain('Desktop Redesign API')

    mounted.destroy()
  })

  it('renders the empty conversation state and creates a new routed conversation from it', async () => {
    await router.push('/workspaces/ws-local/projects/proj-redesign/conversations')
    await router.isReady()

    const mounted = mountApp()

    await waitFor(() => mounted.container.textContent?.includes(String(i18n.global.t('conversation.empty.title'))) ?? false)

    expect(mounted.container.textContent).toContain(String(i18n.global.t('conversation.empty.title')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('conversation.detail.empty.title')))
    expect(mounted.container.textContent).toContain(String(i18n.global.t('conversation.detail.empty.description')))

    const opsButton = mounted.container.querySelector<HTMLButtonElement>('[data-testid="conversation-context-section-ops"]')
    expect(opsButton).not.toBeNull()
    expect(opsButton?.disabled).toBe(true)
    opsButton?.click()
    await nextTick()
    expect(String(router.currentRoute.value.name)).toBe('project-conversations')
    expect(mounted.container.textContent).not.toContain('Fatal startup error')

    const createButton = Array.from(mounted.container.querySelectorAll('button')).find(button =>
      button.textContent?.includes(String(i18n.global.t('conversation.empty.create'))))
    createButton?.click()

    await waitFor(() => String(router.currentRoute.value.name) === 'project-conversation')
    expect(String(router.currentRoute.value.params.conversationId)).toMatch(/^conversation-/)
    await waitFor(() =>
      (mounted.container.querySelector('[data-testid="conversation-context-section-ops"]') as HTMLButtonElement | null)?.disabled === false,
    )
    expect(mounted.container.textContent).not.toContain(String(i18n.global.t('conversation.detail.empty.title')))

    mounted.destroy()
  })
})
