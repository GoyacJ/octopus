<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiField, UiListRow, UiSectionHeading, UiSurface } from '@octopus/ui'

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
  <section class="section-stack">
    <UiSectionHeading :eyebrow="t('teams.header.eyebrow')" :title="t('teams.header.title')" :subtitle="t('teams.header.subtitle')" />

    <div class="surface-grid two">
      <UiSurface :title="t('teams.list.title')" :subtitle="t('teams.list.subtitle')">
        <div v-if="workbench.workspaceTeams.length" class="panel-list">
          <button
            v-for="team in workbench.workspaceTeams"
            :key="team.id"
            type="button"
            class="team-select"
            @click="selectedTeamId = team.id"
          >
            <UiListRow
              :title="resolveMockField('team', team.id, 'name', team.name)"
              :subtitle="resolveMockField('team', team.id, 'description', team.description)"
              :eyebrow="enumLabel('teamScope', team.useScope)"
              :active="team.id === selectedTeamId"
              interactive
            >
              <template #meta>
                <UiBadge :label="team.isProjectCopy ? t('teams.list.projectCopy') : t('teams.list.workspaceTeam')" subtle />
              </template>
            </UiListRow>
          </button>
        </div>
        <UiEmptyState v-else :title="t('teams.list.emptyTitle')" :description="t('teams.list.emptyDescription')" />
      </UiSurface>

      <UiSurface
        v-if="selectedTeam"
        :title="resolveMockField('team', selectedTeam.id, 'name', selectedTeam.name)"
        :subtitle="resolveMockField('team', selectedTeam.id, 'defaultOutput', selectedTeam.defaultOutput)"
      >
        <div class="field-grid">
          <UiField :label="t('teams.form.name')">
            <input v-model="draft.name" type="text" />
          </UiField>
          <UiField :label="t('teams.form.mode')">
            <select v-model="draft.mode">
              <option value="leadered">{{ enumLabel('teamMode', 'leadered') }}</option>
              <option value="hybrid">{{ enumLabel('teamMode', 'hybrid') }}</option>
              <option value="mesh">{{ enumLabel('teamMode', 'mesh') }}</option>
            </select>
          </UiField>
          <UiField :label="t('teams.form.useScope')">
            <select v-model="draft.useScope">
              <option value="workspace">{{ enumLabel('teamScope', 'workspace') }}</option>
              <option value="project">{{ enumLabel('teamScope', 'project') }}</option>
            </select>
          </UiField>
          <UiField :label="t('teams.form.defaultOutput')">
            <input v-model="draft.defaultOutput" type="text" />
          </UiField>
        </div>
        <UiField :label="t('teams.form.description')">
          <textarea v-model="draft.description" rows="4" />
        </UiField>
        <UiField :label="t('teams.form.projectNotes')">
          <textarea v-model="draft.projectNotes" rows="4" />
        </UiField>
        <UiField :label="t('teams.form.members')">
          <input v-model="draft.members" type="text" />
        </UiField>
        <UiField :label="t('teams.form.approvalPreferences')">
          <input v-model="draft.approvalPreferences" type="text" />
        </UiField>
        <div class="action-row">
          <button type="button" class="primary-button" @click="saveTeam">{{ t('common.mockSave') }}</button>
          <button type="button" class="secondary-button" @click="workbench.createProjectTeamCopy(selectedTeam.id)">
            {{ t('common.createProjectCopy') }}
          </button>
        </div>
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.team-select {
  width: 100%;
  text-align: left;
}
</style>
