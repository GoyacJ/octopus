<script setup lang="ts">
import { UiBadge, UiButton, UiPageHero } from '@octopus/ui'

const props = defineProps<{
  title: string
  subtitle: string
  scopeLabel: string
  activeModeLabel: string
}>()

const emit = defineEmits<{
  createAgent: []
  createTeam: []
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

    <template #actions>
      <UiButton data-testid="agent-center-create-agent" @click="emit('createAgent')">
        新建员工
      </UiButton>
      <UiButton variant="outline" data-testid="agent-center-create-team" @click="emit('createTeam')">
        新建团队
      </UiButton>
    </template>
  </UiPageHero>
</template>
