<script setup lang="ts">
import { cva, type VariantProps } from 'class-variance-authority'
import { computed } from 'vue'
import { cn } from '../lib/utils'

const surfaceVariants = cva(
  'transition-all duration-normal ease-apple',
  {
    variants: {
      variant: {
        flat: 'bg-transparent shadow-none border-none',
        raised: 'bg-surface border border-border shadow-xs rounded-[var(--radius-l)]',
        overlay: 'bg-popover border border-border shadow-md rounded-[var(--radius-xl)]',
        panel: 'bg-subtle border border-border/70 shadow-none rounded-[var(--radius-l)]',
        interactive: 'bg-surface border border-border shadow-xs rounded-[var(--radius-l)] hover:bg-accent hover:border-border-strong',
        subtle: 'bg-subtle border border-transparent shadow-none rounded-[var(--radius-l)]',
      },
      padding: {
        none: 'p-0',
        sm: 'p-3',
        md: 'p-4',
        lg: 'p-6',
      }
    },
    defaultVariants: {
      variant: 'raised',
      padding: 'md',
    },
  },
)

interface Props {
  variant?: NonNullable<VariantProps<typeof surfaceVariants>['variant']>
  padding?: NonNullable<VariantProps<typeof surfaceVariants>['padding']>
  eyebrow?: string
  title?: string
  subtitle?: string
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'raised',
  padding: 'md',
  class: '',
})

const classes = computed(() => cn(surfaceVariants({ variant: props.variant, padding: props.padding }), props.class))
</script>

<template>
  <section :class="classes">
    <header v-if="eyebrow || title || subtitle || $slots.actions" class="mb-4 flex flex-wrap items-start justify-between gap-4">
      <div v-if="eyebrow || title || subtitle" class="min-w-0 flex-1 space-y-1">
        <p v-if="eyebrow" class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
          {{ eyebrow }}
        </p>
        <h2 v-if="title" class="text-[22px] font-bold tracking-[-0.02em] text-text-primary">
          {{ title }}
        </h2>
        <p v-if="subtitle" class="text-[14px] leading-relaxed text-text-secondary line-clamp-2">
          {{ subtitle }}
        </p>
      </div>

      <div v-if="$slots.actions" class="flex shrink-0 flex-wrap items-center gap-2">
        <slot name="actions" />
      </div>
    </header>
    <div class="relative">
      <slot />
    </div>
  </section>
</template>
