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
        raised: 'bg-white dark:bg-white/[0.03] border border-border dark:border-white/[0.04] shadow-xs rounded-lg',
        overlay: 'bg-popover border border-border/50 dark:border-white/[0.04] shadow-md rounded-lg',
        panel: 'bg-subtle/40 dark:bg-white/[0.02] border-none rounded-lg',
        interactive: 'bg-white dark:bg-white/[0.03] border border-border dark:border-white/[0.04] shadow-xs hover:border-border/50 hover:bg-accent rounded-lg active:scale-[0.995]',
        subtle: 'bg-subtle/50 dark:bg-white/[0.01] border-none shadow-none rounded-lg',
      },
      padding: {
        none: 'p-0',
        sm: 'p-3',
        md: 'p-5',
        lg: 'p-8',
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
    <header v-if="eyebrow || title || subtitle || $slots.actions" class="mb-5 flex flex-wrap items-start justify-between gap-4 px-1">
      <div v-if="eyebrow || title || subtitle" class="min-w-0 flex-1 space-y-1">
        <p v-if="eyebrow" class="text-[0.62rem] font-bold uppercase tracking-[0.15em] text-text-tertiary">
          {{ eyebrow }}
        </p>
        <h2 v-if="title" class="text-xl font-bold tracking-tight text-text-primary">
          {{ title }}
        </h2>
        <p v-if="subtitle" class="text-sm leading-relaxed text-text-secondary line-clamp-2">
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
