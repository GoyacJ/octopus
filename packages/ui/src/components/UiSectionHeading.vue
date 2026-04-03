<script setup lang="ts">
const props = defineProps<{
  eyebrow?: string
  title: string
  subtitle?: string
}>()
</script>

<template>
  <header class="flex flex-col gap-1 min-w-0">
    <p v-if="props.eyebrow" class="m-0 text-text-tertiary text-[10px] font-bold tracking-[0.1em] uppercase">
      {{ props.eyebrow }}
    </p>
    <h2 class="m-0 text-lg font-bold tracking-tight text-text-primary min-w-0 truncate">
      {{ props.title }}
    </h2>
    <p v-if="props.subtitle" class="m-0 max-w-3xl text-[13px] leading-relaxed text-text-secondary min-w-0 break-words line-clamp-2">
      {{ props.subtitle }}
    </p>
  </header>
</template>
