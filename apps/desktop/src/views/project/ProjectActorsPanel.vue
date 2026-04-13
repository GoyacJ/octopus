<script setup lang="ts">
import { computed } from 'vue'

import type { AgentRecord, TeamRecord } from '@octopus/schema'
import { UiButton, UiCheckbox, UiEmptyState, UiField, UiRecordCard, UiStatusCallout } from '@octopus/ui'

const props = defineProps<{
  candidateAgents: AgentRecord[]
  candidateTeams: TeamRecord[]
  projectOwnedAgents: AgentRecord[]
  projectOwnedTeams: TeamRecord[]
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

function actorOriginLabel(record: AgentRecord | TeamRecord) {
  return record.integrationSource?.kind === 'builtin-template' ? '内置模板' : '工作区'
}

function projectOwnedSummary(record: AgentRecord | TeamRecord) {
  return record.description || '项目自有资产，默认加入当前项目会话。'
}
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
      v-if="!projectOwnedAgents.length && !projectOwnedTeams.length && !candidateAgents.length && !candidateTeams.length"
      :title="$t('projectSettings.agents.emptyTitle')"
      :description="$t('projectSettings.agents.emptyDescription')"
    />

    <div v-else class="space-y-6">
      <section v-if="projectOwnedAgents.length || projectOwnedTeams.length" class="space-y-3">
        <UiField
          label="项目自有数字员工与团队"
          hint="这些资产属于当前项目，默认生效，并会直接出现在会话智能体选择里。"
        >
          <div class="space-y-3">
            <div
              v-for="agent in projectOwnedAgents"
              :key="agent.id"
              :data-testid="`project-owned-agent-${agent.id}`"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="flex items-center gap-2 text-sm font-semibold text-text-primary">
                  {{ agent.name }}
                  <span class="text-[11px] font-medium text-text-tertiary">项目内</span>
                </div>
                <div class="text-xs text-text-secondary">
                  {{ projectOwnedSummary(agent) }}
                </div>
              </div>
              <span class="text-[11px] font-medium text-text-tertiary">默认启用</span>
            </div>

            <div
              v-for="team in projectOwnedTeams"
              :key="team.id"
              :data-testid="`project-owned-team-${team.id}`"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="flex items-center gap-2 text-sm font-semibold text-text-primary">
                  {{ team.name }}
                  <span class="text-[11px] font-medium text-text-tertiary">项目内</span>
                </div>
                <div class="text-xs text-text-secondary">
                  {{ projectOwnedSummary(team) }}
                </div>
              </div>
              <span class="text-[11px] font-medium text-text-tertiary">默认启用</span>
            </div>
          </div>
        </UiField>
      </section>

      <section v-if="candidateAgents.length" class="space-y-3">
        <UiField
          :label="$t('projectSettings.agents.agentsLabel')"
          hint="选择要接入当前项目的工作区员工或内置模板。项目自有员工默认生效。"
        >
          <div class="space-y-3">
            <label
              v-for="agent in candidateAgents"
              :key="agent.id"
              :data-testid="`project-agent-option-${agent.id}`"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="flex items-center gap-2 text-sm font-semibold text-text-primary">
                  {{ agent.name }}
                  <span class="text-[11px] font-medium text-text-tertiary">{{ actorOriginLabel(agent) }}</span>
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

      <section v-if="candidateTeams.length" class="space-y-3">
        <UiField
          :label="$t('projectSettings.agents.teamsLabel')"
          hint="选择要接入当前项目的工作区团队或内置模板。项目自有团队默认生效。"
        >
          <div class="space-y-3">
            <label
              v-for="team in candidateTeams"
              :key="team.id"
              :data-testid="`project-team-option-${team.id}`"
              class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
            >
              <div class="min-w-0 space-y-1">
                <div class="flex items-center gap-2 text-sm font-semibold text-text-primary">
                  {{ team.name }}
                  <span class="text-[11px] font-medium text-text-tertiary">{{ actorOriginLabel(team) }}</span>
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
        data-testid="project-settings-agents-save-button"
        :disabled="savingAgents || (!candidateAgents.length && !candidateTeams.length)"
        @click="emit('save')"
      >
        {{ $t('common.save') }}
      </UiButton>
    </template>
  </UiRecordCard>
</template>
