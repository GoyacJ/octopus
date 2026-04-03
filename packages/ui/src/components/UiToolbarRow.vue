<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  testId?: string
}>(), {
  class: '',
  testId: '',
})
</script>

<template>
  <div
    :data-testid="props.testId || undefined"
    :class="cn(
      'flex flex-col gap-3 rounded-[calc(var(--radius-lg)+2px)] border border-border/80 bg-[color-mix(in_srgb,var(--bg-surface)_94%,transparent)] p-3 shadow-xs',
      props.class,
    )"
  >
    <div
      v-if="$slots.search || $slots.tabs || $slots.views || $slots.actions"
      class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between"
    >
      <div v-if="$slots.search" class="min-w-0 flex-1">
        <slot name="search" />
      </div>

      <div
        v-if="$slots.tabs || $slots.views || $slots.actions"
        class="flex flex-wrap items-center gap-3 lg:justify-end"
      >
        <div v-if="$slots.tabs" class="flex min-w-0 flex-wrap items-center gap-2">
          <slot name="tabs" />
        </div>
        <div v-if="$slots.views" class="flex flex-wrap items-center gap-2">
          <slot name="views" />
        </div>
        <slot name="actions" />
      </div>
    </div>

    <div
      v-if="$slots.filters || $slots.chips"
      class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between"
    >
      <div v-if="$slots.filters" class="flex min-w-0 flex-1 flex-wrap items-center gap-3">
        <slot name="filters" />
      </div>
      <div v-if="$slots.chips" class="flex flex-wrap items-center gap-2 lg:justify-end">
        <slot name="chips" />
      </div>
    </div>
  </div>
</template>
