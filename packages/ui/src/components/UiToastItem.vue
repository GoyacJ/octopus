<script setup lang="ts">
import type { NotificationRecord } from '@octopus/schema'
import { Bell, CheckCircle2, CircleAlert, Info, X } from 'lucide-vue-next'

import UiSurface from './UiSurface.vue'

const props = defineProps<{
  notification: NotificationRecord
  scopeLabel: string
}>()

const emit = defineEmits<{
  close: [id: string]
  select: [notification: NotificationRecord]
}>()

const levelIcons = {
  info: Info,
  success: CheckCircle2,
  warning: CircleAlert,
  error: Bell,
} as const
</script>

<template>
  <UiSurface
    variant="overlay"
    padding="sm"
    class="w-full border-border/40 bg-gradient-to-br from-background via-background to-accent/20 shadow-[0_16px_44px_rgba(15,23,42,0.12)] dark:border-white/[0.08]"
  >
    <div
      class="flex items-start gap-3"
      :data-testid="`ui-toast-item-${props.notification.id}`"
      @click="emit('select', props.notification)"
    >
      <component
        :is="levelIcons[props.notification.level]"
        :size="16"
        class="mt-0.5 shrink-0 text-text-secondary"
      />
      <div class="min-w-0 flex-1 space-y-1">
        <div class="flex items-center gap-2">
          <span class="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">
            {{ props.scopeLabel }}
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
        class="rounded-full p-1 text-text-tertiary transition-colors hover:bg-accent hover:text-text-primary"
        :data-testid="`ui-toast-close-${props.notification.id}`"
        @click.stop="emit('close', props.notification.id)"
      >
        <X :size="14" />
      </button>
    </div>
  </UiSurface>
</template>
