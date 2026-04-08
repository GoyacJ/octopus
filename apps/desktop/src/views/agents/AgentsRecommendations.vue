<script setup lang="ts">
import { ArrowRight, Sparkles, UsersRound } from 'lucide-vue-next'
import { UiButton, UiRecordCard, UiSectionHeading, UiSurface } from '@octopus/ui'

interface RecommendationEmployee {
  id: string
  name: string
  summary: string
}

interface RecommendationTeam {
  id: string
  name: string
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

          <template #actions>
            <UiButton variant="ghost" size="sm" class="gap-1 px-2 text-xs text-text-secondary" @click.stop="emit('openAgent', employee.id)">
              打开
              <ArrowRight :size="14" />
            </UiButton>
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
          @click="emit('openTeam', team.id)"
        >
          <template #leading>
            <div class="flex size-full items-center justify-center rounded-[calc(var(--radius-lg)+2px)] bg-primary/[0.08] text-primary">
              <UsersRound :size="18" />
            </div>
          </template>

          <template #actions>
            <UiButton variant="ghost" size="sm" class="gap-1 px-2 text-xs text-text-secondary" @click.stop="emit('openTeam', team.id)">
              查看
              <ArrowRight :size="14" />
            </UiButton>
          </template>
        </UiRecordCard>
      </div>
    </UiSurface>
  </section>
</template>
