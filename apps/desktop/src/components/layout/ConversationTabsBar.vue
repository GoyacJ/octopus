<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { MessageSquareText, Plus, X } from 'lucide-vue-next'

import { UiButton, UiPanelFrame } from '@octopus/ui'

import { createProjectConversationTarget } from '@/i18n/navigation'
import { useRuntimeStore } from '@/stores/runtime'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const runtime = useRuntimeStore()
const workspaceStore = useWorkspaceStore()

const workspaceId = computed(() =>
  typeof route.params.workspaceId === 'string' ? route.params.workspaceId : workspaceStore.currentWorkspaceId,
)
const projectId = computed(() =>
  typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId,
)
const activeConversationId = computed(() =>
  typeof route.params.conversationId === 'string' ? route.params.conversationId : runtime.activeConversationId,
)
const conversations = computed(() =>
  runtime.sessions.filter(session => session.projectId === projectId.value && session.sessionKind !== 'pet'),
)

function createConversationId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `conversation-${crypto.randomUUID()}`
  }
  return `conversation-${Date.now()}`
}

async function createConversation() {
  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, createConversationId()))
}

async function openConversation(conversationId: string) {
  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, conversationId))
}

async function removeConversation(event: MouseEvent, sessionId: string) {
  event.stopPropagation()
  const sessionToRemove = runtime.sessions.find(s => s.id === sessionId)
  const isRemovingActive = sessionToRemove?.conversationId === activeConversationId.value
  
  await runtime.deleteSession(sessionId)

  if (isRemovingActive) {
    void router.push(createProjectConversationTarget(workspaceId.value, projectId.value, ''))
  }
}
</script>

<template>
  <section class="mb-2" data-testid="conversation-tabs">
    <UiPanelFrame variant="subtle" padding="none" class="px-2 py-1.5">
      <div class="flex items-center gap-2">
        <div class="scrollbar-visible flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto pb-1">
          <div
            v-for="session in conversations"
            :key="session.id"
            class="group relative flex shrink-0 items-center"
          >
            <UiButton
              variant="ghost"
              class="flex h-8 min-w-0 max-w-[14rem] items-center gap-2 rounded-[var(--radius-m)] border border-transparent pl-2.5 pr-8 text-text-secondary hover:border-border hover:bg-surface hover:text-text-primary"
              :class="session.conversationId === activeConversationId ? 'border-border bg-surface text-text-primary shadow-xs' : ''"
              @click="openConversation(session.conversationId)"
            >
              <MessageSquareText :size="12" class="shrink-0" />
              <span class="truncate text-xs font-medium">{{ session.title }}</span>
            </UiButton>

            <button
              class="absolute right-1.5 flex h-5 w-5 items-center justify-center rounded-md text-text-tertiary opacity-0 transition-all hover:bg-muted-foreground/10 hover:text-text-primary group-hover:opacity-100"
              :class="session.conversationId === activeConversationId ? 'opacity-100' : ''"
              @click="removeConversation($event, session.id)"
            >
              <X :size="12" />
            </button>
          </div>

          <UiButton
            variant="outline"
            size="icon"
            class="ml-1 h-7 w-7 shrink-0 rounded-md"
            data-testid="conversation-tab-create"
            :title="t('conversation.tabs.create')"
            @click="createConversation"
          >
            <Plus :size="12" />
          </UiButton>
        </div>
      </div>
    </UiPanelFrame>
  </section>
</template>

<style scoped>
.scrollbar-visible {
  scrollbar-width: thin;
  scrollbar-color: var(--border-subtle) transparent;
}

.scrollbar-visible::-webkit-scrollbar {
  height: 4px;
}

.scrollbar-visible::-webkit-scrollbar-track {
  background: transparent;
}

.scrollbar-visible::-webkit-scrollbar-thumb {
  background: var(--border-subtle);
  border-radius: 10px;
}

.scrollbar-visible::-webkit-scrollbar-thumb:hover {
  background: var(--text-tertiary);
}
</style>
