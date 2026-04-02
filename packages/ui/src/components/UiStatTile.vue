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
      'flex min-h-[8.5rem] flex-col gap-2 rounded-[calc(var(--radius-lg)+2px)] border bg-surface p-6 shadow-xs transition-all duration-normal ease-apple',
      props.tone === 'default' && 'border-border',
      props.tone === 'success' && 'border-status-success/20 bg-status-success/[0.04]',
      props.tone === 'warning' && 'border-status-warning/20 bg-status-warning/[0.04]',
      props.tone === 'error' && 'border-status-error/20 bg-status-error/[0.04]',
      props.tone === 'info' && 'border-primary/15 bg-primary/[0.04]',
      props.class
    )"
  >
    <span class="line-clamp-2 text-[0.74rem] font-semibold uppercase tracking-[0.12em] text-text-tertiary">{{ props.label }}</span>
    <strong class="break-all text-[2rem] font-semibold leading-tight tracking-[-0.04em] text-text-primary">{{ props.value }}</strong>
    <span v-if="props.helper" class="mt-auto break-all text-[0.82rem] leading-relaxed text-text-secondary">{{ props.helper }}</span>
  </div>
</template>