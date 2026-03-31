<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiField, UiListRow, UiSectionHeading, UiSurface } from '@octopus/ui'

import { enumLabel, resolveMockField, resolveMockList } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const selectedAgentId = ref(workbench.workspaceAgents[0]?.id ?? '')
const draft = ref({
  name: '',
  role: '',
  systemPrompt: '',
  scope: 'workspace',
  owner: '',
  model: '',
  tools: '',
  sharedSources: '',
  autonomyLevel: '',
  approvalPreferences: '',
})

const selectedAgent = computed(() =>
  workbench.workspaceAgents.find((agent) => agent.id === selectedAgentId.value),
)

function syncDraft() {
  const agent = selectedAgent.value
  if (!agent) {
    return
  }

  draft.value = {
    name: resolveMockField('agent', agent.id, 'name', agent.name),
    role: resolveMockField('agent', agent.id, 'role', agent.role),
    systemPrompt: resolveMockField('agent', agent.id, 'systemPrompt', agent.systemPrompt),
    scope: agent.scope,
    owner: agent.owner,
    model: agent.capabilityPolicy.model,
    tools: agent.capabilityPolicy.tools.join(', '),
    sharedSources: resolveMockList('agent', agent.id, 'knowledgeScope.sharedSources', agent.knowledgeScope.sharedSources).join(', '),
    autonomyLevel: agent.executionProfile.autonomyLevel,
    approvalPreferences: agent.approvalPreferences.join(', '),
  }
}

watch(
  () => workbench.workspaceAgents.map((agent) => agent.id).join('|'),
  () => {
    if (!selectedAgentId.value && workbench.workspaceAgents[0]) {
      selectedAgentId.value = workbench.workspaceAgents[0].id
    }
    syncDraft()
  },
  { immediate: true },
)

watch(selectedAgentId, () => {
  syncDraft()
})

function splitList(value: string): string[] {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

function saveAgent() {
  const agent = selectedAgent.value
  if (!agent) {
    return
  }

  workbench.updateAgent(agent.id, {
    name: draft.value.name,
    role: draft.value.role,
    systemPrompt: draft.value.systemPrompt,
    scope: draft.value.scope as typeof agent.scope,
    owner: draft.value.owner,
    capabilityPolicy: {
      ...agent.capabilityPolicy,
      model: draft.value.model,
      tools: splitList(draft.value.tools),
    },
    knowledgeScope: {
      ...agent.knowledgeScope,
      sharedSources: splitList(draft.value.sharedSources),
    },
    executionProfile: {
      ...agent.executionProfile,
      autonomyLevel: draft.value.autonomyLevel,
    },
    approvalPreferences: splitList(draft.value.approvalPreferences),
  })
}
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading :eyebrow="t('agents.header.eyebrow')" :title="t('agents.header.title')" :subtitle="t('agents.header.subtitle')" />

    <div class="surface-grid two">
      <UiSurface :title="t('agents.list.title')" :subtitle="t('agents.list.subtitle')">
        <div v-if="workbench.workspaceAgents.length" class="panel-list">
          <button
            v-for="agent in workbench.workspaceAgents"
            :key="agent.id"
            type="button"
            class="agent-select"
            @click="selectedAgentId = agent.id"
          >
            <UiListRow
              :title="resolveMockField('agent', agent.id, 'name', agent.name)"
              :subtitle="resolveMockField('agent', agent.id, 'role', agent.role)"
              :eyebrow="enumLabel('agentScope', agent.scope)"
              :active="agent.id === selectedAgentId"
              interactive
            >
              <template #meta>
                <UiBadge :label="agent.isProjectCopy ? t('agents.list.projectCopy') : t('agents.list.workspaceTemplate')" subtle />
              </template>
            </UiListRow>
          </button>
        </div>
        <UiEmptyState v-else :title="t('agents.list.emptyTitle')" :description="t('agents.list.emptyDescription')" />
      </UiSurface>

      <UiSurface
        v-if="selectedAgent"
        :title="resolveMockField('agent', selectedAgent.id, 'name', selectedAgent.name)"
        :subtitle="resolveMockList('agent', selectedAgent.id, 'persona', selectedAgent.persona).join(' / ')"
      >
        <div class="field-grid">
          <UiField :label="t('agents.form.name')">
            <input v-model="draft.name" type="text" />
          </UiField>
          <UiField :label="t('agents.form.role')">
            <input v-model="draft.role" type="text" />
          </UiField>
          <UiField :label="t('agents.form.scope')">
            <select v-model="draft.scope">
              <option value="personal">{{ t('agents.form.scopeOptions.personal') }}</option>
              <option value="workspace">{{ t('agents.form.scopeOptions.workspace') }}</option>
              <option value="project">{{ t('agents.form.scopeOptions.project') }}</option>
            </select>
          </UiField>
          <UiField :label="t('agents.form.owner')">
            <input v-model="draft.owner" type="text" />
          </UiField>
          <UiField :label="t('agents.form.model')">
            <input v-model="draft.model" type="text" />
          </UiField>
          <UiField :label="t('agents.form.autonomy')">
            <input v-model="draft.autonomyLevel" type="text" />
          </UiField>
        </div>
        <UiField :label="t('agents.form.systemPrompt')">
          <textarea v-model="draft.systemPrompt" rows="5" />
        </UiField>
        <UiField :label="t('agents.form.tools')">
          <input v-model="draft.tools" type="text" />
        </UiField>
        <UiField :label="t('agents.form.sharedSources')">
          <input v-model="draft.sharedSources" type="text" />
        </UiField>
        <UiField :label="t('agents.form.approvalPreferences')">
          <input v-model="draft.approvalPreferences" type="text" />
        </UiField>
        <div class="action-row">
          <button type="button" class="primary-button" @click="saveAgent">{{ t('common.mockSave') }}</button>
          <button type="button" class="secondary-button" @click="workbench.createProjectAgentCopy(selectedAgent.id)">
            {{ t('common.createProjectCopy') }}
          </button>
        </div>
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.agent-select {
  width: 100%;
  text-align: left;
}
</style>
