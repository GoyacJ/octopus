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
      props.layout === 'compact' && 'gap-1 p-2',
      props.active
        ? 'is-active border-border-strong bg-subtle shadow-xs'
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
        <strong :class="cn(
          'block font-semibold leading-tight text-text-primary',
          props.layout === 'compact' ? 'text-[14px]' : 'text-[15px]',
        )">
          {{ props.title }}
        </strong>
        <p :class="cn(
          'text-text-secondary line-clamp-2',
          props.layout === 'compact' ? 'text-[12px] leading-5' : 'text-[13px] leading-relaxed',
        )" v-if="props.description">
          {{ props.description }}
        </p>
        <slot />

        <div :class="cn(
          'flex min-w-0 flex-wrap items-center',
          props.layout === 'compact' ? 'gap-1.5 pt-0.5' : 'gap-2 pt-1',
        )" v-if="$slots.secondary">
          <slot name="secondary" />
        </div>
      </div>

      <div :class="cn(
        'flex shrink-0 flex-wrap justify-end',
        props.layout === 'compact' ? 'items-center gap-1' : 'items-start gap-1.5',
      )" v-if="$slots.badges">
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
      :class="cn(
        'flex flex-col sm:flex-row sm:items-center sm:justify-between',
        props.layout === 'compact'
          ? 'mt-1 gap-1.5 pt-1.5'
          : 'mt-auto gap-2 border-t border-border/55 pt-2',
      )"
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
