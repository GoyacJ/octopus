<script setup lang="ts">
interface Props {
  variant?: 'default' | 'soft' | 'outline' | 'glass'
  padding?: 'none' | 'sm' | 'md' | 'lg'
  hover?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'default',
  padding: 'md',
  hover: false
})

const baseStyles = 'overflow-hidden rounded-[var(--radius-l)] transition-all'

const variants = {
  default: 'bg-[var(--website-surface)] border border-[var(--website-border)] shadow-sm',
  soft: 'bg-[var(--website-surface-soft)]',
  outline: 'bg-transparent border border-[var(--website-border-strong)]',
  glass: 'glass'
}

const paddings = {
  none: '',
  sm: 'p-4',
  md: 'p-6',
  lg: 'p-8'
}

const hoverStyles = 'hover:border-[var(--website-accent)] hover:shadow-lg'
</script>

<template>
  <div :class="[baseStyles, variants[variant], paddings[padding], hover && hoverStyles]">
    <slot />
  </div>
</template>
