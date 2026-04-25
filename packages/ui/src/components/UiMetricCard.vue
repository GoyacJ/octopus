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
    data-ui-performance-contained="true"
    :class="cn(
      'group relative overflow-hidden rounded-[var(--radius-xl)] border border-border bg-surface p-5 shadow-sm transition-all duration-normal hover:border-border-strong hover:shadow-md',
      props.tone === 'accent' && 'bg-accent/30 border-primary/20 highlight-border',
      props.tone === 'muted' && 'bg-subtle',
      props.tone === 'success' && 'border-status-success/20 bg-status-success/5',
      props.tone === 'warning' && 'border-status-warning/20 bg-status-warning/5',
      props.class,
    )"
  >
    <!-- Background Glow for Accent -->
    <div 
      v-if="props.tone === 'accent'" 
      class="absolute -right-8 -top-8 size-24 bg-primary/10 blur-3xl rounded-full pointer-events-none group-hover:bg-primary/20 transition-colors"
    />

    <div class="flex items-start justify-between gap-3 relative z-10">
      <span class="block text-[10px] font-bold uppercase tracking-[0.12em] text-text-tertiary">
        {{ props.label }}
      </span>
      <span v-if="props.progressLabel" class="text-[10px] font-bold text-primary/80">
        {{ props.progressLabel }}
      </span>
    </div>

    <div class="mt-2 flex items-baseline gap-2 relative z-10">
      <strong class="text-3xl font-bold tracking-tight text-text-primary tabular-nums">
        {{ props.value }}
      </strong>
    </div>

    <p v-if="props.helper" class="mt-1 text-xs leading-relaxed text-text-secondary line-clamp-1 relative z-10">
      {{ props.helper }}
    </p>

    <slot />

    <div
      v-if="normalizedProgress !== null && (!props.trend || props.trend.length === 0)"
      class="mt-4 h-1 rounded-full bg-border/40 relative z-10 overflow-hidden"
    >
      <div
        data-testid="ui-metric-progress"
        class="h-full rounded-full bg-primary shadow-[0_0_8px_var(--color-primary)] transition-[width] duration-slow"
        :style="{ width: `${normalizedProgress}%` }"
      />
    </div>

    <div v-if="props.trend && props.trend.length > 0" class="absolute inset-x-0 bottom-0 h-16 opacity-30 pointer-events-none">
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
