import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { PetChatSender, PetMessage, PetMotionState, PetPresenceState, PetProfile, PetWorkspaceSnapshot } from '@octopus/schema'

import { useAgentStore } from '@/stores/agent'

const DEFAULT_PET_PERMISSION_MODE = 'read-only'

import { useCatalogStore } from '@/stores/catalog'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'
import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

function createConversationId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `conversation-${crypto.randomUUID()}`
  }
  return `conversation-${Date.now()}`
}

function defaultProfile(): PetProfile {
  return {
    id: 'pet-octopus',
    displayName: '小章',
    species: 'octopus',
    ownerUserId: 'user-owner',
    avatarLabel: 'Octopus mascot',
    summary: 'Octopus 首席吉祥物，负责卖萌和加油。',
    greeting: '嗨！我是小章，今天也要加油哦！',
    mood: 'happy',
    favoriteSnack: '新鲜小虾',
    promptHints: ['最近有什么好消息？', '给我讲个冷笑话', '我们要加油呀！'],
    fallbackAsset: 'octopus',
    riveAsset: undefined,
    stateMachine: undefined,
  }
}

function defaultPresence(): PetPresenceState {
  return {
    petId: 'pet-octopus',
    isVisible: true,
    chatOpen: false,
    motionState: 'idle',
    unreadCount: 0,
    lastInteractionAt: 0,
    position: { x: 0, y: 0 },
  }
}

function defaultSnapshot(): PetWorkspaceSnapshot {
  return {
    profile: defaultProfile(),
    presence: defaultPresence(),
    messages: [],
  }
}

function isObjectRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

export const usePetStore = defineStore('pet', () => {
  const snapshots = ref<Record<string, PetWorkspaceSnapshot>>({})
  const loadingByScope = ref<Record<string, boolean>>({})
  const errors = ref<Record<string, string>>({})
  const requestTokens = ref<Record<string, number>>({})
  const initializedScopes = ref<Record<string, boolean>>({})

  const shell = useShellStore()
  const workspaceStore = useWorkspaceStore()
  const runtime = useRuntimeStore()
  const catalog = useCatalogStore()
  const agentStore = useAgentStore()
  const teamStore = useTeamStore()
  const userCenterStore = useUserCenterStore()

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const activeProjectId = computed(() => workspaceStore.currentProjectId)
  const activeScopeKey = computed(() => {
    if (!activeConnectionId.value) {
      return ''
    }
    return activeProjectId.value ? `${activeConnectionId.value}:${activeProjectId.value}` : `${activeConnectionId.value}:workspace`
  })
  const snapshot = computed(() => snapshots.value[activeScopeKey.value] ?? defaultSnapshot())
  const petConfig = computed(() => {
    const value = userCenterStore.runtimeConfig?.effectiveConfig?.pet
    return isObjectRecord(value) ? value : null
  })
  const profile = computed<PetProfile>(() => ({
    ...snapshot.value.profile,
    displayName: typeof petConfig.value?.displayName === 'string' && petConfig.value.displayName.trim()
      ? petConfig.value.displayName.trim()
      : snapshot.value.profile.displayName,
    greeting: typeof petConfig.value?.greeting === 'string' && petConfig.value.greeting.trim()
      ? petConfig.value.greeting.trim()
      : snapshot.value.profile.greeting,
    summary: typeof petConfig.value?.summary === 'string' && petConfig.value.summary.trim()
      ? petConfig.value.summary.trim()
      : snapshot.value.profile.summary,
  }))
  const presence = computed(() => snapshot.value.presence)
  const binding = computed(() => snapshot.value.binding)
  const loading = computed(() => loadingByScope.value[activeScopeKey.value] ?? false)
  const error = computed(() => errors.value[activeScopeKey.value] ?? '')
  const currentConversationId = computed(() => binding.value?.conversationId ?? '')
  const runtimeMessages = computed(() => {
    if (!currentConversationId.value || runtime.activeConversationId !== currentConversationId.value) {
      return [] as PetMessage[]
    }
    return runtime.activeSession?.messages.map(message => ({
      id: message.id,
      petId: profile.value.id,
      sender: (message.senderType === 'assistant' ? 'pet' : 'user') as PetChatSender,
      content: message.content,
      timestamp: message.timestamp,
    })) ?? []
  })
  const messages = computed(() => runtimeMessages.value.length > 0 ? runtimeMessages.value : snapshot.value.messages)
  const motionState = computed<PetMotionState>(() => {
    if (presence.value.chatOpen || runtime.isBusy) {
      return 'chat'
    }
    return presence.value.motionState
  })
  const unreadCount = computed(() => presence.value.unreadCount)
  const preferredConfiguredModelId = computed(() => {
    const modelValue = petConfig.value?.configuredModelId
    if (typeof modelValue === 'string' && modelValue.trim()) {
      return modelValue.trim()
    }
    return ''
  })
  const preferredPermissionMode = computed(() => {
    const permissionValue = petConfig.value?.permissionMode
    if (
      permissionValue === 'read-only'
      || permissionValue === 'workspace-write'
      || permissionValue === 'danger-full-access'
    ) {
      return permissionValue
    }
    return DEFAULT_PET_PERMISSION_MODE
  })
  const allowedConfiguredModelIds = computed(() =>
    workspaceStore.getProjectSettings(activeProjectId.value).models?.allowedConfiguredModelIds ?? [],
  )
  const resolvedConfiguredModelId = computed(() => {
    const projectSettings = workspaceStore.getProjectSettings(activeProjectId.value)
    const isAllowed = (value: string) => value && (
      !allowedConfiguredModelIds.value.length || allowedConfiguredModelIds.value.includes(value)
    )

    if (isAllowed(preferredConfiguredModelId.value)) {
      return preferredConfiguredModelId.value
    }
    if (isAllowed(projectSettings.models?.defaultConfiguredModelId ?? '')) {
      return projectSettings.models?.defaultConfiguredModelId ?? ''
    }
    const allowedWorkspaceModel = catalog.workspaceConfiguredModelOptions.find(option => isAllowed(option.value))
    if (allowedWorkspaceModel) {
      return allowedWorkspaceModel.value
    }
    const allowedConfiguredModel = catalog.configuredModelOptions.find(option => isAllowed(option.value))
    if (allowedConfiguredModel) {
      return allowedConfiguredModel.value
    }
    return 'anthropic-primary'
  })
  const preferredActor = computed(() => {
    const projectSettings = workspaceStore.getProjectSettings()
    const preferredTeamId = projectSettings.agents?.enabledTeamIds?.[0] ?? ''
    if (preferredTeamId) {
      return {
        kind: 'team' as const,
        id: preferredTeamId,
      }
    }
    const preferredAgentId = projectSettings.agents?.enabledAgentIds?.[0] ?? ''
    if (preferredAgentId) {
      return {
        kind: 'agent' as const,
        id: preferredAgentId,
      }
    }
    return null
  })

  function resolveScope(projectId = activeProjectId.value, workspaceConnectionId = activeConnectionId.value) {
    if (!workspaceConnectionId) {
      return null
    }
    return {
      connectionId: workspaceConnectionId,
      projectId: projectId || '',
      scopeKey: projectId ? `${workspaceConnectionId}:${projectId}` : `${workspaceConnectionId}:workspace`,
    }
  }

  async function loadSnapshot(projectId = activeProjectId.value, workspaceConnectionId = activeConnectionId.value, force = false) {
    const resolvedScope = resolveScope(projectId, workspaceConnectionId)
    if (!resolvedScope) {
      return null
    }
    const { scopeKey, connectionId } = resolvedScope
    if (!force && initializedScopes.value[scopeKey]) {
      return snapshots.value[scopeKey] ?? defaultSnapshot()
    }
    const resolvedClient = resolveWorkspaceClientForConnection(connectionId)
    if (!resolvedClient) {
      return null
    }
    const token = createWorkspaceRequestToken(requestTokens.value[scopeKey] ?? 0)
    requestTokens.value = {
      ...requestTokens.value,
      [scopeKey]: token,
    }
    loadingByScope.value = {
      ...loadingByScope.value,
      [scopeKey]: true,
    }
    errors.value = {
      ...errors.value,
      [scopeKey]: '',
    }
    try {
      const nextSnapshot = await resolvedClient.client.pet.getSnapshot(projectId || undefined)
      if (requestTokens.value[scopeKey] !== token) {
        return null
      }
      snapshots.value = {
        ...snapshots.value,
        [scopeKey]: nextSnapshot,
      }
      initializedScopes.value = {
        ...initializedScopes.value,
        [scopeKey]: true,
      }
      if (nextSnapshot.binding?.sessionId) {
        await runtime.loadSession(nextSnapshot.binding.sessionId)
      }
      return nextSnapshot
    } catch (cause) {
      if (requestTokens.value[scopeKey] === token) {
        errors.value = {
          ...errors.value,
          [scopeKey]: cause instanceof Error ? cause.message : 'Failed to load pet snapshot',
        }
      }
      return null
    } finally {
      if (requestTokens.value[scopeKey] === token) {
        loadingByScope.value = {
          ...loadingByScope.value,
          [scopeKey]: false,
        }
      }
    }
  }

  async function savePresence(patch: Partial<PetPresenceState>, projectId = activeProjectId.value, workspaceConnectionId = activeConnectionId.value) {
    const resolvedScope = resolveScope(projectId, workspaceConnectionId)
    if (!resolvedScope) {
      return null
    }
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const current = snapshots.value[resolvedScope.scopeKey] ?? defaultSnapshot()
    const nextPresence = await resolvedClient.client.pet.savePresence({
      petId: patch.petId ?? current.profile.id,
      isVisible: patch.isVisible,
      chatOpen: patch.chatOpen,
      motionState: patch.motionState,
      unreadCount: patch.unreadCount,
      lastInteractionAt: patch.lastInteractionAt,
      position: patch.position,
    }, projectId || undefined)
    snapshots.value = {
      ...snapshots.value,
      [resolvedScope.scopeKey]: {
        ...current,
        presence: nextPresence,
      },
    }
    initializedScopes.value = {
      ...initializedScopes.value,
      [resolvedScope.scopeKey]: true,
    }
    return nextPresence
  }

  async function ensureConversation(projectId = activeProjectId.value, workspaceConnectionId = activeConnectionId.value) {
    const resolvedScope = resolveScope(projectId, workspaceConnectionId)
    if (!resolvedScope || !projectId) {
      return null
    }
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }
    const current = snapshots.value[resolvedScope.scopeKey] ?? await loadSnapshot(projectId, workspaceConnectionId) ?? defaultSnapshot()
    const existingBinding = current.binding
    const conversationId = existingBinding?.conversationId ?? createConversationId()
    const session = await runtime.ensureSession({
      conversationId,
      projectId,
      title: `${current.profile.displayName} ${projectId}`,
      sessionKind: 'pet',
    })
    if (!session) {
      return null
    }
    if (existingBinding?.conversationId === conversationId && existingBinding.sessionId === session.summary.id) {
      return existingBinding
    }
    const nextBinding = await resolvedClient.client.pet.bindConversation({
      petId: current.profile.id,
      conversationId,
      sessionId: session.summary.id,
    }, projectId)
    snapshots.value = {
      ...snapshots.value,
      [resolvedScope.scopeKey]: {
        ...current,
        binding: nextBinding,
      },
    }
    initializedScopes.value = {
      ...initializedScopes.value,
      [resolvedScope.scopeKey]: true,
    }
    return nextBinding
  }

  async function openChat() {
    const current = await loadSnapshot(activeProjectId.value, activeConnectionId.value)
    if (!current) {
      return
    }
    await savePresence({
      chatOpen: true,
      unreadCount: 0,
      lastInteractionAt: Date.now(),
      motionState: 'chat',
    })
    if (binding.value?.sessionId) {
      await runtime.loadSession(binding.value.sessionId)
    }
  }

  async function closeChat() {
    await savePresence({
      chatOpen: false,
      lastInteractionAt: Date.now(),
      motionState: 'idle',
    })
  }

  async function sendMessage(content: string) {
    const trimmed = content.trim()
    const resolvedScope = resolveScope(activeProjectId.value, activeConnectionId.value)
    if (!trimmed || !activeProjectId.value || !resolvedScope) {
      return false
    }
    errors.value = {
      ...errors.value,
      [resolvedScope.scopeKey]: '',
    }
    try {
      await Promise.all([
        loadSnapshot(activeProjectId.value, activeConnectionId.value),
        workspaceStore.loadProjectRuntimeConfig(activeProjectId.value),
        userCenterStore.loadCurrentUserRuntimeConfig(false, activeConnectionId.value),
        catalog.load(),
        agentStore.load(),
        teamStore.load(),
        agentStore.loadProjectLinks(activeProjectId.value),
        teamStore.loadProjectLinks(activeProjectId.value),
      ])
      const ensuredBinding = await ensureConversation(activeProjectId.value, activeConnectionId.value)
      if (!ensuredBinding?.sessionId) {
        errors.value = {
          ...errors.value,
          [resolvedScope.scopeKey]: 'Failed to prepare pet conversation',
        }
        return false
      }
      if (runtime.activeSessionId !== ensuredBinding.sessionId) {
        await runtime.loadSession(ensuredBinding.sessionId)
      }
      await savePresence({
        chatOpen: true,
        lastInteractionAt: Date.now(),
        motionState: 'chat',
      })
      const submitted = await runtime.submitTurn({
        content: trimmed,
        configuredModelId: resolvedConfiguredModelId.value,
        permissionMode: preferredPermissionMode.value,
        actorKind: preferredActor.value?.kind,
        actorId: preferredActor.value?.id,
      })
      errors.value = {
        ...errors.value,
        [resolvedScope.scopeKey]: submitted ? '' : (runtime.error || 'Failed to submit pet message'),
      }
      return submitted
    } catch (cause) {
      errors.value = {
        ...errors.value,
        [resolvedScope.scopeKey]: cause instanceof Error ? cause.message : 'Failed to submit pet message',
      }
      return false
    }
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const nextSnapshots = { ...snapshots.value }
    const nextLoading = { ...loadingByScope.value }
    const nextErrors = { ...errors.value }
    const nextTokens = { ...requestTokens.value }
    const nextInitialized = { ...initializedScopes.value }
    Object.keys(nextSnapshots).forEach((key) => {
      if (key.startsWith(`${workspaceConnectionId}:`)) {
        delete nextSnapshots[key]
      }
    })
    Object.keys(nextLoading).forEach((key) => {
      if (key.startsWith(`${workspaceConnectionId}:`)) {
        delete nextLoading[key]
      }
    })
    Object.keys(nextErrors).forEach((key) => {
      if (key.startsWith(`${workspaceConnectionId}:`)) {
        delete nextErrors[key]
      }
    })
    Object.keys(nextTokens).forEach((key) => {
      if (key.startsWith(`${workspaceConnectionId}:`)) {
        delete nextTokens[key]
      }
    })
    Object.keys(nextInitialized).forEach((key) => {
      if (key.startsWith(`${workspaceConnectionId}:`)) {
        delete nextInitialized[key]
      }
    })
    snapshots.value = nextSnapshots
    loadingByScope.value = nextLoading
    errors.value = nextErrors
    requestTokens.value = nextTokens
    initializedScopes.value = nextInitialized
  }

  const isReady = computed(() => !!shell.activeWorkspaceConnectionId && initializedScopes.value[activeScopeKey.value])

  return {
    profile,
    presence,
    binding,
    messages,
    motionState,
    unreadCount,
    loading,
    error,
    isReady,
    currentConversationId,
    preferredConfiguredModelId,
    preferredPermissionMode,
    resolvedConfiguredModelId,
    loadSnapshot,
    savePresence,
    ensureConversation,
    openChat,
    closeChat,
    sendMessage,
    clearWorkspaceScope,
  }
})
