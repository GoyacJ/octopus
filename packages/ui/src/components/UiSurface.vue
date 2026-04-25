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
        raised: 'bg-surface border border-border/40 shadow-[var(--layer-depth-1)] rounded-[var(--radius-xl)]',
        overlay: 'bg-popover border border-border/60 shadow-[var(--shadow-lg)] rounded-[var(--radius-2xl)]',
        panel: 'bg-surface-muted/50 border-none shadow-[var(--layer-depth-1)] rounded-[var(--radius-2xl)]',
        interactive: 'bg-surface border border-border/40 shadow-xs rounded-[var(--radius-xl)] hover:bg-subtle hover:border-border-strong hover:shadow-md',
        subtle: 'bg-surface-muted/40 border-none shadow-none rounded-[var(--radius-xl)]',
        glass: 'glass border border-white/10 shadow-[var(--shadow-md)] rounded-[var(--radius-2xl)] highlight-border backdrop-blur-md',
        'glass-strong': 'glass-strong border border-white/20 shadow-[var(--shadow-xl)] rounded-[var(--radius-2xl)] highlight-border backdrop-blur-lg',
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
        <p v-if="eyebrow" class="text-micro font-semibold uppercase tracking-[0.08em] text-text-tertiary">
          {{ eyebrow }}
        </p>
        <h2 v-if="title" class="text-section-title font-bold tracking-[-0.02em] text-text-primary">
          {{ title }}
        </h2>
        <p v-if="subtitle" class="text-body leading-relaxed text-text-secondary line-clamp-2">
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
