<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  eyebrow?: string
  title?: string
  description?: string
  class?: string
}>(), {
  eyebrow: '',
  title: '',
  description: '',
  class: '',
})
</script>

<template>
  <header :class="cn('flex flex-col gap-4 md:flex-row md:items-end md:justify-between', props.class)">
    <div class="min-w-0 flex-1 space-y-2">
      <p v-if="props.eyebrow" class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
        {{ props.eyebrow }}
      </p>
      <h1 v-if="props.title" class="text-[30px] font-bold leading-[1.15] tracking-[-0.03em] text-text-primary">
        {{ props.title }}
      </h1>
      <p v-if="props.description" class="max-w-3xl text-[14px] leading-relaxed text-text-secondary">
        {{ props.description }}
      </p>
      <div v-if="$slots.meta" class="flex flex-wrap items-center gap-2 pt-1">
        <slot name="meta" />
      </div>
    </div>

    <div v-if="$slots.actions" class="flex shrink-0 flex-wrap items-center gap-2 md:justify-end">
      <slot name="actions" />
    </div>
  </header>
</template>
