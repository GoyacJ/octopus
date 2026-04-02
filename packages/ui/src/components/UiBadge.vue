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
    class="inline-flex max-w-full items-center whitespace-nowrap overflow-hidden text-ellipsis rounded-full border px-2.5 py-1 text-[0.72rem] font-semibold tracking-[0.02em] transition-all duration-fast ease-apple"
    :class="[
      props.tone === 'default' ? 'border-border bg-subtle text-text-secondary' : '',
      props.tone === 'success' ? 'border-status-success/20 bg-status-success/10 text-status-success' : '',
      props.tone === 'warning' ? 'border-status-warning/20 bg-status-warning/10 text-status-warning' : '',
      props.tone === 'error' ? 'border-status-error/20 bg-status-error/10 text-status-error' : '',
      props.tone === 'info' ? 'border-primary/15 bg-primary/10 text-primary' : '',
      props.subtle ? 'border-border/70 bg-transparent text-text-tertiary shadow-none' : ''
    ]"
  >
    {{ props.label }}
  </span>
</template>