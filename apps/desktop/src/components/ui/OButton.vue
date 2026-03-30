<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'success' | 'warning';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  type?: 'button' | 'submit' | 'reset';
  loading?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'primary',
  size: 'md',
  disabled: false,
  type: 'button',
  loading: false,
});

const classes = computed(() => {
  return [
    'o-button',
    `o-button--${props.variant}`,
    `o-button--${props.size}`,
    { 'o-button--loading': props.loading }
  ];
});
</script>

<template>
  <button
    :type="type"
    :class="classes"
    :disabled="disabled || loading"
    class="transition-all"
  >
    <span v-if="loading" class="loader"></span>
    <slot v-else name="icon-left"></slot>
    <span class="button-content">
      <slot></slot>
    </span>
    <slot name="icon-right"></slot>
  </button>
</template>

<style scoped>
.o-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  font-weight: 600;
  border-radius: var(--radius-lg);
  cursor: pointer;
  border: 1px solid transparent;
  white-space: nowrap;
}

.o-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none !important;
}

/* Variants */
.o-button--primary {
  background-color: var(--text-primary);
  color: white;
}
.o-button--primary:hover:not(:disabled) {
  background-color: #1e293b;
  transform: translateY(-1px);
  box-shadow: var(--shadow);
}

.o-button--secondary {
  background-color: white;
  border-color: var(--color-border);
  color: var(--text-primary);
}
.o-button--secondary:hover:not(:disabled) {
  background-color: var(--bg-app);
  border-color: var(--color-border-hover);
}

.o-button--ghost {
  background-color: transparent;
  color: var(--text-muted);
}
.o-button--ghost:hover:not(:disabled) {
  background-color: var(--bg-app);
  color: var(--text-primary);
}

.o-button--danger {
  background-color: var(--color-danger-soft);
  color: var(--color-danger);
}
.o-button--danger:hover:not(:disabled) {
  background-color: var(--color-danger);
  color: white;
}

.o-button--success {
  background-color: var(--color-success-soft);
  color: var(--color-success);
}

/* Sizes */
.o-button--sm {
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  border-radius: var(--radius-lg);
}

.o-button--md {
  padding: 0.625rem 1.25rem;
  font-size: 0.875rem;
}

.o-button--lg {
  padding: 0.875rem 1.75rem;
  font-size: 1rem;
  border-radius: var(--radius-xl);
}

.loader {
  width: 1rem;
  height: 1rem;
  border: 2px solid currentColor;
  border-bottom-color: transparent;
  border-radius: 50%;
  display: inline-block;
  animation: rotation 1s linear infinite;
}

@keyframes rotation {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}
</style>
