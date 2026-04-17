<script setup lang="ts">
import { cva, type VariantProps } from 'class-variance-authority'
import { computed } from 'vue'
import { Loader2 } from 'lucide-vue-next'
import { cn } from '../lib/utils'

const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[var(--radius-xs)] border text-[13px] font-semibold leading-none transition-colors duration-fast focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50',
  {
    variants: {
      variant: {
        primary:
          'border-transparent bg-primary text-primary-foreground hover:bg-primary-hover shadow-xs',
        destructive:
          'border-transparent bg-destructive text-destructive-foreground hover:opacity-92 shadow-xs',
        outline:
          'border-border-subtle bg-surface text-text-primary hover:bg-subtle',
        secondary:
          'border-border-subtle bg-surface text-secondary-foreground hover:bg-subtle',
        ghost: 'border-transparent text-text-secondary hover:bg-subtle hover:text-text-primary',
        link: 'text-primary underline-offset-4 hover:underline',
      },
      size: {
        default: 'h-8 px-3',
        sm: 'h-7 px-2.5 text-[12px]',
        lg: 'h-9 px-4 text-[14px]',
        icon: 'h-8 w-8 px-0',
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
