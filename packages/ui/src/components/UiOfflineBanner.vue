<script setup lang="ts">
import { cn } from '../lib/utils'

type UiOfflineBannerTone = 'warning' | 'danger'

const props = withDefaults(defineProps<{
  tone?: UiOfflineBannerTone
  title: string
  description?: string
  testId?: string
  actionsTestId?: string
  class?: string
}>(), {
  tone: 'warning',
  description: '',
  testId: 'ui-offline-banner',
  actionsTestId: 'ui-offline-banner-actions',
  class: '',
})

function containerClasses(tone: UiOfflineBannerTone) {
  if (tone === 'danger') {
    return 'border-status-error/35 bg-[color-mix(in_srgb,var(--status-error)_10%,var(--background)_90%)]'
  }

  return 'border-status-warning/35 bg-[color-mix(in_srgb,var(--status-warning)_14%,var(--background)_86%)]'
}

function iconClasses(tone: UiOfflineBannerTone) {
  return tone === 'danger' ? 'text-status-error' : 'text-status-warning'
}
</script>

<template>
  <div
    :data-testid="props.testId"
    :data-ui-offline-tone="props.tone"
    :class="cn('border-b', containerClasses(props.tone), props.class)"
  >
    <div class="flex flex-col gap-3 px-4 py-2.5 md:flex-row md:items-center md:justify-between">
      <div class="flex min-w-0 items-start gap-3">
        <div
          v-if="$slots.icon"
          :class="cn('flex h-8 w-8 shrink-0 items-center justify-center rounded-[var(--radius-s)] border border-current/15 bg-background/70', iconClasses(props.tone))"
        >
          <slot name="icon" />
        </div>

        <div class="min-w-0">
          <p class="text-label font-semibold text-text-primary">
            {{ props.title }}
          </p>
          <p v-if="props.description" class="text-caption text-text-secondary">
            {{ props.description }}
          </p>
        </div>
      </div>

      <div
        v-if="$slots.actions"
        :data-testid="props.actionsTestId"
        class="flex shrink-0 items-center gap-2"
      >
        <slot name="actions" />
      </div>
    </div>
  </div>
</template>
