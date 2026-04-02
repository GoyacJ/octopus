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
  Search,
  SendHorizontal,
  Shield,
  Upload,
  X,
} from 'lucide-vue-next'

import {
  type PermissionMode,
  type ProjectResource,
} from '@octopus/schema'
import { UiButton, UiEmptyState, UiPopover, UiSurface } from '@octopus/ui'

import ConversationMessageBubble from '@/components/conversation/ConversationMessageBubble.vue'
import ConversationQueueList from '@/components/conversation/ConversationQueueList.vue'
import ConversationContextPane from '@/components/layout/ConversationContextPane.vue'
import ConversationTabsBar from '@/components/layout/ConversationTabsBar.vue'
import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
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
const modelMenuOpen = ref(false)
const actorMenuOpen = ref(false)
const permissionMenuOpen = ref(false)
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
const attachableResources = computed(() =>
  availableResources.value.filter((resource) => !selectedResourceIds.value.includes(resource.id)),
)
const actorGroups = computed(() => ({
  agents: workbench.workspaceAgents,
  teams: workbench.workspaceTeams,
}))
const hasMessageDraft = computed(() => messageDraft.value.trim().length > 0)
const canSend = computed(() => hasMessageDraft.value)
const detailPaneWidth = computed(() => (detailPaneCollapsed.value ? '76px' : '392px'))

const selectedActor = computed(() => {
  if (!selectedActorValue.value) {
    return { actorKind: undefined, actorId: undefined } as const
  }
  const [kind, id] = selectedActorValue.value.split(':')
  return {
    actorKind: kind === 'team' ? 'team' : 'agent',
    actorId: id ?? undefined,
  } as const
})

const defaultActorLabel = computed(() => {
  const resolved = workbench.activeConversationDefaultActor
  if (!resolved) return t('common.na')
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
    if (routeName !== 'project-conversations' || !workspaceId || !projectId || !conversationId) return
    await router.replace(createProjectConversationTarget(workspaceId, projectId, conversationId))
  },
  { immediate: true },
)

watch(
  [() => activeConversation.value?.id],
  () => {
    selectedActorValue.value = ''
    selectedResourceIds.value = []
    expandedMessageIds.value = []
  },
  { immediate: true },
)

function attachResource(resourceId: string) {
  if (!selectedResourceIds.value.includes(resourceId)) {
    selectedResourceIds.value.push(resourceId)
  }
}

function createAndAttachResource(kind: 'file' | 'folder') {
  const resource = workbench.createProjectResource(kind)
  attachResource(resource.id)
  resourceMenuOpen.value = false
}

function attachExistingResource() {
  const resource = attachableResources.value[0]
  if (!resource) {
    return
  }
  attachResource(resource.id)
  resourceMenuOpen.value = false
}

function removeSelectedResource(resourceId: string) {
  selectedResourceIds.value = selectedResourceIds.value.filter((item) => item !== resourceId)
}

function sendMessage() {
  if (!canSend.value) return
  workbench.sendMessage({
    content: messageDraft.value,
    modelId: selectedModelId.value,
    permissionMode: selectedPermissionMode.value,
    actorKind: selectedActor.value.actorKind,
    actorId: selectedActor.value.actorId,
    resourceIds: [...selectedResourceIds.value],
    attachments: selectedResources.value.map((resource) => ({
      id: resource.id,
      name: resource.name,
      kind: resource.kind === 'artifact' ? 'artifact' : resource.kind === 'folder' ? 'folder' : 'file',
    })),
  })
  messageDraft.value = ''
  selectedResourceIds.value = []
}

function actorLabel(actorKind?: 'agent' | 'team', actorId?: string): string {
  if (!actorKind || !actorId) return ''
  if (actorKind === 'team') {
    const team = workbench.teams.find((item) => item.id === actorId)
    return team ? resolveMockField('team', team.id, 'name', team.name) : actorId
  }
  const agent = workbench.agents.find((item) => item.id === actorId)
  return agent ? resolveMockField('agent', agent.id, 'name', agent.name) : actorId
}

function messageSenderLabel(senderType: 'user' | 'agent' | 'system', senderId: string, actorKind?: 'agent' | 'team') {
  if (senderType === 'user') {
    return '你'
  }
  return actorLabel(actorKind ?? 'agent', senderId) || 'Octopus'
}

function messageAvatarLabel(senderType: 'user' | 'agent' | 'system', senderId: string, actorKind?: 'agent' | 'team') {
  if (senderType === 'user') {
    return '你'
  }

  if (actorKind === 'team') {
    const team = workbench.teams.find((item) => item.id === senderId || item.id === selectedActor.value.actorId)
    return team?.avatar || 'OT'
  }

  const agent = workbench.agents.find((item) => item.id === senderId)
  return agent?.avatar || 'OA'
}

function permissionLabel(permissionMode?: PermissionMode): string {
  return permissionMode === 'readonly' ? t('conversation.composer.readonlyPermission') : t('conversation.composer.autoPermission')
}

function toggleMessageDetail(messageId: string) {
  expandedMessageIds.value = expandedMessageIds.value.includes(messageId)
    ? expandedMessageIds.value.filter((item) => item !== messageId)
    : [...expandedMessageIds.value, messageId]
}

function handleComposerKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter' && canSend.value) {
    event.preventDefault()
    sendMessage()
  }
}

async function createConversation() {
  const conversation = workbench.createConversation()
  await router.push(createProjectConversationTarget(workbench.currentWorkspaceId, workbench.currentProjectId, conversation.id))
}
</script>

<template>
  <section class="conversation-page section-stack">
    <ConversationTabsBar />

    <div v-if="!activeConversation" class="conversation-empty-shell">
      <UiSurface
        class="conversation-empty-state"
        data-testid="conversation-empty-state"
        :title="t('conversation.empty.title')"
        :subtitle="t('conversation.empty.subtitle')"
      >
        <div class="conversation-empty-content">
          <UiEmptyState
            class="conversation-empty-card"
            :title="t('conversation.empty.guideTitle')"
            :description="t('conversation.empty.guideDescription')"
          />
          <UiButton data-testid="conversation-empty-create" @click="createConversation">
            <Plus :size="18" />
            {{ t('conversation.empty.create') }}
          </UiButton>
        </div>
      </UiSurface>
    </div>

    <div
      v-else
      class="conversation-chat-layout"
      data-testid="conversation-chat-layout"
      :style="{ '--conversation-detail-width': detailPaneWidth }"
    >
      <section class="conversation-main-column">
        <div class="conversation-stream-panel">
          <div class="conversation-stream-header">
            <div>
              <p class="conversation-stream-eyebrow">{{ t('conversation.header.eyebrow') }}</p>
              <h1 class="conversation-stream-title">
                {{ resolveMockField('conversation', activeConversation.id, 'title', activeConversation.title) }}
              </h1>
            </div>
            <p class="conversation-stream-subtitle">
              {{ resolveMockField('conversation', activeConversation.id, 'summary', activeConversation.summary) }}
            </p>
          </div>

          <div class="scroll-y conversation-scroll-region" data-testid="conversation-message-scroll">
            <div class="message-stream">
              <ConversationMessageBubble
                v-for="message in workbench.conversationMessages"
                :key="message.id"
                :message="message"
                :sender-label="messageSenderLabel(message.senderType, message.senderId, message.actorKind)"
                :avatar-label="messageAvatarLabel(message.senderType, message.senderId, message.actorKind)"
                :actor-label="actorLabel(message.actorKind, message.actorId)"
                :permission-label="permissionLabel(message.permissionMode)"
                :resources="[]"
                :attachments="message.attachments ?? []"
                :artifacts="[]"
                :is-expanded="expandedMessageIds.includes(message.id)"
                @toggle-detail="toggleMessageDetail"
              />
              <UiEmptyState
                v-if="!workbench.conversationMessages.length"
                class="conversation-stream-empty"
                :title="t('conversation.stream.emptyTitle')"
                :description="t('conversation.stream.emptyDescription')"
              />
            </div>
          </div>
        </div>

        <div class="conversation-composer-dock" data-testid="conversation-composer-dock">
          <div class="composer-shell">
            <ConversationQueueList
              v-if="queueItems.length"
              class="composer-queue"
              :items="queueItems"
              @remove="(id) => workbench.removeQueuedMessage(id)"
            />

            <section class="conversation-composer" data-testid="conversation-composer">
              <textarea
                v-model="messageDraft"
                class="conversation-composer-input"
                data-testid="conversation-composer-input"
                :placeholder="t('conversation.composer.placeholder')"
                @keydown="handleComposerKeydown"
              />

              <div v-if="selectedResources.length" class="composer-resource-strip">
                <div
                  v-for="resource in selectedResources"
                  :key="resource.id"
                  class="composer-resource-chip"
                >
                  <FolderOpen v-if="resource.kind === 'folder'" :size="12" />
                  <FileText v-else-if="resource.kind === 'artifact'" :size="12" />
                  <Paperclip v-else :size="12" />
                  <span>{{ resource.name }}</span>
                  <button type="button" class="composer-resource-remove" @click="removeSelectedResource(resource.id)">
                    <X :size="12" />
                  </button>
                </div>
              </div>

              <div class="composer-toolbar">
                <div class="composer-toolbar-main">
                  <UiPopover v-model:open="resourceMenuOpen">
                    <template #trigger>
                      <button
                        type="button"
                        class="composer-icon-button"
                        data-testid="composer-resource-trigger"
                        :title="t('conversation.composer.resourceTrigger')"
                        @click="resourceMenuOpen = !resourceMenuOpen"
                      >
                        <Plus :size="18" />
                      </button>
                    </template>
                    <div class="composer-popover-list">
                      <button
                        type="button"
                        class="composer-popover-item"
                        data-testid="resource-action-upload-file"
                        @click="createAndAttachResource('file')"
                      >
                        <Upload :size="16" class="composer-popover-icon" />
                        {{ t('conversation.composer.uploadFile') }}
                      </button>
                      <button
                        type="button"
                        class="composer-popover-item"
                        data-testid="resource-action-upload-folder"
                        @click="createAndAttachResource('folder')"
                      >
                        <FolderOpen :size="16" class="composer-popover-icon" />
                        {{ t('conversation.composer.uploadFolder') }}
                      </button>
                      <button
                        type="button"
                        class="composer-popover-item"
                        data-testid="resource-action-attach-existing"
                        @click="attachExistingResource"
                      >
                        <Search :size="16" class="composer-popover-icon" />
                        {{ t('conversation.composer.attachExisting') }}
                      </button>
                    </div>
                  </UiPopover>

                  <UiPopover v-model:open="permissionMenuOpen">
                    <template #trigger>
                      <div class="composer-select">
                        <select
                          class="composer-native-select"
                          data-testid="composer-permission-select"
                          :value="selectedPermissionMode"
                          tabindex="-1"
                          aria-hidden="true"
                        >
                          <option value="auto">{{ t('conversation.composer.autoPermission') }}</option>
                          <option value="readonly">{{ t('conversation.composer.readonlyPermission') }}</option>
                        </select>
                        <button
                          type="button"
                          class="composer-select-trigger"
                          @click="permissionMenuOpen = !permissionMenuOpen"
                        >
                          <Shield :size="14" />
                          <span class="composer-select-value">{{ selectedPermissionLabel }}</span>
                          <ChevronDown :size="12" />
                        </button>
                      </div>
                    </template>
                    <div class="composer-popover-list">
                      <button
                        type="button"
                        class="composer-popover-item"
                        :class="{ active: selectedPermissionMode === 'auto' }"
                        @click="selectedPermissionMode = 'auto'; permissionMenuOpen = false"
                      >
                        {{ t('conversation.composer.autoPermission') }}
                      </button>
                      <button
                        type="button"
                        class="composer-popover-item"
                        :class="{ active: selectedPermissionMode === 'readonly' }"
                        @click="selectedPermissionMode = 'readonly'; permissionMenuOpen = false"
                      >
                        {{ t('conversation.composer.readonlyPermission') }}
                      </button>
                    </div>
                  </UiPopover>

                  <UiPopover v-model:open="actorMenuOpen">
                    <template #trigger>
                      <div class="composer-select composer-select-wide">
                        <select
                          class="composer-native-select"
                          data-testid="composer-actor-select"
                          :value="selectedActorValue"
                          tabindex="-1"
                          aria-hidden="true"
                        >
                          <option value="">{{ selectedActorLabel }}</option>
                        </select>
                        <button
                          type="button"
                          class="composer-select-trigger"
                          @click="actorMenuOpen = !actorMenuOpen"
                        >
                          <Bot :size="14" />
                          <span class="composer-select-value">{{ selectedActorLabel }}</span>
                          <ChevronDown :size="12" />
                        </button>
                      </div>
                    </template>
                    <div class="composer-popover-list composer-popover-scroll">
                      <div v-if="actorGroups.agents.length">
                        <p class="composer-popover-label">{{ t('conversation.composer.agentGroup') }}</p>
                        <button
                          type="button"
                          class="composer-popover-item"
                          :class="{ active: !selectedActorValue }"
                          @click="selectedActorValue = ''; actorMenuOpen = false"
                        >
                          {{ t('conversation.composer.defaultActorOption', { name: defaultActorLabel }) }}
                        </button>
                        <button
                          v-for="agent in actorGroups.agents"
                          :key="agent.id"
                          type="button"
                          class="composer-popover-item"
                          :class="{ active: selectedActorValue === `agent:${agent.id}` }"
                          @click="selectedActorValue = `agent:${agent.id}`; actorMenuOpen = false"
                        >
                          {{ resolveMockField('agent', agent.id, 'name', agent.name) }}
                        </button>
                      </div>
                      <div v-if="actorGroups.teams.length">
                        <p class="composer-popover-label">{{ t('conversation.composer.teamGroup') }}</p>
                        <button
                          v-for="team in actorGroups.teams"
                          :key="team.id"
                          type="button"
                          class="composer-popover-item"
                          :class="{ active: selectedActorValue === `team:${team.id}` }"
                          @click="selectedActorValue = `team:${team.id}`; actorMenuOpen = false"
                        >
                          {{ resolveMockField('team', team.id, 'name', team.name) }}
                        </button>
                      </div>
                    </div>
                  </UiPopover>

                  <UiPopover v-model:open="modelMenuOpen">
                    <template #trigger>
                      <div class="composer-select">
                        <select
                          class="composer-native-select"
                          data-testid="composer-model-select"
                          :value="selectedModelId"
                          tabindex="-1"
                          aria-hidden="true"
                        >
                          <option :value="selectedModelId">{{ selectedModelLabel }}</option>
                        </select>
                        <button
                          type="button"
                          class="composer-select-trigger"
                          @click="modelMenuOpen = !modelMenuOpen"
                        >
                          <FileText :size="14" />
                          <span class="composer-select-value">{{ selectedModelLabel }}</span>
                          <ChevronDown :size="12" />
                        </button>
                      </div>
                    </template>
                    <div class="composer-popover-list">
                      <button
                        v-for="model in modelOptions"
                        :key="model.id"
                        type="button"
                        class="composer-popover-item"
                        :class="{ active: selectedModelId === model.id }"
                        @click="selectedModelId = model.id; modelMenuOpen = false"
                      >
                        {{ model.label }}
                      </button>
                    </div>
                  </UiPopover>
                </div>

                <UiButton
                  class="composer-send-button"
                  data-testid="conversation-composer-send"
                  :disabled="!canSend"
                  @click="sendMessage"
                >
                  <SendHorizontal :size="18" />
                </UiButton>
              </div>
            </section>
          </div>
        </div>
      </section>

      <div class="conversation-detail-shell">
        <ConversationContextPane class="conversation-detail-pane" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.conversation-page {
  min-height: 100%;
  height: 100%;
  overflow: hidden;
  gap: 0.8rem;
}

.conversation-empty-shell {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  min-height: 0;
  padding: 1rem 0;
}

.conversation-empty-state {
  width: min(100%, 42rem);
}

.conversation-empty-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
  padding: 1rem 0 0.25rem;
}

.conversation-empty-card {
  border: 0;
  background: transparent;
  padding: 0;
}

.conversation-chat-layout {
  --conversation-detail-width: 352px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) var(--conversation-detail-width);
  gap: 0.9rem;
  min-height: 0;
  flex: 1;
  align-items: stretch;
}

.conversation-main-column,
.conversation-stream-panel,
.conversation-detail-shell,
.composer-shell {
  min-width: 0;
  min-height: 0;
}

.conversation-main-column {
  display: grid;
  grid-template-rows: minmax(0, 1fr) auto;
  gap: 0.75rem;
  overflow: hidden;
}

.conversation-stream-panel,
.conversation-composer,
.conversation-detail-shell {
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 96%, transparent), color-mix(in srgb, var(--bg-surface) 92%, var(--bg-subtle))),
    var(--bg-surface);
  box-shadow: var(--shadow-sm);
}

.conversation-stream-panel,
.conversation-detail-shell {
  border-radius: calc(var(--radius-xl) + 4px);
}

.conversation-stream-panel {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  overflow: hidden;
}

.conversation-stream-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.8rem;
  padding: 0.9rem 1rem 0.8rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 90%, transparent);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--brand-primary) 8%, var(--bg-surface)), transparent 140%),
    color-mix(in srgb, var(--bg-surface) 94%, transparent);
}

.conversation-stream-eyebrow {
  color: var(--brand-primary);
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.conversation-stream-title {
  margin-top: 0.2rem;
  font-size: clamp(1.12rem, 1.35vw, 1.45rem);
  line-height: 1.08;
  letter-spacing: -0.03em;
}

.conversation-stream-subtitle {
  max-width: 22rem;
  color: var(--text-secondary);
  font-size: 0.82rem;
  line-height: 1.55;
  text-align: right;
}

.conversation-scroll-region {
  min-height: 0;
  padding: 0.75rem 0.45rem 0.2rem;
}

.message-stream {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  width: min(100%, 100%);
  min-height: 100%;
  margin: 0 auto;
}

.conversation-stream-empty {
  margin: auto 0 1rem;
}

.conversation-composer-dock {
  position: relative;
}

.composer-shell {
  position: relative;
  width: min(100%, 100%);
  margin: 0 auto;
}

.composer-queue {
  position: absolute;
  right: 1rem;
  bottom: calc(100% + 0.55rem);
  left: 1rem;
  z-index: 3;
}

.conversation-composer {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  padding: 0.75rem;
  border-radius: 1.2rem;
  transition:
    border-color var(--duration-fast) var(--ease-apple),
    box-shadow var(--duration-fast) var(--ease-apple),
    transform var(--duration-fast) var(--ease-apple);
}

.conversation-composer:focus-within {
  border-color: color-mix(in srgb, var(--brand-primary) 24%, var(--border-strong));
  box-shadow: var(--shadow-md);
}

.conversation-composer-input {
  min-height: 5.75rem;
  border: 0;
  background: transparent;
  box-shadow: none;
  resize: none;
  padding: 0.15rem 0.2rem 0;
  font-size: 0.9rem;
  line-height: 1.55;
}

.conversation-composer-input:focus-visible {
  box-shadow: none;
}

.composer-resource-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  padding: 0 0.2rem;
}

.composer-resource-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
  padding: 0.4rem 0.7rem;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 82%, transparent);
  color: var(--text-secondary);
  font-size: 0.78rem;
}

.composer-resource-chip span {
  overflow: hidden;
  max-width: 10rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.composer-resource-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1rem;
  height: 1rem;
  color: var(--text-tertiary);
}

.composer-resource-remove:hover {
  color: var(--status-error);
}

.composer-toolbar,
.composer-toolbar-main,
.composer-icon-button,
.composer-select-trigger {
  display: flex;
  align-items: center;
}

.composer-toolbar {
  justify-content: space-between;
  gap: 0.9rem;
}

.composer-toolbar-main {
  flex: 1;
  min-width: 0;
  gap: 0.55rem;
  flex-wrap: wrap;
}

.composer-icon-button,
.composer-select-trigger {
  min-height: 2.2rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 76%, transparent);
  box-shadow: var(--shadow-xs);
  color: var(--text-secondary);
  transition:
    border-color var(--duration-fast) var(--ease-apple),
    background-color var(--duration-fast) var(--ease-apple),
    color var(--duration-fast) var(--ease-apple),
    transform var(--duration-fast) var(--ease-apple);
}

.composer-icon-button:hover,
.composer-select-trigger:hover {
  border-color: color-mix(in srgb, var(--brand-primary) 18%, var(--border-strong));
  background: color-mix(in srgb, var(--bg-surface) 92%, var(--brand-primary) 6%);
  color: var(--text-primary);
}

.composer-icon-button {
  justify-content: center;
  width: 2.2rem;
  border-radius: 999px;
}

.composer-select {
  position: relative;
}

.composer-select-wide {
  max-width: min(100%, 20rem);
}

.composer-native-select {
  position: absolute;
  inset: 0;
  opacity: 0;
  pointer-events: none;
}

.composer-select-trigger {
  gap: 0.45rem;
  max-width: 100%;
  padding: 0 0.85rem;
  border-radius: 999px;
}

.composer-select-value {
  overflow: hidden;
  max-width: 12rem;
  font-size: 0.74rem;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.composer-send-button {
  width: 2.45rem;
  min-width: 2.45rem;
  height: 2.45rem;
  padding: 0;
  border-radius: 999px;
}

.composer-popover-list {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  min-width: 14rem;
}

.composer-popover-scroll {
  max-height: 18rem;
  overflow-y: auto;
}

.composer-popover-item {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  width: 100%;
  padding: 0.7rem 0.8rem;
  border-radius: var(--radius-m);
  text-align: left;
  color: var(--text-primary);
  transition: background-color var(--duration-fast) var(--ease-apple), color var(--duration-fast) var(--ease-apple);
}

.composer-popover-item:hover,
.composer-popover-item.active {
  background: color-mix(in srgb, var(--brand-primary) 9%, transparent);
}

.composer-popover-item.active {
  color: var(--brand-primary);
  font-weight: 600;
}

.composer-popover-icon {
  color: var(--text-tertiary);
}

.composer-popover-label {
  margin: 0.35rem 0 0.2rem;
  padding: 0 0.25rem;
  color: var(--text-tertiary);
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.conversation-detail-shell {
  overflow: hidden;
}

.conversation-detail-pane {
  height: 100%;
}

@media (max-width: 1240px) {
  .conversation-chat-layout {
    grid-template-columns: minmax(0, 1fr);
  }

  .conversation-detail-shell {
    display: none;
  }
}

@media (max-width: 820px) {
  .conversation-stream-header {
    flex-direction: column;
  }

  .conversation-stream-subtitle {
    max-width: none;
    text-align: left;
  }

  .conversation-scroll-region {
    padding: 1rem 0.7rem 0.25rem;
  }

  .conversation-composer {
    padding: 0.8rem;
    border-radius: 1.4rem;
  }

  .composer-toolbar {
    align-items: stretch;
    flex-direction: column;
  }

  .composer-send-button {
    width: 100%;
    min-width: 0;
    border-radius: var(--radius-l);
  }
}
</style>
