<script setup lang="ts">
interface Props {
  eyebrow: string
  title: string
  description: string
  src: string
  alt: string
}

defineProps<Props>()
</script>

<template>
  <article class="brand-panel overflow-hidden">
    <div class="border-b border-border-subtle/70 px-5 py-4 md:px-6">
      <p class="text-[11px] font-semibold uppercase tracking-[0.16em] text-text-tertiary">
        {{ eyebrow }}
      </p>
      <div class="mt-2 space-y-2">
        <h3 class="text-[1.5rem] leading-tight">
          {{ title }}
        </h3>
        <p class="text-sm leading-7 text-text-secondary">
          {{ description }}
        </p>
      </div>
    </div>
    <div class="bg-subtle/70 p-3 md:p-4">
      <img :src="src" :alt="alt" class="w-full rounded-[20px] border border-border-subtle/80 shadow-sm">
    </div>
  </article>
</template>
