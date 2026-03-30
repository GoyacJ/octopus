<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  variant?: 'default' | 'success' | 'warning' | 'danger' | 'info' | 'highlight';
  size?: 'sm' | 'md';
  outline?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'default',
  size: 'md',
  outline: false,
});

const classes = computed(() => {
  return [
    'o-pill',
    `o-pill--${props.variant}`,
    `o-pill--${props.size}`,
    { 'o-pill--outline': props.outline }
  ];
});
</script>

<template>
  <span :class="classes">
    <slot></slot>
  </span>
</template>

<style scoped>
.o-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.25rem 0.625rem;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  border-radius: var(--radius-full);
  line-height: 1;
}

.o-pill--sm {
  padding: 0.125rem 0.5rem;
  font-size: 0.625rem;
}

/* Variants */
.o-pill--default {
  background-color: var(--bg-app);
  color: var(--text-muted);
}

.o-pill--success {
  background-color: var(--color-success-soft);
  color: var(--color-success);
}

.o-pill--warning {
  background-color: var(--color-warning-soft);
  color: #92400e;
}

.o-pill--danger {
  background-color: var(--color-danger-soft);
  color: var(--color-danger);
}

.o-pill--info {
  background-color: var(--color-accent-soft);
  color: var(--color-accent);
}

.o-pill--highlight {
  background-color: var(--text-primary);
  color: white;
}

.o-pill--outline {
  background-color: transparent;
  border: 1px solid currentColor;
}
</style>
