<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    title: string
    detail: string
    actor: string
    timestampLabel: string
    tone?: 'default' | 'success' | 'warning' | 'error' | 'info'
  }>(),
  {
    tone: 'default',
  },
)
</script>

<template>
  <article class="trace-block" :class="`tone-${props.tone}`">
    <div class="meta">
      <span>{{ props.actor }}</span>
      <span>{{ props.timestampLabel }}</span>
    </div>
    <strong>{{ props.title }}</strong>
    <p>{{ props.detail }}</p>
    <slot name="actions" />
  </article>
</template>

<style scoped>
.trace-block {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
}

.meta {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 0.75rem;
  color: var(--text-secondary);
  font-size: 0.8rem;
}

p {
  color: var(--text-secondary);
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.tone-success {
  border-color: color-mix(in srgb, var(--status-success) 35%, transparent);
}

.tone-warning {
  border-color: color-mix(in srgb, var(--status-warning) 35%, transparent);
}

.tone-error {
  border-color: color-mix(in srgb, var(--status-error) 35%, transparent);
}

.tone-info {
  border-color: color-mix(in srgb, var(--status-info) 35%, transparent);
}
</style>
