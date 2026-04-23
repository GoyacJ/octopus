<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(
  defineProps<{
    label: string
    value: string
    helper?: string
    tone?: 'default' | 'success' | 'warning' | 'error' | 'info'
    class?: string
  }>(),
  {
    helper: '',
    tone: 'default',
    class: '',
  },
)
</script>

<template>
  <div
    :data-ui-tone="props.tone"
    :class="cn(
      'flex flex-col gap-1.5 rounded-[var(--radius-l)] border border-border bg-surface p-4 shadow-xs transition-colors',
      props.tone === 'success' && 'border-transparent bg-[var(--color-status-success-soft)]',
      props.tone === 'warning' && 'border-transparent bg-[var(--color-status-warning-soft)]',
      props.tone === 'error' && 'border-transparent bg-[var(--color-status-error-soft)]',
      props.tone === 'info' && 'border-transparent bg-accent',
      props.class
    )"
  >
    <span class="text-micro font-bold uppercase tracking-wider text-text-tertiary opacity-70">{{ props.label }}</span>
    <strong class="text-2xl font-bold tracking-tight text-text-primary tabular-nums">{{ props.value }}</strong>
    <span v-if="props.helper" class="text-micro text-text-secondary line-clamp-1">{{ props.helper }}</span>
  </div>
</template>
