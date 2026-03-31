<script setup lang="ts">
const props = defineProps<{
  eyebrow?: string
  title?: string
  subtitle?: string
}>()
</script>

<template>
  <section class="ui-surface">
    <header v-if="props.eyebrow || props.title || props.subtitle" class="surface-header">
      <p v-if="props.eyebrow" class="eyebrow">{{ props.eyebrow }}</p>
      <h2 v-if="props.title">{{ props.title }}</h2>
      <p v-if="props.subtitle" class="subtitle">{{ props.subtitle }}</p>
    </header>
    <slot />
  </section>
</template>

<style scoped>
.ui-surface {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-width: 0;
  padding: 1.1rem;
  border-radius: var(--radius-xl);
  border: 1px solid var(--border-subtle);
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 10%, transparent), transparent 35%),
    var(--bg-surface);
  box-shadow: var(--shadow-md);
}

.surface-header {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  min-width: 0;
}

.eyebrow {
  margin: 0;
  color: var(--brand-primary);
  font-size: 0.74rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  font-weight: 700;
}

h2 {
  margin: 0;
  font-size: 1.1rem;
  line-height: 1.3;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.subtitle {
  margin: 0;
  color: var(--text-secondary);
  line-height: 1.5;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow-wrap: anywhere;
}
</style>
