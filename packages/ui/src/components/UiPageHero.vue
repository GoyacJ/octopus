<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  title?: string
  description?: string
  class?: string
}>(), {
  title: '',
  description: '',
  class: '',
})
</script>

<template>
  <div :class="cn('flex flex-col gap-5 py-1', props.class)">
    <div
      class="grid gap-6"
      :class="$slots.aside ? 'lg:grid-cols-[1fr_300px] lg:items-start' : 'grid-cols-1'"
    >
      <div class="flex min-w-0 flex-col gap-4">
        <div v-if="$slots.meta" class="flex flex-wrap gap-2">
          <slot name="meta" />
        </div>

        <div v-if="props.title || props.description" class="space-y-2">
          <h2 v-if="props.title" class="text-[30px] font-bold leading-[1.15] tracking-[-0.03em] text-text-primary">
            {{ props.title }}
          </h2>
          <p v-if="props.description" class="max-w-3xl text-[14px] leading-relaxed text-text-secondary">
            {{ props.description }}
          </p>
        </div>

        <div v-if="$slots.default" class="flex min-w-0 flex-col gap-4">
          <slot />
        </div>

        <div v-if="$slots.actions" class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 pt-1">
          <slot name="actions" />
        </div>
      </div>

      <aside v-if="$slots.aside" class="flex min-w-0 flex-col gap-3">
        <slot name="aside" />
      </aside>
    </div>
  </div>
</template>
