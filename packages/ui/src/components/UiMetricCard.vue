<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'

type MetricTone = 'default' | 'accent' | 'muted' | 'success' | 'warning'

const props = withDefaults(defineProps<{
  label: string
  value: string | number
  helper?: string
  progress?: number | null
  progressLabel?: string
  tone?: MetricTone
  class?: string
}>(), {
  helper: '',
  progress: null,
  progressLabel: '',
  tone: 'default',
  class: '',
})

const normalizedProgress = computed(() => {
  if (props.progress === null || Number.isNaN(Number(props.progress))) {
    return null
  }

  return Math.max(0, Math.min(100, Number(props.progress)))
})
</script>

<template>
  <article
    :class="cn(
      'flex flex-col gap-2 rounded-md border border-border-subtle p-4 transition-colors bg-background',
      props.tone === 'accent' && 'border-primary/20 bg-primary/5',
      props.tone === 'muted' && 'bg-subtle',
      props.tone === 'success' && 'border-status-success/20 bg-status-success/5',
      props.tone === 'warning' && 'border-status-warning/20 bg-status-warning/5',
      props.class,
    )"
  >
    <div class="flex items-start justify-between gap-3">
      <span class="block text-[10px] font-bold uppercase tracking-wider text-text-tertiary">
        {{ props.label }}
      </span>
      <span v-if="props.progressLabel" class="text-[10px] font-medium text-text-tertiary">
        {{ props.progressLabel }}
      </span>
    </div>

    <strong class="text-2xl font-bold tracking-tight text-text-primary tabular-nums">
      {{ props.value }}
    </strong>

    <p v-if="props.helper" class="text-[12px] leading-relaxed text-text-secondary">
      {{ props.helper }}
    </p>

    <slot />

    <div
      v-if="normalizedProgress !== null"
      class="mt-2 h-1 rounded-full bg-subtle"
    >
      <div
        data-testid="ui-metric-progress"
        class="h-full rounded-full bg-primary transition-[width] duration-normal"
        :style="{ width: `${normalizedProgress}%` }"
      />
    </div>
  </article>
</template>
