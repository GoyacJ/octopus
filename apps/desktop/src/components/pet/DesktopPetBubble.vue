<script setup lang="ts">
import { BellDot, ExternalLink } from 'lucide-vue-next'

import type { NotificationRecord } from '@octopus/schema'

import { UiButton, UiSurface } from '@octopus/ui'

defineProps<{
  notification: NotificationRecord
  scopeLabel: string
  actionLabel: string
}>()

const emit = defineEmits<{
  select: [notification: NotificationRecord]
}>()
</script>

<template>
  <UiSurface
    variant="overlay"
    padding="sm"
    class="w-[17rem] max-w-[calc(100vw-2.5rem)]"
    data-testid="desktop-pet-bubble"
  >
    <div class="flex items-start gap-3">
      <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-[var(--radius-m)] bg-primary/10 text-primary">
        <BellDot :size="16" />
      </div>

      <div class="min-w-0 flex-1 space-y-2">
        <div class="space-y-1">
          <p class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
            {{ scopeLabel }}
          </p>
          <p class="line-clamp-1 text-sm font-semibold text-text-primary">
            {{ notification.title }}
          </p>
          <p v-if="notification.body" class="line-clamp-2 text-[13px] leading-5 text-text-secondary">
            {{ notification.body }}
          </p>
        </div>

        <UiButton
          variant="ghost"
          size="sm"
          class="h-7 justify-start px-0 text-primary hover:bg-transparent"
          data-testid="desktop-pet-bubble-action"
          @click="emit('select', notification)"
        >
          <span>{{ actionLabel }}</span>
          <ExternalLink :size="14" />
        </UiButton>
      </div>
    </div>
  </UiSurface>
</template>
