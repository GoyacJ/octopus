<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'

const props = withDefaults(
  defineProps<{
    title: string
    detail: string
    actor: string
    timestampLabel: string
    metaItems?: string[]
    tone?: 'default' | 'success' | 'warning' | 'error' | 'info'
    class?: string
  }>(),
  {
    metaItems: () => [],
    tone: 'default',
    class: '',
  },
)

const headerClass = computed(() => {
  if (props.tone === 'success') {
    return 'bg-[var(--color-status-success-soft)]'
  }
  if (props.tone === 'warning') {
    return 'bg-[var(--color-status-warning-soft)]'
  }
  if (props.tone === 'error') {
    return 'bg-[var(--color-status-error-soft)]'
  }
  if (props.tone === 'info') {
    return 'bg-[color-mix(in_srgb,var(--color-accent-soft)_74%,var(--surface)_26%)]'
  }
  return 'bg-subtle'
})

const markerClass = computed(() => {
  if (props.tone === 'success') {
    return 'bg-[var(--color-status-success)]'
  }
  if (props.tone === 'warning') {
    return 'bg-[var(--color-status-warning)]'
  }
  if (props.tone === 'error') {
    return 'bg-[var(--color-status-error)]'
  }
  if (props.tone === 'info') {
    return 'bg-primary'
  }
  return 'bg-text-tertiary'
})

const normalizedMetaItems = computed(() =>
  props.metaItems
    .map(item => item.trim())
    .filter(item => item.length > 0),
)
</script>

<template>
  <article
    :data-ui-tone="props.tone"
    :class="cn(
      'group relative flex flex-col overflow-hidden rounded-[var(--radius-l)] border border-border bg-surface/40 backdrop-blur-sm transition-all duration-normal hover:border-border-strong',
      props.tone === 'info' && 'border-primary/20 highlight-border',
      props.class
    )"
  >
    <div
      data-testid="ui-trace-block-header"
      :class="cn('flex items-center justify-between gap-4 border-b border-border/50 px-4 py-2.5 text-[10px] font-bold uppercase tracking-[0.1em] text-text-tertiary', headerClass)"
    >
      <div class="flex min-w-0 items-center gap-2">
        <span :class="cn('h-1.5 w-1.5 shrink-0 rounded-full shadow-[0_0_8px_currentColor]', markerClass)" />
        <span class="truncate">{{ props.actor }}</span>
      </div>
      <span class="shrink-0 tabular-nums opacity-60">{{ props.timestampLabel }}</span>
    </div>

    <div class="space-y-3 px-4 py-4">
      <div class="flex items-start justify-between gap-4">
        <strong class="text-sm font-bold text-text-primary leading-tight">{{ props.title }}</strong>
        <div
          v-if="normalizedMetaItems.length"
          data-testid="ui-trace-block-meta"
          class="flex flex-wrap gap-1.5"
        >
          <span
            v-for="item in normalizedMetaItems"
            :key="item"
            data-testid="ui-trace-block-meta-item"
            class="inline-flex items-center rounded-md border border-border/50 bg-subtle/50 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider text-text-tertiary"
          >
            {{ item }}
          </span>
        </div>
      </div>

      <p class="text-[13px] leading-relaxed text-text-secondary/90 break-words font-mono bg-black/5 p-2 rounded-md">
        {{ props.detail }}
      </p>
    </div>

    <div v-if="$slots.actions" class="flex items-center gap-2 border-t border-border/40 bg-subtle/30 px-4 py-2.5">
      <slot name="actions" />
    </div>
  </article>
</template>
