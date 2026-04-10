<script setup lang="ts">
import { computed } from 'vue'

import type { AgentRecord, TeamRecord } from '@octopus/schema'
import { UiButton, UiCheckbox, UiEmptyState, UiField, UiRecordCard, UiStatusCallout } from '@octopus/ui'

const props = defineProps<{
  workspaceAssignedAgents: AgentRecord[]
  workspaceAssignedTeams: TeamRecord[]
  enabledAgentIds: string[]
  enabledTeamIds: string[]
  agentsError: string
  savingAgents: boolean
}>()

const emit = defineEmits<{
  reset: []
  save: []
  'update:enabled-agent-ids': [value: string[]]
  'update:enabled-team-ids': [value: string[]]
}>()

const enabledAgentIdsModel = computed({
  get: () => props.enabledAgentIds,
  set: value => emit('update:enabled-agent-ids', value),
})

const enabledTeamIdsModel = computed({
  get: () => props.enabledTeamIds,
  set: value => emit('update:enabled-team-ids', value),
})
</script>

<template>
  <UiRecordCard
    :title="$t('projectSettings.agents.title')"
    :description="$t('projectSettings.agents.description')"
  >
    <template #eyebrow>
      {{ $t('projectSettings.tabs.agents') }}
    </template>

    <UiEmptyState
      v-if="!workspaceAssignedAgents.length && !workspaceAssignedTeams.length"
      :title="$t('projectSettings.agents.emptyTitle')"
      :description="$t('projectSettings.agents.emptyDescription')"
    />

    <div v-else class="space-y-6">
      <section v-if="workspaceAssignedAgents.length" class="space-y-3">
        <UiField
          :label="$t('projectSettings.agents.agentsLabel')"
          :hint="$t('projectSettings.agents.agentsHint')"
        >
          <div class="space-y-3">
            <label
              v-for="agent in workspaceAssignedAgents"
              :key="agent.id"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="text-sm font-semibold text-text-primary">
                  {{ agent.name }}
                </div>
                <div class="text-xs text-text-secondary">
                  {{ agent.description || $t('common.na') }}
                </div>
              </div>
              <UiCheckbox
                v-model="enabledAgentIdsModel"
                :value="agent.id"
                :aria-label="agent.name"
              />
            </label>
          </div>
        </UiField>
      </section>

      <section v-if="workspaceAssignedTeams.length" class="space-y-3">
        <UiField
          :label="$t('projectSettings.agents.teamsLabel')"
          :hint="$t('projectSettings.agents.teamsHint')"
        >
          <div class="space-y-3">
            <label
              v-for="team in workspaceAssignedTeams"
              :key="team.id"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="text-sm font-semibold text-text-primary">
                  {{ team.name }}
                </div>
                <div class="text-xs text-text-secondary">
                  {{ team.description || $t('common.na') }}
                </div>
              </div>
              <UiCheckbox
                v-model="enabledTeamIdsModel"
                :value="team.id"
                :aria-label="team.name"
              />
            </label>
          </div>
        </UiField>
      </section>

      <UiStatusCallout v-if="agentsError" tone="error" :description="agentsError" />
    </div>

    <template #actions>
      <UiButton variant="ghost" :disabled="savingAgents" @click="emit('reset')">
        {{ $t('common.reset') }}
      </UiButton>
      <UiButton
        :disabled="savingAgents || (!workspaceAssignedAgents.length && !workspaceAssignedTeams.length)"
        @click="emit('save')"
      >
        {{ $t('common.save') }}
      </UiButton>
    </template>
  </UiRecordCard>
</template>
