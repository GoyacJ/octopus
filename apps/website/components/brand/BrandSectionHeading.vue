<script setup lang="ts">
interface Props {
  eyebrow?: string
  title: string
  description?: string
}

defineProps<Props>()
</script>

<template>
  <header class="max-w-3xl space-y-4">
    <div v-if="eyebrow" class="brand-metadata">
      {{ eyebrow }}
    </div>
    <h2 class="text-[clamp(2.2rem,5vw,4rem)]">
      {{ title }}
    </h2>
    <p v-if="description" class="text-base leading-8 text-text-secondary md:text-[1.05rem]">
      {{ description }}
    </p>
  </header>
</template>
