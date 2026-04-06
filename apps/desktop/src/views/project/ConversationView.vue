<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { Bot, FolderOpen, SendHorizontal, Sparkles } from 'lucide-vue-next'

import { type Message, type PermissionMode } from '@octopus/schema'
import { UiButton, UiEmptyState, UiTextarea } from '@octopus/ui'

import ConversationMessageBubble from '@/components/conversation/ConversationMessageBubble.vue'
import ConversationQueueList from '@/components/conversation/ConversationQueueList.vue'
import ConversationContextPane from '@/components/layout/ConversationContextPane.vue'
import ConversationTabsBar from '@/components/layout/ConversationTabsBar.vue'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const runtime = useRuntimeStore()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const messageDraft = ref('')
const selectedModelId = ref('gpt-4o')
const selectedPermissionMode = ref<PermissionMode>('auto')
const expandedMessageIds = ref<string[]>([])
const scrollContainer = ref<HTMLElement | null>(null)

const activeConversation = computed(() => workbench.activeConversation)
const runtimeOwnsConversation = computed(() =>
  !!activeConversation.value && runtime.activeConversationId === activeConversation.value.id,
)
const renderedMessages = computed<Message[]>(() =>
  runtimeOwnsConversation.value && runtime.activeMessages.length
    ? runtime.activeMessages
    : workbench.conversationMessages,
)
const queueItems = computed(() =>
  runtime.activeQueue.map((item) => ({
    id: item.id,
    content: item.content,
    actorLabel: item.actorLabel,
    createdAt: item.createdAt,
  })),
)
const detailPaneWidth = computed(() => (shell.rightSidebarCollapsed ? '48px' : '360px'))
const selectedModelLabel = computed(() =>
  workbench.workspaceModelCatalog.find((item) => item.id === selectedModelId.value)?.label ?? selectedModelId.value,
)
const defaultActor = computed(() => workbench.activeConversationDefaultActor)
const selectedActorLabel = computed(() => workbench.conversationDefaultActorLabel)
const permissionLabel = computed(() =>
  selectedPermissionMode.value === 'readonly' ? t('conversation.composer.readonlyPermission') : t('conversation.composer.autoPermission'),
)

function messageSenderLabel(message: Message) {
  if (message.senderType === 'user') {
    return 'You'
  }

  if (message.senderId === 'Octopus Runtime') {
    return 'Octopus Runtime'
  }

  return selectedActorLabel.value.replace('默认智能体 · ', '') || 'Octopus'
}

function messageAvatarLabel(message: Message) {
  if (message.senderType === 'user') {
    return 'Y'
  }

  return 'O'
}

function scrollToBottom() {
  nextTick(() => {
    if (scrollContainer.value) {
      scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight
    }
  })
}

watch(renderedMessages, scrollToBottom, { deep: true })

watch(
  () => route.params.conversationId,
  async (conversationId) => {
    if (typeof conversationId === 'string') {
      workbench.selectConversation(conversationId)
      await ensureRuntimeSession()
    }
  },
  { immediate: true },
)

watch(
  () => route.params.projectId,
  (projectId) => {
    if (typeof projectId === 'string') {
      workbench.selectProject(projectId)
    }
  },
  { immediate: true },
)

onMounted(async () => {
  await ensureRuntimeSession()
})

async function ensureRuntimeSession() {
  if (!activeConversation.value) {
    return
  }

  await runtime.ensureSession({
    conversationId: activeConversation.value.id,
    projectId: workbench.currentProjectId,
    title: workbench.conversationDisplayTitle(activeConversation.value.id),
  })
}

async function createConversationFromEmpty() {
  const conversation = workbench.createConversation(workbench.currentProjectId)
  await router.push(createProjectConversationTarget(workbench.currentWorkspaceId, workbench.currentProjectId, conversation.id))
}

async function submitRuntimeTurn() {
  if (!activeConversation.value || !messageDraft.value.trim()) {
    return
  }

  await ensureRuntimeSession()
  await runtime.submitTurn({
    content: messageDraft.value,
    modelId: selectedModelId.value,
    permissionMode: selectedPermissionMode.value,
    actorLabel: selectedActorLabel.value,
  })
  messageDraft.value = ''
}

function handleComposerKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
    event.preventDefault()
    void submitRuntimeTurn()
  }
}

function toggleMessageDetail(messageId: string) {
  expandedMessageIds.value = expandedMessageIds.value.includes(messageId)
    ? expandedMessageIds.value.filter((item) => item !== messageId)
    : [...expandedMessageIds.value, messageId]
}

function removeQueuedTurn(queueItemId: string) {
  runtime.removeQueuedTurn(queueItemId)
}
</script>

<template>
  <div class="flex h-full w-full overflow-hidden bg-background flex-col">
    <ConversationTabsBar v-if="activeConversation || workbench.projectConversations.length" />

    <div v-if="!activeConversation" class="flex flex-1 min-h-0 overflow-hidden">
      <main class="flex flex-1 items-center justify-center px-6" data-testid="conversation-empty-state">
        <UiEmptyState
          :title="t('conversation.empty.guideTitle')"
          :description="t('conversation.empty.guideDescription')"
        >
          <template #actions>
            <UiButton data-testid="conversation-empty-create" @click="createConversationFromEmpty">
              {{ t('conversation.empty.create') }}
            </UiButton>
          </template>
        </UiEmptyState>
      </main>
    </div>

    <div v-else data-testid="conversation-chat-layout" class="flex flex-1 min-h-0 overflow-hidden">
      <div class="flex min-w-0 flex-1 flex-col overflow-hidden">
        <main
          ref="scrollContainer"
          data-testid="conversation-message-scroll"
          class="message-stream flex-1 overflow-y-auto px-6 py-6"
        >
          <ConversationMessageBubble
            v-for="message in renderedMessages"
            :key="message.id"
            :message="message"
            :sender-label="messageSenderLabel(message)"
            :avatar-label="messageAvatarLabel(message)"
            :actor-label="selectedActorLabel"
            :permission-label="permissionLabel"
            :resources="[]"
            :attachments="message.attachments ?? []"
            :artifacts="[]"
            :is-expanded="expandedMessageIds.includes(message.id)"
            @toggle-detail="toggleMessageDetail"
          />
        </main>

        <footer data-testid="conversation-composer" class="shrink-0 border-t border-border-subtle dark:border-white/[0.05] bg-background px-6 py-4">
          <div class="mx-auto flex max-w-[960px] flex-col gap-3" data-testid="conversation-composer-dock">
            <ConversationQueueList :items="queueItems" @remove="removeQueuedTurn" />

            <div class="rounded-2xl border border-border-subtle dark:border-white/[0.08] bg-background p-4 shadow-sm">
              <UiTextarea
                v-model="messageDraft"
                data-testid="conversation-runtime-composer-input"
                class="w-full resize-none border-0 bg-transparent p-0 text-[15px] shadow-none focus-visible:ring-0"
                :placeholder="t('conversation.composer.placeholder')"
                auto-height
                @keydown="handleComposerKeydown"
              />

              <div class="mt-4 flex items-center justify-between gap-3">
                <div class="flex flex-wrap items-center gap-2">
                  <button
                    type="button"
                    data-testid="composer-actor-trigger"
                    class="flex items-center gap-2 rounded-lg px-3 py-1.5 text-[12px] font-bold text-text-secondary hover:bg-accent"
                  >
                    <Bot :size="14" class="text-primary opacity-80" />
                    {{ selectedActorLabel }}
                  </button>
                  <button
                    type="button"
                    data-testid="composer-model-trigger"
                    class="flex items-center gap-2 rounded-lg px-3 py-1.5 text-[12px] font-bold text-text-secondary hover:bg-accent"
                  >
                    <Sparkles :size="14" class="text-primary opacity-80" />
                    {{ selectedModelLabel }}
                  </button>
                  <button
                    type="button"
                    data-testid="composer-permission-trigger"
                    class="rounded-lg px-3 py-1.5 text-[12px] font-bold text-text-secondary hover:bg-accent"
                  >
                    {{ permissionLabel }}
                  </button>
                  <button
                    type="button"
                    data-testid="composer-resource-trigger"
                    class="flex items-center gap-2 rounded-lg px-3 py-1.5 text-[12px] font-bold text-text-secondary hover:bg-accent"
                  >
                    <FolderOpen :size="14" class="text-text-tertiary" />
                    {{ t('conversation.composer.attachExisting') }}
                  </button>
                </div>

                <UiButton
                  data-testid="conversation-runtime-send"
                  variant="primary"
                  size="icon"
                  class="h-9 w-9 rounded-xl"
                  :disabled="!messageDraft.trim()"
                  @click="submitRuntimeTurn"
                >
                  <SendHorizontal :size="18" />
                </UiButton>
              </div>
            </div>
          </div>
        </footer>
      </div>

      <aside class="shrink-0 overflow-hidden border-l border-border-subtle dark:border-white/[0.05] bg-sidebar transition-all duration-300" :style="{ width: detailPaneWidth }">
        <ConversationContextPane class="h-full w-[360px]" />
      </aside>
    </div>
  </div>
</template>
