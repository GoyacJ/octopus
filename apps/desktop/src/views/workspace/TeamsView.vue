<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { TeamRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiEmptyState, UiField, UiInput, UiMetricCard, UiRecordCard, UiSectionHeading, UiSelect, UiTextarea } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const shell = useShellStore()
const teamStore = useTeamStore()
const workspaceStore = useWorkspaceStore()

const selectedTeamId = ref('')
const form = reactive({
  name: '',
  description: '',
  status: 'active',
  memberIds: '',
})

const statusOptions = [
  { value: 'active', label: 'active' },
  { value: 'archived', label: 'archived' },
]

watch(
  () => shell.activeWorkspaceConnectionId,
  (connectionId) => {
    if (connectionId) {
      void teamStore.load(connectionId)
    }
  },
  { immediate: true },
)

watch(
  () => teamStore.workspaceTeams.map(team => team.id).join('|'),
  () => {
    if (!selectedTeamId.value || !teamStore.workspaceTeams.some(team => team.id === selectedTeamId.value)) {
      applyTeam(teamStore.workspaceTeams[0]?.id)
      return
    }
    applyTeam(selectedTeamId.value)
  },
  { immediate: true },
)

const metrics = computed(() => [
  { id: 'total', label: t('teams.metrics.total'), value: String(teamStore.workspaceTeams.length) },
  { id: 'active', label: t('teams.metrics.active'), value: String(teamStore.workspaceTeams.filter(team => team.status === 'active').length) },
])

function applyTeam(teamId?: string) {
  const team = teamStore.workspaceTeams.find(item => item.id === teamId)
  selectedTeamId.value = team?.id ?? ''
  form.name = team?.name ?? ''
  form.description = team?.description ?? ''
  form.status = team?.status ?? 'active'
  form.memberIds = team?.memberIds.join(', ') ?? ''
}

async function saveTeam() {
  if (!workspaceStore.currentWorkspaceId || !form.name.trim()) {
    return
  }

  const record: TeamRecord = {
    id: selectedTeamId.value || `team-${Date.now()}`,
    workspaceId: workspaceStore.currentWorkspaceId,
    scope: 'workspace',
    name: form.name.trim(),
    description: form.description.trim(),
    status: form.status as TeamRecord['status'],
    memberIds: form.memberIds.split(',').map(item => item.trim()).filter(Boolean),
    updatedAt: Date.now(),
  }

  if (selectedTeamId.value) {
    await teamStore.update(selectedTeamId.value, record)
  } else {
    const created = await teamStore.create(record)
    selectedTeamId.value = created.id
  }
}

async function removeTeam() {
  if (!selectedTeamId.value) {
    return
  }
  await teamStore.remove(selectedTeamId.value)
  applyTeam(teamStore.workspaceTeams[0]?.id)
}
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="space-y-4 px-2">
      <UiSectionHeading :eyebrow="t('teams.header.eyebrow')" :title="t('sidebar.navigation.teams')" :subtitle="teamStore.error || t('teams.header.subtitle')" />
      <div class="grid gap-3 sm:grid-cols-2">
        <UiMetricCard v-for="metric in metrics" :key="metric.id" :label="metric.label" :value="metric.value" />
      </div>
    </header>

    <div class="grid gap-6 px-2 xl:grid-cols-[minmax(0,1fr)_360px]">
      <section class="space-y-3">
        <UiRecordCard
          v-for="team in teamStore.workspaceTeams"
          :key="team.id"
          :title="team.name"
          :description="team.description"
          interactive
          class="cursor-pointer"
          :class="selectedTeamId === team.id ? 'ring-1 ring-primary' : ''"
          @click="applyTeam(team.id)"
        >
          <template #badges>
            <UiBadge :label="team.status" subtle />
            <UiBadge :label="`${team.memberIds.length} members`" subtle />
          </template>
          <template #meta>
            <span class="text-xs text-text-tertiary">{{ formatDateTime(team.updatedAt) }}</span>
          </template>
        </UiRecordCard>
        <UiEmptyState v-if="!teamStore.workspaceTeams.length" :title="t('teams.empty.title')" :description="t('teams.empty.description')" />
      </section>

      <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
        <h3 class="text-base font-semibold text-text-primary">{{ selectedTeamId ? t('teams.actions.edit') : t('teams.actions.create') }}</h3>
        <UiField :label="t('teams.fields.name')">
          <UiInput v-model="form.name" />
        </UiField>
        <UiField :label="t('common.status')">
          <UiSelect v-model="form.status" :options="statusOptions" />
        </UiField>
        <UiField :label="t('teams.fields.memberIds')">
          <UiInput v-model="form.memberIds" />
        </UiField>
        <UiField :label="t('teams.fields.description')">
          <UiTextarea v-model="form.description" :rows="6" />
        </UiField>
        <div class="flex gap-3">
          <UiButton @click="saveTeam">{{ t('common.save') }}</UiButton>
          <UiButton variant="ghost" @click="applyTeam()">{{ t('common.reset') }}</UiButton>
          <UiButton v-if="selectedTeamId" variant="ghost" @click="removeTeam">{{ t('common.delete') }}</UiButton>
        </div>
      </section>
    </div>
  </div>
</template>
