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
      'flex flex-col gap-2 rounded-md border border-border/40 dark:border-white/[0.03] p-4 transition-colors bg-background relative overflow-hidden',
      props.tone === 'accent' && 'border-primary/50 dark:border-primary/5 bg-primary/5',
      props.tone === 'muted' && 'bg-subtle/50',
      props.tone === 'success' && 'border-status-success/15 bg-status-success/5',
      props.tone === 'warning' && 'border-status-warning/15 bg-status-warning/5',
      props.class,
    )"
  >
    <div class="flex items-start justify-between gap-3 relative z-10">
      <span class="block text-[10px] font-bold uppercase tracking-wider text-text-tertiary">
        {{ props.label }}
      </span>
      <span v-if="props.progressLabel" class="text-[10px] font-medium text-text-tertiary">
        {{ props.progressLabel }}
      </span>
    </div>

    <strong class="text-2xl font-bold tracking-tight text-text-primary tabular-nums relative z-10">
      {{ props.value }}
    </strong>

    <p v-if="props.helper" class="text-[12px] leading-relaxed text-text-secondary relative z-10">
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
