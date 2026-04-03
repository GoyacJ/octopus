<script setup lang="ts">
import { cva, type VariantProps } from 'class-variance-authority'
import { computed } from 'vue'
import { cn } from '../lib/utils'

const surfaceVariants = cva(
  'transition-all duration-normal ease-apple',
  {
    variants: {
      variant: {
        flat: 'bg-transparent shadow-none border-transparent',
        raised: 'bg-white dark:bg-white/5 border border-border-subtle shadow-xs rounded-md',
        overlay: 'bg-popover border border-border-strong shadow-lg rounded-lg ring-1 ring-black/5',
        panel: 'border border-border-subtle bg-subtle/30 dark:bg-white/5 rounded-md',
        interactive: 'border border-border-subtle bg-white dark:bg-white/5 shadow-xs hover:border-border-strong hover:bg-accent rounded-md active:scale-[0.995]',
        subtle: 'bg-subtle dark:bg-white/[0.02] border-transparent shadow-none rounded-md',
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
