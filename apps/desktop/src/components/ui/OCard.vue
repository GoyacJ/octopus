<script setup lang="ts">
import { computed } from 'vue';

interface Props {
  hover?: boolean;
  variant?: 'default' | 'highlight' | 'glass';
  padding?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  hover: false,
  variant: 'default',
  padding: true,
});

const classes = computed(() => {
  return [
    'o-card',
    `o-card--${props.variant}`,
    { 'o-card--hover': props.hover },
    { 'o-card--no-padding': !props.padding }
  ];
});
</script>

<template>
  <div :class="classes">
    <slot></slot>
  </div>
</template>

<style scoped>
.o-card {
  background-color: var(--bg-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-sm);
  transition: var(--transition);
  overflow: hidden;
}

.o-card--no-padding :deep(.o-card-content) {
  padding: 0;
}

.o-card--hover:hover {
  box-shadow: var(--shadow);
  border-color: var(--color-border-hover);
  transform: translateY(-1px);
}

.o-card--highlight {
  border-color: var(--color-accent-soft);
  background-color: var(--bg-surface);
}

.o-card--glass {
  background-color: var(--bg-surface-glass);
  backdrop-filter: blur(8px);
}
</style>
