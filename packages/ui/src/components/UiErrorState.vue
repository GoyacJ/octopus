<script setup lang="ts">
import { cn } from '../lib/utils'
import UiPageShell from './UiPageShell.vue'
import UiPanelFrame from './UiPanelFrame.vue'

const props = withDefaults(defineProps<{
  eyebrow?: string
  title: string
  description?: string
  testId?: string
  introTestId?: string
  actionsTestId?: string
  detailsTestId?: string
  panelClass?: string
  contentClass?: string
}>(), {
  eyebrow: '',
  description: '',
  testId: 'ui-error-state',
  introTestId: 'ui-error-state-intro',
  actionsTestId: 'ui-error-state-actions',
  detailsTestId: 'ui-error-state-details',
  panelClass: '',
  contentClass: '',
})
</script>

<template>
  <UiPageShell width="wide" class="h-full" content-class="min-h-full">
    <div class="flex min-h-full items-center py-6">
      <UiPanelFrame
        :data-testid="props.testId"
        variant="raised"
        padding="none"
        :class="cn('mx-auto w-full max-w-[880px] overflow-hidden', props.panelClass)"
        inner-class="overflow-hidden"
      >
        <div
          :data-testid="props.introTestId"
          class="flex items-start gap-4 border-b border-border bg-subtle px-6 py-5"
        >
          <div
            v-if="$slots.icon"
            class="flex h-10 w-10 shrink-0 items-center justify-center rounded-[var(--radius-m)] border border-border bg-surface text-status-error"
          >
            <slot name="icon" />
          </div>

          <div class="min-w-0 space-y-2">
            <p v-if="props.eyebrow" class="text-micro font-semibold uppercase tracking-[0.08em] text-text-tertiary">
              {{ props.eyebrow }}
            </p>
            <h1 class="text-section-title font-semibold text-text-primary">
              {{ props.title }}
            </h1>
            <p v-if="props.description" class="text-body text-text-secondary">
              {{ props.description }}
            </p>
          </div>
        </div>

        <div :class="cn('space-y-6 px-6 py-6', props.contentClass)">
          <div v-if="$slots.summary">
            <slot name="summary" />
          </div>

          <div
            v-if="$slots.actions"
            :data-testid="props.actionsTestId"
            class="flex flex-wrap items-center gap-3 rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-3"
          >
            <slot name="actions" />
          </div>

          <div
            v-if="$slots.details"
            :data-testid="props.detailsTestId"
            class="space-y-3 rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-4"
          >
            <slot name="details" />
          </div>
        </div>
      </UiPanelFrame>
    </div>
  </UiPageShell>
</template>
