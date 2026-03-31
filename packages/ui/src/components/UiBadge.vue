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
  <span class="ui-badge" :class="[`tone-${props.tone}`, { subtle: props.subtle }]">
    {{ props.label }}
  </span>
</template>

<style scoped>
.ui-badge {
  display: inline-flex;
  align-items: center;
  min-height: 1.75rem;
  min-width: 0;
  max-width: 100%;
  padding: 0 0.75rem;
  border-radius: 999px;
  border: 1px solid transparent;
  font-size: 0.78rem;
  font-weight: 600;
  letter-spacing: 0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tone-default {
  background: color-mix(in srgb, var(--bg-subtle) 88%, transparent);
  color: var(--text-secondary);
  border-color: var(--border-subtle);
}

.tone-success {
  background: color-mix(in srgb, var(--status-success) 14%, transparent);
  color: var(--status-success);
  border-color: color-mix(in srgb, var(--status-success) 35%, transparent);
}

.tone-warning {
  background: color-mix(in srgb, var(--status-warning) 16%, transparent);
  color: var(--status-warning);
  border-color: color-mix(in srgb, var(--status-warning) 35%, transparent);
}

.tone-error {
  background: color-mix(in srgb, var(--status-error) 16%, transparent);
  color: var(--status-error);
  border-color: color-mix(in srgb, var(--status-error) 35%, transparent);
}

.tone-info {
  background: color-mix(in srgb, var(--status-info) 16%, transparent);
  color: var(--status-info);
  border-color: color-mix(in srgb, var(--status-info) 35%, transparent);
}

.subtle {
  background: transparent;
}
</style>
