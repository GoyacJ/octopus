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
      'flex flex-col overflow-hidden rounded-[var(--radius-m)] border border-[color-mix(in_srgb,var(--border)_76%,transparent)] bg-[color-mix(in_srgb,var(--surface)_86%,var(--subtle)_14%)] transition-colors',
      props.class
    )"
  >
    <div
      data-testid="ui-trace-block-header"
      :class="cn('flex items-center justify-between gap-4 border-b border-border px-4 py-3 text-micro font-medium text-text-tertiary', headerClass)"
    >
      <div class="flex min-w-0 items-center gap-2">
        <span :class="cn('h-2 w-2 shrink-0 rounded-full', markerClass)" />
        <span class="truncate">{{ props.actor }}</span>
      </div>
      <span class="shrink-0 tabular-nums">{{ props.timestampLabel }}</span>
    </div>

    <div class="space-y-2 px-4 py-4">
      <strong class="text-label font-bold text-text-primary">{{ props.title }}</strong>

      <div
        v-if="normalizedMetaItems.length"
        data-testid="ui-trace-block-meta"
        class="flex flex-wrap gap-2"
      >
        <span
          v-for="item in normalizedMetaItems"
          :key="item"
          data-testid="ui-trace-block-meta-item"
          class="inline-flex items-center rounded-full border border-[color-mix(in_srgb,var(--border)_82%,transparent)] bg-[color-mix(in_srgb,var(--surface)_82%,var(--subtle)_18%)] px-2.5 py-1 text-micro font-medium uppercase tracking-[0.08em] text-text-tertiary"
        >
          {{ item }}
        </span>
      </div>

      <p class="text-caption text-text-secondary break-words">
        {{ props.detail }}
      </p>
    </div>

    <div v-if="$slots.actions" class="flex items-center gap-2 border-t border-border bg-[color-mix(in_srgb,var(--surface)_76%,var(--subtle)_24%)] px-4 py-3">
      <slot name="actions" />
    </div>
  </article>
</template>
