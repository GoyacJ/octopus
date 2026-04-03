<script setup lang="ts">
import { cva, type VariantProps } from 'class-variance-authority'
import { computed } from 'vue'
import { Loader2 } from 'lucide-vue-next'
import { cn } from '../lib/utils'

const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all duration-fast disabled:pointer-events-none disabled:opacity-50 active:scale-[0.98]',
  {
    variants: {
      variant: {
        primary:
          'bg-primary text-primary-foreground hover:bg-primary/90 shadow-[0_1px_2px_rgba(15,15,15,0.1)]',
        destructive:
          'bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-[0_1px_2px_rgba(15,15,15,0.1)]',
        outline:
          'border border-border-strong bg-background text-text-primary hover:bg-accent',
        secondary:
          'bg-secondary text-secondary-foreground hover:bg-secondary/80 border border-border-subtle',
        ghost: 'text-text-secondary hover:bg-accent hover:text-text-primary',
        link: 'text-primary underline-offset-4 hover:underline',
      },
      size: {
        default: 'h-8 px-3 py-1.5',
        sm: 'h-7 px-2 text-xs',
        lg: 'h-10 px-6 text-base',
        icon: 'h-8 w-8',
      },
    },
    defaultVariants: {
      variant: 'primary',
      size: 'default',
    },
  },
)

interface Props {
  variant?: NonNullable<VariantProps<typeof buttonVariants>['variant']>
  size?: NonNullable<VariantProps<typeof buttonVariants>['size']>
  as?: string
  loading?: boolean
  loadingLabel?: string
  disabled?: boolean
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  as: 'button',
  variant: 'primary',
  size: 'default',
  loading: false,
  loadingLabel: '',
  disabled: false,
})

const classes = computed(() => cn(buttonVariants({ variant: props.variant, size: props.size }), props.class))
</script>

<template>
  <component
    :is="as"
    :class="classes"
    :disabled="disabled || loading"
    :aria-busy="loading ? 'true' : undefined"
  >
    <Loader2 v-if="loading" data-testid="ui-button-spinner" class="h-4 w-4 animate-spin" />
    <template v-if="loading && loadingLabel">
      {{ loadingLabel }}
    </template>
    <slot v-else />
  </component>
</template>
