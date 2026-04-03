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
    :class="cn(
      'flex flex-col gap-1.5 p-4 rounded-md border border-border-subtle bg-background transition-colors',
      props.tone === 'success' && 'border-status-success/20 bg-status-success/5',
      props.tone === 'warning' && 'border-status-warning/20 bg-status-warning/5',
      props.tone === 'error' && 'border-status-error/20 bg-status-error/5',
      props.tone === 'info' && 'border-primary/20 bg-primary/5',
      props.class
    )"
  >
    <span class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary opacity-70">{{ props.label }}</span>
    <strong class="text-2xl font-bold tracking-tight text-text-primary tabular-nums">{{ props.value }}</strong>
    <span v-if="props.helper" class="text-[11px] text-text-secondary line-clamp-1">{{ props.helper }}</span>
  </div>
</template>
