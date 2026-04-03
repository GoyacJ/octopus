<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(
  defineProps<{
    title: string
    detail: string
    actor: string
    timestampLabel: string
    tone?: 'default' | 'success' | 'warning' | 'error' | 'info'
    class?: string
  }>(),
  {
    tone: 'default',
    class: '',
  },
)
</script>

<template>
  <article 
    :class="cn(
      'flex flex-col gap-1.5 p-3 rounded-md border border-border-subtle bg-background transition-colors',
      props.tone === 'success' && 'border-status-success/30',
      props.tone === 'warning' && 'border-status-warning/30',
      props.tone === 'error' && 'border-status-error/30',
      props.tone === 'info' && 'border-primary/30',
      props.class
    )"
  >
    <div class="flex items-center justify-between gap-4 text-[11px] text-text-tertiary font-medium">
      <span class="truncate">{{ props.actor }}</span>
      <span class="shrink-0">{{ props.timestampLabel }}</span>
    </div>
    
    <strong class="text-[13px] font-bold text-text-primary leading-tight">{{ props.title }}</strong>
    
    <p class="text-[12px] leading-relaxed text-text-secondary break-words">
      {{ props.detail }}
    </p>
    
    <div v-if="$slots.actions" class="pt-2 flex items-center gap-2">
      <slot name="actions" />
    </div>
  </article>
</template>
