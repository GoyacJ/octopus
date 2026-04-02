<script setup lang="ts">
const props = defineProps<{
  eyebrow?: string
  title: string
  subtitle?: string
}>()
</script>

<template>
  <header class="ui-section-heading">
    <p v-if="props.eyebrow" class="eyebrow">{{ props.eyebrow }}</p>
    <h2>{{ props.title }}</h2>
    <p v-if="props.subtitle" class="subtitle">{{ props.subtitle }}</p>
  </header>
</template>

<style scoped>
.ui-section-heading {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
}

.eyebrow {
  margin: 0;
  color: var(--brand-primary);
  font-size: 0.72rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  font-weight: 700;
}

h2 {
  margin: 0;
  font-size: clamp(1.28rem, 1.8vw, 1.9rem);
  line-height: 1.12;
  letter-spacing: -0.03em;
  font-weight: 700;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.subtitle {
  margin: 0;
  max-width: 60rem;
  color: var(--text-secondary);
  font-size: 0.95rem;
  line-height: 1.6;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow-wrap: anywhere;
}
</style>
