<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { MessageSquareText, Plus, X, PanelRight } from 'lucide-vue-next'

import { UiButton, UiPanelFrame } from '@octopus/ui'

import { createProjectConversationTarget } from '@/i18n/navigation'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const conversations = computed(() => workbench.projectConversations)
const workspaceId = computed(() =>
  typeof route.params.workspaceId === 'string' ? route.params.workspaceId : workbench.currentWorkspaceId,
)
const projectId = computed(() =>
  typeof route.params.projectId === 'string' ? route.params.projectId : workbench.currentProjectId,
)
const activeRouteConversationId = computed(() =>
  typeof route.params.conversationId === 'string' ? route.params.conversationId : '',
)

async function createConversation() {
  const conversation = workbench.createConversation(projectId.value)
  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, conversation.id))
}

async function openConversation(conversationId: string) {
  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, conversationId))
}

async function removeConversation(conversationId: string) {
  const wasActiveRouteConversation = activeRouteConversationId.value === conversationId
  const targetConversationId = workbench.removeConversation(conversationId)
  const remainingConversationIds = workbench.conversations
    .filter((item) => item.projectId === projectId.value)
    .map((item) => item.id)

  if (!remainingConversationIds.length) {
    await router.replace({
      name: 'project-conversations',
      params: {
        workspaceId: workspaceId.value,
        projectId: projectId.value,
      },
    })
    return
  }

  if (!wasActiveRouteConversation && remainingConversationIds.includes(activeRouteConversationId.value)) {
    return
  }

  const nextConversationId = targetConversationId && remainingConversationIds.includes(targetConversationId)
    ? targetConversationId
    : remainingConversationIds[0]

  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, nextConversationId))
}
</script>

<template>
  <section class="mb-2" data-testid="conversation-tabs">
    <UiPanelFrame
      data-testid="conversation-tabs-panel"
      variant="subtle"
      padding="none"
      class="px-2 py-1.5"
    >
      <div class="flex items-center justify-between gap-2">
        <div class="flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto">
          <div
            v-for="conversation in conversations"
            :key="conversation.id"
            class="flex shrink-0 items-center gap-0.5"
          >
            <UiButton
              variant="ghost"
              class="flex h-8 min-w-0 max-w-[14rem] items-center gap-2 rounded-lg border border-transparent px-2.5 text-text-secondary hover:border-border/60 hover:bg-background/60 hover:text-text-primary"
              :class="conversation.id === workbench.currentConversationId ? 'active border-primary/20 bg-primary/[0.06] text-text-primary shadow-xs' : ''"
              :data-testid="conversation.id === workbench.currentConversationId ? 'conversation-tab-active' : `conversation-tab-${conversation.id}`"
              :aria-current="conversation.id === workbench.currentConversationId ? 'page' : undefined"
              @click="openConversation(conversation.id)"
            >
              <MessageSquareText :size="12" class="shrink-0" />
              <span class="truncate text-xs font-medium">
                {{ workbench.conversationDisplayTitle(conversation.id) }}
              </span>
            </UiButton>

            <UiButton
              variant="ghost"
              size="icon"
              class="size-7 shrink-0 text-text-tertiary hover:bg-muted/80 hover:text-text-primary"
              :data-testid="`conversation-tab-delete-${conversation.id}`"
              :title="t('conversation.tabs.delete')"
              @click.stop="removeConversation(conversation.id)"
            >
              <X :size="10" />
            </UiButton>
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

        <div class="flex shrink-0 items-center border-l border-border/40 pl-2">
          <UiButton
            variant="ghost"
            size="icon"
            class="h-8 w-8 text-text-tertiary hover:bg-muted/80 hover:text-text-primary"
            :class="{ 'text-primary': !shell.rightSidebarCollapsed }"
            data-testid="toggle-conversation-details"
            :title="t('conversation.tabs.toggleDetails')"
            @click="shell.toggleRightSidebar()"
          >
            <PanelRight :size="16" />
          </UiButton>
        </div>
      </div>

      <div
        data-testid="conversation-tabs-divider"
        class="h-px w-full bg-border/70"
        aria-hidden="true"
      />
    </UiPanelFrame>
  </section>
</template>
