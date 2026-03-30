<template>
  <div class="page-container" :class="{ 'page-container--narrow': narrow }">
    <slot></slot>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  narrow?: boolean;
}>(), {
  narrow: false,
});
</script>

<style scoped>
.page-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1rem;
}

.page-container--narrow {
  max-width: 800px;
}
</style>
