<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'
import UiSparkline from './UiSparkline.vue'

type MetricTone = 'default' | 'accent' | 'muted' | 'success' | 'warning'

const props = withDefaults(defineProps<{
  label: string
  value: string | number
  helper?: string
  progress?: number | null
  progressLabel?: string
  trend?: number[]
  tone?: MetricTone
  class?: string
}>(), {
  helper: '',
  progress: null,
  progressLabel: '',
  trend: undefined,
  tone: 'default',
  class: '',
})

const normalizedProgress = computed(() => {
  if (props.progress === null || Number.isNaN(Number(props.progress))) {
    return null
  }

  return Math.max(0, Math.min(100, Number(props.progress)))
})

const toneColor = computed(() => {
  switch (props.tone) {
    case 'accent': return 'var(--color-primary)'
    case 'success': return 'var(--color-status-success)'
    case 'warning': return 'var(--color-status-warning)'
    default: return 'var(--color-text-tertiary)'
  }
})
</script>

<template>
  <article
    :class="cn(
      'relative overflow-hidden rounded-[var(--radius-l)] border border-border bg-surface p-4 shadow-xs transition-colors',
      props.tone === 'accent' && 'bg-accent border-border-strong',
      props.tone === 'muted' && 'bg-subtle',
      props.tone === 'success' && 'border-transparent bg-[var(--color-status-success-soft)]',
      props.tone === 'warning' && 'border-transparent bg-[var(--color-status-warning-soft)]',
      props.class,
    )"
  >
    <div class="flex items-start justify-between gap-3 relative z-10">
      <span class="block text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
        {{ props.label }}
      </span>
      <span v-if="props.progressLabel" class="text-[11px] font-medium text-text-tertiary">
        {{ props.progressLabel }}
      </span>
    </div>

    <strong class="text-[30px] font-bold tracking-[-0.03em] text-text-primary tabular-nums relative z-10">
      {{ props.value }}
    </strong>

    <p v-if="props.helper" class="text-[13px] leading-relaxed text-text-secondary relative z-10">
      {{ props.helper }}
    </p>

    <slot />

    <div
      v-if="normalizedProgress !== null && (!props.trend || props.trend.length === 0)"
      class="mt-2 h-1 rounded-full bg-subtle relative z-10"
    >
      <div
        data-testid="ui-metric-progress"
        class="h-full rounded-full bg-primary transition-[width] duration-normal"
        :style="{ width: `${normalizedProgress}%` }"
      />
    </div>

    <div v-if="props.trend && props.trend.length > 0" class="absolute bottom-0 left-0 right-0 h-1/2 opacity-20 pointer-events-none">
      <UiSparkline 
        :data="props.trend" 
        :width="200" 
        :height="50" 
        :stroke-color="toneColor"
        class="w-full h-full"
        preserveAspectRatio="none"
      />
    </div>
  </article>
</template>
