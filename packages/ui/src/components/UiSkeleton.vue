<script setup lang="ts">
import { computed } from 'vue'

import { prefersReducedMotion } from '../lib/motion'
import { cn } from '../lib/utils'

type UiSkeletonVariant = 'line' | 'card' | 'table-row'

const props = withDefaults(defineProps<{
  variant?: UiSkeletonVariant
  count?: number
  class?: string
  respectReducedMotion?: boolean
  reducedMotion?: boolean
}>(), {
  variant: 'line',
  count: 3,
  class: '',
  respectReducedMotion: true,
  reducedMotion: undefined,
})

const items = computed(() =>
  Array.from({ length: Math.max(1, props.count) }, (_, index) => index),
)

const isAnimated = computed(() => {
  const reducedMotion = props.reducedMotion ?? prefersReducedMotion()

  if (!props.respectReducedMotion) {
    return true
  }

  return !reducedMotion
})

function resolveLineWidth(index: number) {
  const widths = ['w-full', 'w-5/6', 'w-2/3', 'w-4/5']
  return widths[index % widths.length]
}
</script>

<template>
  <div
    data-testid="ui-skeleton"
    :data-ui-skeleton-variant="props.variant"
    :data-ui-skeleton-animated="String(isAnimated)"
    :class="cn(
      'flex min-w-0 flex-col',
      props.variant === 'line' ? 'gap-2' : '',
      props.variant === 'card' ? 'gap-3' : '',
      props.variant === 'table-row' ? 'gap-2.5' : '',
      props.class,
    )"
  >
    <template v-if="props.variant === 'line'">
      <div
        v-for="item in items"
        :key="`line-${item}`"
        data-testid="ui-skeleton-item"
        :class="cn(
          'ui-skeleton-block h-3 rounded-full',
          resolveLineWidth(item),
          isAnimated && 'ui-skeleton-block--animated',
        )"
      />
    </template>

    <template v-else-if="props.variant === 'card'">
      <div
        v-for="item in items"
        :key="`card-${item}`"
        data-testid="ui-skeleton-item"
        class="flex flex-col gap-3 rounded-[var(--radius-l)] border border-border/50 bg-surface p-4"
      >
        <div class="flex items-center justify-between gap-3">
          <div :class="cn('ui-skeleton-block h-3 w-16 rounded-full', isAnimated && 'ui-skeleton-block--animated')" />
          <div :class="cn('ui-skeleton-block h-3 w-10 rounded-full', isAnimated && 'ui-skeleton-block--animated')" />
        </div>

        <div class="space-y-2">
          <div :class="cn('ui-skeleton-block h-4 w-3/5 rounded-full', isAnimated && 'ui-skeleton-block--animated')" />
          <div :class="cn('ui-skeleton-block h-3 w-full rounded-full', isAnimated && 'ui-skeleton-block--animated')" />
          <div :class="cn('ui-skeleton-block h-3 w-4/5 rounded-full', isAnimated && 'ui-skeleton-block--animated')" />
        </div>

        <div class="flex items-center gap-2 pt-1">
          <div :class="cn('ui-skeleton-block h-8 w-20 rounded-[var(--radius-full)]', isAnimated && 'ui-skeleton-block--animated')" />
          <div :class="cn('ui-skeleton-block h-8 w-16 rounded-[var(--radius-full)]', isAnimated && 'ui-skeleton-block--animated')" />
        </div>
      </div>
    </template>

    <template v-else>
      <div
        v-for="item in items"
        :key="`table-row-${item}`"
        data-testid="ui-skeleton-item"
        class="grid grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_auto] items-center gap-3 rounded-[var(--radius-m)] border border-border/40 bg-background px-3 py-3"
      >
        <div :class="cn('ui-skeleton-block h-4 w-full rounded-full', isAnimated && 'ui-skeleton-block--animated')" />
        <div :class="cn('ui-skeleton-block h-4 w-4/5 rounded-full', isAnimated && 'ui-skeleton-block--animated')" />
        <div :class="cn('ui-skeleton-block h-4 w-16 rounded-full', isAnimated && 'ui-skeleton-block--animated')" />
      </div>
    </template>
  </div>
</template>

<style scoped>
.ui-skeleton-block {
  background: linear-gradient(
    90deg,
    color-mix(in srgb, var(--subtle) 72%, var(--surface) 28%) 0%,
    color-mix(in srgb, var(--subtle) 46%, var(--surface) 54%) 50%,
    color-mix(in srgb, var(--subtle) 72%, var(--surface) 28%) 100%
  );
  background-size: 200% 100%;
}

.ui-skeleton-block--animated {
  animation: ui-skeleton-shimmer var(--duration-slow) linear infinite;
}

@keyframes ui-skeleton-shimmer {
  0% {
    background-position: 200% 0;
  }

  100% {
    background-position: -200% 0;
  }
}
</style>
