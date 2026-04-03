<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiButton, UiEmptyState, UiField, UiInput, UiListRow, UiSectionHeading, UiSelect, UiTextarea } from '@octopus/ui'

import { enumLabel, resolveMockField } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const selectedTeamId = ref(workbench.workspaceTeams[0]?.id ?? '')
const draft = ref({
  name: '',
  description: '',
  mode: 'leadered',
  useScope: 'workspace',
  defaultOutput: '',
  projectNotes: '',
  members: '',
  approvalPreferences: '',
})

const selectedTeam = computed(() =>
  workbench.workspaceTeams.find((team) => team.id === selectedTeamId.value),
)
const modeOptions = computed(() => [
  { value: 'leadered', label: enumLabel('teamMode', 'leadered') },
  { value: 'hybrid', label: enumLabel('teamMode', 'hybrid') },
  { value: 'mesh', label: enumLabel('teamMode', 'mesh') },
])
const scopeOptions = computed(() => [
  { value: 'workspace', label: enumLabel('teamScope', 'workspace') },
  { value: 'project', label: enumLabel('teamScope', 'project') },
])

function splitList(value: string): string[] {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

function syncDraft() {
  const team = selectedTeam.value
  if (!team) {
    return
  }

  draft.value = {
    name: resolveMockField('team', team.id, 'name', team.name),
    description: resolveMockField('team', team.id, 'description', team.description),
    mode: team.mode,
    useScope: team.useScope,
    defaultOutput: resolveMockField('team', team.id, 'defaultOutput', team.defaultOutput),
    projectNotes: resolveMockField('team', team.id, 'projectNotes', team.projectNotes),
    members: team.members.join(', '),
    approvalPreferences: team.approvalPreferences.join(', '),
  }
}

watch(
  () => workbench.workspaceTeams.map((team) => team.id).join('|'),
  () => {
    if (!selectedTeamId.value && workbench.workspaceTeams[0]) {
      selectedTeamId.value = workbench.workspaceTeams[0].id
    }
    syncDraft()
  },
  { immediate: true },
)

watch(selectedTeamId, () => {
  syncDraft()
})

function saveTeam() {
  const team = selectedTeam.value
  if (!team) {
    return
  }

  workbench.updateTeam(team.id, {
    name: draft.value.name,
    description: draft.value.description,
    mode: draft.value.mode as typeof team.mode,
    useScope: draft.value.useScope as typeof team.useScope,
    defaultOutput: draft.value.defaultOutput,
    projectNotes: draft.value.projectNotes,
    members: splitList(draft.value.members),
    approvalPreferences: splitList(draft.value.approvalPreferences),
  })
}
</script>

<template>
  <div class="w-full flex flex-col gap-8 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0">
      <UiSectionHeading 
        :eyebrow="t('teams.header.eyebrow')" 
        :title="t('teams.header.title')" 
        :subtitle="t('teams.header.subtitle')" 
      />
    </header>

    <div class="flex flex-1 min-h-0 gap-8 px-2">
      <!-- Left: List -->
      <aside class="flex flex-col w-80 shrink-0 border-r border-border-subtle pr-8">
        <div class="space-y-1 mb-4">
          <h3 class="text-sm font-bold text-text-primary">{{ t('teams.list.title') }}</h3>
          <p class="text-[11px] text-text-tertiary">{{ t('teams.list.subtitle') }}</p>
        </div>

        <div data-testid="teams-list" class="flex-1 overflow-y-auto min-h-0 space-y-1 pr-2">
          <UiListRow
            v-for="team in workbench.workspaceTeams"
            :key="team.id"
            :data-testid="`team-row-${team.id}`"
            :title="resolveMockField('team', team.id, 'name', team.name)"
            :subtitle="resolveMockField('team', team.id, 'description', team.description)"
            :eyebrow="enumLabel('teamScope', team.useScope)"
            :active="team.id === selectedTeamId"
            interactive
            @click="selectedTeamId = team.id"
          >
            <template #meta>
              <UiBadge :label="team.isProjectCopy ? t('teams.list.projectCopy') : t('teams.list.workspaceTeam')" subtle />
            </template>
          </UiListRow>

          <UiEmptyState 
            v-if="!workbench.workspaceTeams.length" 
            :title="t('teams.list.emptyTitle')" 
            :description="t('teams.list.emptyDescription')" 
          />
        </div>
      </aside>

      <!-- Right: Form -->
      <main class="flex-1 overflow-y-auto min-h-0 pb-8">
        <template v-if="selectedTeam">
          <header class="space-y-2 mb-8">
            <h2 class="text-2xl font-bold text-text-primary">{{ resolveMockField('team', selectedTeam.id, 'name', selectedTeam.name) }}</h2>
            <p class="text-[14px] text-text-secondary leading-relaxed">{{ resolveMockField('team', selectedTeam.id, 'defaultOutput', selectedTeam.defaultOutput) }}</p>
          </header>

          <div class="grid gap-x-8 gap-y-6 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            <UiField :label="t('teams.form.name')">
              <UiInput v-model="draft.name" data-testid="teams-form-name" />
            </UiField>
            <UiField :label="t('teams.form.mode')">
              <UiSelect v-model="draft.mode" :options="modeOptions" />
            </UiField>
            <UiField :label="t('teams.form.useScope')">
              <UiSelect v-model="draft.useScope" :options="scopeOptions" />
            </UiField>
            <UiField :label="t('teams.form.defaultOutput')">
              <UiInput v-model="draft.defaultOutput" />
            </UiField>

            <UiField class="md:col-span-2 lg:col-span-2" :label="t('teams.form.description')">
              <UiTextarea v-model="draft.description" :rows="3" />
            </UiField>
            <UiField class="md:col-span-2 lg:col-span-2" :label="t('teams.form.projectNotes')">
              <UiTextarea v-model="draft.projectNotes" :rows="3" />
            </UiField>
            
            <UiField class="md:col-span-2" :label="t('teams.form.members')">
              <UiInput v-model="draft.members" />
            </UiField>
            <UiField class="md:col-span-2" :label="t('teams.form.approvalPreferences')">
              <UiInput v-model="draft.approvalPreferences" />
            </UiField>
          </div>

          <div class="mt-8 pt-6 border-t border-border-subtle flex flex-wrap gap-3">
            <UiButton variant="primary" data-testid="teams-form-save" @click="saveTeam">{{ t('common.mockSave') }}</UiButton>
            <UiButton variant="ghost" @click="workbench.createProjectTeamCopy(selectedTeam.id)">
              {{ t('common.createProjectCopy') }}
            </UiButton>
          </div>
        </template>
      </main>
    </div>
  </div>
</template>
