<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { MessageSquareText, PanelRight, Plus } from 'lucide-vue-next'

import { UiButton, UiPanelFrame } from '@octopus/ui'

import { createProjectConversationTarget } from '@/i18n/navigation'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
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
  runtime.sessions.filter(session => session.projectId === projectId.value),
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
</script>

<template>
  <section class="mb-2" data-testid="conversation-tabs">
    <UiPanelFrame variant="subtle" padding="none" class="px-2 py-1.5">
      <div class="flex items-center justify-between gap-2">
        <div class="flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto">
          <UiButton
            v-for="session in conversations"
            :key="session.id"
            variant="ghost"
            class="flex h-8 min-w-0 max-w-[14rem] items-center gap-2 rounded-lg border border-transparent px-2.5 text-text-secondary hover:border-border/60 hover:bg-background/60 hover:text-text-primary"
            :class="session.conversationId === activeConversationId ? 'border-primary/20 bg-primary/[0.06] text-text-primary shadow-xs' : ''"
            @click="openConversation(session.conversationId)"
          >
            <MessageSquareText :size="12" class="shrink-0" />
            <span class="truncate text-xs font-medium">{{ session.title }}</span>
          </UiButton>

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

        <div class="flex shrink-0 items-center border-l border-border/40 pl-2">
          <UiButton
            variant="ghost"
            size="icon"
            class="h-8 w-8 text-text-tertiary hover:bg-muted/80 hover:text-text-primary"
            @click="shell.toggleRightSidebar()"
          >
            <PanelRight :size="16" />
          </UiButton>
        </div>
      </div>
    </UiPanelFrame>
  </section>
</template>
