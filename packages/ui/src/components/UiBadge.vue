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
    class="inline-flex max-w-full items-center whitespace-nowrap overflow-hidden text-ellipsis rounded-md px-2 py-0.5 text-[11px] font-medium transition-colors"
    :class="[
      props.tone === 'default' ? 'bg-subtle text-text-secondary border border-border/40 dark:border-white/[0.04]' : '',
      props.tone === 'success' ? 'bg-status-success/10 text-status-success border border-status-success/20' : '',
      props.tone === 'warning' ? 'bg-status-warning/10 text-status-warning border border-status-warning/20' : '',
      props.tone === 'error' ? 'bg-status-error/10 text-status-error border border-status-error/20' : '',
      props.tone === 'info' ? 'bg-primary/10 text-primary border border-primary/20 dark:border-primary/20' : '',
      props.subtle ? 'border-transparent bg-transparent text-text-tertiary px-1' : ''
    ]"
  >
    {{ props.label }}
  </span>
</template>