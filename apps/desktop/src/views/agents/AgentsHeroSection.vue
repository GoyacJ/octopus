<script setup lang="ts">
import { UiBadge, UiPageHero } from '@octopus/ui'

const props = defineProps<{
  title: string
  subtitle: string
  scopeLabel: string
  activeModeLabel: string
}>()
</script>

<template>
  <UiPageHero
    :title="props.title"
    :description="props.subtitle"
    data-testid="agent-center-hero"
  >
    <template #meta>
      <UiBadge :label="props.scopeLabel" subtle />
      <UiBadge :label="props.activeModeLabel" tone="info" />
    </template>

    <div class="space-y-2">
      <span class="block text-[0.72rem] font-semibold uppercase tracking-[0.16em] text-text-tertiary">
        Digital Workforce
      </span>
    </div>
  </UiPageHero>
</template>
