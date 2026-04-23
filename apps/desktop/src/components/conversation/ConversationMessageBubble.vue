<script setup lang="ts">
import { computed } from 'vue'
import {
  AlertTriangle,
  Bot,
  Brain,
  ChevronDown,
  ChevronRight,
  FileText,
  FolderOpen,
  Paperclip,
  RotateCcw,
  Users,
  Wrench,
} from 'lucide-vue-next'

import { UiBadge, UiButton, UiStatusCallout } from '@octopus/ui'
import type { ConversationAttachment, Message, MessageProcessEntry, WorkspaceResourceRecord } from '@octopus/schema'

const props = defineProps<{
  message: Message
  senderLabel: string
  avatarLabel: string
  avatarSrc?: string
  actorLabel: string
  permissionLabel: string
  resources: WorkspaceResourceRecord[]
  attachments: ConversationAttachment[]
  artifacts: Array<{ id: string, label: string, kindLabel?: string, version?: number }>
  isExpanded: boolean
  focusedToolId?: string
  approvalResolving?: boolean
}>()

const emit = defineEmits<{
  (event: 'toggle-detail', messageId: string): void
  (event: 'rollback', messageId: string): void
  (event: 'open-resource', resourceId: string): void
  (event: 'open-artifact', payload: { id: string, version?: number }): void
  (event: 'approve', approvalId: string): void
  (event: 'reject', approvalId: string): void
  (event: 'focus-tool', payload: { messageId: string, toolId: string }): void
}>()

const isUserMessage = computed(() => props.message.senderType === 'user')
const detailEntries = computed<MessageProcessEntry[]>(() => props.message.processEntries ?? [])
const toolCalls = computed(() => props.message.toolCalls ?? [])
const hasProcessPanel = computed(() => detailEntries.value.length > 0 || toolCalls.value.length > 0)
const hasFocusedToolEntry = computed(() => detailEntries.value.some(entry => entry.toolId === props.focusedToolId))
const showProcessPanel = computed(() => !isUserMessage.value && hasProcessPanel.value)
const hasPendingApproval = computed(() => props.message.approval?.status !== 'approved' && props.message.approval?.status !== 'rejected')
const approvalRiskLabel = computed(() => props.message.approval?.riskLevel ?? '')
const isWaitingApproval = computed(() => props.message.status === 'waiting_approval')
const isWaitingInput = computed(() => props.message.status === 'waiting_input')
const isRunning = computed(() => props.message.status === 'running')
const actorKindLabel = computed(() => {
  if (props.message.actorKind === 'team') {
    return 'Team'
  }
  if (props.message.actorKind === 'agent') {
    return 'Agent'
  }
  return ''
})
const actorKindIcon = computed(() => (props.message.actorKind === 'team' ? Users : Bot))

const latestProcessEntry = computed(() => detailEntries.value[detailEntries.value.length - 1])
const latestResultEntry = computed(() =>
  [...detailEntries.value].reverse().find(entry => entry.type === 'result')
  ?? latestProcessEntry.value,
)
const timestampLabel = computed(() =>
  new Date(props.message.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
)

const processSummary = computed(() => {
  if (isWaitingApproval.value) {
    return {
      label: 'Waiting for approval',
      detail: props.message.approval?.summary ?? latestResultEntry.value?.title ?? 'Execution paused for a decision.',
      icon: AlertTriangle,
    }
  }

  if (isWaitingInput.value) {
    return {
      label: 'Waiting for input',
      detail: latestResultEntry.value?.title ?? 'Assistant needs more input to continue.',
      icon: AlertTriangle,
    }
  }

  if (isRunning.value && toolCalls.value.length) {
    return {
      label: 'Using tools',
      detail: `${toolCalls.value.length} ${toolCalls.value.length === 1 ? 'tool' : 'tools'} active`,
      icon: Wrench,
    }
  }

  if (isRunning.value && detailEntries.value.some(entry => entry.type === 'thinking')) {
    return {
      label: 'Thinking',
      detail: latestProcessEntry.value?.detail ?? 'Preparing the assistant response.',
      icon: Brain,
    }
  }

  if (isRunning.value) {
    return {
      label: 'Processing',
      detail: latestProcessEntry.value?.title ?? 'Assistant is still working.',
      icon: Wrench,
    }
  }

  if (props.message.status === 'blocked' || props.message.status === 'paused') {
    return {
      label: 'Paused',
      detail: latestResultEntry.value?.detail ?? 'Execution is waiting before it can continue.',
      icon: AlertTriangle,
    }
  }

  if (props.message.status === 'failed' || props.message.status === 'terminated') {
    return {
      label: 'Stopped',
      detail: latestResultEntry.value?.detail ?? 'Execution ended before completing.',
      icon: AlertTriangle,
    }
  }

  return {
    label: 'Completed',
    detail: toolCalls.value.length
      ? `Used ${toolCalls.value.length} ${toolCalls.value.length === 1 ? 'tool' : 'tools'} in this response.`
      : (latestResultEntry.value?.title ?? 'Assistant finished this response.'),
    icon: Bot,
  }
})

const waitingInputTitle = computed(() => latestResultEntry.value?.title ?? 'Waiting for input')
const waitingInputDescription = computed(() =>
  latestResultEntry.value?.detail ?? 'Assistant needs authentication or additional input to continue.',
)

function formatToolCallTitle(label: string) {
  if (isWaitingApproval.value) {
    return `Paused on ${label}`
  }
  if (isWaitingInput.value) {
    return `Needs input for ${label}`
  }
  if (isRunning.value) {
    return `Using ${label}`
  }
  return `Used ${label}`
}

function formatToolCallMeta(count: number) {
  const countLabel = `Called ${count} ${count === 1 ? 'time' : 'times'}`

  if (isWaitingApproval.value) {
    return `${countLabel} · Waiting for approval`
  }
  if (isWaitingInput.value) {
    return `${countLabel} · Waiting for input`
  }
  if (isRunning.value) {
    return `${countLabel} · In progress`
  }
  return countLabel
}
</script>

<template>
  <div
    class="mb-8 flex w-full"
    :class="isUserMessage ? 'justify-end' : 'justify-start'"
  >
    <article
      :data-testid="`conversation-message-bubble-${message.id}`"
      :data-message-id="message.id"
      class="group relative flex max-w-[90%] gap-3 rounded-[var(--radius-xl)] border px-4 py-3 transition-colors"
      :class="[
        isUserMessage
          ? 'flex-row-reverse border-border bg-surface'
          : 'flex-row border-border bg-[color-mix(in_srgb,var(--bg-surface)_94%,transparent)]'
      ]"
    >
      <!-- Avatar Column -->
      <div class="flex shrink-0 flex-col items-center pt-1">
        <div
          class="flex h-8 w-8 items-center justify-center overflow-hidden rounded-[var(--radius-m)] border border-border bg-subtle text-micro font-bold text-text-secondary"
          :class="props.avatarSrc ? 'bg-transparent p-0' : (isUserMessage ? 'bg-accent text-primary' : '')"
        >
          <img
            v-if="props.avatarSrc"
            :src="props.avatarSrc"
            :alt="senderLabel"
            class="h-full w-full object-cover"
            data-testid="conversation-avatar-image"
          >
          <span v-else>{{ avatarLabel }}</span>
        </div>
      </div>

      <!-- Content Column -->
      <div class="flex min-w-0 flex-1 flex-col gap-2">
          <!-- Sender & Meta Info -->
          <div class="flex min-h-6 items-center gap-3" :class="isUserMessage ? 'flex-row-reverse' : ''">
            <span class="text-label font-semibold text-text-primary">{{ isUserMessage ? 'You' : senderLabel }}</span>
            <span :data-testid="`conversation-message-timestamp-${message.id}`" class="text-micro font-medium tabular-nums text-text-tertiary opacity-60">
              {{ timestampLabel }}
            </span>

            <div v-if="!isUserMessage && actorLabel" class="flex min-w-0 items-center gap-2 text-micro font-semibold text-text-secondary">
              <UiBadge v-if="actorKindLabel" :label="actorKindLabel" tone="info" />
              <span class="flex min-w-0 items-center gap-1">
                <component :is="actorKindIcon" :size="11" class="shrink-0 text-text-tertiary" />
                <span class="max-w-[180px] truncate">{{ actorLabel }}</span>
              </span>
            </div>

            <div class="flex-1" />

            <!-- Actions (Only visible on hover) -->
            <div class="flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
              <UiButton v-if="isUserMessage" variant="ghost" size="icon" class="h-6 w-6 rounded-md" @click="emit('rollback', message.id)">
                <RotateCcw :size="12" />
              </UiButton>
            </div>
          </div>

          <!-- AI Thinking/Process Toggle (Only for AI) -->
          <div v-if="showProcessPanel" class="mt-1">
            <button
              type="button"
              class="flex items-center gap-2 rounded-[var(--radius-s)] px-2 py-1 text-text-tertiary transition-colors hover:bg-subtle hover:text-text-secondary"
              data-testid="conversation-process-toggle"
              @click="emit('toggle-detail', message.id)"
            >
              <component :is="isExpanded ? ChevronDown : ChevronRight" :size="14" class="mt-0.5 shrink-0" />
              <div class="flex min-w-0 flex-col items-start gap-0.5 text-left">
                <div class="flex min-w-0 items-center gap-2 text-caption font-semibold text-text-secondary">
                  <component :is="processSummary.icon" :size="12" class="shrink-0" />
                  <span class="truncate">
                    {{ processSummary.label }}<span v-if="isRunning">...</span>
                  </span>
                </div>
                <span class="truncate text-micro font-medium text-text-tertiary">
                  {{ processSummary.detail }}
                </span>
              </div>
            </button>

            <div v-if="isExpanded" class="ml-2 mt-2 space-y-3 border-l border-border pl-4 py-1 animate-in fade-in slide-in-from-top-1 duration-200">
              <div
                v-for="entry in detailEntries"
                :key="entry.id"
                class="space-y-1.5 rounded-[var(--radius-m)] border border-transparent px-3 py-2 transition-colors"
                :class="entry.toolId && entry.toolId === focusedToolId ? 'border-border bg-accent' : 'bg-subtle/60'"
                :data-testid="entry.toolId && entry.toolId === focusedToolId ? 'conversation-focused-tool-entry' : undefined"
              >
                <div class="flex items-center gap-2 text-caption font-semibold text-text-secondary">
                  <div class="h-1.5 w-1.5 rounded-full bg-border-strong"></div>
                  {{ entry.title }}
                </div>
                <p class="pl-3.5 text-caption text-text-tertiary">{{ entry.detail }}</p>
              </div>
            </div>
          </div>

          <!-- Message Body -->
          <div
            class="whitespace-pre-wrap break-words text-[15px] leading-[1.6] text-text-primary"
            :class="isUserMessage ? 'text-right' : 'text-left'"
          >
            {{ message.content }}
          </div>

        <div
          v-if="!isUserMessage && toolCalls.length"
          class="flex flex-col gap-2"
          data-testid="conversation-inline-tool-calls"
        >
          <button
            v-for="toolCall in toolCalls"
            :key="toolCall.toolId"
            type="button"
            class="flex items-center gap-2 rounded-[var(--radius-l)] border border-border bg-surface px-3 py-2 text-left text-caption text-text-secondary transition-colors hover:bg-subtle"
            :data-testid="`conversation-inline-tool-${toolCall.toolId}`"
            @click="emit('focus-tool', { messageId: message.id, toolId: toolCall.toolId })"
          >
            <div class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-subtle text-text-secondary">
              <Wrench :size="12" class="shrink-0" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="truncate font-semibold text-text-primary">
                {{ formatToolCallTitle(toolCall.label) }}
              </div>
              <div class="text-micro tabular-nums text-text-tertiary">
                {{ formatToolCallMeta(toolCall.count) }}
              </div>
            </div>
          </button>
        </div>

        <UiStatusCallout
          v-if="!isUserMessage && isWaitingInput"
          class="gap-3"
          tone="info"
          :title="waitingInputTitle"
          :description="waitingInputDescription"
          data-testid="conversation-inline-input-wait"
        >
          <div class="flex flex-wrap items-center gap-2 text-micro font-semibold text-status-info">
            <AlertTriangle :size="13" class="shrink-0" />
            <span>Assistant needs more input before this run can continue.</span>
          </div>
        </UiStatusCallout>

        <UiStatusCallout
          v-if="!isUserMessage && message.approval"
          class="gap-3"
          tone="warning"
          :title="message.approval.summary"
          :description="message.approval.detail"
          data-testid="conversation-inline-approval"
        >
          <div class="flex flex-wrap items-center gap-2 text-micro font-semibold">
            <span class="inline-flex items-center gap-1.5 text-status-warning">
              <AlertTriangle :size="13" class="shrink-0" />
              <span>{{ message.approval.toolName }}</span>
            </span>
            <UiBadge v-if="approvalRiskLabel" :label="approvalRiskLabel" subtle />
          </div>
          <div v-if="hasPendingApproval" class="flex flex-wrap gap-2">
            <UiButton
              size="sm"
              data-testid="conversation-inline-approve"
              :disabled="approvalResolving"
              @click="message.approval && emit('approve', message.approval.id)"
            >
              Approve
            </UiButton>
            <UiButton
              size="sm"
              variant="ghost"
              data-testid="conversation-inline-reject"
              :disabled="approvalResolving"
              @click="message.approval && emit('reject', message.approval.id)"
            >
              Reject
            </UiButton>
          </div>
        </UiStatusCallout>

        <!-- Assets / Resources -->
        <div
          v-if="resources.length || attachments.length || artifacts.length"
          class="flex flex-wrap gap-2 pt-2"
          :class="isUserMessage ? 'justify-end' : 'justify-start'"
        >
          <button
            v-for="resource in resources"
            :key="resource.id"
            class="flex items-center gap-2 rounded-[var(--radius-m)] border border-border bg-surface px-2.5 py-1.5 text-[12px] font-medium transition-colors hover:bg-subtle"
            @click="emit('open-resource', resource.id)"
          >
            <FolderOpen v-if="resource.kind === 'folder'" :size="13" class="text-text-tertiary" />
            <FileText v-else-if="resource.kind === 'artifact'" :size="13" class="text-text-tertiary" />
            <Paperclip v-else :size="13" class="text-text-tertiary" />
            <span>{{ resource.name }}</span>
          </button>

          <button
            v-for="artifact in artifacts"
            :key="artifact.id"
            class="flex items-center gap-2 rounded-[var(--radius-m)] border border-border bg-accent px-2.5 py-1.5 text-[12px] font-semibold text-primary transition-colors hover:bg-accent/80"
            @click="emit('open-artifact', { id: artifact.id, version: artifact.version })"
          >
            <FileText :size="13" />
            <span>{{ artifact.label }}</span>
            <UiBadge v-if="artifact.kindLabel" :label="artifact.kindLabel" subtle />
          </button>
        </div>

        <!-- Usage info -->
        <div v-if="!isUserMessage && message.usage" class="pt-2 text-micro font-medium tabular-nums text-text-tertiary opacity-40">
          {{ message.usage.totalTokens }} tokens · {{ permissionLabel }}
        </div>
      </div>
    </article>
  </div>
</template>
