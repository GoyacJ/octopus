<script setup lang="ts">
import { UiBadge, UiEmptyState, UiSurface } from '@octopus/ui'

const props = defineProps<{
  title: string
  description: string
  variantLabel: string
}>()
</script>

<template>
  <UiSurface variant="subtle" padding="lg" class="flex flex-col gap-4">
    <UiBadge :label="props.variantLabel" subtle />
    <UiEmptyState :title="props.title" :description="props.description" class="min-h-60" />
  </UiSurface>
</template>
