<script setup lang="ts">
import { UiSurface } from '@octopus/ui'

interface ProofItem {
  label: string
  value: string
  description: string
}

defineProps<{
  items: ProofItem[]
}>()
</script>

<template>
  <div class="grid gap-4 md:grid-cols-3">
    <UiSurface
      v-for="item in items"
      :key="item.label"
      variant="raised"
      padding="lg"
      class="brand-panel border-border-subtle/80"
    >
      <div class="space-y-3">
        <p class="text-[12px] font-semibold uppercase tracking-[0.12em] text-text-tertiary">
          {{ item.label }}
        </p>
        <p class="font-display text-4xl text-text-primary">
          {{ item.value }}
        </p>
        <p class="text-sm leading-7 text-text-secondary">
          {{ item.description }}
        </p>
      </div>
    </UiSurface>
  </div>
</template>
