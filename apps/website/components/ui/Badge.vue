<script setup lang="ts">
interface Props {
  variant?: 'primary' | 'secondary' | 'outline' | 'glass'
  size?: 'sm' | 'md'
}

withDefaults(defineProps<Props>(), {
  variant: 'outline',
  size: 'md'
})

const baseStyles = 'inline-flex items-center rounded-full font-semibold uppercase tracking-wider'

const variants = {
  primary: 'bg-[var(--website-accent)] text-white',
  secondary: 'bg-[var(--website-surface-soft)] text-[var(--website-text)]',
  outline: 'border border-[var(--website-border-strong)] text-[var(--website-text-muted)]',
  glass: 'glass text-[var(--website-text)]'
}

const sizes = {
  sm: 'px-2.5 py-0.5 text-[10px]',
  md: 'px-3 py-1 text-[12px]'
}
</script>

<template>
  <span :class="[baseStyles, variants[variant], sizes[size]]">
    <slot />
  </span>
</template>
