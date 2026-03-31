<script setup lang="ts">
const props = defineProps<{
  title: string
  excerpt: string
  typeLabel: string
  versionLabel: string
  statusLabel: string
}>()
</script>

<template>
  <article class="artifact-block">
    <div class="topline">
      <span>{{ props.typeLabel }}</span>
      <span>{{ props.versionLabel }}</span>
    </div>
    <strong>{{ props.title }}</strong>
    <p>{{ props.excerpt }}</p>
    <footer>
      <span>{{ props.statusLabel }}</span>
      <slot name="actions" />
    </footer>
  </article>
</template>

<style scoped>
.artifact-block {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
}

.topline,
footer {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 0.75rem;
  color: var(--text-secondary);
  font-size: 0.8rem;
}

strong {
  font-size: 1rem;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

p {
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

footer {
  align-items: center;
}
</style>
