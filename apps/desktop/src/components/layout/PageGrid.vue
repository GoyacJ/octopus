<template>
  <div class="page-grid" :class="[`page-grid--${cols}`]">
    <div class="grid-main">
      <slot name="main"></slot>
    </div>
    <aside v-if="$slots.side" class="grid-side">
      <slot name="side"></slot>
    </aside>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  cols?: '1' | '2' | '1-side';
}>(), {
  cols: '1-side',
});
</script>

<style scoped>
.page-grid {
  display: grid;
  gap: 1.5rem;
  align-items: start;
}

.page-grid--1 {
  grid-template-columns: 1fr;
}

.page-grid--2 {
  grid-template-columns: 1fr 1fr;
}

.page-grid--1-side {
  grid-template-columns: 1fr 320px;
}

.grid-main, .grid-side {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

@media (max-width: 1024px) {
  .page-grid--1-side {
    grid-template-columns: 1fr;
  }
  
  .grid-side {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1rem;
  }
}
</style>
