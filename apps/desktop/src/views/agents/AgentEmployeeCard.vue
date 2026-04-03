<script setup lang="ts">
import { Sparkles, Zap } from 'lucide-vue-next'
import { UiBadge, UiButton, UiRecordCard } from '@octopus/ui'

const props = defineProps<{
  id: string
  name: string
  title: string
  role: string
  summary: string
  recentTask: string
  avatar: string
  statusLabel: string
  statusTone: 'success' | 'warning' | 'default'
  skills: string[]
  metrics: Array<{ label: string; value: string }>
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
    :description="props.summary"
    :test-id="`agent-center-item-agent-${props.id}`"
    @click="emit('open', props.id)"
  >
    <template #leading>
      <div class="flex size-full items-center justify-center overflow-hidden rounded-[calc(var(--radius-lg)+2px)] bg-primary/[0.08] text-sm font-bold">
        <img v-if="props.avatar.startsWith('data:image/')" :src="props.avatar" alt="" class="size-full object-cover">
        <span v-else>{{ props.avatar.slice(0, 2).toUpperCase() }}</span>
      </div>
    </template>

    <template #eyebrow>
      {{ props.title }}
    </template>

    <template #badges>
      <UiBadge :label="props.statusLabel" :tone="props.statusTone" />
    </template>

    <template #secondary>
      <span>{{ props.role }}</span>
      <UiBadge v-if="props.originLabel" :label="props.originLabel" subtle />
    </template>

    <div class="flex flex-wrap items-center gap-2">
      <UiBadge v-for="skill in props.skills" :key="skill" :label="skill" subtle />
    </div>

    <div class="flex items-center gap-2 rounded-[calc(var(--radius-lg)+2px)] border border-border/60 bg-[color-mix(in_srgb,var(--bg-subtle)_72%,transparent)] px-3 py-2 text-sm text-text-secondary">
      <Sparkles :size="16" class="text-primary" />
      <strong class="min-w-0 truncate text-text-primary">{{ props.recentTask }}</strong>
    </div>

    <template #metrics>
      <div
        v-for="metric in props.metrics"
        :key="metric.label"
        class="flex min-w-0 flex-col gap-1"
      >
        <span class="text-xs font-medium uppercase tracking-[0.08em] text-text-tertiary">{{ metric.label }}</span>
        <strong class="truncate text-sm text-text-primary">{{ metric.value }}</strong>
      </div>
    </template>

    <template #actions>
      <UiButton as="span" size="sm">
        <Zap :size="14" />
        打开
      </UiButton>
    </template>
  </UiRecordCard>
</template>
