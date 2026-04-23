<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  tone?: 'info' | 'success' | 'warning' | 'error'
  title?: string
  description?: string
  class?: string
}>(), {
  tone: 'info',
  title: '',
  description: '',
  class: '',
})

const toneClass = computed(() => {
  switch (props.tone) {
    case 'success':
      return {
        container: 'border-[color-mix(in_srgb,var(--color-status-success)_18%,var(--border))] bg-[color-mix(in_srgb,var(--color-status-success-soft)_72%,var(--surface)_28%)]',
        title: 'text-status-success',
      }
    case 'warning':
      return {
        container: 'border-[color-mix(in_srgb,var(--color-status-warning)_18%,var(--border))] bg-[color-mix(in_srgb,var(--color-status-warning-soft)_72%,var(--surface)_28%)]',
        title: 'text-status-warning',
      }
    case 'error':
      return {
        container: 'border-[color-mix(in_srgb,var(--color-status-error)_18%,var(--border))] bg-[color-mix(in_srgb,var(--color-status-error-soft)_72%,var(--surface)_28%)]',
        title: 'text-status-error',
      }
    default:
      return {
        container: 'border-[color-mix(in_srgb,var(--color-status-info)_18%,var(--border))] bg-[color-mix(in_srgb,var(--color-status-info-soft)_72%,var(--surface)_28%)]',
        title: 'text-status-info',
      }
  }
})
</script>

<template>
  <div :class="cn('flex flex-col gap-2 rounded-[var(--radius-l)] border p-3', toneClass.container, props.class)">
    <div
      v-if="props.title"
      :class="cn('text-label font-semibold leading-none', toneClass.title)"
    >
      {{ props.title }}
    </div>
    <div v-if="props.description" class="text-label leading-relaxed text-text-secondary">
      {{ props.description }}
    </div>
    <div v-if="$slots.default" class="text-label leading-relaxed text-text-secondary">
      <slot />
    </div>
  </div>
</template>
