<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  eyebrow?: string
  title: string
  description?: string
  class?: string
}>(), {
  eyebrow: '',
  description: '',
  class: '',
})
</script>

<template>
  <article
    :class="cn(
      'group flex items-start gap-3 rounded-md border border-border/40 bg-background p-3 transition-[transform,colors,border-color] duration-fast hover:border-border-strong hover:bg-subtle active:scale-[0.99] motion-reduce:active:scale-100 cursor-pointer',
      props.class,
    )"
  >
    <div v-if="$slots.icon" class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded bg-primary/10 text-primary">
      <slot name="icon" />
    </div>

    <div class="flex min-w-0 flex-1 flex-col gap-1">
      <span v-if="props.eyebrow" class="text-micro font-bold uppercase tracking-wider text-text-tertiary">
        {{ props.eyebrow }}
      </span>
      <strong class="text-card-title font-bold text-text-primary">
        {{ props.title }}
      </strong>
      <p v-if="props.description" class="text-caption text-text-secondary">
        {{ props.description }}
      </p>
      <div v-if="$slots.default" class="mt-auto pt-1">
        <slot />
      </div>
    </div>

    <div v-if="$slots.suffix" class="shrink-0 text-text-tertiary">
      <slot name="suffix" />
    </div>
  </article>
</template>
