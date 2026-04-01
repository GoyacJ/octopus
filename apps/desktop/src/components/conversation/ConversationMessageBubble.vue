<script setup lang="ts">
import { computed } from 'vue'
import {
  Brain,
  ChevronDown,
  ChevronUp,
  FileText,
  FolderOpen,
  Paperclip,
  RotateCcw,
  Wrench,
} from 'lucide-vue-next'

import type { ConversationAttachment, Message, MessageProcessEntry, ProjectResource } from '@octopus/schema'

const props = defineProps<{
  message: Message
  senderLabel: string
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

  return '查看本条消息的过程详情'
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
</script>

<template>
  <article
    class="message-row"
    :class="message.senderType"
    :data-testid="`conversation-message-bubble-${message.id}`"
  >
    <div class="message-bubble-shell">
      <div v-if="showProcessPanel" class="message-process-panel">
        <button
          type="button"
          class="process-summary"
          :class="{ expanded: isExpanded }"
          :data-testid="`conversation-message-process-summary-${message.id}`"
          @click="emit('toggle-detail', message.id)"
        >
          <div class="process-summary-leading">
            <Brain v-if="hasThinkingEntry" :size="14" />
            <Wrench v-else :size="14" />
            <strong>{{ processLabel }}</strong>
            <span>{{ processSummary }}</span>
          </div>
          <div class="process-summary-trailing">
            <small v-if="processMeta">{{ processMeta }}</small>
            <component :is="isExpanded ? ChevronUp : ChevronDown" :size="14" />
          </div>
        </button>

        <div v-if="isExpanded" class="message-detail">
          <div v-if="detailEntries.length" class="detail-list">
            <article
              v-for="entry in detailEntries"
              :key="entry.id"
              class="detail-entry"
            >
              <div class="detail-entry-title">
                <Brain v-if="entry.type === 'thinking'" :size="14" />
                <Wrench v-else-if="entry.type === 'tool'" :size="14" />
                <FileText v-else :size="14" />
                <strong>{{ entry.title }}</strong>
              </div>
              <p>{{ entry.detail }}</p>
            </article>
          </div>

          <div v-if="message.toolCalls?.length" class="tool-call-list">
            <div v-for="tool in message.toolCalls" :key="tool.toolId" class="tool-call-row">
              <span>{{ tool.label }}</span>
              <small>{{ tool.kind }} · {{ tool.count }} 次</small>
            </div>
          </div>
        </div>
      </div>

      <div class="message-bubble">
        <div class="message-header">
          <strong>{{ senderLabel }}</strong>
          <span v-if="message.usedDefaultActor" class="message-badge">默认智能体</span>
        </div>

        <div v-if="message.actorId || message.permissionMode" class="message-context-row">
          <span v-if="message.actorId" class="context-chip">{{ actorLabel }}</span>
          <span v-if="message.permissionMode" class="context-chip subtle">{{ permissionLabel }}</span>
        </div>

        <p class="message-body">{{ message.content }}</p>

        <div v-if="resources.length" class="attachment-row">
          <button
            v-for="resource in resources"
            :key="resource.id"
            type="button"
            class="attachment-pill"
            @click="emit('open-resource', resource.id)"
          >
            <FolderOpen v-if="resource.kind === 'folder'" :size="12" />
            <FileText v-else-if="resource.kind === 'artifact'" :size="12" />
            <Paperclip v-else :size="12" />
            <span>{{ resource.name }}</span>
          </button>
        </div>

        <div v-if="attachments.length || artifacts.length" class="attachment-row">
          <button
            v-for="attachment in attachments"
            :key="attachment.id"
            type="button"
            class="attachment-pill"
            @click="emit('open-resource', attachment.id)"
          >
            <Paperclip :size="12" />
            <span>{{ attachment.name }}</span>
          </button>
          <button
            v-for="artifact in artifacts"
            :key="artifact.id"
            type="button"
            class="attachment-pill artifact-pill"
            @click="emit('open-artifact', artifact.id)"
          >
            <FileText :size="12" />
            <span>{{ artifact.label }}</span>
          </button>
        </div>

        <div class="message-footer">
          <span>{{ new Date(message.timestamp).toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) }}</span>
          <template v-if="showUsageMeta">
            <span>{{ message.usage?.totalTokens ?? 0 }} tokens</span>
            <span>{{ totalToolCalls }} 次工具调用</span>
          </template>
        </div>
      </div>

      <div v-if="canRollback" class="message-actions">
        <button
          type="button"
          class="detail-toggle rollback"
          :data-testid="`conversation-message-rollback-${message.id}`"
          @click="emit('rollback', message.id)"
        >
          <RotateCcw :size="14" />
          <span>回滚到此</span>
        </button>
      </div>
    </div>
  </article>
</template>

<style scoped>
.message-row {
  display: flex;
  width: 100%;
}

.message-row.user {
  justify-content: flex-end;
}

.message-row.agent,
.message-row.system {
  justify-content: flex-start;
}

.message-bubble-shell {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  width: min(100%, 44rem);
}

.message-bubble {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  padding: 0.95rem 1rem 0.8rem;
  border-radius: 1.35rem;
  border: 0;
  background: color-mix(in srgb, var(--bg-surface) 92%, var(--bg-subtle));
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.03),
    0 10px 28px rgb(15 23 42 / 0.08);
}

.message-header,
.message-context-row,
.attachment-row,
.message-actions,
.message-footer,
.process-summary,
.process-summary-leading,
.process-summary-trailing,
.detail-entry-title,
.tool-call-row {
  display: flex;
  align-items: center;
}

.message-header,
.tool-call-row {
  justify-content: space-between;
}

.message-header,
.message-context-row,
.attachment-row,
.message-actions,
.message-footer,
.message-process-panel,
.message-detail,
.detail-list {
  gap: 0.45rem;
}

.message-header,
.message-context-row,
.attachment-row,
.message-actions {
  flex-wrap: wrap;
}

.message-badge,
.context-chip,
.attachment-pill,
.detail-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-radius: 999px;
}

.message-badge,
.context-chip {
  padding: 0.18rem 0.5rem;
  font-size: 0.74rem;
  border: 0;
  background: color-mix(in srgb, var(--bg-subtle) 82%, transparent);
}

.context-chip.subtle {
  color: var(--text-secondary);
}

.message-body,
.detail-entry p {
  margin: 0;
  line-height: 1.65;
  color: var(--text-primary);
  overflow-wrap: anywhere;
}

.attachment-pill,
.detail-toggle {
  padding: 0.34rem 0.68rem;
  border: 0;
  background: color-mix(in srgb, var(--bg-subtle) 82%, transparent);
  color: var(--text-secondary);
}

.artifact-pill {
  color: var(--text-primary);
}

.detail-toggle.rollback {
  color: var(--text-primary);
}

.message-actions {
  padding-inline: 0.2rem;
}

.message-row.agent .message-actions,
.message-row.system .message-actions {
  justify-content: flex-start;
}

.message-row.user .message-footer {
  width: 100%;
}

.message-footer {
  align-self: flex-end;
  justify-content: flex-end;
  flex-wrap: wrap;
  color: var(--text-muted);
  font-size: 0.76rem;
}

.message-row.user .message-actions {
  justify-content: flex-end;
}

.message-process-panel,
.message-detail,
.detail-list {
  display: flex;
  flex-direction: column;
}

.message-process-panel {
  gap: 0.35rem;
  padding-inline: 0.15rem;
}

.process-summary {
  justify-content: space-between;
  gap: 0.55rem;
  width: 100%;
  padding: 0.1rem 0;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
}

.process-summary.expanded {
  color: var(--text-primary);
}

.process-summary-leading {
  min-width: 0;
}

.process-summary-leading {
  gap: 0.42rem;
  flex: 1;
  color: var(--text-secondary);
  flex-wrap: wrap;
}

.process-summary-leading strong,
.process-summary-leading span,
.process-summary-trailing small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.process-summary-leading strong {
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.process-summary-leading span {
  font-size: 0.82rem;
  color: var(--text-primary);
}

.process-summary-trailing {
  gap: 0.4rem;
  color: var(--text-muted);
  flex-shrink: 0;
}

.message-detail,
.detail-list {
  gap: 0.35rem;
}

.detail-entry,
.tool-call-row {
  padding: 0;
  border: 0;
  background: transparent;
}

.detail-entry {
  display: flex;
  flex-direction: column;
  gap: 0.18rem;
}

.detail-entry-title {
  gap: 0.35rem;
  color: var(--text-secondary);
}

.tool-call-list {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.tool-call-row small,
.message-footer {
  color: var(--text-muted);
  font-size: 0.76rem;
}

@media (max-width: 720px) {
  .process-summary,
  .process-summary-leading,
  .process-summary-trailing {
    align-items: flex-start;
  }

  .process-summary {
    flex-direction: column;
  }

  .process-summary-trailing {
    width: 100%;
    justify-content: space-between;
  }
}
</style>
