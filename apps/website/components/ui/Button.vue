<script setup lang="ts">
interface Props {
  variant?: 'primary' | 'secondary' | 'outline' | 'ghost' | 'glass'
  size?: 'sm' | 'md' | 'lg'
  to?: string
  href?: string
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'primary',
  size: 'md'
})

const baseStyles = 'inline-flex items-center justify-center font-medium transition-all focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:pointer-events-none rounded-[var(--radius-m)]'

const variants = {
  primary: 'bg-[var(--website-accent)] text-white hover:bg-[var(--website-accent-hover)] shadow-sm',
  secondary: 'bg-[var(--website-surface-soft)] text-[var(--website-text)] hover:bg-[var(--website-border)]',
  outline: 'border border-[var(--website-border-strong)] bg-transparent text-[var(--website-text)] hover:bg-[var(--website-surface-soft)]',
  ghost: 'bg-transparent text-[var(--website-text)] hover:bg-[var(--website-surface-soft)]',
  glass: 'glass text-[var(--website-text)] hover:bg-[var(--website-surface-soft)]'
}

const sizes = {
  sm: 'px-3 py-1.5 text-xs',
  md: 'px-5 py-2.5 text-sm',
  lg: 'px-8 py-3.5 text-base'
}

const componentType = computed(() => {
  if (props.to) return resolveComponent('NuxtLink')
  if (props.href) return 'a'
  return 'button'
})
</script>

<template>
  <component
    :is="componentType"
    :to="to"
    :href="href"
    :class="[baseStyles, variants[variant], sizes[size]]"
  >
    <slot />
  </component>
</template>
