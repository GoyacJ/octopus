<script setup lang="ts">
import { ArrowRight, Orbit, UsersRound } from 'lucide-vue-next'
import { UiBadge, UiButton, UiRecordCard } from '@octopus/ui'

const props = defineProps<{
  id: string
  name: string
  title: string
  description: string
  leadLabel: string
  members: string[]
  workflow: string[]
  recentOutcome: string
  originLabel?: string
}>()

const emit = defineEmits<{
  open: [id: string]
}>()
</script>

<template>
  <UiRecordCard
    layout="tile"
    interactive
    :title="props.name"
    :description="props.description"
    :test-id="`agent-center-item-team-${props.id}`"
    @click="emit('open', props.id)"
  >
    <template #leading>
      <div class="flex size-full items-center justify-center rounded-[calc(var(--radius-lg)+2px)] bg-primary/[0.08] text-primary">
        <UsersRound :size="20" />
      </div>
    </template>

    <template #eyebrow>
      {{ props.title }}
    </template>

    <template #badges>
      <UiBadge v-if="props.originLabel" :label="props.originLabel" subtle />
    </template>

    <template #secondary>
      <span class="inline-flex items-center gap-2">
        <Orbit :size="14" class="text-primary" />
        协作链路
      </span>
      <span>Lead · {{ props.leadLabel }}</span>
    </template>

    <div class="flex flex-wrap items-center gap-2">
      <UiBadge v-for="step in props.workflow" :key="step" :label="step" subtle />
    </div>

    <template #metrics>
      <div class="flex min-w-0 flex-col gap-1">
        <span class="text-xs font-medium uppercase tracking-[0.08em] text-text-tertiary">编组关系</span>
        <div class="flex flex-wrap gap-2">
          <UiBadge v-for="member in props.members" :key="member" :label="member" subtle />
        </div>
      </div>
      <div class="flex min-w-0 flex-col gap-1">
        <span class="text-xs font-medium uppercase tracking-[0.08em] text-text-tertiary">最近成果</span>
        <strong class="truncate text-sm text-text-primary">{{ props.recentOutcome }}</strong>
      </div>
    </template>

    <template #actions>
      <UiButton as="span" size="sm">
        打开
      </UiButton>
      <UiButton as="span" variant="ghost" size="icon" aria-label="Open team">
        <ArrowRight :size="14" />
      </UiButton>
    </template>
  </UiRecordCard>
</template>
