<script setup lang="ts">
import { computed, type HTMLAttributes } from 'vue'

import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  keys?: string[]
  size?: 'xs' | 'sm' | 'md'
  class?: HTMLAttributes['class']
}>(), {
  keys: () => [],
  size: 'md',
  class: '',
})

const normalizedKeys = computed(() => props.keys.filter((key) => key.trim().length > 0))

const sizeClass = computed(() => {
  if (props.size === 'xs') {
    return 'gap-0.5 px-1 py-0.5 text-[10px]'
  }

  return props.size === 'sm'
    ? 'gap-0.5 px-1.5 py-0.5'
    : 'gap-1 px-2 py-1'
})
</script>

<template>
  <kbd
    v-if="normalizedKeys.length > 0"
    data-testid="ui-kbd"
    :class="cn(
      'inline-flex items-center rounded-[var(--radius-xs)] border border-border-strong bg-muted text-micro font-semibold uppercase text-text-secondary shadow-xs',
      sizeClass,
      props.class,
    )"
  >
    <template v-for="(key, index) in normalizedKeys" :key="`${key}-${index}`">
      <span>{{ key }}</span>
      <span v-if="index < normalizedKeys.length - 1" aria-hidden="true">+</span>
    </template>
  </kbd>
</template>
