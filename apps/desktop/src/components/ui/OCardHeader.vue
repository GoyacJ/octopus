<template>
  <div class="o-card-header">
    <div v-if="$slots.icon" class="o-card-icon">
      <slot name="icon"></slot>
    </div>
    <div class="o-card-title-group">
      <h3 v-if="$slots.title || title" class="o-card-title">
        <slot name="title">{{ title }}</slot>
      </h3>
      <p v-if="$slots.subtitle || subtitle" class="o-card-subtitle">
        <slot name="subtitle">{{ subtitle }}</slot>
      </p>
    </div>
    <div v-if="$slots.actions" class="o-card-actions">
      <slot name="actions"></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  title?: string;
  subtitle?: string;
}>();
</script>

<style scoped>
.o-card-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1.5rem;
  border-bottom: 1px solid var(--color-border);
}

.o-card-icon {
  width: 2.5rem;
  height: 2.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--bg-app);
  border-radius: var(--radius-lg);
  font-size: 1.25rem;
  color: var(--color-accent);
  flex-shrink: 0;
}

.o-card-title-group {
  flex: 1;
}

.o-card-title {
  font-size: 1.125rem;
  font-weight: 700;
  margin: 0;
  color: var(--text-primary);
}

.o-card-subtitle {
  font-size: 0.875rem;
  color: var(--text-muted);
  margin: 0.25rem 0 0;
}

.o-card-actions {
  display: flex;
  gap: 0.5rem;
}
</style>
