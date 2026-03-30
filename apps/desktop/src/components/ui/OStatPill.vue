<template>
  <div class="stat-pill" :class="{ highlight }">
    <span class="stat-label">{{ label }}</span>
    <span class="stat-value" :class="{ mono }">
      <slot>{{ value }}</slot>
    </span>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  label: string;
  value?: string | number;
  highlight?: boolean;
  mono?: boolean;
}>();
</script>

<style scoped>
.stat-pill {
  display: flex;
  flex-direction: column;
  padding: 0.5rem 0.875rem;
  background-color: var(--bg-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  min-width: 100px;
}

.stat-pill.highlight {
  border-color: var(--color-accent-soft);
  background-color: var(--color-accent-soft);
}

.stat-label {
  font-size: 0.625rem;
  font-weight: 700;
  color: var(--text-subtle);
  text-transform: uppercase;
  margin-bottom: 0.125rem;
}

.stat-value {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.stat-value.mono {
  font-family: monospace;
}
</style>
