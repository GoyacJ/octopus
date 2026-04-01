<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import {
  Bot,
  ChevronDown,
  FileText,
  FolderOpen,
  Paperclip,
  Plus,
  SendHorizontal,
  Shield,
} from 'lucide-vue-next'

import {
  type ConversationAttachment,
  type Message,
  type PermissionMode,
  type ProjectResource,
} from '@octopus/schema'
import { UiEmptyState, UiSurface } from '@octopus/ui'

import ConversationMessageBubble from '@/components/conversation/ConversationMessageBubble.vue'
import ConversationQueueList from '@/components/conversation/ConversationQueueList.vue'
import ConversationContextPane from '@/components/layout/ConversationContextPane.vue'
import ConversationTabsBar from '@/components/layout/ConversationTabsBar.vue'
import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import type { ConversationDetailFocus } from '@/stores/shell'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const messageDraft = ref('')
const selectedModelId = ref('gpt-4o')
const selectedPermissionMode = ref<PermissionMode>('auto')
const selectedActorValue = ref('')
const selectedResourceIds = ref<string[]>([])
const resourceMenuOpen = ref(false)
const existingResourcesOpen = ref(false)
const expandedMessageIds = ref<string[]>([])

const activeConversation = computed(() => workbench.activeConversation)
const firstConversationId = computed(() => workbench.firstConversationIdForProject(workbench.currentProjectId))
const detailPaneCollapsed = computed(() => shell.rightSidebarCollapsed)
const modelOptions = computed(() => workbench.workspaceModelCatalog)
const availableResources = computed(() => workbench.projectResources)
const selectedResources = computed(() =>
  selectedResourceIds.value
    .map((resourceId) => availableResources.value.find((resource) => resource.id === resourceId))
    .filter(Boolean) as ProjectResource[],
)
const actorGroups = computed(() => ({
  agents: workbench.workspaceAgents,
  teams: workbench.workspaceTeams,
}))
const hasMessageDraft = computed(() => messageDraft.value.trim().length > 0)
const canSend = computed(() => hasMessageDraft.value)
const selectedActor = computed(() => {
  if (!selectedActorValue.value) {
    return {
      actorKind: undefined,
      actorId: undefined,
    } as const
  }

  const [kind, id] = selectedActorValue.value.split(':')
  return {
    actorKind: kind === 'team' ? 'team' : 'agent',
    actorId: id ?? undefined,
  } as const
})
const selectedAttachments = computed<ConversationAttachment[]>(() =>
  selectedResources.value.map((resource) => ({
    id: resource.id,
    name: resource.name,
    kind: resource.kind === 'artifact'
      ? 'artifact'
      : resource.kind === 'folder'
        ? 'folder'
        : 'file',
  })),
)
const defaultActorLabel = computed(() => {
  const resolved = workbench.activeConversationDefaultActor
  if (!resolved) {
    return t('common.na')
  }

  return actorLabel(resolved.actorKind, resolved.actorId)
})
const selectedPermissionLabel = computed(() => permissionLabel(selectedPermissionMode.value))
const selectedActorLabel = computed(() => {
  if (!selectedActor.value.actorKind || !selectedActor.value.actorId) {
    return t('conversation.composer.defaultActorOption', { name: defaultActorLabel.value })
  }

  return actorLabel(selectedActor.value.actorKind, selectedActor.value.actorId)
})
const selectedModelLabel = computed(() =>
  modelOptions.value.find((model) => model.id === selectedModelId.value)?.label ?? selectedModelId.value,
)
const queueItems = computed(() =>
  workbench.activeConversationQueue.map((item) => ({
    id: item.id,
    content: item.content,
    actorLabel: actorLabel(item.resolvedActorKind, item.resolvedActorId),
    createdAt: item.createdAt,
  })),
)

watch(
  [() => route.name, () => workbench.currentWorkspaceId, () => workbench.currentProjectId, () => firstConversationId.value],
  async ([routeName, workspaceId, projectId, conversationId]) => {
    if (routeName !== 'project-conversations' || !workspaceId || !projectId || !conversationId) {
      return
    }

    await router.replace(createProjectConversationTarget(workspaceId, projectId, conversationId))
  },
  { immediate: true },
)

watch(
  [
    () => route.name,
    () => route.params.workspaceId,
    () => route.params.projectId,
    () => route.params.conversationId,
    () => workbench.conversations.map((item) => `${item.projectId}:${item.id}`).join('|'),
  ],
  async ([routeName, workspaceIdParam, projectIdParam, conversationIdParam]) => {
    if (routeName !== 'conversation') {
      return
    }

    const workspaceId = typeof workspaceIdParam === 'string' ? workspaceIdParam : workbench.currentWorkspaceId
    const projectId = typeof projectIdParam === 'string' ? projectIdParam : workbench.currentProjectId
    const conversationId = typeof conversationIdParam === 'string' ? conversationIdParam : ''
    const routeConversationExists = workbench.conversations.some((item) => item.id === conversationId && item.projectId === projectId)

    if (!workspaceId || !projectId || !conversationId || routeConversationExists) {
      return
    }

    await router.replace(createProjectConversationTarget(workspaceId, projectId, workbench.firstConversationIdForProject(projectId) || null))
  },
  { immediate: true },
)

watch(
  [() => activeConversation.value?.id],
  () => {
    selectedActorValue.value = ''
    expandedMessageIds.value = []
  },
  { immediate: true },
)

watch(
  modelOptions,
  (options) => {
    if (!options.some((option) => option.id === selectedModelId.value)) {
      selectedModelId.value = options[0]?.id ?? 'gpt-4o'
    }
  },
  { immediate: true },
)

watch(
  availableResources,
  (resources) => {
    const validIds = new Set(resources.map((resource) => resource.id))
    selectedResourceIds.value = selectedResourceIds.value.filter((resourceId) => validIds.has(resourceId))
  },
  { immediate: true },
)

function updateDetailQuery(detail: ConversationDetailFocus, artifactId?: string) {
  void router.replace({
    query: {
      ...route.query,
      detail,
      ...(artifactId ? { artifact: artifactId } : {}),
    },
  })
}

function toggleResourceMenu() {
  resourceMenuOpen.value = !resourceMenuOpen.value
  if (!resourceMenuOpen.value) {
    existingResourcesOpen.value = false
  }
}

function attachResource(resourceId: string) {
  if (!selectedResourceIds.value.includes(resourceId)) {
    selectedResourceIds.value.push(resourceId)
  }
}

function createAndAttachResource(kind: 'file' | 'folder') {
  const resource = workbench.createProjectResource(kind)
  attachResource(resource.id)
  resourceMenuOpen.value = false
  existingResourcesOpen.value = false
}

function toggleExistingResources() {
  existingResourcesOpen.value = !existingResourcesOpen.value
}

function removeSelectedResource(resourceId: string) {
  selectedResourceIds.value = selectedResourceIds.value.filter((item) => item !== resourceId)
}

function sendMessage() {
  if (!canSend.value) {
    return
  }

  workbench.sendMessage({
    content: messageDraft.value,
    modelId: selectedModelId.value,
    permissionMode: selectedPermissionMode.value,
    actorKind: selectedActor.value.actorKind,
    actorId: selectedActor.value.actorId,
    resourceIds: [...selectedResourceIds.value],
    attachments: selectedAttachments.value,
  })

  messageDraft.value = ''
  selectedResourceIds.value = []
  resourceMenuOpen.value = false
  existingResourcesOpen.value = false
}

function openArtifact(artifactId: string) {
  shell.selectArtifact(artifactId)
  shell.setDetailFocus('artifacts')
  shell.setRightSidebarCollapsed(false)
  updateDetailQuery('artifacts', artifactId)
}

function openResource(resourceId: string) {
  const resource = availableResources.value.find((item) => item.id === resourceId)
  if (resource?.kind === 'artifact') {
    openArtifact(resource.sourceArtifactId ?? resource.id)
    return
  }

  shell.setDetailFocus('resources')
  shell.setRightSidebarCollapsed(false)
  updateDetailQuery('resources')
}

async function createConversation() {
  const conversation = workbench.createConversation()
  await router.push(createProjectConversationTarget(workbench.currentWorkspaceId, workbench.currentProjectId, conversation.id))
}

function handleComposerKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter' && canSend.value) {
    event.preventDefault()
    sendMessage()
  }
}

function senderLabel(senderId: string, senderType: 'user' | 'agent' | 'system'): string {
  if (senderType === 'user') {
    return t('conversation.senderType.user')
  }

  if (senderType === 'system') {
    return t('conversation.senderType.system')
  }

  const team = workbench.teams.find((item) => item.id === senderId)
  if (team) {
    return resolveMockField('team', team.id, 'name', team.name)
  }

  const agent = workbench.agents.find((item) => item.id === senderId)
  return agent ? resolveMockField('agent', agent.id, 'name', agent.name) : senderId
}

function actorLabel(actorKind?: 'agent' | 'team', actorId?: string): string {
  if (!actorKind || !actorId) {
    return ''
  }

  if (actorKind === 'team') {
    const team = workbench.teams.find((item) => item.id === actorId)
    return team ? resolveMockField('team', team.id, 'name', team.name) : actorId
  }

  const agent = workbench.agents.find((item) => item.id === actorId)
  return agent ? resolveMockField('agent', agent.id, 'name', agent.name) : actorId
}

function permissionLabel(permissionMode?: PermissionMode): string {
  if (permissionMode === 'readonly') {
    return t('conversation.composer.readonlyPermission')
  }

  return t('conversation.composer.autoPermission')
}

function artifactLabel(artifactId: string): string {
  const artifact = workbench.artifacts.find((item) => item.id === artifactId)
  return artifact ? resolveMockField('artifact', artifact.id, 'title', artifact.title) : artifactId
}

function messageResources(message: Message): ProjectResource[] {
  return (message.resourceIds ?? [])
    .map((resourceId) => availableResources.value.find((resource) => resource.id === resourceId))
    .filter(Boolean) as ProjectResource[]
}

function messageAttachments(message: Message): ConversationAttachment[] {
  const selectedIds = new Set(message.resourceIds ?? [])
  return (message.attachments ?? []).filter((attachment) => attachment.kind !== 'artifact' && !selectedIds.has(attachment.id))
}

function messageArtifacts(message: Message): Array<{ id: string, label: string }> {
  const selectedIds = new Set(message.resourceIds ?? [])
  return (message.artifacts ?? [])
    .filter((artifactId) => !selectedIds.has(artifactId))
    .map((artifactId) => ({
      id: artifactId,
      label: artifactLabel(artifactId),
    }))
}

function toggleMessageDetail(messageId: string) {
  expandedMessageIds.value = expandedMessageIds.value.includes(messageId)
    ? expandedMessageIds.value.filter((item) => item !== messageId)
    : [...expandedMessageIds.value, messageId]
}

function removeQueueItem(queueItemId: string) {
  workbench.removeQueuedMessage(queueItemId)
}

function rollbackToMessage(messageId: string) {
  workbench.rollbackConversationToMessage(messageId)
  shell.hydrateArtifactSelection(workbench.activeConversationArtifacts.map((artifact) => artifact.id))
}
</script>

<template>
  <section class="conversation-page">
    <ConversationTabsBar />

    <section v-if="!activeConversation" class="conversation-empty-state" data-testid="conversation-empty-state">
      <UiSurface :title="t('conversation.empty.title')" :subtitle="t('conversation.empty.subtitle')">
        <div class="empty-stack">
          <UiEmptyState
            :title="t('conversation.empty.guideTitle')"
            :description="t('conversation.empty.guideDescription')"
          />
          <div class="action-row">
            <button
              type="button"
              class="primary-button"
              data-testid="conversation-empty-create"
              @click="createConversation"
            >
              {{ t('conversation.empty.create') }}
            </button>
          </div>
        </div>
      </UiSurface>
    </section>

    <section
      v-else
      class="conversation-chat-layout"
      :class="{ 'detail-collapsed': detailPaneCollapsed }"
      data-testid="conversation-chat-layout"
    >
      <div class="conversation-chat-main">
        <div class="message-scroll scroll-y" data-testid="conversation-message-scroll">
          <div class="message-stream">
            <ConversationMessageBubble
              v-for="message in workbench.conversationMessages"
              :key="message.id"
              :message="message"
              :sender-label="senderLabel(message.senderId, message.senderType)"
              :actor-label="actorLabel(message.actorKind, message.actorId)"
              :permission-label="permissionLabel(message.permissionMode)"
              :resources="messageResources(message)"
              :attachments="messageAttachments(message)"
              :artifacts="messageArtifacts(message)"
              :is-expanded="expandedMessageIds.includes(message.id)"
              @toggle-detail="toggleMessageDetail"
              @rollback="rollbackToMessage"
              @open-resource="openResource"
              @open-artifact="openArtifact"
            />
            <UiEmptyState
              v-if="!workbench.conversationMessages.length"
              :title="t('conversation.stream.emptyTitle')"
              :description="t('conversation.stream.emptyDescription')"
            />
          </div>
        </div>

        <div class="conversation-composer-dock" data-testid="conversation-composer-dock">
          <ConversationQueueList
            class="conversation-queue-floating"
            :items="queueItems"
            @remove="removeQueueItem"
          />

          <section class="conversation-composer" data-testid="conversation-composer">
            <textarea
              v-model="messageDraft"
              data-testid="conversation-composer-input"
              rows="4"
              :placeholder="t('conversation.composer.placeholder')"
              @keydown="handleComposerKeydown"
            />

            <div v-if="selectedResources.length" class="composer-resource-row">
              <button
                v-for="resource in selectedResources"
                :key="resource.id"
                type="button"
                class="attachment-pill"
                @click="openResource(resource.id)"
              >
                <FolderOpen v-if="resource.kind === 'folder'" :size="12" />
                <FileText v-else-if="resource.kind === 'artifact'" :size="12" />
                <Paperclip v-else :size="12" />
                <span>{{ resource.name }}</span>
              </button>
              <button
                v-for="resource in selectedResources"
                :key="`${resource.id}-remove`"
                type="button"
                class="resource-remove"
                @click="removeSelectedResource(resource.id)"
              >
                {{ t('conversation.composer.removeResource', { name: resource.name }) }}
              </button>
            </div>

            <div class="composer-toolbar">
              <div class="composer-controls">
                <div class="composer-menu-shell">
                  <button
                    type="button"
                    class="resource-trigger"
                    data-testid="composer-resource-trigger"
                    :title="t('conversation.composer.resourceTrigger')"
                    @click="toggleResourceMenu"
                  >
                    <Plus :size="16" />
                  </button>

                  <div v-if="resourceMenuOpen" class="resource-menu">
                    <button
                      type="button"
                      class="resource-action"
                      data-testid="resource-action-upload-file"
                      @click="createAndAttachResource('file')"
                    >
                      {{ t('conversation.composer.uploadFile') }}
                    </button>
                    <button
                      type="button"
                      class="resource-action"
                      data-testid="resource-action-upload-folder"
                      @click="createAndAttachResource('folder')"
                    >
                      {{ t('conversation.composer.uploadFolder') }}
                    </button>
                    <button
                      type="button"
                      class="resource-action"
                      data-testid="resource-action-attach-existing"
                      @click="toggleExistingResources"
                    >
                      {{ t('conversation.composer.attachExisting') }}
                    </button>

                    <div v-if="existingResourcesOpen" class="resource-list">
                      <button
                        v-for="resource in availableResources"
                        :key="resource.id"
                        type="button"
                        class="resource-list-item"
                        :disabled="selectedResourceIds.includes(resource.id)"
                        @click="attachResource(resource.id)"
                      >
                        <span>{{ resource.name }}</span>
                        <small>{{ resource.kind }}</small>
                      </button>
                    </div>
                  </div>
                </div>

                <label
                  class="composer-select"
                  :title="t('conversation.composer.permissionLabel')"
                >
                  <Shield :size="14" />
                  <span class="composer-select-value">{{ selectedPermissionLabel }}</span>
                  <ChevronDown :size="14" />
                  <select
                    v-model="selectedPermissionMode"
                    data-testid="composer-permission-select"
                    :aria-label="t('conversation.composer.permissionLabel')"
                  >
                    <option value="auto">{{ t('conversation.composer.autoPermission') }}</option>
                    <option value="readonly">{{ t('conversation.composer.readonlyPermission') }}</option>
                  </select>
                </label>

                <label
                  class="composer-select"
                  :title="t('conversation.composer.actorLabel')"
                >
                  <Bot :size="14" />
                  <span class="composer-select-value">{{ selectedActorLabel }}</span>
                  <ChevronDown :size="14" />
                  <select
                    v-model="selectedActorValue"
                    data-testid="composer-actor-select"
                    :aria-label="t('conversation.composer.actorLabel')"
                  >
                    <option value="">{{ t('conversation.composer.defaultActorOption', { name: defaultActorLabel }) }}</option>
                    <optgroup :label="t('conversation.composer.agentGroup')">
                      <option v-for="agent in actorGroups.agents" :key="agent.id" :value="`agent:${agent.id}`">
                        {{ resolveMockField('agent', agent.id, 'name', agent.name) }}
                      </option>
                    </optgroup>
                    <optgroup :label="t('conversation.composer.teamGroup')">
                      <option v-for="team in actorGroups.teams" :key="team.id" :value="`team:${team.id}`">
                        {{ resolveMockField('team', team.id, 'name', team.name) }}
                      </option>
                    </optgroup>
                  </select>
                </label>

                <label
                  class="composer-select"
                  :title="t('conversation.composer.modelLabel')"
                >
                  <FileText :size="14" />
                  <span class="composer-select-value">{{ selectedModelLabel }}</span>
                  <ChevronDown :size="14" />
                  <select
                    v-model="selectedModelId"
                    data-testid="composer-model-select"
                    :aria-label="t('conversation.composer.modelLabel')"
                  >
                    <option v-for="model in modelOptions" :key="model.id" :value="model.id">
                      {{ model.label }}
                    </option>
                  </select>
                </label>
              </div>

              <button
                type="button"
                class="composer-send"
                data-testid="conversation-composer-send"
                :disabled="!canSend"
                :aria-label="t('common.send')"
                :title="t('common.send')"
                @click="sendMessage"
              >
                <SendHorizontal :size="15" />
              </button>
            </div>
          </section>
        </div>
      </div>

      <ConversationContextPane class="conversation-chat-detail" />
    </section>
  </section>
</template>

<style scoped>
.conversation-page,
.empty-stack,
.conversation-chat-main,
.message-stream {
  display: flex;
  flex-direction: column;
}

.conversation-page {
  gap: 0.75rem;
  height: 100%;
  min-height: 100%;
  overflow: hidden;
}

.empty-stack {
  gap: 1rem;
}

.conversation-chat-layout {
  --conversation-detail-width: clamp(300px, 26vw, 380px);
  display: grid;
  gap: 1rem;
  grid-template-columns: minmax(0, 1fr) var(--conversation-detail-width);
  align-items: stretch;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.conversation-chat-layout.detail-collapsed {
  gap: 0.75rem;
  grid-template-columns: minmax(0, 1fr) 3.5rem;
}

.conversation-chat-main {
  display: grid;
  grid-template-rows: minmax(0, 1fr) auto;
  gap: 0.85rem;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.conversation-chat-detail {
  display: flex;
  min-width: 0;
  min-height: 0;
  height: 100%;
  max-height: 100%;
  width: var(--conversation-detail-width);
  max-width: var(--conversation-detail-width);
  overflow-x: hidden;
}

.conversation-chat-layout.detail-collapsed .conversation-chat-detail {
  width: 3.5rem;
  max-width: 3.5rem;
}

.message-scroll {
  min-height: 0;
  padding-right: 0.35rem;
  overflow-x: hidden;
}

.message-stream {
  gap: 0.95rem;
  min-height: 100%;
  padding-bottom: 0.35rem;
}

.conversation-composer-dock {
  position: relative;
  width: min(100%, 52rem);
  margin-inline: auto;
}

.conversation-queue-floating {
  position: absolute;
  inset-inline: 0;
  bottom: calc(100% + 0.18rem);
  z-index: 3;
  width: 90%;
  margin-inline: auto;
  box-shadow: 0 12px 28px rgb(15 23 42 / 0.08);
}

.conversation-composer {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  padding: 0.95rem 1rem 0.95rem;
  border-radius: 1.4rem;
  border: 0;
  background: color-mix(in srgb, var(--bg-surface) 92%, var(--bg-subtle));
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.03),
    0 14px 36px rgb(15 23 42 / 0.12);
}

textarea {
  width: 100%;
  min-height: 7.25rem;
  resize: vertical;
  border-radius: 1rem;
  border: 0;
  background: transparent;
  color: var(--text-primary);
  padding: 0.2rem 0.15rem 0.3rem;
  line-height: 1.65;
}

.composer-resource-row,
.composer-toolbar,
.composer-controls,
.composer-select,
.resource-list-item,
.action-row {
  display: flex;
  align-items: center;
}

.composer-resource-row,
.composer-controls {
  gap: 0.45rem;
  flex-wrap: wrap;
}

.composer-toolbar {
  justify-content: space-between;
  gap: 0.7rem;
}

.composer-controls {
  align-items: flex-start;
}

.composer-select {
  position: relative;
  justify-content: flex-start;
  gap: 0.32rem;
  min-width: 0;
  height: 2.4rem;
  padding: 0 0.7rem;
  border-radius: 999px;
  border: 0;
  background: color-mix(in srgb, var(--bg-subtle) 82%, transparent);
  color: var(--text-secondary);
  overflow: hidden;
}

.composer-select :deep(svg) {
  pointer-events: none;
  flex-shrink: 0;
}

.composer-select-value {
  min-width: 0;
  max-width: 8rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-primary);
  font-size: 0.82rem;
}

.composer-select select {
  position: absolute;
  inset: 0;
  width: 100%;
  min-width: 100%;
  height: 100%;
  border: 0;
  background: transparent;
  color: transparent;
  opacity: 0;
  cursor: pointer;
}

.resource-trigger,
.composer-send,
.primary-button,
.secondary-button,
.resource-remove,
.resource-action,
.resource-list-item {
  border-radius: 999px;
  border: 0;
}

.resource-trigger,
.composer-send,
.primary-button,
.secondary-button,
.resource-remove,
.resource-action {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.42rem 0.72rem;
}

.resource-trigger {
  justify-content: center;
  width: 2.4rem;
  height: 2.4rem;
  padding: 0;
}

.composer-send,
.primary-button {
  justify-content: center;
  width: 2.4rem;
  height: 2.4rem;
  padding: 0;
  background: color-mix(in srgb, var(--brand-primary) 85%, #5243c2);
  color: white;
  box-shadow: 0 8px 24px rgb(82 67 194 / 0.32);
}

.resource-trigger,
.secondary-button,
.resource-remove,
.resource-action,
.resource-list-item {
  background: color-mix(in srgb, var(--bg-subtle) 82%, transparent);
  color: var(--text-secondary);
}

.resource-remove {
  font-size: 0.76rem;
}

.composer-menu-shell {
  position: relative;
}

.resource-menu,
.resource-list {
  display: flex;
  flex-direction: column;
}

.resource-menu {
  position: absolute;
  bottom: calc(100% + 0.5rem);
  left: 0;
  z-index: 4;
  gap: 0.45rem;
  min-width: 15rem;
  padding: 0.75rem;
  border-radius: 1rem;
  border: 0;
  background: color-mix(in srgb, var(--bg-surface) 96%, var(--bg-subtle));
  box-shadow: 0 18px 40px rgb(15 23 42 / 10%);
}

.resource-list {
  gap: 0.45rem;
  max-height: 12rem;
  overflow-y: auto;
}

.resource-list-item {
  justify-content: space-between;
  padding: 0.6rem 0.7rem;
}

@media (max-width: 1120px) {
  .conversation-chat-layout {
    grid-template-columns: minmax(0, 1fr);
  }

  .conversation-chat-layout.detail-collapsed {
    grid-template-columns: minmax(0, 1fr);
  }

  .conversation-composer-dock {
    width: 100%;
  }
}
</style>
