<script setup lang="ts">
import { cva, type VariantProps } from 'class-variance-authority'
import { computed } from 'vue'
import { cn } from '../lib/utils'

const surfaceVariants = cva(
  'rounded-[calc(var(--radius-lg)+2px)] transition-all duration-normal ease-apple',
  {
    variants: {
      variant: {
        flat: 'bg-background border border-border/70 shadow-none',
        raised: 'bg-card border border-border/80 shadow-sm',
        overlay: 'bg-popover border border-border/80 shadow-lg backdrop-blur-xl',
        subtle: 'bg-muted/72 border border-border/40 shadow-none',
      },
      padding: {
        none: 'p-0',
        sm: 'p-4',
        md: 'p-6',
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
    <header v-if="eyebrow || title || subtitle" class="mb-4 space-y-1.5">
      <p v-if="eyebrow" class="text-[10px] font-bold uppercase tracking-wider text-muted-foreground/80">
        {{ eyebrow }}
      </p>
      <h2 v-if="title" class="text-lg font-semibold leading-tight tracking-tight text-foreground">
        {{ title }}
      </h2>
      <p v-if="subtitle" class="text-sm leading-relaxed text-muted-foreground line-clamp-2">
        {{ subtitle }}
      </p>
    </header>
    <div class="relative">
      <slot />
    </div>
  </section>
</template>
