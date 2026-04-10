import { vi } from 'vitest'

import * as tauriClient from '@/tauri/client'

import { WORKSPACE_CONNECTIONS, WORKSPACE_SESSIONS, clone, createHostBootstrap } from './workspace-fixture-bootstrap'
import { createWorkspaceClientFixture } from './workspace-fixture-client'
import { createWorkspaceFixtureState, type FixtureOptions } from './workspace-fixture-state'
import {
  createRuntimeMessage,
  createSessionDetail,
} from './workspace-fixture-runtime'
import type { RuntimeSessionState } from './workspace-fixture-runtime'

export function installWorkspaceApiFixture(options: FixtureOptions = {}): void {
  const hostBootstrap = createHostBootstrap()
  const workspaceStates = new Map(
    WORKSPACE_CONNECTIONS.map(connection => [connection.workspaceConnectionId, createWorkspaceFixtureState(connection, options)]),
  )

  const syncStoredSessions = () => {
    if (typeof window === 'undefined') {
      return
    }

    if (options.preloadWorkspaceSessions === false) {
      window.localStorage.removeItem('octopus-workspace-sessions')
      return
    }

    window.localStorage.setItem('octopus-workspace-sessions', JSON.stringify(
      Object.fromEntries(WORKSPACE_SESSIONS.map(session => [session.workspaceConnectionId, session])),
    ))
  }

  if (options.preloadConversationMessages) {
    const state = workspaceStates.get('conn-local')
    if (state) {
      const detail = createSessionDetail('conv-redesign', 'proj-redesign', 'Conversation Redesign')
      const runtimeState: RuntimeSessionState = {
        detail,
        events: [],
        nextSequence: 1,
      }
      const preloadedMessages = [
        createRuntimeMessage(runtimeState, 'user', 'You', '请先查看当前桌面端实现状态'),
        (() => {
          runtimeState.nextSequence += 1
          return createRuntimeMessage(runtimeState, 'assistant', 'Studio Direction Team · Team', '建议先把 schema、共享 UI 和工作台布局拆开', 'gpt-4o', 'gpt-4o', 'GPT-4o', 'team', 'team-studio')
        })(),
        (() => {
          runtimeState.nextSequence += 1
          return createRuntimeMessage(runtimeState, 'assistant', 'Architect Agent · Agent', 'Thinking...', 'gpt-4o', 'gpt-4o', 'GPT-4o', 'agent', 'agent-architect')
        })(),
      ]
      runtimeState.nextSequence += 1
      runtimeState.detail.messages = preloadedMessages
      runtimeState.detail.summary.lastMessagePreview = preloadedMessages.at(-1)?.content
      runtimeState.detail.summary.updatedAt = 90
      runtimeState.detail.run = {
        ...runtimeState.detail.run,
        status: 'completed',
        currentStep: 'runtime.run.completed',
        updatedAt: 90,
        nextAction: 'runtime.run.idle',
        modelId: 'gpt-4o',
      }
      runtimeState.detail.summary.status = 'completed'
      state.runtimeSessions.set(runtimeState.detail.summary.id, runtimeState)
    }
  }

  if (options.localRuntimeConfigTransform) {
    const localState = workspaceStates.get('conn-local')
    if (localState) {
      localState.runtimeWorkspaceConfig = options.localRuntimeConfigTransform(clone(localState.runtimeWorkspaceConfig))
    }
  }

  syncStoredSessions()

  vi.spyOn(tauriClient, 'bootstrapShellHost').mockImplementation(async () => {
    syncStoredSessions()
    return clone(hostBootstrap)
  })
  vi.spyOn(tauriClient, 'savePreferences').mockImplementation(async preferences => clone(preferences))
  vi.spyOn(tauriClient, 'getHostUpdateStatus').mockResolvedValue({
    currentVersion: hostBootstrap.hostState.appVersion,
    currentChannel: hostBootstrap.preferences.updateChannel,
    state: 'idle',
    latestRelease: null,
    lastCheckedAt: null,
    progress: null,
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
  })
  vi.spyOn(tauriClient, 'checkHostUpdate').mockImplementation(async channel => ({
    currentVersion: hostBootstrap.hostState.appVersion,
    currentChannel: channel ?? hostBootstrap.preferences.updateChannel,
    state: 'update_available',
    latestRelease: {
      version: '0.2.1',
      channel: channel ?? hostBootstrap.preferences.updateChannel,
      publishedAt: '2026-04-09T08:00:00.000Z',
      notes: '本次更新聚焦版本中心、更新流程和更清晰的产品化说明。',
      notesUrl: 'https://example.test/releases/0.2.1',
    },
    lastCheckedAt: 1_710_000_000_000,
    progress: null,
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
  }))
  vi.spyOn(tauriClient, 'downloadHostUpdate').mockResolvedValue({
    currentVersion: hostBootstrap.hostState.appVersion,
    currentChannel: hostBootstrap.preferences.updateChannel,
    state: 'downloaded',
    latestRelease: {
      version: '0.2.1',
      channel: hostBootstrap.preferences.updateChannel,
      publishedAt: '2026-04-09T08:00:00.000Z',
      notes: '本次更新聚焦版本中心、更新流程和更清晰的产品化说明。',
      notesUrl: 'https://example.test/releases/0.2.1',
    },
    lastCheckedAt: 1_710_000_000_000,
    progress: {
      downloadedBytes: 1024,
      totalBytes: 1024,
      percent: 100,
    },
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
  })
  vi.spyOn(tauriClient, 'installHostUpdate').mockResolvedValue({
    currentVersion: hostBootstrap.hostState.appVersion,
    currentChannel: hostBootstrap.preferences.updateChannel,
    state: 'installing',
    latestRelease: {
      version: '0.2.1',
      channel: hostBootstrap.preferences.updateChannel,
      publishedAt: '2026-04-09T08:00:00.000Z',
      notes: '本次更新聚焦版本中心、更新流程和更清晰的产品化说明。',
      notesUrl: 'https://example.test/releases/0.2.1',
    },
    lastCheckedAt: 1_710_000_000_000,
    progress: {
      downloadedBytes: 1024,
      totalBytes: 1024,
      percent: 100,
    },
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
  })
  vi.spyOn(tauriClient, 'healthcheck').mockResolvedValue({
    backend: { state: 'ready', transport: 'http' },
  })
  vi.spyOn(tauriClient, 'pickAgentBundleArchive').mockResolvedValue(null)
  vi.spyOn(tauriClient, 'pickAgentBundleFolder').mockResolvedValue(null)
  vi.spyOn(tauriClient, 'pickSkillArchive').mockResolvedValue(null)
  vi.spyOn(tauriClient, 'pickSkillFolder').mockResolvedValue(null)
  vi.spyOn(tauriClient, 'saveAgentBundleExport').mockResolvedValue()
  vi.spyOn(tauriClient, 'listNotifications').mockResolvedValue({
    notifications: [],
    unread: {
      total: 0,
      byScope: {
        app: 0,
        workspace: 0,
        user: 0,
      },
    },
  })
  vi.spyOn(tauriClient, 'createNotification').mockImplementation(async input => ({
    id: `notif-${Date.now()}`,
    scopeKind: input.scopeKind,
    scopeOwnerId: input.scopeOwnerId,
    level: input.level ?? 'info',
    title: input.title ?? 'Notification',
    body: input.body ?? '',
    source: input.source ?? 'fixture',
    createdAt: Date.now(),
    toastVisibleUntil: input.toastDurationMs ? Date.now() + input.toastDurationMs : undefined,
    routeTo: input.routeTo,
    actionLabel: input.actionLabel,
    readAt: undefined,
  }))
  vi.spyOn(tauriClient, 'markNotificationRead').mockImplementation(async id => ({
    id,
    scopeKind: 'app',
    level: 'info',
    title: 'Notification',
    body: '',
    source: 'fixture',
    createdAt: Date.now(),
    readAt: Date.now(),
    toastVisibleUntil: undefined,
  }))
  vi.spyOn(tauriClient, 'markAllNotificationsRead').mockResolvedValue({
    total: 0,
    byScope: {
      app: 0,
      workspace: 0,
      user: 0,
    },
  })
  vi.spyOn(tauriClient, 'dismissNotificationToast').mockImplementation(async id => ({
    id,
    scopeKind: 'app',
    level: 'info',
    title: 'Notification',
    body: '',
    source: 'fixture',
    createdAt: Date.now(),
    readAt: undefined,
    toastVisibleUntil: undefined,
  }))
  vi.spyOn(tauriClient, 'subscribeToNotifications').mockImplementation(() => () => {})
  vi.spyOn(tauriClient, 'restartDesktopBackend').mockResolvedValue({
    baseUrl: hostBootstrap.backend?.baseUrl ?? 'http://127.0.0.1:43127',
    authToken: hostBootstrap.backend?.authToken,
    state: 'ready',
    transport: 'http',
  })
  vi.spyOn(tauriClient, 'createWorkspaceClient').mockImplementation(({ connection }) => {
    const workspaceState = workspaceStates.get(connection.workspaceConnectionId)
    if (!workspaceState) {
      throw new Error(`Unknown workspace connection ${connection.workspaceConnectionId}`)
    }
    return createWorkspaceClientFixture(connection, workspaceState, options) as unknown as ReturnType<typeof tauriClient.createWorkspaceClient>
  })
}
