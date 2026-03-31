<script setup lang="ts">
const props = defineProps<{
  title: string
  description: string
}>()
</script>

<template>
  <div class="ui-empty-state">
    <strong>{{ props.title }}</strong>
    <p>{{ props.description }}</p>
  </div>
</template>

<style scoped>
.ui-empty-state {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px dashed var(--border-strong);
  background: color-mix(in srgb, var(--bg-subtle) 72%, transparent);
}

strong {
  font-size: 0.98rem;
}

p {
  margin: 0;
  color: var(--text-secondary);
  line-height: 1.5;
  overflow-wrap: anywhere;
}
</style>
