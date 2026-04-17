<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  testId?: string
  layout?: 'default' | 'inline'
}>(), {
  class: '',
  testId: '',
  layout: 'default',
})
</script>

<template>
  <div
    :data-testid="props.testId || undefined"
    :class="cn(
      'flex flex-col gap-3 rounded-[var(--radius-m)] border border-border bg-surface p-3 shadow-xs',
      props.class,
    )"
  >
    <template v-if="props.layout === 'inline'">
      <div class="flex flex-col gap-3 xl:flex-row xl:items-center">
        <div v-if="$slots.search" class="min-w-0 xl:min-w-[260px] xl:flex-1">
          <slot name="search" />
        </div>

        <div
          v-if="$slots.filters || $slots.tabs || $slots.views || $slots.chips || $slots.actions"
          class="flex min-w-0 flex-1 flex-wrap items-center gap-3 xl:justify-end"
        >
          <div v-if="$slots.filters" class="flex min-w-0 flex-wrap items-center gap-3">
            <slot name="filters" />
          </div>
          <div v-if="$slots.tabs" class="flex min-w-0 flex-wrap items-center gap-2">
            <slot name="tabs" />
          </div>
          <div v-if="$slots.views" class="flex flex-wrap items-center gap-2">
            <slot name="views" />
          </div>
          <div v-if="$slots.chips" class="flex flex-wrap items-center gap-2">
            <slot name="chips" />
          </div>
          <slot name="actions" />
        </div>
      </div>
    </template>

    <template v-else>
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
    </template>
  </div>
</template>
