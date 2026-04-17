<script setup lang="ts">
import { computed } from 'vue'
import type { NotificationRecord } from '@octopus/schema'
import { CheckCheck } from 'lucide-vue-next'

import { formatDateTime } from '../lib/formatDateTime'
import { getNotificationPresentation } from '../lib/notificationPresentation'
import { cn } from '../lib/utils'

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
const presentation = computed(() => getNotificationPresentation(props.notification.level))
</script>

<template>
  <article
    :data-testid="`ui-notification-row-${props.notification.id}`"
    class="group flex min-w-0 flex-col overflow-hidden rounded-[var(--radius-l)] border border-[color-mix(in_srgb,var(--border)_76%,transparent)] bg-[color-mix(in_srgb,var(--surface)_86%,var(--subtle)_14%)] text-left transition-colors duration-fast hover:border-border-strong hover:bg-[color-mix(in_srgb,var(--surface)_74%,var(--subtle)_26%)]"
    :class="cn(
      presentation.rowClass,
      { 'opacity-70': props.notification.readAt },
    )"
    @click="handleSelect"
  >
    <div
      :data-testid="`ui-notification-row-header-${props.notification.id}`"
      class="flex items-start justify-between gap-3 border-b border-border px-4 py-3"
      :class="presentation.rowHeaderClass"
    >
      <div class="flex min-w-0 items-center gap-2">
        <span
          :data-testid="`ui-notification-row-marker-${props.notification.id}`"
          class="h-2 w-2 shrink-0 rounded-full"
          :class="presentation.dotClass"
        />
        <span class="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">
          {{ props.scopeLabel }}
        </span>
        <span
          v-if="!props.notification.readAt"
          class="rounded-full border border-border bg-[color-mix(in_srgb,var(--surface)_76%,transparent)] px-2 py-0.5 text-[10px] font-medium text-text-secondary"
        >
          New
        </span>
      </div>

      <div class="flex shrink-0 items-center gap-1.5">
        <span v-if="timestampLabel" class="text-[11px] font-medium text-text-tertiary">
          {{ timestampLabel }}
        </span>
        <button
          type="button"
          class="rounded-[var(--radius-full)] p-1 text-text-tertiary transition-colors hover:bg-subtle hover:text-text-primary"
          :data-testid="`ui-notification-row-mark-read-${props.notification.id}`"
          @click.stop="handleMarkRead"
        >
          <CheckCheck :size="14" />
        </button>
      </div>
    </div>

    <div class="space-y-1 px-4 py-4">
      <p class="truncate text-sm font-semibold" :class="presentation.titleClass">
        {{ props.notification.title }}
      </p>
      <p class="text-xs leading-5 text-text-secondary">
        {{ props.notification.body }}
      </p>
    </div>
  </article>
</template>
