<template>
  <div class="o-card-footer" :class="{ 'o-card-footer--border': border }">
    <slot></slot>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  border?: boolean;
}>(), {
  border: false,
});
</script>

<style scoped>
.o-card-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 1rem 1.5rem;
  gap: 0.75rem;
}

.o-card-footer--border {
  border-top: 1px solid var(--color-border);
}
</style>
