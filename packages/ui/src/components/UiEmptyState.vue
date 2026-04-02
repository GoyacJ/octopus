<script setup lang="ts">
import { cn } from '../lib/utils'

const props = defineProps<{
  title: string
  description: string
  class?: string
}>()
</script>

<template>
  <div :class="cn('flex flex-col items-center justify-center gap-2 rounded-[calc(var(--radius-lg)+2px)] border border-dashed border-border-strong/80 bg-subtle/55 px-8 py-10 text-center', props.class)">
    <strong class="text-[1rem] font-semibold tracking-[-0.02em] text-text-primary">{{ props.title }}</strong>
    <p class="m-0 max-w-[320px] text-sm leading-7 text-text-secondary break-words">{{ props.description }}</p>
  </div>
</template>