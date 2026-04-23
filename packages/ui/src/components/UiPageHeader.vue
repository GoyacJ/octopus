<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  eyebrow?: string
  title?: string
  description?: string
  compact?: boolean
  class?: string
}>(), {
  eyebrow: '',
  title: '',
  description: '',
  compact: false,
  class: '',
})
</script>

<template>
  <header :class="cn(
    'flex flex-col md:flex-row md:items-end md:justify-between',
    props.compact ? 'gap-3' : 'gap-4',
    props.class,
  )">
    <div class="min-w-0 flex-1 space-y-2">
      <p v-if="props.eyebrow" class="text-micro font-semibold uppercase tracking-[0.08em] text-text-tertiary">
        {{ props.eyebrow }}
      </p>
      <h1
        v-if="props.title"
        :class="cn(
          'font-bold text-text-primary',
          props.compact ? 'text-section-title' : 'text-page-title',
        )"
      >
        {{ props.title }}
      </h1>
      <p
        v-if="props.description"
        :class="cn(
          'max-w-3xl text-text-secondary',
          props.compact ? 'text-label' : 'text-body',
        )"
      >
        {{ props.description }}
      </p>
      <div v-if="$slots.meta" class="flex flex-wrap items-center gap-2 pt-1 text-micro text-text-tertiary tabular-nums">
        <slot name="meta" />
      </div>
    </div>

    <div v-if="$slots.actions" class="flex shrink-0 flex-wrap items-center gap-2 md:justify-end">
      <slot name="actions" />
    </div>
  </header>
</template>
