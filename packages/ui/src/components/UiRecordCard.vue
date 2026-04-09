<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  title: string
  description?: string
  active?: boolean
  interactive?: boolean
  layout?: 'default' | 'tile' | 'compact'
  testId?: string
  class?: string
}>(), {
  description: '',
  active: false,
  interactive: false,
  layout: 'default',
  testId: '',
  class: '',
})

const emit = defineEmits<{
  click: [event: MouseEvent | KeyboardEvent]
}>()

function emitClick(event: MouseEvent | KeyboardEvent) {
  if (!props.interactive) {
    return
  }

  emit('click', event)
}
</script>

<template>
  <article
    :data-testid="props.testId || undefined"
    :data-ui-record-layout="props.layout"
    :tabindex="props.interactive ? 0 : undefined"
    :role="props.interactive ? 'button' : undefined"
    :class="cn(
      'flex min-w-0 flex-col gap-2 rounded-[var(--radius-l)] border border-border bg-surface p-3 shadow-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background',
      props.layout === 'tile' && 'gap-3 p-4',
      props.layout === 'compact' && 'gap-1.5 p-2',
      props.active
        ? 'is-active border-border-strong bg-accent shadow-xs'
        : 'border-border bg-surface',
      props.interactive && !props.active && 'cursor-pointer hover:bg-subtle hover:border-border-strong',
      props.class,
    )"
    @click="emitClick"
    @keydown.enter.prevent="emitClick"
    @keydown.space.prevent="emitClick"
  >
    <div class="flex items-start justify-between gap-3">
      <div
        v-if="$slots.leading"
        class="flex size-10 shrink-0 items-center justify-center rounded-[var(--radius-m)] bg-subtle text-text-primary"
      >
        <slot name="leading" />
      </div>

      <div class="min-w-0 flex-1 space-y-1">
        <div v-if="$slots.eyebrow" class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
          <slot name="eyebrow" />
        </div>
        <strong class="block text-[15px] font-semibold leading-tight text-text-primary">
          {{ props.title }}
        </strong>
        <p v-if="props.description" class="text-[13px] leading-relaxed text-text-secondary line-clamp-2">
          {{ props.description }}
        </p>
        <slot />

        <div v-if="$slots.secondary" class="flex min-w-0 flex-wrap items-center gap-2 pt-1">
          <slot name="secondary" />
        </div>
      </div>

      <div v-if="$slots.badges" class="flex shrink-0 flex-wrap items-start justify-end gap-1.5">
        <slot name="badges" />
      </div>
    </div>

    <div
      v-if="$slots.metrics"
      class="mt-1 grid gap-2 rounded-[var(--radius-m)] bg-subtle p-2 sm:grid-cols-[repeat(auto-fit,minmax(0,1fr))]"
    >
      <slot name="metrics" />
    </div>

    <div
      v-if="$slots.meta || $slots.actions"
      class="mt-auto flex flex-col gap-2 border-t border-border/70 pt-2 sm:flex-row sm:items-center sm:justify-between"
    >
      <div v-if="$slots.meta" class="flex min-w-0 flex-wrap items-center gap-2">
        <slot name="meta" />
      </div>
      <div v-if="$slots.actions" class="flex flex-wrap items-center gap-1.5 sm:justify-end">
        <slot name="actions" />
      </div>
    </div>
  </article>
</template>
