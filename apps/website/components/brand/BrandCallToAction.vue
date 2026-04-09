<script setup lang="ts">
import { UiSurface } from '@octopus/ui'

defineProps<{
  eyebrow: string
  title: string
  description: string
  primaryHref: string
  primaryLabel: string
  secondaryHref: string
  secondaryLabel: string
}>()
</script>

<template>
  <UiSurface variant="overlay" padding="lg" class="brand-panel overflow-hidden p-8 md:p-10">
    <div class="grid gap-6 md:grid-cols-[1.1fr_0.9fr] md:items-end">
      <div class="space-y-4">
        <div class="brand-metadata">
          {{ eyebrow }}
        </div>
        <h2 class="text-[clamp(2rem,4vw,3.4rem)]">
          {{ title }}
        </h2>
        <p class="max-w-2xl text-base leading-8 text-text-secondary">
          {{ description }}
        </p>
      </div>
      <div class="flex flex-wrap gap-3 md:justify-end">
        <SiteButtonLink :href="primaryHref" :label="primaryLabel" size="lg" />
        <SiteButtonLink :href="secondaryHref" :label="secondaryLabel" variant="outline" size="lg" />
      </div>
    </div>
  </UiSurface>
</template>
