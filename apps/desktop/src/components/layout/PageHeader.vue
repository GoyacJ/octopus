<template>
  <header class="page-header">
    <div class="header-main">
      <p v-if="eyebrow" class="eyebrow">{{ eyebrow }}</p>
      <h1 v-if="title" class="title">{{ title }}</h1>
      <p v-if="subtitle" class="subtitle">{{ subtitle }}</p>
      <slot name="description"></slot>
    </div>
    <div v-if="$slots.stats || $slots.actions" class="header-aside">
      <div v-if="$slots.stats" class="header-stats">
        <slot name="stats"></slot>
      </div>
      <div v-if="$slots.actions" class="header-actions">
        <slot name="actions"></slot>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
defineProps<{
  eyebrow?: string;
  title?: string;
  subtitle?: string;
}>();
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  margin-bottom: 2.5rem;
  padding-bottom: 1.5rem;
  border-bottom: 1px solid var(--color-border);
  gap: 2rem;
}

.header-main {
  flex: 1;
}

.eyebrow {
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--color-accent);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 0.5rem;
}

.title {
  font-size: 2.25rem;
  font-weight: 800;
  margin: 0 0 0.25rem;
  color: var(--text-primary);
}

.subtitle {
  font-size: 1rem;
  color: var(--text-muted);
  margin: 0;
}

.header-aside {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 1rem;
}

.header-stats {
  display: flex;
  gap: 0.75rem;
}

.header-actions {
  display: flex;
  gap: 0.75rem;
}

@media (max-width: 768px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
  
  .header-aside {
    align-items: flex-start;
    width: 100%;
  }
  
  .header-stats {
    flex-wrap: wrap;
  }
}
</style>
