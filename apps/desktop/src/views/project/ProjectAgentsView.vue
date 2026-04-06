<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import type { AgentRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiEmptyState, UiField, UiInput, UiMetricCard, UiRecordCard, UiSectionHeading, UiSelect, UiTextarea } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useAgentStore } from '@/stores/agent'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const agentStore = useAgentStore()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()

const selectedAgentId = ref('')
const form = reactive({
  name: '',
  title: '',
  description: '',
  status: 'active',
})

const statusOptions = [
  { value: 'active', label: 'active' },
  { value: 'archived', label: 'archived' },
]

watch(
  () => [shell.activeWorkspaceConnectionId, route.params.projectId],
  ([connectionId]) => {
    if (typeof connectionId === 'string' && connectionId) {
      void agentStore.load(connectionId)
    }
  },
  { immediate: true },
)

watch(
  () => [route.params.projectId, agentStore.projectAgents.map(agent => agent.id).join('|')],
  () => {
    if (!selectedAgentId.value || !agentStore.projectAgents.some(agent => agent.id === selectedAgentId.value)) {
      applyAgent(agentStore.projectAgents[0]?.id)
      return
    }
    applyAgent(selectedAgentId.value)
  },
  { immediate: true },
)

const metrics = computed(() => [
  { id: 'total', label: t('agents.stats.total'), value: String(agentStore.projectAgents.length) },
  { id: 'active', label: t('agents.stats.active'), value: String(agentStore.projectAgents.filter(agent => agent.status === 'active').length) },
])

function applyAgent(agentId?: string) {
  const agent = agentStore.projectAgents.find(item => item.id === agentId)
  selectedAgentId.value = agent?.id ?? ''
  form.name = agent?.name ?? ''
  form.title = agent?.title ?? ''
  form.description = agent?.description ?? ''
  form.status = agent?.status ?? 'active'
}

async function saveAgent() {
  if (!workspaceStore.currentWorkspaceId || !workspaceStore.currentProjectId || !form.name.trim()) {
    return
  }

  const record: AgentRecord = {
    id: selectedAgentId.value || `agent-${Date.now()}`,
    workspaceId: workspaceStore.currentWorkspaceId,
    projectId: workspaceStore.currentProjectId,
    scope: 'project',
    name: form.name.trim(),
    title: form.title.trim(),
    description: form.description.trim(),
    status: form.status as AgentRecord['status'],
    updatedAt: Date.now(),
  }

  if (selectedAgentId.value) {
    await agentStore.update(selectedAgentId.value, record)
  } else {
    const created = await agentStore.create(record)
    selectedAgentId.value = created.id
  }
}

async function removeAgent() {
  if (!selectedAgentId.value) {
    return
  }
  await agentStore.remove(selectedAgentId.value)
  applyAgent(agentStore.projectAgents[0]?.id)
}
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="space-y-4 px-2">
      <UiSectionHeading :eyebrow="t('agents.header.eyebrow')" :title="workspaceStore.activeProject?.name ?? t('agents.project.titleFallback')" :subtitle="agentStore.error || workspaceStore.activeProject?.description || t('agents.project.descriptionAgent')" />
      <div class="grid gap-3 sm:grid-cols-2">
        <UiMetricCard v-for="metric in metrics" :key="metric.id" :label="metric.label" :value="metric.value" />
      </div>
    </header>

    <div class="grid gap-6 px-2 xl:grid-cols-[minmax(0,1fr)_360px]">
      <section class="space-y-3">
        <UiRecordCard
          v-for="agent in agentStore.projectAgents"
          :key="agent.id"
          :title="agent.name"
          :description="agent.description"
          interactive
          class="cursor-pointer"
          :class="selectedAgentId === agent.id ? 'ring-1 ring-primary' : ''"
          @click="applyAgent(agent.id)"
        >
          <template #badges>
            <UiBadge :label="agent.status" subtle />
          </template>
          <template #meta>
            <span class="text-xs text-text-tertiary">{{ formatDateTime(agent.updatedAt) }}</span>
          </template>
        </UiRecordCard>
        <UiEmptyState v-if="!agentStore.projectAgents.length" :title="t('agents.empty.projectTitle')" :description="t('agents.empty.projectDescription')" />
      </section>

      <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
        <h3 class="text-base font-semibold text-text-primary">{{ selectedAgentId ? t('agents.actions.edit') : t('agents.actions.create') }}</h3>
        <UiField :label="t('agents.fields.name')">
          <UiInput v-model="form.name" />
        </UiField>
        <UiField :label="t('agents.fields.title')">
          <UiInput v-model="form.title" />
        </UiField>
        <UiField :label="t('common.status')">
          <UiSelect v-model="form.status" :options="statusOptions" />
        </UiField>
        <UiField :label="t('agents.fields.description')">
          <UiTextarea v-model="form.description" :rows="6" />
        </UiField>
        <div class="flex gap-3">
          <UiButton @click="saveAgent">{{ t('common.save') }}</UiButton>
          <UiButton variant="ghost" @click="applyAgent()">{{ t('common.reset') }}</UiButton>
          <UiButton v-if="selectedAgentId" variant="ghost" @click="removeAgent">{{ t('common.delete') }}</UiButton>
        </div>
      </section>
    </div>
  </div>
</template>
