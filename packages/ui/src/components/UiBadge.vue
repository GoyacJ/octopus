<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    label: string
    tone?: 'default' | 'success' | 'warning' | 'error' | 'info'
    subtle?: boolean
  }>(),
  {
    tone: 'default',
    subtle: false,
  },
)
</script>

<template>
  <span
    class="inline-flex max-w-full items-center whitespace-nowrap overflow-hidden text-ellipsis rounded-full border px-2 py-1 text-[12px] font-semibold leading-none transition-colors"
    :class="[
      props.tone === 'default' ? 'border-border bg-subtle text-text-secondary' : '',
      props.tone === 'success' ? 'border-transparent bg-[var(--color-status-success-soft)] text-status-success' : '',
      props.tone === 'warning' ? 'border-transparent bg-[var(--color-status-warning-soft)] text-status-warning' : '',
      props.tone === 'error' ? 'border-transparent bg-[var(--color-status-error-soft)] text-status-error' : '',
      props.tone === 'info' ? 'border-transparent bg-accent text-primary' : '',
      props.subtle ? 'border-transparent bg-transparent text-text-tertiary px-1.5 py-0' : ''
    ]"
  >
    {{ props.label }}
  </span>
</template>
