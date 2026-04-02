<script setup lang="ts">
import { computed } from 'vue'
import {
  Brain,
  ChevronDown,
  ChevronUp,
  Clock,
  FileText,
  FolderOpen,
  Paperclip,
  RotateCcw,
  Wrench,
} from 'lucide-vue-next'
import { UiBadge, UiButton } from '@octopus/ui'
import type { ConversationAttachment, Message, MessageProcessEntry, ProjectResource } from '@octopus/schema'

const props = defineProps<{
  message: Message
  senderLabel: string
  avatarLabel: string
  actorLabel: string
  permissionLabel: string
  resources: ProjectResource[]
  attachments: ConversationAttachment[]
  artifacts: Array<{ id: string, label: string }>
  isExpanded: boolean
}>()

const emit = defineEmits<{
  (event: 'toggle-detail', messageId: string): void
  (event: 'rollback', messageId: string): void
  (event: 'open-resource', resourceId: string): void
  (event: 'open-artifact', artifactId: string): void
}>()

const totalToolCalls = computed(() =>
  (props.message.toolCalls ?? []).reduce((total, tool) => total + tool.count, 0),
)
const isUserMessage = computed(() => props.message.senderType === 'user')
const detailEntries = computed<MessageProcessEntry[]>(() => props.message.processEntries ?? [])
const primaryProcessEntry = computed(() => detailEntries.value[0] ?? null)
const hasThinkingEntry = computed(() => detailEntries.value.some((entry) => entry.type === 'thinking'))
const hasProcessPanel = computed(() => detailEntries.value.length > 0 || totalToolCalls.value > 0)
const showProcessPanel = computed(() => !isUserMessage.value && hasProcessPanel.value)
const showUsageMeta = computed(() => !isUserMessage.value)
const canRollback = computed(() => isUserMessage.value)
const processLabel = computed(() => (hasThinkingEntry.value ? '思考过程' : '执行过程'))
const processSummary = computed(() => {
  if (primaryProcessEntry.value?.title) {
    return primaryProcessEntry.value.title
  }
  if (totalToolCalls.value > 0) {
    return `已执行 ${totalToolCalls.value} 次工具调用`
  }
  return '查看过程详情'
})
const processMeta = computed(() => {
  const parts: string[] = []
  if (detailEntries.value.length > 0) {
    parts.push(`${detailEntries.value.length} 个步骤`)
  }
  if (totalToolCalls.value > 0) {
    parts.push(`${totalToolCalls.value} 次工具`)
  }
  return parts.join(' · ')
})
const senderToneClass = computed(() =>
  isUserMessage.value ? 'message-bubble-user' : 'message-bubble-agent',
)
</script>

<template>
  <article
    class="message-row"
    :class="isUserMessage ? 'message-row-user' : 'message-row-agent'"
    :data-testid="`conversation-message-bubble-${message.id}`"
  >
    <div class="message-avatar" :class="isUserMessage ? 'message-avatar-user' : 'message-avatar-agent'">
      <span>{{ avatarLabel }}</span>
    </div>
    <div class="message-stack">
      <div v-if="showProcessPanel" class="message-process-shell">
        <button
          type="button"
          class="message-process-toggle"
          :class="{ expanded: isExpanded }"
          :data-testid="`conversation-message-process-summary-${message.id}`"
          @click="emit('toggle-detail', message.id)"
        >
          <div class="message-process-copy">
            <span class="message-process-icon">
              <Brain v-if="hasThinkingEntry" :size="14" />
              <Wrench v-else :size="14" />
            </span>
            <div class="message-process-text">
              <strong>{{ processLabel }}</strong>
              <span>{{ processSummary }}</span>
            </div>
          </div>
          <div class="message-process-meta">
            <small v-if="processMeta">{{ processMeta }}</small>
            <component :is="isExpanded ? ChevronUp : ChevronDown" :size="14" />
          </div>
        </button>

        <div v-if="isExpanded" class="message-process-panel">
          <div v-if="detailEntries.length" class="message-process-list">
            <article
              v-for="entry in detailEntries"
              :key="entry.id"
              class="message-process-entry"
            >
              <div class="message-process-entry-title">
                <Brain v-if="entry.type === 'thinking'" :size="14" />
                <Wrench v-else-if="entry.type === 'tool'" :size="14" />
                <FileText v-else :size="14" />
                <strong>{{ entry.title }}</strong>
              </div>
              <p>{{ entry.detail }}</p>
            </article>
          </div>

          <div v-if="message.toolCalls?.length" class="message-tool-list">
            <div v-for="tool in message.toolCalls" :key="tool.toolId" class="message-tool-row">
              <span>{{ tool.label }}</span>
              <span>{{ tool.kind }} · {{ tool.count }} 次</span>
            </div>
          </div>
        </div>
      </div>

      <div class="message-bubble" :class="senderToneClass">
        <div class="message-header">
          <div class="message-header-main">
            <strong class="message-sender">{{ senderLabel }}</strong>
            <UiBadge v-if="message.usedDefaultActor" label="默认" tone="info" class="message-default-badge" />
          </div>
          <div class="message-timestamp">
            <Clock :size="10" />
            <span>{{ new Date(message.timestamp).toLocaleString('zh-CN', { hour: '2-digit', minute: '2-digit' }) }}</span>
          </div>
        </div>

        <div v-if="message.actorId || message.permissionMode" class="message-context-meta">
          <span v-if="message.actorId" class="message-context-chip message-context-chip-primary">
            {{ actorLabel }}
          </span>
          <span v-if="message.permissionMode" class="message-context-chip">
            {{ permissionLabel }}
          </span>
        </div>

        <p class="message-content">{{ message.content }}</p>

        <div v-if="resources.length || attachments.length || artifacts.length" class="message-asset-list">
          <button
            v-for="resource in resources"
            :key="resource.id"
            type="button"
            class="message-asset-chip"
            @click="emit('open-resource', resource.id)"
          >
            <FolderOpen v-if="resource.kind === 'folder'" :size="14" />
            <FileText v-else-if="resource.kind === 'artifact'" :size="14" />
            <Paperclip v-else :size="14" />
            <span>{{ resource.name }}</span>
          </button>

          <button
            v-for="attachment in attachments"
            :key="attachment.id"
            type="button"
            class="message-asset-chip"
            @click="emit('open-resource', attachment.id)"
          >
            <Paperclip :size="14" />
            <span>{{ attachment.name }}</span>
          </button>

          <button
            v-for="artifact in artifacts"
            :key="artifact.id"
            type="button"
            class="message-asset-chip message-asset-chip-primary"
            @click="emit('open-artifact', artifact.id)"
          >
            <FileText :size="14" />
            <span>{{ artifact.label }}</span>
          </button>
        </div>

        <div class="message-footer" data-testid="conversation-message-footer">
          <template v-if="showUsageMeta">
            <span>{{ message.usage?.totalTokens ?? 0 }} tokens</span>
            <span>{{ totalToolCalls }} 次工具调用</span>
          </template>
        </div>
      </div>

      <div v-if="canRollback" class="message-actions">
        <UiButton
          variant="ghost"
          size="sm"
          class="message-rollback"
          :data-testid="`conversation-message-rollback-${message.id}`"
          @click="emit('rollback', message.id)"
        >
          <RotateCcw :size="12" />
          <span>回滚到此</span>
        </UiButton>
      </div>
    </div>
  </article>
</template>

<style scoped>
.message-row {
  display: flex;
  align-items: flex-end;
  gap: 0.45rem;
  width: 100%;
  margin-bottom: 0.7rem;
}

.message-row-user {
  justify-content: flex-end;
}

.message-row-agent {
  justify-content: flex-start;
}

.message-stack {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  width: min(100%, 100%);
  max-width: 42rem;
}

.message-row-user .message-stack {
  align-items: flex-end;
}

.message-avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-subtle) 88%, transparent);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  color: var(--text-primary);
  font-size: 0.68rem;
  font-weight: 700;
  box-shadow: var(--shadow-xs);
  flex-shrink: 0;
}

.message-avatar-user {
  order: 2;
  background: color-mix(in srgb, var(--brand-primary) 14%, transparent);
  color: var(--brand-primary);
}

.message-avatar-agent {
  order: 0;
}

.message-process-shell,
.message-process-panel,
.message-process-list,
.message-process-entry,
.message-bubble,
.message-header-main,
.message-context-meta,
.message-asset-list,
.message-footer,
.message-actions {
  display: flex;
}

.message-process-shell,
.message-process-panel,
.message-process-list,
.message-process-entry,
.message-bubble {
  flex-direction: column;
}

.message-process-toggle,
.message-process-copy,
.message-process-meta,
.message-header,
.message-timestamp,
.message-asset-chip,
.message-tool-row {
  display: flex;
  align-items: center;
}

.message-process-toggle,
.message-header,
.message-tool-row {
  justify-content: space-between;
}

.message-process-toggle {
  gap: 0.65rem;
  width: 100%;
  padding: 0.58rem 0.72rem;
  border-radius: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 90%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 76%, transparent);
  color: var(--text-secondary);
  text-align: left;
  transition: border-color var(--duration-fast) var(--ease-apple), background-color var(--duration-fast) var(--ease-apple);
}

.message-process-toggle:hover,
.message-process-toggle.expanded {
  border-color: color-mix(in srgb, var(--brand-primary) 18%, var(--border-strong));
  background: color-mix(in srgb, var(--brand-primary) 7%, var(--bg-surface));
}

.message-process-copy {
  gap: 0.7rem;
  min-width: 0;
}

.message-process-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.8rem;
  height: 1.8rem;
  border-radius: 0.7rem;
  background: color-mix(in srgb, var(--bg-surface) 88%, transparent);
  color: var(--text-secondary);
  flex-shrink: 0;
}

.message-process-text {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
  font-size: 0.77rem;
}

.message-process-text strong,
.message-process-entry strong {
  color: var(--text-primary);
}

.message-process-text span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.message-process-meta {
  gap: 0.45rem;
  flex-shrink: 0;
  color: var(--text-tertiary);
  font-size: 0.68rem;
}

.message-process-panel {
  gap: 0.7rem;
  padding: 0.7rem 0.8rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 90%, transparent);
  border-top: 0;
  border-radius: 0 0 1rem 1rem;
  background: color-mix(in srgb, var(--bg-subtle) 68%, transparent);
}

.message-process-list {
  gap: 0.85rem;
}

.message-process-entry {
  gap: 0.3rem;
}

.message-process-entry-title {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  color: var(--text-secondary);
  font-size: 0.82rem;
}

.message-process-entry p,
.message-tool-row {
  color: var(--text-secondary);
  font-size: 0.78rem;
  line-height: 1.65;
}

.message-process-entry p {
  padding-left: 1.2rem;
}

.message-tool-list {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding-top: 0.75rem;
  border-top: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
}

.message-bubble {
  gap: 0.65rem;
  padding: 0.78rem 0.88rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: 1.15rem;
  box-shadow: var(--shadow-xs);
}

.message-bubble-agent {
  background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
}

.message-bubble-user {
  background: linear-gradient(180deg, color-mix(in srgb, var(--brand-primary) 90%, #fff 10%), var(--brand-primary));
  border-color: transparent;
  color: var(--text-on-brand);
  box-shadow: var(--shadow-sm);
}

.message-header {
  gap: 1rem;
}

.message-header-main {
  gap: 0.5rem;
  min-width: 0;
}

.message-sender {
  font-size: 0.74rem;
  line-height: 1.2;
}

.message-default-badge {
  transform: scale(0.84);
  transform-origin: left center;
}

.message-timestamp {
  gap: 0.25rem;
  color: color-mix(in srgb, currentColor 72%, transparent);
  font-size: 0.6rem;
  flex-shrink: 0;
}

.message-context-meta,
.message-asset-list,
.message-footer {
  flex-wrap: wrap;
}

.message-context-meta {
  gap: 0.45rem;
}

.message-context-chip,
.message-asset-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.38rem;
  border-radius: 999px;
  font-size: 0.7rem;
}

.message-context-chip {
  padding: 0.28rem 0.58rem;
  background: color-mix(in srgb, currentColor 10%, transparent);
  color: inherit;
}

.message-context-chip-primary {
  background: color-mix(in srgb, currentColor 16%, transparent);
}

.message-content {
  font-size: 0.84rem;
  line-height: 1.55;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.message-asset-list {
  gap: 0.45rem;
}

.message-asset-chip {
  padding: 0.42rem 0.7rem;
  border: 1px solid color-mix(in srgb, currentColor 12%, transparent);
  background: color-mix(in srgb, currentColor 6%, transparent);
}

.message-asset-chip-primary {
  border-color: color-mix(in srgb, var(--brand-primary) 24%, transparent);
}

.message-footer {
  gap: 0.6rem;
  padding-top: 0.55rem;
  border-top: 1px solid color-mix(in srgb, currentColor 10%, transparent);
  color: color-mix(in srgb, currentColor 72%, transparent);
  font-size: 0.68rem;
}

.message-actions {
  justify-content: flex-end;
}

.message-rollback {
  min-height: 1.65rem;
  border-radius: 999px;
}

@media (max-width: 820px) {
  .message-stack {
    width: min(100%, 100%);
  }

  .message-process-text {
    flex-direction: column;
    align-items: flex-start;
  }

  .message-avatar {
    width: 1.7rem;
    height: 1.7rem;
    font-size: 0.62rem;
  }

  .message-bubble {
    padding: 0.72rem 0.8rem;
    border-radius: 1rem;
  }
}
</style>
