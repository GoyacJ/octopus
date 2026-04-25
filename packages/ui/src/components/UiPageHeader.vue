<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  eyebrow?: string
  title?: string
  description?: string
  compact?: boolean
  class?: string
}>(), {
  eyebrow: '',
  title: '',
  description: '',
  compact: false,
  class: '',
})
</script>

<template>
  <header :class="cn(
    'relative flex flex-col md:flex-row md:items-start md:justify-between transition-all duration-normal pb-8',
    props.compact ? 'gap-3 pt-4' : 'gap-8 pt-8',
    props.class,
  )">
    <div class="min-w-0 flex-1 space-y-3">
      <p v-if="props.eyebrow" class="text-[11px] font-bold uppercase tracking-[0.2em] text-primary">
        {{ props.eyebrow }}
      </p>
      <h1
        v-if="props.title"
        :class="cn(
          'font-extrabold tracking-[-0.03em] text-text-primary',
          props.compact ? 'text-2xl' : 'text-5xl leading-tight',
        )"
      >
        {{ props.title }}
      </h1>
      <p
        v-if="props.description"
        :class="cn(
          'max-w-2xl leading-relaxed text-text-secondary',
          props.compact ? 'text-sm' : 'text-[15px]',
        )"
      >
        {{ props.description }}
      </p>
      <div v-if="$slots.meta" class="flex flex-wrap items-center gap-3 pt-2">
        <slot name="meta" />
      </div>
    </div>

    <div v-if="$slots.actions" class="flex shrink-0 flex-wrap items-center gap-3 md:justify-end pt-2">
      <slot name="actions" />
    </div>

    <!-- Subtle separator gradient -->
    <div class="absolute inset-x-0 bottom-0 h-px bg-gradient-to-r from-border/40 via-border/10 to-transparent" />
  </header>
</template>
