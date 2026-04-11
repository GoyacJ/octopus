<script setup lang="ts">
import { computed } from 'vue'
import type { NotificationRecord } from '@octopus/schema'
import { CheckCheck } from 'lucide-vue-next'

import { formatDateTime } from '../lib/formatDateTime'

const props = defineProps<{
  notification: NotificationRecord
  scopeLabel: string
}>()

const emit = defineEmits<{
  'mark-read': [id: string]
  select: [notification: NotificationRecord]
}>()

function handleSelect() {
  emit('select', props.notification)
}

function handleMarkRead() {
  emit('mark-read', props.notification.id)
}

const timestampLabel = computed(() => formatDateTime(props.notification.createdAt))
</script>

<template>
  <article
    :data-testid="`ui-notification-row-${props.notification.id}`"
    class="group flex items-start gap-3 rounded-[var(--radius-xl)] border border-border bg-surface px-3 py-3 text-left transition-colors duration-fast hover:border-border-strong hover:bg-subtle"
    :class="{ 'opacity-70': props.notification.readAt }"
    @click="handleSelect"
  >
    <div class="mt-1 h-2.5 w-2.5 rounded-full bg-foreground/70" :class="{
      'bg-status-success': props.notification.level === 'success',
      'bg-status-warning': props.notification.level === 'warning',
      'bg-status-error': props.notification.level === 'error',
      'bg-status-info': props.notification.level === 'info',
    }" />
    <div class="min-w-0 flex-1 space-y-1">
      <div class="flex items-start justify-between gap-3">
        <div class="flex min-w-0 items-center gap-2">
          <span class="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">
            {{ props.scopeLabel }}
          </span>
          <span v-if="!props.notification.readAt" class="rounded-full bg-foreground/8 px-2 py-0.5 text-[10px] font-medium text-text-secondary">
            New
          </span>
        </div>
        <span v-if="timestampLabel" class="shrink-0 text-[11px] font-medium text-text-tertiary">
          {{ timestampLabel }}
        </span>
      </div>
      <p class="truncate text-sm font-semibold text-text-primary">
        {{ props.notification.title }}
      </p>
      <p class="text-xs leading-5 text-text-secondary">
        {{ props.notification.body }}
      </p>
    </div>
    <button
      type="button"
      class="rounded-[var(--radius-full)] p-1 text-text-tertiary transition-colors hover:bg-accent hover:text-text-primary"
      :data-testid="`ui-notification-row-mark-read-${props.notification.id}`"
      @click.stop="handleMarkRead"
    >
      <CheckCheck :size="14" />
    </button>
  </article>
</template>
