<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { AlertTriangle, Plus, SendHorizontal } from 'lucide-vue-next'

import type { Message, PermissionMode } from '@octopus/schema'
import { UiButton, UiEmptyState, UiField, UiSelect, UiTextarea } from '@octopus/ui'

import ConversationMessageBubble from '@/components/conversation/ConversationMessageBubble.vue'
import ConversationQueueList from '@/components/conversation/ConversationQueueList.vue'
import ConversationContextPane from '@/components/layout/ConversationContextPane.vue'
import ConversationTabsBar from '@/components/layout/ConversationTabsBar.vue'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const runtime = useRuntimeStore()
const shell = useShellStore()
const catalogStore = useCatalogStore()
const agentStore = useAgentStore()
const workspaceStore = useWorkspaceStore()

const messageDraft = ref('')
const selectedModelId = ref('')
const selectedPermissionMode = ref<PermissionMode>('auto')
const selectedActorLabel = ref('')
const expandedMessageIds = ref<string[]>([])
const scrollContainer = ref<HTMLElement | null>(null)

const conversationId = computed(() =>
  typeof route.params.conversationId === 'string' ? route.params.conversationId : '',
)
const projectId = computed(() =>
  typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId,
)
const workspaceId = computed(() =>
  typeof route.params.workspaceId === 'string' ? route.params.workspaceId : workspaceStore.currentWorkspaceId,
)

const modelOptions = computed(() =>
  catalogStore.models.map(model => ({ value: model.id, label: model.label })),
)
const actorOptions = computed(() =>
  [...agentStore.projectAgents, ...agentStore.workspaceAgents].map(agent => ({
    value: agent.name,
    label: agent.name,
  })),
)
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
    actorLabel: item.actorLabel,
    createdAt: item.createdAt,
  })),
)

function createConversationId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `conversation-${crypto.randomUUID()}`
  }
  return `conversation-${Date.now()}`
}

async function ensureRuntimeSession() {
  if (!conversationId.value || !projectId.value) {
    return
  }

  await Promise.all([
    catalogStore.load(),
    agentStore.load(),
  ])

  if (!selectedModelId.value) {
    selectedModelId.value = catalogStore.models[0]?.id ?? 'gpt-4o'
  }
  if (!selectedActorLabel.value) {
    selectedActorLabel.value = agentStore.projectAgents[0]?.name ?? agentStore.workspaceAgents[0]?.name ?? 'Assistant'
  }

  await runtime.ensureSession({
    conversationId: conversationId.value,
    projectId: projectId.value,
    title: `Conversation ${conversationId.value.slice(-6)}`,
  })
}

watch(renderedMessages, () => {
  nextTick(() => {
    if (scrollContainer.value) {
      scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight
    }
  })
}, { deep: true })

watch(
  () => [conversationId.value, projectId.value, shell.activeWorkspaceConnectionId, shell.activeWorkspaceSession?.token],
  () => {
    if (shell.activeWorkspaceConnectionId && shell.activeWorkspaceSession?.token) {
      void ensureRuntimeSession()
    }
  },
  { immediate: true },
)

onMounted(() => {
  void ensureRuntimeSession()
})

async function createConversationFromEmpty() {
  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, createConversationId()))
}

async function submitRuntimeTurn() {
  if (!messageDraft.value.trim()) {
    return
  }

  await ensureRuntimeSession()
  await runtime.submitTurn({
    content: messageDraft.value,
    modelId: selectedModelId.value || 'gpt-4o',
    permissionMode: selectedPermissionMode.value,
    actorLabel: selectedActorLabel.value || 'Assistant',
  })
  messageDraft.value = ''
}

function handleComposerKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
    event.preventDefault()
    void submitRuntimeTurn()
  }
}

function toggleDetail(messageId: string) {
  expandedMessageIds.value = expandedMessageIds.value.includes(messageId)
    ? expandedMessageIds.value.filter(id => id !== messageId)
    : [...expandedMessageIds.value, messageId]
}
</script>

<template>
  <div class="flex h-full min-h-0 w-full">
    <div class="flex min-w-0 flex-1 flex-col px-2 pb-6">
      <ConversationTabsBar />

      <div v-if="!conversationId" class="flex flex-1 items-center justify-center">
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
        <ConversationQueueList :items="queueItems" @remove="runtime.removeQueuedTurn" />

        <div ref="scrollContainer" class="flex-1 overflow-y-auto py-4">
          <ConversationMessageBubble
            v-for="message in renderedMessages"
            :key="message.id"
            :message="message"
            :sender-label="message.senderType === 'user' ? 'You' : message.senderId"
            :avatar-label="message.senderType === 'user' ? 'Y' : 'O'"
            :actor-label="selectedActorLabel"
            :permission-label="selectedPermissionMode"
            :resources="[]"
            :attachments="[]"
            :artifacts="[]"
            :is-expanded="expandedMessageIds.includes(message.id)"
            @toggle-detail="toggleDetail"
          />

          <UiEmptyState
            v-if="!renderedMessages.length"
            :title="t('conversation.messages.emptyTitle')"
            :description="t('conversation.messages.emptyDescription')"
          />
        </div>

        <div class="mt-4 rounded-2xl border border-border-subtle bg-card p-4 dark:border-white/[0.05]">
          <div v-if="runtime.pendingApproval" class="mb-4 flex items-start gap-3 rounded-xl border border-status-warning/20 bg-status-warning/5 p-3">
            <AlertTriangle :size="16" class="mt-0.5 text-status-warning" />
            <div class="space-y-1">
              <div class="text-sm font-semibold text-text-primary">{{ runtime.pendingApproval.summary }}</div>
              <div class="text-sm text-text-secondary">{{ runtime.pendingApproval.detail }}</div>
            </div>
          </div>

          <div class="grid gap-3 md:grid-cols-3">
            <UiField :label="t('conversation.composer.modelLabel')">
              <UiSelect v-model="selectedModelId" :options="modelOptions" />
            </UiField>
            <UiField :label="t('conversation.composer.agentSection')">
              <UiSelect v-model="selectedActorLabel" :options="actorOptions" />
            </UiField>
            <UiField :label="t('conversation.composer.permissionLabel')">
              <UiSelect v-model="selectedPermissionMode" :options="permissionOptions" />
            </UiField>
          </div>

          <div class="mt-3 flex items-end gap-3">
            <UiTextarea
              v-model="messageDraft"
              class="min-h-[120px] flex-1"
              :rows="5"
              :placeholder="t('conversation.composer.placeholder')"
              @keydown="handleComposerKeydown"
            />
            <UiButton class="shrink-0" @click="submitRuntimeTurn">
              <SendHorizontal :size="14" />
              {{ t('conversation.composer.send') }}
            </UiButton>
          </div>
        </div>
      </template>
    </div>

    <ConversationContextPane />
  </div>
</template>
