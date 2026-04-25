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
      'flex flex-col gap-2 rounded-[var(--radius-l)] border border-border bg-surface/50 p-4 backdrop-blur-sm transition-all duration-normal hover:border-border-strong hover:bg-surface',
      props.tone === 'success' && 'border-status-success/20 bg-status-success/5',
      props.tone === 'warning' && 'border-status-warning/20 bg-status-warning/5',
      props.tone === 'error' && 'border-status-error/20 bg-status-error/5',
      props.tone === 'info' && 'border-primary/20 bg-primary/5',
      props.class
    )"
  >
    <span class="text-[10px] font-bold uppercase tracking-[0.1em] text-text-tertiary">{{ props.label }}</span>
    <div class="flex items-baseline gap-2">
      <strong class="text-2xl font-bold tracking-tight text-text-primary tabular-nums">{{ props.value }}</strong>
    </div>
    <span v-if="props.helper" class="text-caption text-text-secondary line-clamp-1 opacity-80">{{ props.helper }}</span>
  </div>
</template>
