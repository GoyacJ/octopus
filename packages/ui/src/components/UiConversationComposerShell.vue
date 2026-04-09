<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  class?: string
}>(), {
  class: '',
})
</script>

<template>
  <div :class="cn('rounded-[var(--radius-xl)] border border-border bg-surface p-3 shadow-sm', props.class)">
    <slot />
  </div>
</template>
