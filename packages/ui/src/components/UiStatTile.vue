<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    label: string
    value: string
    helper?: string
    tone?: 'default' | 'success' | 'warning' | 'error' | 'info'
  }>(),
  {
    helper: '',
    tone: 'default',
  },
)
</script>

<template>
  <div class="ui-stat-tile" :class="`tone-${props.tone}`">
    <span class="label">{{ props.label }}</span>
    <strong class="value">{{ props.value }}</strong>
    <span v-if="props.helper" class="helper">{{ props.helper }}</span>
  </div>
</template>

<style scoped>
.ui-stat-tile {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  min-height: 7rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
  box-shadow: var(--shadow-sm);
}

.label {
  color: var(--text-secondary);
  font-size: 0.8rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.value {
  font-size: 1.85rem;
  line-height: 1;
  overflow-wrap: anywhere;
}

.helper {
  margin-top: auto;
  color: var(--text-tertiary);
  font-size: 0.8rem;
  overflow-wrap: anywhere;
}

.tone-warning {
  border-color: color-mix(in srgb, var(--status-warning) 35%, var(--border-subtle));
}

.tone-error {
  border-color: color-mix(in srgb, var(--status-error) 35%, var(--border-subtle));
}

.tone-info {
  border-color: color-mix(in srgb, var(--status-info) 35%, var(--border-subtle));
}

.tone-success {
  border-color: color-mix(in srgb, var(--status-success) 35%, var(--border-subtle));
}
</style>
