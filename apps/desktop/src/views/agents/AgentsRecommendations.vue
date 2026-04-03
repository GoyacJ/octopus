<script setup lang="ts">
import { ArrowRight, Sparkles, UsersRound } from 'lucide-vue-next'
import { UiBadge, UiRecordCard, UiSectionHeading, UiSurface } from '@octopus/ui'

interface RecommendationEmployee {
  id: string
  name: string
  title: string
  summary: string
}

interface RecommendationTeam {
  id: string
  name: string
  title: string
  workflow: string[]
}

const props = defineProps<{
  employees: RecommendationEmployee[]
  teams: RecommendationTeam[]
}>()

const emit = defineEmits<{
  openAgent: [id: string]
  openTeam: [id: string]
}>()
</script>

<template>
  <section class="grid gap-4 xl:grid-cols-1" data-testid="agent-center-recommendations">
    <UiSurface variant="panel" padding="md" class="flex flex-col gap-4">
      <UiSectionHeading eyebrow="Recent picks" title="最近常用员工" subtitle="快速打开常用对象。" />
      <div class="flex flex-col gap-3">
        <UiRecordCard
          v-for="employee in props.employees"
          :key="employee.id"
          layout="compact"
          interactive
          :title="employee.name"
          :description="employee.summary"
          @click="emit('openAgent', employee.id)"
        >
          <template #leading>
            <div class="flex size-full items-center justify-center rounded-[calc(var(--radius-lg)+2px)] bg-primary/[0.08] text-primary">
              <Sparkles :size="18" />
            </div>
          </template>

          <template #eyebrow>
            {{ employee.title }}
          </template>

          <template #actions>
            <span class="inline-flex items-center gap-1 text-sm text-text-secondary">
              打开
              <ArrowRight :size="14" />
            </span>
          </template>
        </UiRecordCard>
      </div>
    </UiSurface>

    <UiSurface variant="panel" padding="md" class="flex flex-col gap-4">
      <UiSectionHeading eyebrow="Suggested squads" title="推荐团队" subtitle="快速查看推荐协作单元。" />
      <div class="flex flex-col gap-3">
        <UiRecordCard
          v-for="team in props.teams"
          :key="team.id"
          layout="compact"
          interactive
          :title="team.name"
          :description="team.title"
          @click="emit('openTeam', team.id)"
        >
          <template #leading>
            <div class="flex size-full items-center justify-center rounded-[calc(var(--radius-lg)+2px)] bg-primary/[0.08] text-primary">
              <UsersRound :size="18" />
            </div>
          </template>

          <template #secondary>
            <div class="flex flex-wrap gap-2">
              <UiBadge v-for="step in team.workflow" :key="step" :label="step" subtle />
            </div>
          </template>

          <template #actions>
            <span class="inline-flex items-center gap-1 text-sm text-text-secondary">
              查看
              <ArrowRight :size="14" />
            </span>
          </template>
        </UiRecordCard>
      </div>
    </UiSurface>
  </section>
</template>
