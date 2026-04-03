<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import {
  Bot,
  Plus,
  X,
  Sparkles,
  MessageSquare,
  MoreVertical,
  Trash2,
  SendHorizontal,
  FolderOpen,
  Paperclip,
  PanelRight
} from 'lucide-vue-next'

import {
  type PermissionMode,
  type ProjectResource,
} from '@octopus/schema'
import { UiButton, UiEmptyState, UiTextarea } from '@octopus/ui'

import ConversationMessageBubble from '@/components/conversation/ConversationMessageBubble.vue'
import ConversationContextPane from '@/components/layout/ConversationContextPane.vue'
import { resolveMockField } from '@/i18n/copy'
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
const selectedActorValue = ref('')
const selectedResourceIds = ref<string[]>([])
const expandedMessageIds = ref<string[]>([])

const activeConversation = computed(() => workbench.activeConversation)
const conversations = computed(() => workbench.projectConversations)
const conversationMessages = computed(() => {
  if (runtime.activeConversationId === activeConversation.value?.id && runtime.activeMessages.length) {
    return runtime.activeMessages
  }

  return workbench.conversationMessages
})

const detailPaneCollapsed = computed(() => shell.rightSidebarCollapsed)
const detailPaneWidth = computed(() => (detailPaneCollapsed.value ? '0px' : '360px'))

const modelOptions = computed(() => workbench.workspaceModelCatalog)
const selectedActorLabel = computed(() => {
  if (!selectedActorValue.value) return 'Octopus'
  const [kind, id] = selectedActorValue.value.split(':')
  return actorLabel(kind, id)
})
const selectedModelLabel = computed(() => modelOptions.value.find((m) => m.id === selectedModelId.value)?.label ?? selectedModelId.value)

// --- Session Management ---
const editingSessionId = ref<string | null>(null)
const sessionNameDraft = ref('')

function startRename(conv: any) {
  editingSessionId.value = conv.id
  sessionNameDraft.value = resolveMockField('conversation', conv.id, 'title', conv.title)
}

function finishRename() {
  if (editingSessionId.value && sessionNameDraft.value.trim()) {
    workbench.renameConversation(editingSessionId.value, sessionNameDraft.value.trim())
  }
  editingSessionId.value = null
}

async function createNewSession() {
  const conv = workbench.createConversation()
  await router.push(createProjectConversationTarget(workbench.currentWorkspaceId, workbench.currentProjectId, conv.id))
}

async function removeSession(id: string) {
  const nextId = workbench.removeConversation(id)
  if (nextId) {
    await router.push(createProjectConversationTarget(workbench.currentWorkspaceId, workbench.currentProjectId, nextId))
  }
}

async function switchSession(id: string) {
  await router.push(createProjectConversationTarget(workbench.currentWorkspaceId, workbench.currentProjectId, id))
}

// --- Messaging & Scroll ---
const scrollContainer = ref<HTMLElement | null>(null)

function scrollToBottom() {
  nextTick(() => {
    if (scrollContainer.value) {
      scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight
    }
  })
}

watch(() => conversationMessages.value.length, scrollToBottom)

watch(
  () => activeConversation.value?.id,
  async (conversationId) => {
    if (!conversationId || !workbench.activeProject) {
      return
    }

    await runtime.ensureSession({
      conversationId,
      projectId: workbench.activeProject.id,
      title: activeConversation.value?.title ?? 'Runtime Session',
    })
  },
  { immediate: true },
)

async function sendMessage() {
  const trimmed = messageDraft.value.trim()
  if (!trimmed || !activeConversation.value || !workbench.activeProject) return

  await runtime.ensureSession({
    conversationId: activeConversation.value.id,
    projectId: workbench.activeProject.id,
    title: activeConversation.value.title,
  })
  await runtime.submitTurn({
    content: trimmed,
    modelId: selectedModelId.value,
    permissionMode: selectedPermissionMode.value,
    actorLabel: selectedActorLabel.value,
  })

  messageDraft.value = ''
  selectedResourceIds.value = []
}

function handleComposerKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
    event.preventDefault()
    sendMessage()
  }
}

// --- Helpers ---
function actorLabel(kind?: string, id?: string) {
  const source = kind === 'team' ? workbench.teams : workbench.agents
  const item = source.find(i => i.id === id)
  return item ? resolveMockField(kind as any, item.id, 'name', (item as any).name) : id || ''
}

function messageSenderLabel(senderType: string, senderId: string, actorKind?: string) {
  if (senderType === 'user') return 'You'
  return actorLabel(actorKind ?? 'agent', senderId) || 'Octopus'
}

function messageAvatarLabel(senderType: string, senderId: string, actorKind?: string) {
  if (senderType === 'user') return 'Y'
  const source = actorKind === 'team' ? workbench.teams : workbench.agents
  const item = source.find(i => i.id === senderId)
  return (item as any)?.avatar || 'O'
}
</script>

<template>
  <div class="flex h-full w-full overflow-hidden bg-background flex-col">
    <!-- 1. Top Session Tabs -->
    <header class="flex shrink-0 items-center gap-1 border-b border-border-subtle bg-subtle/20 px-4 h-11 z-30">
      <div class="flex flex-1 items-center gap-1 overflow-x-auto no-scrollbar py-1">
        <div
          v-for="conv in conversations"
          :key="conv.id"
          class="group relative flex items-center h-8 px-3 rounded-md transition-all cursor-pointer whitespace-nowrap min-w-[120px]"
          :class="activeConversation?.id === conv.id ? 'bg-background shadow-sm text-text-primary border border-border-subtle' : 'text-text-tertiary hover:bg-accent hover:text-text-secondary'"
          @click="switchSession(conv.id)"
        >
          <MessageSquare :size="12" class="mr-2 opacity-60" />
          <input
            v-if="editingSessionId === conv.id"
            v-model="sessionNameDraft"
            class="bg-transparent border-none outline-none text-[12px] font-bold w-24"
            auto-focus
            @blur="finishRename"
            @keydown.enter="finishRename"
          />
          <span v-else class="text-[12px] font-bold truncate max-w-[140px]" @dblclick="startRename(conv)">
            {{ resolveMockField('conversation', conv.id, 'title', conv.title) }}
          </span>
          <button v-if="conversations.length > 1" class="ml-2 opacity-0 group-hover:opacity-100 p-0.5 hover:bg-subtle rounded" @click.stop="removeSession(conv.id)"><X :size="10" /></button>
        </div>
        <UiButton variant="ghost" size="icon" class="h-7 w-7 rounded-md ml-1" @click="createNewSession"><Plus :size="14" /></UiButton>
      </div>
      <UiButton variant="ghost" size="icon" class="h-8 w-8 ml-2 text-text-tertiary hover:text-text-primary" @click="shell.toggleRightSidebar()"><PanelRight :size="16" /></UiButton>
    </header>

    <!-- 2. Main Area (Message Area + Sidebar) -->
    <div class="flex flex-1 min-h-0 relative overflow-hidden">
      
      <!-- Central Chat Column -->
      <div class="flex flex-1 flex-col min-w-0 h-full relative">
        
        <!-- Scrolling Message List (Ends exactly where composer starts) -->
        <main ref="scrollContainer" class="flex-1 overflow-y-auto bg-background py-8 px-6">
          <div v-if="!activeConversation" class="flex h-full items-center justify-center p-12">
            <UiEmptyState :title="t('conversation.empty.guideTitle')" :description="t('conversation.empty.guideDescription')" />
          </div>
          
          <div v-else class="w-full flex flex-col min-h-0">
            <div class="space-y-4">
              <ConversationMessageBubble
                v-for="message in conversationMessages"
                :key="message.id"
                :message="message"
                :sender-label="messageSenderLabel(message.senderType, message.senderId, message.actorKind)"
                :avatar-label="messageAvatarLabel(message.senderType, message.senderId, message.actorKind)"
                :actor-label="actorLabel(message.actorKind, message.actorId)"
                :permission-label="selectedPermissionMode"
                :resources="[]"
                :attachments="message.attachments ?? []"
                :artifacts="[]"
                :is-expanded="expandedMessageIds.includes(message.id)"
                @toggle-detail="(id) => expandedMessageIds.includes(id) ? (expandedMessageIds = expandedMessageIds.filter(i => i !== id)) : expandedMessageIds.push(id)"
              />
            </div>
          </div>
        </main>

        <!-- 3. Fixed Bottom Composer (Part of the vertical flex flow, NO overlap) -->
        <footer class="shrink-0 bg-background px-6 py-6">
          <div class="mx-auto max-w-[900px] border border-border-strong bg-background shadow-lg rounded-2xl overflow-hidden">
            <div class="p-4">
              <UiTextarea
                v-model="messageDraft"
                data-testid="conversation-runtime-composer-input"
                class="w-full min-h-[48px] max-h-[300px] resize-none border-0 bg-transparent p-0 text-[15px] focus-visible:ring-0 shadow-none leading-relaxed"
                :placeholder="t('conversation.composer.placeholder')"
                auto-height
                @keydown="handleComposerKeydown"
              />
              
              <!-- Clean Actions (No border line) -->
              <div class="flex items-center justify-between mt-4">
                <div class="flex items-center gap-1.5">
                  <UiButton variant="ghost" size="icon" class="h-8 w-8 rounded-lg text-text-tertiary hover:bg-accent"><Plus :size="18" /></UiButton>
                  <div class="h-4 w-px bg-border-subtle mx-1" />
                  
                  <button class="px-3 py-1.5 text-[12px] font-bold text-text-secondary hover:bg-accent rounded-lg transition-colors flex items-center gap-2">
                    <Bot :size="14" class="text-primary opacity-80" /> {{ selectedActorLabel }}
                  </button>
                  
                  <button class="px-3 py-1.5 text-[12px] font-bold text-text-secondary hover:bg-accent rounded-lg transition-colors flex items-center gap-2">
                    <Sparkles :size="14" class="text-primary opacity-80" /> {{ selectedModelLabel }}
                  </button>
                </div>

                <UiButton
                  data-testid="conversation-runtime-send"
                  variant="primary"
                  size="icon"
                  class="h-9 w-9 rounded-xl shadow-md"
                  :disabled="!messageDraft.trim()"
                  @click="sendMessage"
                >
                  <SendHorizontal :size="18" />
                </UiButton>
              </div>
            </div>
          </div>
        </footer>
      </div>

      <!-- Right Sidebar -->
      <aside
        class="transition-all duration-300 overflow-hidden shrink-0 h-full border-l border-border-subtle bg-sidebar"
        :style="{ width: detailPaneWidth }"
      >
        <ConversationContextPane class="h-full w-[360px]" />
      </aside>
    </div>
  </div>
</template>

<style scoped>
.no-scrollbar::-webkit-scrollbar {
  display: none;
}
.no-scrollbar {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
</style>
