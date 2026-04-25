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

import { UiBadge, UiButton, UiStatusCallout, cn } from '@octopus/ui'
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
      class="group relative flex max-w-[85%] gap-4 rounded-[var(--radius-xl)] px-5 py-4 transition-all duration-normal"
      :class="[
        isUserMessage
          ? 'flex-row-reverse bg-surface/40 backdrop-blur-md border border-border/50 shadow-sm'
          : 'flex-row bg-sidebar/20 backdrop-blur-xl border border-primary/10 shadow-lg highlight-border'
      ]"
    >
      <!-- Background Glow for AI Running -->
      <div 
        v-if="isRunning && !isUserMessage" 
        class="absolute inset-0 rounded-[var(--radius-xl)] bg-primary/5 animate-pulse pointer-events-none"
      />

      <!-- Avatar Column -->
      <div class="flex shrink-0 flex-col items-center pt-1">
        <div
          class="flex h-10 w-10 items-center justify-center overflow-hidden rounded-xl border transition-all duration-500"
          :class="[
            props.avatarSrc ? 'bg-black/20 p-0 border-border/50' : (isUserMessage ? 'bg-accent/20 border-primary/30 text-primary font-bold' : 'bg-primary/5 border-primary/20 text-primary font-bold shadow-[0_0_10px_rgba(var(--color-primary-rgb),0.1)]'),
            isRunning && !isUserMessage ? 'scale-110 shadow-[0_0_15px_rgba(var(--color-primary-rgb),0.3)]' : ''
          ]"
        >
          <img
            v-if="props.avatarSrc"
            :src="props.avatarSrc"
            :alt="senderLabel"
            class="h-full w-full object-cover transition-transform group-hover:scale-110"
            data-testid="conversation-avatar-image"
          >
          <span v-else class="text-sm uppercase tracking-tighter">{{ avatarLabel }}</span>
        </div>
      </div>

      <!-- Content Column -->
      <div class="flex min-w-0 flex-1 flex-col gap-2.5">
          <!-- Sender & Meta Info -->
          <div class="flex min-h-6 items-center gap-3" :class="isUserMessage ? 'flex-row-reverse' : ''">
            <span class="text-[13px] font-bold tracking-tight text-text-primary uppercase opacity-90">{{ isUserMessage ? 'You' : senderLabel }}</span>
            <span :data-testid="`conversation-message-timestamp-${message.id}`" class="text-[10px] font-bold tabular-nums text-text-tertiary/60 tracking-widest uppercase">
              {{ timestampLabel }}
            </span>

            <div v-if="!isUserMessage && actorLabel" class="flex min-w-0 items-center gap-2">
              <UiBadge v-if="actorKindLabel" :label="actorKindLabel" class="bg-primary/10 text-primary border-primary/20 text-[9px] px-1.5 py-0" />
              <span class="flex min-w-0 items-center gap-1.5 opacity-60">
                <component :is="actorKindIcon" :size="12" class="shrink-0 text-text-tertiary" />
                <span class="max-w-[150px] truncate text-[11px] font-bold text-text-tertiary uppercase tracking-tighter">{{ actorLabel }}</span>
              </span>
            </div>

            <div class="flex-1" />

            <!-- Actions (Only visible on hover) -->
            <div class="flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
              <UiButton v-if="isUserMessage" variant="ghost" size="icon" class="h-7 w-7 rounded-lg bg-black/10 hover:bg-black/20" @click="emit('rollback', message.id)">
                <RotateCcw :size="14" class="text-text-tertiary" />
              </UiButton>
            </div>
          </div>

          <!-- AI Thinking/Process Toggle (Only for AI) -->
          <div v-if="showProcessPanel" class="mt-1">
            <button
              type="button"
              class="flex items-center gap-3 rounded-xl border border-border/30 bg-black/20 px-3 py-2 text-text-tertiary transition-all hover:bg-black/30 hover:border-primary/30"
              data-testid="conversation-process-toggle"
              @click="emit('toggle-detail', message.id)"
            >
              <div :class="cn(
                'flex size-6 shrink-0 items-center justify-center rounded-lg transition-colors',
                isRunning ? 'bg-primary/20 text-primary shadow-[0_0_10px_rgba(var(--color-primary-rgb),0.2)]' : 'bg-black/40 text-text-tertiary'
              )">
                 <component :is="isExpanded ? ChevronDown : ChevronRight" :size="14" />
              </div>
              
              <div class="flex min-w-0 flex-col items-start gap-0 text-left">
                <div class="flex min-w-0 items-center gap-2 text-[11px] font-bold text-text-secondary uppercase tracking-tight">
                  <component :is="processSummary.icon" :size="12" class="shrink-0" :class="isRunning ? 'text-primary animate-pulse' : ''" />
                  <span class="truncate">
                    {{ processSummary.label }}<span v-if="isRunning">...</span>
                  </span>
                </div>
                <span class="truncate text-[10px] font-medium text-text-tertiary opacity-70">
                  {{ processSummary.detail }}
                </span>
              </div>
            </button>

            <div v-if="isExpanded" class="ml-3 mt-3 space-y-3 border-l border-primary/20 pl-5 py-1 animate-in fade-in slide-in-from-top-2 duration-300">
              <div
                v-for="entry in detailEntries"
                :key="entry.id"
                class="group/entry space-y-1.5 rounded-xl border border-transparent px-4 py-2.5 transition-all"
                :class="entry.toolId && entry.toolId === focusedToolId ? 'border-primary/30 bg-primary/10 shadow-[0_0_15px_rgba(var(--color-primary-rgb),0.05)]' : 'bg-black/10 hover:bg-black/20'"
                :data-testid="entry.toolId && entry.toolId === focusedToolId ? 'conversation-focused-tool-entry' : undefined"
              >
                <div class="flex items-center gap-2 text-[12px] font-bold text-text-secondary">
                  <div class="h-1 w-1 rounded-full bg-primary/60 shadow-[0_0_4px_var(--color-primary)]"></div>
                  {{ entry.title }}
                </div>
                <p class="pl-3 text-[12px] leading-relaxed text-text-tertiary font-mono break-all opacity-80">{{ entry.detail }}</p>
              </div>
            </div>
          </div>

          <!-- Message Body -->
          <div
            class="whitespace-pre-wrap break-words text-[14.5px] leading-[1.7] text-text-primary tracking-normal selection:bg-primary/30"
            :class="isUserMessage ? 'text-right' : 'text-left'"
          >
            {{ message.content }}
          </div>

        <div
          v-if="!isUserMessage && toolCalls.length"
          class="flex flex-col gap-2 mt-2"
          data-testid="conversation-inline-tool-calls"
        >
          <button
            v-for="toolCall in toolCalls"
            :key="toolCall.toolId"
            type="button"
            class="group/tool flex items-center gap-3 rounded-xl border border-border/40 bg-black/10 px-4 py-2.5 text-left transition-all hover:bg-black/20 hover:border-primary/40 hover:shadow-md"
            :data-testid="`conversation-inline-tool-${toolCall.toolId}`"
            @click="emit('focus-tool', { messageId: message.id, toolId: toolCall.toolId })"
          >
            <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl bg-black/20 text-text-tertiary group-hover/tool:bg-primary/20 group-hover/tool:text-primary transition-colors">
              <Wrench :size="16" class="shrink-0" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="truncate text-[13px] font-bold text-text-primary group-hover/tool:text-primary transition-colors">
                {{ formatToolCallTitle(toolCall.label) }}
              </div>
              <div class="text-[10px] font-bold tabular-nums text-text-tertiary uppercase tracking-wider opacity-60">
                {{ formatToolCallMeta(toolCall.count) }}
              </div>
            </div>
          </button>
        </div>

        <!-- Approval / Decision needed (More prominent) -->
        <UiStatusCallout
          v-if="!isUserMessage && message.approval"
          class="mt-4 border-status-warning/40 bg-status-warning/5 rounded-2xl shadow-lg animate-in zoom-in-95 duration-500"
          tone="warning"
          :title="message.approval.summary"
          :description="message.approval.detail"
          data-testid="conversation-inline-approval"
        >
          <div class="flex flex-wrap items-center gap-3 mb-2">
            <span class="inline-flex items-center gap-2 text-[11px] font-bold uppercase tracking-wider text-status-warning">
              <Shield :size="14" class="shrink-0" />
              <span>Security Validation Required: {{ message.approval.toolName }}</span>
            </span>
            <UiBadge v-if="approvalRiskLabel" :label="approvalRiskLabel" tone="warning" class="text-[9px]" />
          </div>
          <div v-if="hasPendingApproval" class="flex flex-wrap gap-3 mt-4 pt-3 border-t border-status-warning/20">
            <UiButton
              size="sm"
              class="flex-1 shadow-md shadow-primary/20"
              data-testid="conversation-inline-approve"
              :disabled="approvalResolving"
              @click="message.approval && emit('approve', message.approval.id)"
            >
              Authorize Execution
            </UiButton>
            <UiButton
              size="sm"
              variant="ghost"
              class="flex-1 hover:bg-status-error/10 hover:text-status-error"
              data-testid="conversation-inline-reject"
              :disabled="approvalResolving"
              @click="message.approval && emit('reject', message.approval.id)"
            >
              Decline
            </UiButton>
          </div>
        </UiStatusCallout>

        <!-- Assets / Resources -->
        <div
          v-if="resources.length || attachments.length || artifacts.length"
          class="flex flex-wrap gap-2.5 pt-3 mt-1"
          :class="isUserMessage ? 'justify-end' : 'justify-start'"
        >
          <button
            v-for="resource in resources"
            :key="resource.id"
            class="flex items-center gap-2.5 rounded-xl border border-border/50 bg-black/10 px-3 py-2 text-[12px] font-bold text-text-secondary transition-all hover:bg-black/30 hover:border-primary/30 hover:shadow-sm"
            @click="emit('open-resource', resource.id)"
          >
            <div class="size-6 flex items-center justify-center rounded-lg bg-black/20 text-text-tertiary">
               <FolderOpen v-if="resource.kind === 'folder'" :size="13" />
               <FileText v-else-if="resource.kind === 'artifact'" :size="13" />
               <Paperclip v-else :size="13" />
            </div>
            <span>{{ resource.name }}</span>
          </button>

          <button
            v-for="artifact in artifacts"
            :key="artifact.id"
            class="group/artifact flex items-center gap-3 rounded-xl border border-primary/20 bg-primary/5 px-4 py-2 text-[12px] font-bold text-primary transition-all hover:bg-primary/10 hover:shadow-[0_0_15px_rgba(var(--color-primary-rgb),0.1)]"
            @click="emit('open-artifact', { id: artifact.id, version: artifact.version })"
          >
            <div class="size-7 flex items-center justify-center rounded-lg bg-primary/10 text-primary transition-transform group-hover/artifact:rotate-6">
              <FileText :size="14" />
            </div>
            <div class="flex flex-col items-start leading-none">
              <span class="truncate">{{ artifact.label }}</span>
              <span v-if="artifact.kindLabel" class="text-[9px] uppercase opacity-60 mt-0.5">{{ artifact.kindLabel }}</span>
            </div>
            <div v-if="artifact.version" class="text-[9px] font-mono bg-primary/20 px-1 rounded ml-1">V{{ artifact.version }}</div>
          </button>
        </div>

        <!-- Usage info -->
        <div v-if="!isUserMessage && message.usage" class="pt-3 flex items-center gap-2 text-[9px] font-bold uppercase tracking-[0.1em] text-text-tertiary/40">
           <Sparkles :size="10" />
           <span>{{ message.usage.totalTokens }} tokens consumed</span>
           <span class="mx-1 opacity-50">·</span>
           <span>Mode: {{ permissionLabel }}</span>
        </div>
      </div>
    </article>
  </div>
</template>
