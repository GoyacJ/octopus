<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { MessageSquareText, Plus, X } from 'lucide-vue-next'

import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workbench = useWorkbenchStore()

const conversations = computed(() => workbench.projectConversations)
const workspaceId = computed(() =>
  typeof route.params.workspaceId === 'string' ? route.params.workspaceId : workbench.currentWorkspaceId,
)
const projectId = computed(() =>
  typeof route.params.projectId === 'string' ? route.params.projectId : workbench.currentProjectId,
)

async function createConversation() {
  const conversation = workbench.createConversation(projectId.value)
  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, conversation.id))
}

async function openConversation(conversationId: string) {
  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, conversationId))
}

async function removeConversation(conversationId: string) {
  const targetConversationId = workbench.removeConversation(conversationId)
  const nextConversationId = targetConversationId && workbench.projectConversations.some((item) => item.id === targetConversationId)
    ? targetConversationId
    : (workbench.projectConversations[0]?.id ?? null)

  const target = createProjectConversationTarget(workspaceId.value, projectId.value, nextConversationId)
  if (nextConversationId) {
    await router.push(target)
    return
  }

  await router.replace(target)
}
</script>

<template>
  <section class="conversation-tabs-shell" data-testid="conversation-tabs">
    <div class="conversation-tabs-track">
      <div
        v-for="conversation in conversations"
        :key="conversation.id"
        class="conversation-tab"
        :class="{ active: conversation.id === workbench.currentConversationId }"
      >
        <button
          type="button"
          class="conversation-tab-trigger"
          :data-testid="conversation.id === workbench.currentConversationId ? 'conversation-tab-active' : `conversation-tab-${conversation.id}`"
          @click="openConversation(conversation.id)"
        >
          <MessageSquareText :size="13" />
          <span class="conversation-tab-title">
            {{ resolveMockField('conversation', conversation.id, 'title', conversation.title) }}
          </span>
        </button>
        <button
          type="button"
          class="conversation-tab-delete"
          :data-testid="`conversation-tab-delete-${conversation.id}`"
          :title="t('conversation.tabs.delete')"
          @click.stop="removeConversation(conversation.id)"
        >
          <X :size="12" />
        </button>
      </div>

      <button
        type="button"
        class="conversation-tab-create"
        data-testid="conversation-tab-create"
        :title="t('conversation.tabs.create')"
        @click="createConversation"
      >
        <Plus :size="14" />
      </button>
    </div>
    <div class="conversation-tabs-divider" data-testid="conversation-tabs-divider" aria-hidden="true" />
  </section>
</template>

<style scoped>
.conversation-tabs-shell {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.conversation-tabs-track {
  display: flex;
  align-items: center;
  gap: 0.15rem;
  overflow-x: auto;
  min-height: 2.5rem;
  padding: 0 0.1rem 0.15rem;
}

.conversation-tab,
.conversation-tab-trigger,
.conversation-tab-create,
.conversation-tab-delete {
  display: inline-flex;
  align-items: center;
}

.conversation-tab {
  flex: 0 0 auto;
  position: relative;
  gap: 0.05rem;
  min-width: 0;
  min-height: 2.35rem;
  border-bottom: 2px solid transparent;
}

.conversation-tab.active {
  border-bottom-color: var(--brand-primary);
}

.conversation-tab-trigger {
  gap: 0.4rem;
  min-width: 0;
  min-height: 2.35rem;
  padding: 0 0.55rem 0 0.45rem;
  border-radius: 0.75rem;
  color: var(--text-secondary);
}

.conversation-tab.active .conversation-tab-trigger {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--brand-primary) 7%, transparent);
}

.conversation-tab-title {
  overflow: hidden;
  max-width: 14rem;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: 0.88rem;
  font-weight: 600;
}

.conversation-tab-delete,
.conversation-tab-create {
  justify-content: center;
  min-width: 2rem;
  height: 2rem;
  border-radius: 0.75rem;
  border: 1px solid transparent;
  color: var(--text-muted);
}

.conversation-tab-delete:hover,
.conversation-tab-create:hover {
  border-color: color-mix(in srgb, var(--border-subtle) 90%, transparent);
  color: var(--text-primary);
  background: color-mix(in srgb, var(--bg-subtle) 84%, transparent);
}

.conversation-tab-delete {
  align-self: center;
}

.conversation-tab-create {
  flex: 0 0 auto;
  min-width: 2.2rem;
  color: var(--text-secondary);
}

.conversation-tabs-divider {
  width: 100%;
  height: 1px;
  background: color-mix(in srgb, var(--border-subtle) 92%, transparent);
}
</style>
