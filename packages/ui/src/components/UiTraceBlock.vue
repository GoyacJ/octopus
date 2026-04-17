<script setup lang="ts">
import { computed } from 'vue'

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
</script>

<template>
  <article
    :data-ui-tone="props.tone"
    :class="cn(
      'flex flex-col overflow-hidden rounded-[var(--radius-m)] border border-[color-mix(in_srgb,var(--border)_76%,transparent)] bg-[color-mix(in_srgb,var(--surface)_86%,var(--subtle)_14%)] transition-colors',
      props.class
    )"
  >
    <div
      data-testid="ui-trace-block-header"
      :class="cn('flex items-center justify-between gap-4 border-b border-border px-4 py-3 text-[11px] font-medium text-text-tertiary', headerClass)"
    >
      <div class="flex min-w-0 items-center gap-2">
        <span :class="cn('h-2 w-2 shrink-0 rounded-full', markerClass)" />
        <span class="truncate">{{ props.actor }}</span>
      </div>
      <span class="shrink-0">{{ props.timestampLabel }}</span>
    </div>

    <div class="space-y-2 px-4 py-4">
      <strong class="text-[13px] font-bold leading-tight text-text-primary">{{ props.title }}</strong>

      <p class="text-[12px] leading-relaxed text-text-secondary break-words">
        {{ props.detail }}
      </p>
    </div>

    <div v-if="$slots.actions" class="flex items-center gap-2 border-t border-border bg-[color-mix(in_srgb,var(--surface)_76%,var(--subtle)_24%)] px-4 py-3">
      <slot name="actions" />
    </div>
  </article>
</template>
