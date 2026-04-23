<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { ArrowUp, Bot, Plus, Shield, Sparkles } from 'lucide-vue-next'

import { resolveRuntimePermissionMode, resolveUiPermissionMode, type AgentRecord, type ConversationActorKind, type Message, type PermissionMode, type RuntimeSessionDetail, type TeamRecord, type WorkspaceResourceRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiConversationComposerShell, UiEmptyState, UiSelect, UiStatusCallout, UiTextarea } from '@octopus/ui'

import ConversationMessageBubble from '@/components/conversation/ConversationMessageBubble.vue'
import ConversationQueueList from '@/components/conversation/ConversationQueueList.vue'
import ConversationContextPane from '@/components/layout/ConversationContextPane.vue'
import ConversationTabsBar from '@/components/layout/ConversationTabsBar.vue'
import {
  createProjectConversationTarget,
  createProjectSurfaceTarget,
  createWorkspaceConsoleSurfaceTarget,
} from '@/i18n/navigation'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useResourceStore } from '@/stores/resource'
import { useArtifactStore } from '@/stores/artifact'
import { useUserProfileStore } from '@/stores/user-profile'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'
import {
  resolveProjectAgentSettings,
  resolveProjectGrantedModelIds,
  resolveProjectModelSettings,
} from '@/stores/project_settings'
import {
  resolveProjectGrantedAgents,
  resolveProjectGrantedTeams,
  resolveProjectPreferredActorValue,
} from '@/stores/project_setup'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const runtime = useRuntimeStore()
const shell = useShellStore()
const catalogStore = useCatalogStore()
const agentStore = useAgentStore()
const teamStore = useTeamStore()
const resourceStore = useResourceStore()
const artifactStore = useArtifactStore()
const userProfileStore = useUserProfileStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()
const workspaceStore = useWorkspaceStore()

interface ActorOption {
  value: string
  label: string
  kind: ConversationActorKind
}

interface MessageArtifactOption {
  id: string
  label: string
  kindLabel?: string
  version?: number
}

interface ConversationSetupState {
  title: string
  description: string
  actions: Array<{
    id: 'open-models' | 'open-settings'
    label: string
  }>
}

const messageDraft = ref('')
const selectedModelId = ref('')
const selectedPermissionMode = ref<PermissionMode>('auto')
const selectedActorValue = ref('')
const composerContextReadyKey = ref('')
const expandedMessageIds = ref<string[]>([])
const focusedToolByMessageId = ref<Record<string, string>>({})
const scrollContainer = ref<HTMLElement | null>(null)
const resolvingConversationEntry = ref(false)
let lastComposerContextKey = ''
let lastContextPaneKey = ''
let lastPermissionSeedKey = ''
let lastSessionKey = ''
let sessionLoadPromise: Promise<void> | null = null
let conversationEntryKey = ''
let conversationEntryPromise: Promise<string | null> | null = null

const conversationId = computed(() =>
  typeof route.params.conversationId === 'string' ? route.params.conversationId : '',
)
const projectId = computed(() =>
  typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId,
)
const workspaceId = computed(() =>
  typeof route.params.workspaceId === 'string' ? route.params.workspaceId : workspaceStore.currentWorkspaceId,
)
const projectSettings = computed(() =>
  projectId.value ? workspaceStore.getProjectSettings(projectId.value) : {},
)
const project = computed(() =>
  workspaceStore.projects.find(item => item.id === projectId.value) ?? null,
)
const assignedConfiguredModelOptions = computed(() => {
  const assignedIds = resolveProjectGrantedModelIds(
    projectSettings.value,
    catalogStore.runnableConfiguredModelOptions.map(item => item.value),
  )
  return catalogStore.runnableConfiguredModelOptions.filter(item => assignedIds.includes(item.value))
})
const resolvedModelSettings = computed(() =>
  resolveProjectModelSettings(
    projectSettings.value,
    assignedConfiguredModelOptions.value.map(item => item.value),
    projectSettings.value.models?.defaultConfiguredModelId ?? '',
  ),
)
const projectOwnedAgents = computed(() =>
  agentStore.agents.filter(agent => agent.projectId === projectId.value),
)
const projectOwnedTeams = computed(() =>
  teamStore.teams.filter(team => team.projectId === projectId.value),
)
const grantedAgents = computed(() =>
  resolveProjectGrantedAgents(
    project.value,
    agentStore.workspaceAgents,
    projectOwnedAgents.value,
    projectSettings.value,
  ),
)
const grantedTeams = computed(() =>
  resolveProjectGrantedTeams(
    project.value,
    teamStore.workspaceTeams,
    projectOwnedTeams.value,
    projectSettings.value,
  ),
)
const resolvedAgentSettings = computed(() =>
  resolveProjectAgentSettings(
    projectSettings.value,
    grantedAgents.value.map(agent => agent.id),
    grantedTeams.value.map(team => team.id),
  ),
)

const modelOptions = computed(() => {
  const allowedConfiguredModelIds = resolvedModelSettings.value.allowedConfiguredModelIds
  return assignedConfiguredModelOptions.value
    .filter(model => allowedConfiguredModelIds.includes(model.value))
    .map(model => ({
      value: model.value,
      label: model.label,
    }))
})
const actorOptions = computed<ActorOption[]>(() => {
  const disabledAgentIds = new Set(resolvedAgentSettings.value.disabledAgentIds)
  const disabledTeamIds = new Set(resolvedAgentSettings.value.disabledTeamIds)
  const visibleAgents = grantedAgents.value
    .filter((agent: AgentRecord) => agent.projectId === projectId.value || !disabledAgentIds.has(agent.id))
  const visibleTeams = grantedTeams.value
    .filter((team: TeamRecord) => team.projectId === projectId.value || !disabledTeamIds.has(team.id))

  return [
    ...visibleAgents
      .map((agent: AgentRecord) => ({
        value: `agent:${agent.id}`,
        label: agent.name,
        kind: 'agent' as const,
      })),
    ...visibleTeams
      .map((team: TeamRecord) => ({
        value: `team:${team.id}`,
        label: team.name,
        kind: 'team' as const,
      })),
  ]
})
const selectedActor = computed(() => actorOptions.value.find(option => option.value === selectedActorValue.value) ?? null)
const actorLabelMap = computed<Map<string, string>>(() => new Map(actorOptions.value.map(option => [`${option.kind}:${option.value.split(':')[1]}`, option.label])))
const actorAvatarMap = computed<Map<string, string>>(() => new Map([
  ...agentStore.agents.map(agent => [`agent:${agent.id}`, agent.avatar ?? ''] as const),
  ...teamStore.teams.map(team => [`team:${team.id}`, team.avatar ?? ''] as const),
]))
const currentUserAvatar = computed(() => userProfileStore.currentUser?.avatar ?? '')
const currentUserLabel = computed(() => userProfileStore.currentUser?.displayName || 'You')
const resourceMap = computed(() => new Map(resourceStore.activeProjectResources.map(resource => [resource.id, resource])))
const artifactMap = computed(() => new Map(artifactStore.activeProjectDeliverables.map(artifact => [artifact.id, artifact])))
const permissionOptions = computed(() => [
  { value: 'auto', label: t('conversation.composer.autoPermission') },
  { value: 'readonly', label: t('conversation.composer.readonlyPermission') },
  { value: 'danger-full-access', label: t('conversation.composer.dangerPermission') },
])

const renderedMessages = computed<Message[]>(() => (
  conversationId.value && runtime.activeConversationId === conversationId.value
    ? runtime.activeMessages
    : []
))

const queueItems = computed(() =>
  runtime.activeQueue.map(item => ({
    id: item.id,
    content: item.content,
    actorLabel: actorLabelMap.value.get(runtime.activeSession?.summary.selectedActorRef ?? '') ?? 'Assistant',
    createdAt: item.createdAt,
  })),
)
const runtimeOrchestrationBadges = computed(() => {
  const session = runtime.activeSession
  const badges: Array<{ label: string, tone?: 'default' | 'info' | 'success' | 'warning' }> = []
  if (!session) {
    return badges
  }

  if (session.workflow) {
    badges.push({
      label: `Workflow ${session.workflow.status}`,
      tone: session.workflow.status === 'completed' ? 'success' : 'info',
    })
  }
  if (session.subrunCount > 0) {
    badges.push({
      label: `${session.subrunCount} workers`,
      tone: 'info',
    })
  }
  if (session.pendingMailbox) {
    badges.push({
      label: `Mailbox ${session.pendingMailbox.status}`,
      tone: session.pendingMailbox.pendingCount > 0 ? 'warning' : 'default',
    })
  }
  if (session.backgroundRun) {
    badges.push({
      label: `Background ${session.backgroundRun.status}`,
      tone: session.backgroundRun.status === 'completed' ? 'success' : 'info',
    })
  }
  return badges
})
const canResolveApproval = computed(() =>
  workspaceAccessControlStore.currentResourceActionGrants.some(grant =>
    (grant.resourceType === 'runtime.approval' && grant.actions.includes('resolve'))
    || (grant.resourceType === 'runtime' && grant.actions.includes('approval.resolve'))),
)
const canResolveAuth = computed(() =>
  workspaceAccessControlStore.currentResourceActionGrants.some(grant =>
    (grant.resourceType === 'runtime.auth' && grant.actions.includes('resolve'))
    || (grant.resourceType === 'runtime' && grant.actions.includes('auth.resolve'))),
)
const pendingMemoryProposal = computed(() => runtime.pendingMemoryProposal)
const activeMediationKind = computed(() => {
  const mediationKind = runtime.pendingMediation?.mediationKind
  if (mediationKind && mediationKind !== 'none') {
    return mediationKind
  }
  if (runtime.pendingApproval) {
    return 'approval'
  }
  if (runtime.authTarget) {
    return 'auth'
  }
  return pendingMemoryProposal.value ? 'memory' : ''
})
const activeMediationTitle = computed(() =>
  runtime.pendingMediation?.summary
  ?? runtime.pendingApproval?.summary
  ?? runtime.authTarget?.summary
  ?? pendingMemoryProposal.value?.summary
  ?? '',
)
const activeMediationDetail = computed(() =>
  runtime.pendingMediation?.detail
  ?? runtime.pendingApproval?.detail
  ?? runtime.authTarget?.detail
  ?? pendingMemoryProposal.value?.proposalReason
  ?? '',
)
const hasModelOptions = computed(() => modelOptions.value.length > 0)
const hasActorOptions = computed(() => actorOptions.value.length > 0)
const currentComposerContextKey = computed(() => (
  shell.activeWorkspaceConnectionId && projectId.value
    ? `${shell.activeWorkspaceConnectionId}:${projectId.value}`
    : ''
))
const isComposerContextReady = computed(() =>
  Boolean(currentComposerContextKey.value) && composerContextReadyKey.value === currentComposerContextKey.value,
)
const baseConversationSetupState = computed<ConversationSetupState | null>(() => {
  if (!isComposerContextReady.value) {
    return null
  }

  if (hasModelOptions.value && hasActorOptions.value) {
    return null
  }

  if (!hasModelOptions.value && !hasActorOptions.value) {
    return {
      title: t('conversation.setup.missingModelAndActor.title'),
      description: t('conversation.setup.missingModelAndActor.description'),
      actions: [
        {
          id: 'open-models',
          label: t('conversation.setup.actions.openModels'),
        },
        {
          id: 'open-settings',
          label: t('conversation.setup.actions.openSettings'),
        },
      ],
    }
  }

  if (!hasModelOptions.value) {
    return {
      title: t('conversation.setup.missingModel.title'),
      description: t('conversation.setup.missingModel.description'),
      actions: [
        {
          id: 'open-models',
          label: t('conversation.setup.actions.openModels'),
        },
      ],
    }
  }

  return {
    title: t('conversation.setup.missingActor.title'),
    description: t('conversation.setup.missingActor.description'),
    actions: [
      {
        id: 'open-settings',
        label: t('conversation.setup.actions.openSettings'),
      },
    ],
  }
})
const conversationSetupState = computed(() =>
  renderedMessages.value.length
    ? null
    : baseConversationSetupState.value,
)
const composerPlaceholder = computed(() =>
  conversationSetupState.value
    ? t('conversation.composer.setupPlaceholder')
    : t('conversation.composer.placeholder'),
)
const visibleRuntimeError = computed(() => {
  const error = runtime.error.trim()
  if (!error) {
    return ''
  }

  if (baseConversationSetupState.value && isRecoverableConversationSetupError(error)) {
    return ''
  }

  return error
})
const canSubmit = computed(() => messageDraft.value.trim().length > 0 && hasModelOptions.value && !!selectedActor.value)

function isRecoverableConversationSetupError(message: string) {
  return message.includes('actor ref id is missing')
    || message.includes('actor ref kind is missing')
    || message.includes('missing configured model binding')
}

function resolveConfiguredPermissionMode(value: unknown): PermissionMode | null {
  if (value === 'auto' || value === 'readonly' || value === 'danger-full-access') {
    return value
  }
  if (value === 'read-only' || value === 'workspace-write') {
    return resolveUiPermissionMode(value)
  }
  return null
}

function isObjectRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function resolveSessionConfiguredModelId(detail: RuntimeSessionDetail | null | undefined) {
  if (!detail) {
    return ''
  }
  return detail.run.configuredModelId
    || detail.sessionPolicy.selectedConfiguredModelId
    || detail.summary.sessionPolicy.selectedConfiguredModelId
    || ''
}

function syncComposerSelectionsFromSession(detail: RuntimeSessionDetail | null | undefined) {
  if (!detail) {
    return
  }

  const configuredModelId = resolveSessionConfiguredModelId(detail)
  if (configuredModelId && modelOptions.value.some(option => option.value === configuredModelId)) {
    selectedModelId.value = configuredModelId
  }

  const selectedActorRef = detail.selectedActorRef
    || detail.sessionPolicy.selectedActorRef
    || detail.summary.selectedActorRef
    || detail.summary.sessionPolicy.selectedActorRef
  if (selectedActorRef && actorOptions.value.some(option => option.value === selectedActorRef)) {
    selectedActorValue.value = selectedActorRef
  }
}

function resolveProjectDefaultPermissionMode(): PermissionMode | null {
  const effectiveConfig = workspaceStore.activeProjectRuntimeConfig?.effectiveConfig
  if (!isObjectRecord(effectiveConfig)) {
    return null
  }

  const permissions = effectiveConfig.permissions
  if (!isObjectRecord(permissions)) {
    return null
  }

  return resolveConfiguredPermissionMode(permissions.defaultMode)
}

function createConversationId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `conversation-${crypto.randomUUID()}`
  }
  return `conversation-${Date.now()}`
}

function logDevTiming(label: string, startedAt: number, detail?: string) {
  if (!import.meta.env.DEV) {
    return
  }

  const suffix = detail ? ` ${detail}` : ''
  console.debug(`[conversation] ${label}${suffix} ${Math.round(performance.now() - startedAt)}ms`)
}

function seedComposerSelections(projectContextKey: string) {
  if (!modelOptions.value.some(option => option.value === selectedModelId.value)) {
    selectedModelId.value = modelOptions.value[0]?.value ?? ''
  }
  if (!actorOptions.value.some(option => option.value === selectedActorValue.value)) {
    selectedActorValue.value = resolveProjectPreferredActorValue({
      project: project.value,
      projectSettings: projectSettings.value,
      grantedAgents: grantedAgents.value,
      grantedTeams: grantedTeams.value,
    }) || actorOptions.value[0]?.value || ''
  }
  if (lastPermissionSeedKey !== projectContextKey) {
    const configuredDefaultMode = resolveProjectDefaultPermissionMode()
    if (configuredDefaultMode) {
      selectedPermissionMode.value = configuredDefaultMode
    }
    lastPermissionSeedKey = projectContextKey
  }
}

async function ensureConversationComposerContext(connectionId: string, nextProjectId: string) {
  const projectContextKey = `${connectionId}:${nextProjectId}`
  if (lastComposerContextKey === projectContextKey) {
    return
  }

  composerContextReadyKey.value = ''
  const startedAt = performance.now()
  await Promise.all([
    workspaceStore.ensureProjectRuntimeConfig(nextProjectId, connectionId),
    catalogStore.ensureLoaded(connectionId),
    agentStore.ensureLoaded(connectionId),
    teamStore.ensureLoaded(connectionId),
  ])
  seedComposerSelections(projectContextKey)
  lastComposerContextKey = projectContextKey
  composerContextReadyKey.value = projectContextKey
  logDevTiming('composer-context', startedAt, projectContextKey)
}

async function ensureConversationContextPaneData(connectionId: string, nextProjectId: string) {
  const projectContextKey = `${connectionId}:${nextProjectId}`
  if (lastContextPaneKey === projectContextKey) {
    return
  }

  const startedAt = performance.now()
  await Promise.all([
    resourceStore.ensureProjectResources(nextProjectId, connectionId),
    artifactStore.ensureProjectDeliverables(nextProjectId, connectionId),
  ])
  lastContextPaneKey = projectContextKey
  logDevTiming('context-pane', startedAt, projectContextKey)
}

function recentProjectConversations(nextProjectId: string) {
  const combined = [
    ...(workspaceStore.activeOverview?.recentConversations ?? []),
    ...(workspaceStore.getProjectDashboard(nextProjectId)?.recentConversations ?? []),
  ].filter(conversation => conversation.projectId === nextProjectId)

  const byId = new Map(combined.map(conversation => [conversation.id, conversation]))
  return [...byId.values()].sort((left, right) => right.updatedAt - left.updatedAt)
}

async function resolveBareConversationRoute() {
  const nextProjectId = projectId.value
  const nextWorkspaceId = workspaceId.value
  const connectionId = shell.activeWorkspaceConnectionId

  if (conversationId.value || !nextProjectId || !nextWorkspaceId) {
    return conversationId.value || null
  }

  const taskKey = `${connectionId}:${nextProjectId}`
  if (conversationEntryPromise && conversationEntryKey === taskKey) {
    return await conversationEntryPromise
  }

  const task = (async () => {
    resolvingConversationEntry.value = true
    const startedAt = performance.now()
    try {
      let conversations = recentProjectConversations(nextProjectId)
      if (!conversations.length && connectionId) {
        await workspaceStore.ensureWorkspaceBootstrap(connectionId)
        conversations = recentProjectConversations(nextProjectId)
      }
      if (!conversations.length && connectionId) {
        await workspaceStore.loadProjectDashboard(nextProjectId, connectionId)
        conversations = recentProjectConversations(nextProjectId)
      }

      const targetConversation = conversations[0] ?? null
      if (!targetConversation) {
        return null
      }

      await router.replace(
        createProjectConversationTarget(nextWorkspaceId, nextProjectId, targetConversation.id),
      )
      return targetConversation.id
    } finally {
      resolvingConversationEntry.value = false
      logDevTiming('entry-resolve', startedAt, taskKey)
    }
  })()

  conversationEntryKey = taskKey
  conversationEntryPromise = task
  try {
    return await task
  } finally {
    if (conversationEntryPromise === task) {
      conversationEntryPromise = null
    }
  }
}

async function ensureRuntimeSession() {
  const nextProjectId = projectId.value
  const connectionId = shell.activeWorkspaceConnectionId
  const sessionToken = shell.activeWorkspaceSession?.token ?? ''
  const nextConversationId = conversationId.value || await resolveBareConversationRoute()

  if (!nextConversationId || !nextProjectId || !connectionId || !sessionToken) {
    return
  }

  const sessionKey = `${connectionId}:${sessionToken}:${nextProjectId}:${nextConversationId}`
  if (
    sessionKey === lastSessionKey
    && runtime.activeConversationId === nextConversationId
    && runtime.activeSession?.summary.projectId === nextProjectId
  ) {
    return
  }

  if (sessionLoadPromise && sessionKey === lastSessionKey) {
    await sessionLoadPromise
    return
  }

  const task = (async () => {
    const startedAt = performance.now()
    await workspaceAccessControlStore.ensureAuthorizationContext(connectionId)
    await userProfileStore.load(connectionId)
    await runtime.bootstrap()

    const existingSession = runtime.sessions.find(session =>
      session.conversationId === nextConversationId
      && session.projectId === nextProjectId
      && session.sessionKind !== 'pet')

    if (existingSession) {
      let detail = await runtime.loadSession(existingSession.id)
      await ensureConversationComposerContext(connectionId, nextProjectId)
      const activeConfiguredModelId = resolveSessionConfiguredModelId(detail)
      const preferredConfiguredModelId = selectedModelId.value || modelOptions.value[0]?.value || ''
      if (
        detail
        && preferredConfiguredModelId
        && activeConfiguredModelId
        && !modelOptions.value.some(option => option.value === activeConfiguredModelId)
      ) {
        detail = await runtime.rebindSessionConfiguredModel(detail.summary.id, preferredConfiguredModelId) ?? detail
      }
      syncComposerSelectionsFromSession(detail)
      void ensureConversationContextPaneData(connectionId, nextProjectId)
      logDevTiming('session-load', startedAt, `${nextProjectId}:${nextConversationId}`)
      return
    }

    await ensureConversationComposerContext(connectionId, nextProjectId)
    if (!hasModelOptions.value || !hasActorOptions.value) {
      logDevTiming('session-create-skipped', startedAt, `${nextProjectId}:${nextConversationId}`)
      return
    }
    seedComposerSelections(`${connectionId}:${nextProjectId}`)
    const preferredActorValue = resolveProjectPreferredActorValue({
      project: project.value,
      projectSettings: projectSettings.value,
      grantedAgents: grantedAgents.value,
      grantedTeams: grantedTeams.value,
    }) || actorOptions.value[0]?.value || ''
    const detail = await runtime.ensureSession({
      conversationId: nextConversationId,
      projectId: nextProjectId,
      title: `Conversation ${nextConversationId.slice(-6)}`,
      selectedActorRef: selectedActorValue.value || preferredActorValue,
      selectedConfiguredModelId: selectedModelId.value || modelOptions.value[0]?.value || undefined,
      executionPermissionMode: resolveRuntimePermissionMode(selectedPermissionMode.value),
    })
    syncComposerSelectionsFromSession(detail)
    void ensureConversationContextPaneData(connectionId, nextProjectId)
    logDevTiming('session-create', startedAt, `${nextProjectId}:${nextConversationId}`)
  })()

  lastSessionKey = sessionKey
  sessionLoadPromise = task
  try {
    await task
  } finally {
    if (sessionLoadPromise === task) {
      sessionLoadPromise = null
    }
  }
}

watch(renderedMessages, (messages) => {
  nextTick(() => {
    if (scrollContainer.value) {
      scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight
    }
  })
}, { deep: true })

watch(
  () => [
    conversationId.value,
    projectId.value,
    shell.activeWorkspaceConnectionId,
    shell.activeWorkspaceSession?.token,
    hasModelOptions.value,
    hasActorOptions.value,
  ],
  () => {
    if (shell.activeWorkspaceConnectionId && shell.activeWorkspaceSession?.token) {
      void ensureRuntimeSession()
    }
  },
  { immediate: true },
)

async function createConversationFromEmpty() {
  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, createConversationId()))
}

async function openConversationSetupDestination(actionId: ConversationSetupState['actions'][number]['id']) {
  if (!workspaceId.value || !projectId.value) {
    return
  }

  if (actionId === 'open-models') {
    await router.push(createWorkspaceConsoleSurfaceTarget('workspace-console-models', workspaceId.value))
    return
  }

  await router.push(createProjectSurfaceTarget('project-settings', workspaceId.value, projectId.value))
}

async function submitRuntimeTurn() {
  if (!canSubmit.value) {
    return
  }

  const draft = messageDraft.value
  messageDraft.value = ''

  await ensureRuntimeSession()
  const submitted = await runtime.submitTurn({
    content: draft,
    permissionMode: resolveRuntimePermissionMode(selectedPermissionMode.value),
  })

  if (!submitted) {
    messageDraft.value = draft
  }
}

function handleComposerKeydown(event: KeyboardEvent) {
  if (event.isComposing || event.key !== 'Enter' || event.shiftKey) {
    return
  }

  event.preventDefault()
  void submitRuntimeTurn()
}

function toggleDetail(messageId: string) {
  expandedMessageIds.value = expandedMessageIds.value.includes(messageId)
    ? expandedMessageIds.value.filter(id => id !== messageId)
    : [...expandedMessageIds.value, messageId]

  if (!expandedMessageIds.value.includes(messageId)) {
    focusedToolByMessageId.value = {
      ...focusedToolByMessageId.value,
      [messageId]: '',
    }
  }
}

function focusMessageTool(payload: { messageId: string, toolId: string }) {
  if (!expandedMessageIds.value.includes(payload.messageId)) {
    expandedMessageIds.value = [...expandedMessageIds.value, payload.messageId]
  }

  focusedToolByMessageId.value = {
    ...focusedToolByMessageId.value,
    [payload.messageId]: payload.toolId,
  }
}

function resolveActorKey(kind?: ConversationActorKind, id?: string): string {
  if (!kind || !id) {
    return ''
  }

  return `${kind}:${id}`
}

function resolveMessageActorLabel(message: Message): string {
  const resolvedActorKey = resolveActorKey(message.actorKind, message.actorId)
  const requestedActorKey = resolveActorKey(message.requestedActorKind, message.requestedActorId)

  return (resolvedActorKey ? actorLabelMap.value.get(resolvedActorKey) : undefined)
    ?? (requestedActorKey ? actorLabelMap.value.get(requestedActorKey) : undefined)
    ?? message.senderId
}

function resolveMessageAvatarSrc(message: Message): string {
  if (message.senderType === 'user') {
    return currentUserAvatar.value
  }

  const resolvedActorKey = resolveActorKey(message.actorKind, message.actorId)
  const requestedActorKey = resolveActorKey(message.requestedActorKind, message.requestedActorId)

  return (resolvedActorKey ? actorAvatarMap.value.get(resolvedActorKey) : undefined)
    ?? (requestedActorKey ? actorAvatarMap.value.get(requestedActorKey) : undefined)
    ?? ''
}

function resolveMessageAvatarLabel(message: Message): string {
  if (message.senderType === 'user') {
    return currentUserLabel.value.slice(0, 1).toUpperCase() || 'U'
  }

  const label = resolveMessageActorLabel(message)
  return label.slice(0, 1).toUpperCase() || 'A'
}

function resolveMessageResources(message: Message): WorkspaceResourceRecord[] {
  return (message.resourceIds ?? [])
    .map(id => resourceMap.value.get(id))
    .filter((resource): resource is WorkspaceResourceRecord => Boolean(resource))
}

function resolveMessageArtifacts(message: Message): MessageArtifactOption[] {
  return (message.deliverableRefs ?? []).map((deliverableRef) => {
    const artifactId = typeof deliverableRef === 'string' ? deliverableRef : deliverableRef.artifactId
    const artifact = artifactMap.value.get(artifactId)
    const version = typeof deliverableRef === 'string'
      ? artifact?.latestVersion
      : (deliverableRef.version ?? artifact?.latestVersion)

    return {
      id: artifactId,
      label: artifact?.title ?? (typeof deliverableRef === 'string' ? artifactId : deliverableRef.title) ?? artifactId,
      kindLabel: version ? `v${version}` : undefined,
      version,
    }
  })
}

function openArtifact(artifactId: string, version?: number) {
  shell.selectDeliverable(artifactId, version)
  shell.setRightSidebarCollapsed(false)
  void router.replace({
    name: 'project-conversation',
    params: route.params,
    query: {
      ...route.query,
      mode: 'deliverable',
      deliverable: artifactId,
      version: shell.selectedDeliverableVersion ? String(shell.selectedDeliverableVersion) : undefined,
    },
  })
}

async function approveMessageApproval(approvalId: string) {
  await runtime.resolveApproval('approve', approvalId)
}

async function rejectMessageApproval(approvalId: string) {
  await runtime.resolveApproval('reject', approvalId)
}

async function resolveMessageAuthChallenge() {
  await runtime.resolveAuthChallenge('resolved')
}

async function cancelMessageAuthChallenge() {
  await runtime.resolveAuthChallenge('cancelled')
}

async function approveMemoryProposal() {
  await runtime.resolveMemoryProposal('approve')
}

async function rejectMemoryProposal() {
  await runtime.resolveMemoryProposal('reject')
}
</script>

<template>
  <div class="flex h-full min-h-0 w-full">
    <div class="flex min-w-0 flex-1 flex-col px-2 pb-6">
      <ConversationTabsBar />

      <div v-if="!conversationId && resolvingConversationEntry" class="flex flex-1" />

      <div v-else-if="!conversationId" class="flex flex-1 items-center justify-center">
        <UiEmptyState :title="t('conversation.empty.title')" :description="t('conversation.empty.description')">
          <template #actions>
            <UiButton @click="createConversationFromEmpty">
              <Plus :size="14" />
              {{ t('conversation.empty.create') }}
            </UiButton>
          </template>
        </UiEmptyState>
      </div>

      <template v-else>
        <div ref="scrollContainer" class="flex-1 overflow-y-auto py-4">
          <div data-testid="conversation-message-list" class="mx-auto flex w-full max-w-[800px] flex-col">
            <ConversationMessageBubble
              v-for="message in renderedMessages"
              :key="message.id"
              :message="message"
              :sender-label="message.senderType === 'user' ? currentUserLabel : resolveMessageActorLabel(message)"
              :avatar-label="resolveMessageAvatarLabel(message)"
              :avatar-src="resolveMessageAvatarSrc(message)"
              :actor-label="message.senderType === 'user' ? '' : resolveMessageActorLabel(message)"
              :permission-label="selectedPermissionMode"
              :resources="resolveMessageResources(message)"
              :attachments="message.attachments ?? []"
              :artifacts="resolveMessageArtifacts(message)"
              :is-expanded="expandedMessageIds.includes(message.id)"
              :focused-tool-id="focusedToolByMessageId[message.id]"
              :approval-resolving="runtime.isApprovalResolving(message.approval?.id)"
              @toggle-detail="toggleDetail"
              @open-artifact="(payload) => openArtifact(payload.id, payload.version)"
              @approve="approveMessageApproval"
              @reject="rejectMessageApproval"
              @focus-tool="focusMessageTool"
            />

            <UiEmptyState
              v-if="!renderedMessages.length"
              :title="conversationSetupState?.title ?? t('conversation.messages.emptyTitle')"
              :description="conversationSetupState?.description ?? t('conversation.messages.emptyDescription')"
            >
              <template v-if="conversationSetupState" #actions>
                <UiButton
                  v-for="action in conversationSetupState.actions"
                  :key="action.id"
                  :data-testid="`conversation-setup-${action.id}`"
                  @click="openConversationSetupDestination(action.id)"
                >
                  {{ action.label }}
                </UiButton>
              </template>
            </UiEmptyState>
          </div>
        </div>

        <div v-if="queueItems.length" class="mx-auto mt-4 w-full max-w-[840px]">
          <ConversationQueueList :items="queueItems" @remove="runtime.removeQueuedTurn" />
        </div>

        <div
          v-if="runtimeOrchestrationBadges.length"
          class="mx-auto mt-4 flex w-full max-w-[840px] flex-wrap gap-2 px-1"
        >
          <UiBadge
            v-for="badge in runtimeOrchestrationBadges"
            :key="badge.label"
            :label="badge.label"
            :tone="badge.tone"
            subtle
          />
        </div>

        <UiStatusCallout
          v-if="activeMediationKind"
          data-testid="conversation-runtime-mediation"
          class="mx-auto mt-4 w-full max-w-[840px]"
          tone="warning"
          :title="activeMediationTitle"
          :description="activeMediationDetail"
        >
          <div class="flex flex-wrap gap-2.5">
            <UiBadge
              v-if="runtime.pendingApproval?.toolName"
              :label="runtime.pendingApproval.toolName"
              subtle
            />
            <UiBadge
              v-if="runtime.authTarget?.providerKey"
              :label="runtime.authTarget.providerKey"
              subtle
            />
            <UiBadge
              v-if="runtime.pendingMediation?.targetKind"
              :label="runtime.pendingMediation.targetKind"
              subtle
            />
          </div>
          <div class="flex flex-wrap gap-2 pt-1">
            <template v-if="activeMediationKind === 'auth' && runtime.authTarget && canResolveAuth">
              <UiButton size="sm" @click="resolveMessageAuthChallenge">{{ t('common.resolveAuth') }}</UiButton>
              <UiButton variant="ghost" size="sm" @click="cancelMessageAuthChallenge">{{ t('common.cancel') }}</UiButton>
            </template>
            <template v-else-if="activeMediationKind === 'memory' && pendingMemoryProposal">
              <UiButton size="sm" @click="approveMemoryProposal">{{ t('common.approve') }}</UiButton>
              <UiButton variant="ghost" size="sm" @click="rejectMemoryProposal">{{ t('common.reject') }}</UiButton>
            </template>
          </div>
        </UiStatusCallout>

        <UiConversationComposerShell
          data-testid="conversation-composer"
          class="mx-auto mt-4 w-full max-w-[840px]"
        >
          <UiStatusCallout
            v-if="conversationSetupState"
            data-testid="conversation-setup-callout"
            class="mx-1 mb-1"
            :title="conversationSetupState.title"
            :description="conversationSetupState.description"
          >
            <div class="flex flex-wrap gap-2">
              <UiButton
                v-for="action in conversationSetupState.actions"
                :key="action.id"
                size="sm"
                :data-testid="`conversation-setup-${action.id}`"
                @click="openConversationSetupDestination(action.id)"
              >
                {{ action.label }}
              </UiButton>
            </div>
          </UiStatusCallout>

          <UiStatusCallout
            v-else-if="visibleRuntimeError"
            class="mx-1 mb-1"
            tone="error"
            :description="visibleRuntimeError"
            role="alert"
          />

          <div class="px-5 pb-3 pt-3">
            <UiTextarea
              v-model="messageDraft"
              :disabled="!!conversationSetupState"
              class="min-h-[96px] max-h-[220px] resize-none border-0 bg-transparent px-0 py-0 text-[15px] leading-6 shadow-none placeholder:text-text-tertiary focus:border-transparent focus:outline-none focus:ring-0 focus:ring-offset-0 focus-visible:border-transparent focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0"
              :rows="3"
              :placeholder="composerPlaceholder"
              @keydown="handleComposerKeydown"
            />

            <div class="mt-3 flex items-end gap-3 pt-2">
              <div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
                <div
                  data-testid="conversation-add-trigger"
                  aria-hidden="true"
                  class="flex h-8 w-8 shrink-0 items-center justify-center rounded-[var(--radius-m)] border border-border bg-subtle text-text-secondary"
                >
                  <Plus :size="14" />
                </div>

                <div class="w-full sm:w-[10.5rem]">
                  <div
                    data-testid="conversation-model-shell"
                    class="flex min-w-0 items-center gap-1 rounded-[var(--radius-m)] border border-border bg-subtle px-1.5"
                  >
                    <Sparkles :size="14" class="ml-2 shrink-0 text-text-secondary" />
                    <UiSelect
                      v-model="selectedModelId"
                      data-testid="conversation-model-select"
                      :options="modelOptions"
                      :disabled="!hasModelOptions"
                      class="min-w-0 h-8 border-0 bg-transparent px-1 pr-7 text-sm font-medium text-text-secondary shadow-none focus-visible:border-transparent focus-visible:ring-0"
                    />
                  </div>
                </div>

                <div class="w-full sm:w-[10rem]">
                  <div
                    data-testid="conversation-permission-shell"
                    class="flex min-w-[100px] items-center gap-1 rounded-[var(--radius-m)] border border-border bg-subtle px-1.5"
                  >
                    <Shield :size="14" class="ml-2 shrink-0 text-text-secondary" />
                    <UiSelect
                      v-model="selectedPermissionMode"
                      data-testid="conversation-permission-select"
                      :options="permissionOptions"
                      :disabled="!!conversationSetupState"
                      class="h-8 border-0 bg-transparent px-1 pr-7 text-sm font-medium text-text-secondary shadow-none focus-visible:border-transparent focus-visible:ring-0"
                    />
                  </div>
                </div>

                <div class="w-full sm:w-[9.5rem]">
                  <div
                    data-testid="conversation-actor-shell"
                    class="flex min-w-0 items-center gap-1 rounded-[var(--radius-m)] border border-border bg-subtle px-1.5"
                  >
                    <Bot :size="14" class="ml-2 shrink-0 text-text-secondary" />
                    <UiSelect
                      v-model="selectedActorValue"
                      data-testid="conversation-actor-select"
                      :options="actorOptions"
                      :disabled="!hasActorOptions"
                      class="min-w-0 h-8 border-0 bg-transparent px-1 pr-7 text-sm font-medium text-text-secondary shadow-none focus-visible:border-transparent focus-visible:ring-0"
                    />
                  </div>
                </div>
              </div>

              <UiButton
                data-testid="conversation-send-button"
                size="icon"
                :aria-label="t('conversation.composer.send')"
                :disabled="!canSubmit"
                class="h-10 w-10 shrink-0 self-end rounded-[var(--radius-m)] bg-primary text-primary-foreground transition-all duration-normal ease-apple hover:bg-primary/90 disabled:bg-muted disabled:text-text-tertiary"
                @click="submitRuntimeTurn"
              >
                <ArrowUp :size="18" />
              </UiButton>
            </div>
          </div>
        </UiConversationComposerShell>
      </template>
    </div>

    <ConversationContextPane />
  </div>
</template>
