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
      return 'bg-[var(--color-status-success-soft)] text-status-success'
    case 'warning':
      return 'bg-[var(--color-status-warning-soft)] text-status-warning'
    case 'error':
      return 'bg-[var(--color-status-error-soft)] text-status-error'
    default:
      return 'bg-accent text-primary'
  }
})
</script>

<template>
  <div :class="cn('flex flex-col gap-2 rounded-[var(--radius-l)] border border-transparent p-3', toneClass, props.class)">
    <div v-if="props.title" class="text-[13px] font-semibold leading-none">
      {{ props.title }}
    </div>
    <div v-if="props.description" class="text-[13px] leading-relaxed text-current/85">
      {{ props.description }}
    </div>
    <slot />
  </div>
</template>
