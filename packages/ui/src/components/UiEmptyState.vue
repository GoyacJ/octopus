<script setup lang="ts">
import { cn } from '../lib/utils'

const props = defineProps<{
  title: string
  description: string
  class?: string
}>()
</script>

<template>
  <div :class="cn('flex flex-col items-center justify-center py-10 px-4 text-center', props.class)">
    <div v-if="$slots.icon" class="mb-3 flex size-12 items-center justify-center rounded-lg bg-subtle text-text-tertiary">
      <slot name="icon" />
    </div>
    <strong v-if="props.title" class="block text-sm font-semibold text-text-secondary">{{ props.title }}</strong>
    <p v-if="props.description" class="mt-1 max-w-sm text-[13px] leading-relaxed text-text-tertiary">{{ props.description }}</p>
    <div v-if="$slots.actions" class="mt-4 flex flex-wrap items-center justify-center gap-2">
      <slot name="actions" />
    </div>
  </div>
</template>