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
      'flex min-h-[7.5rem] flex-col gap-2 rounded-[calc(var(--radius-lg)+4px)] p-4 transition-all duration-normal ease-apple hover:shadow-sm',
      props.tone === 'default' && 'bg-black/[0.025] dark:bg-white/[0.04] text-text-secondary',
      props.tone === 'accent' && 'bg-primary/[0.06] text-primary shadow-sm ring-1 ring-primary/5',
      props.tone === 'muted' && 'bg-muted/50 text-text-tertiary',
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
