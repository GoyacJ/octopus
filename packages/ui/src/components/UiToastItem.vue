<script setup lang="ts">
import { computed } from 'vue'
import type { NotificationRecord } from '@octopus/schema'
import { X } from 'lucide-vue-next'

import { formatDateTime } from '../lib/formatDateTime'
import { getNotificationPresentation, notificationLevelIcons } from '../lib/notificationPresentation'
import UiSurface from './UiSurface.vue'

const props = defineProps<{
  notification: NotificationRecord
  scopeLabel: string
}>()

const emit = defineEmits<{
  close: [id: string]
  select: [notification: NotificationRecord]
}>()

const timestampLabel = computed(() => formatDateTime(props.notification.createdAt))
const presentation = computed(() => getNotificationPresentation(props.notification.level))
</script>

<template>
  <UiSurface
    variant="overlay"
    padding="sm"
    :class="`w-full shadow-sm ${presentation.toastSurfaceClass}`"
  >
    <div
      class="flex items-start gap-3"
      :data-testid="`ui-toast-item-${props.notification.id}`"
      @click="emit('select', props.notification)"
    >
      <component
        :is="notificationLevelIcons[props.notification.level ?? 'info']"
        :size="16"
        class="mt-0.5 shrink-0"
        :class="presentation.toastIconClass"
      />
      <div class="min-w-0 flex-1 space-y-1">
        <div class="flex items-start justify-between gap-3">
          <span class="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">
            {{ props.scopeLabel }}
          </span>
          <span v-if="timestampLabel" class="shrink-0 text-[11px] font-medium text-text-tertiary">
            {{ timestampLabel }}
          </span>
        </div>
        <p class="truncate text-sm font-semibold" :class="presentation.toastTitleClass">
          {{ props.notification.title }}
        </p>
        <p class="text-xs leading-5 text-text-secondary">
          {{ props.notification.body }}
        </p>
      </div>
      <button
        type="button"
        class="rounded-[var(--radius-s)] p-1 text-text-tertiary transition-colors hover:bg-subtle hover:text-text-primary"
        :data-testid="`ui-toast-close-${props.notification.id}`"
        @click.stop="emit('close', props.notification.id)"
      >
        <X :size="14" />
      </button>
    </div>
  </UiSurface>
</template>
