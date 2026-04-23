<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'
import UiPanelFrame from './UiPanelFrame.vue'

type UiRestrictedStateTone = 'neutral' | 'warning' | 'accent'

const props = withDefaults(defineProps<{
  tone?: UiRestrictedStateTone
  eyebrow?: string
  title: string
  description?: string
  testId?: string
  introTestId?: string
  bodyTestId?: string
  actionsTestId?: string
  panelClass?: string
  bodyClass?: string
}>(), {
  tone: 'neutral',
  eyebrow: '',
  description: '',
  testId: 'ui-restricted-state',
  introTestId: 'ui-restricted-state-intro',
  bodyTestId: 'ui-restricted-state-body',
  actionsTestId: 'ui-restricted-state-actions',
  panelClass: '',
  bodyClass: '',
})

const toneClasses = computed(() => {
  switch (props.tone) {
    case 'warning':
      return {
        intro: 'bg-[color-mix(in_srgb,var(--status-warning)_10%,var(--surface)_90%)]',
        icon: 'border-status-warning/20 bg-background/80 text-status-warning',
      }
    case 'accent':
      return {
        intro: 'bg-[color-mix(in_srgb,var(--accent)_12%,var(--surface)_88%)]',
        icon: 'border-primary/15 bg-background/80 text-primary',
      }
    default:
      return {
        intro: 'bg-subtle',
        icon: 'border-border bg-background/80 text-text-secondary',
      }
  }
})
</script>

<template>
  <UiPanelFrame
    :data-testid="props.testId"
    variant="raised"
    padding="none"
    :class="cn('overflow-hidden', props.panelClass)"
    inner-class="overflow-hidden"
  >
    <div
      :data-testid="props.introTestId"
      :data-ui-restricted-tone="props.tone"
      :class="cn('flex items-start gap-4 border-b border-border px-5 py-5', toneClasses.intro)"
    >
      <div
        v-if="$slots.icon"
        :class="cn('flex h-10 w-10 shrink-0 items-center justify-center rounded-[var(--radius-m)] border', toneClasses.icon)"
      >
        <slot name="icon" />
      </div>

      <div class="min-w-0 space-y-2">
        <p v-if="props.eyebrow" class="text-micro font-semibold uppercase tracking-[0.08em] text-text-tertiary">
          {{ props.eyebrow }}
        </p>
        <h2 class="text-section-title font-semibold text-text-primary">
          {{ props.title }}
        </h2>
        <p v-if="props.description" class="text-body text-text-secondary">
          {{ props.description }}
        </p>
      </div>
    </div>

    <div :data-testid="props.bodyTestId" :class="cn('space-y-4 px-5 py-5', props.bodyClass)">
      <div v-if="$slots.meta" class="flex flex-wrap items-center gap-2">
        <slot name="meta" />
      </div>

      <div v-if="$slots.default" class="space-y-3 text-body text-text-secondary">
        <slot />
      </div>
    </div>

    <div
      v-if="$slots.actions"
      :data-testid="props.actionsTestId"
      class="flex flex-wrap items-center gap-3 border-t border-border bg-subtle px-5 py-4"
    >
      <slot name="actions" />
    </div>
  </UiPanelFrame>
</template>
