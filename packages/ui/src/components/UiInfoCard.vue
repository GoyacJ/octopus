<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  label?: string
  title: string
  description?: string
  tone?: 'default' | 'accent' | 'muted'
  class?: string
}>(), {
  label: '',
  description: '',
  tone: 'default',
  class: '',
})
</script>

<template>
  <article
    :class="cn(
      'flex min-h-[7.5rem] flex-col gap-2 rounded-[var(--radius-xl)] border p-4 transition-colors duration-fast',
      props.tone === 'default' && 'border-border bg-surface text-text-secondary',
      props.tone === 'accent' && 'border-primary/15 bg-accent text-primary',
      props.tone === 'muted' && 'border-border bg-subtle text-text-tertiary',
      props.class,
    )"
  >
    <span v-if="props.label" class="text-[0.72rem] font-semibold uppercase tracking-[0.12em] text-text-tertiary">
      {{ props.label }}
    </span>
    <strong class="break-words text-base font-semibold leading-tight text-text-primary">
      {{ props.title }}
    </strong>
    <p v-if="props.description" class="mt-auto text-sm leading-6 text-text-secondary">
      {{ props.description }}
    </p>
    <slot />
  </article>
</template>
