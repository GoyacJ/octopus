<script setup lang="ts">
import UiSurface from './UiSurface.vue'

const props = withDefaults(defineProps<{
  title?: string
  subtitle?: string
  class?: string
}>(), {
  title: '',
  subtitle: '',
  class: '',
})
</script>

<template>
  <UiSurface variant="raised" padding="md" :title="props.title" :subtitle="props.subtitle" :class="props.class">
    <slot />
  </UiSurface>
</template>
