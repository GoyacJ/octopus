<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { AutomationRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiEmptyState, UiField, UiInput, UiRecordCard, UiSectionHeading, UiSelect, UiTextarea } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useAgentStore } from '@/stores/agent'
import { useAutomationStore } from '@/stores/automation'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const automationStore = useAutomationStore()
const agentStore = useAgentStore()
const shell = useShellStore()
const teamStore = useTeamStore()
const workspaceStore = useWorkspaceStore()

const selectedAutomationId = ref('')
const form = reactive({
  title: '',
  description: '',
  cadence: '',
  ownerType: 'agent',
  ownerId: '',
  status: 'active',
  output: '',
})

const ownerTypeOptions = [
  { value: 'agent', label: 'agent' },
  { value: 'team', label: 'team' },
]

const statusOptions = [
  { value: 'active', label: 'active' },
  { value: 'paused', label: 'paused' },
  { value: 'error', label: 'error' },
]

const ownerOptions = computed(() =>
  (form.ownerType === 'agent' ? agentStore.workspaceAgents : teamStore.workspaceTeams).map(item => ({
    value: item.id,
    label: item.name,
  })),
)

watch(
  () => shell.activeWorkspaceConnectionId,
  async (connectionId) => {
    if (!connectionId) {
      return
    }

    await Promise.all([
      automationStore.load(connectionId),
      agentStore.load(connectionId),
      teamStore.load(connectionId),
    ])
  },
  { immediate: true },
)

watch(
  () => automationStore.automations.map(record => record.id).join('|'),
  () => {
    if (!selectedAutomationId.value || !automationStore.automations.some(record => record.id === selectedAutomationId.value)) {
      applyAutomation(automationStore.automations[0]?.id)
      return
    }
    applyAutomation(selectedAutomationId.value)
  },
  { immediate: true },
)

function applyAutomation(automationId?: string) {
  const record = automationStore.automations.find(item => item.id === automationId)
  selectedAutomationId.value = record?.id ?? ''
  form.title = record?.title ?? ''
  form.description = record?.description ?? ''
  form.cadence = record?.cadence ?? ''
  form.ownerType = record?.ownerType ?? 'agent'
  form.ownerId = record?.ownerId ?? ''
  form.status = record?.status ?? 'active'
  form.output = record?.output ?? ''
}

async function saveAutomation() {
  if (!workspaceStore.currentWorkspaceId || !form.title.trim() || !form.ownerId) {
    return
  }

  const record: AutomationRecord = {
    id: selectedAutomationId.value || `automation-${Date.now()}`,
    workspaceId: workspaceStore.currentWorkspaceId,
    title: form.title.trim(),
    description: form.description.trim(),
    cadence: form.cadence.trim(),
    ownerType: form.ownerType as AutomationRecord['ownerType'],
    ownerId: form.ownerId,
    status: form.status as AutomationRecord['status'],
    output: form.output.trim(),
    nextRunAt: undefined,
    lastRunAt: undefined,
  }

  if (selectedAutomationId.value) {
    await automationStore.update(selectedAutomationId.value, record)
  } else {
    const created = await automationStore.create(record)
    selectedAutomationId.value = created.id
  }
}

async function removeAutomation() {
  if (!selectedAutomationId.value) {
    return
  }
  await automationStore.remove(selectedAutomationId.value)
  applyAutomation(automationStore.automations[0]?.id)
}
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="px-2">
      <UiSectionHeading :eyebrow="t('automations.header.eyebrow')" :title="t('sidebar.navigation.automations')" :subtitle="automationStore.error || t('automations.header.subtitle')" />
    </header>

    <div class="grid gap-6 px-2 xl:grid-cols-[minmax(0,1fr)_360px]">
      <section class="space-y-3">
        <UiRecordCard
          v-for="automation in automationStore.automations"
          :key="automation.id"
          :title="automation.title"
          :description="automation.description"
          interactive
          class="cursor-pointer"
          :class="selectedAutomationId === automation.id ? 'ring-1 ring-primary' : ''"
          @click="applyAutomation(automation.id)"
        >
          <template #badges>
            <UiBadge :label="automation.status" subtle />
            <UiBadge :label="automation.ownerType" subtle />
          </template>
          <template #meta>
            <span class="text-xs text-text-tertiary">{{ automation.lastRunAt ? formatDateTime(automation.lastRunAt) : automation.cadence }}</span>
          </template>
        </UiRecordCard>
        <UiEmptyState v-if="!automationStore.automations.length" :title="t('automations.empty.title')" :description="t('automations.empty.description')" />
      </section>

      <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
        <h3 class="text-base font-semibold text-text-primary">{{ selectedAutomationId ? t('automations.actions.edit') : t('automations.actions.create') }}</h3>
        <UiField :label="t('automations.fields.title')">
          <UiInput v-model="form.title" />
        </UiField>
        <UiField :label="t('automations.fields.ownerType')">
          <UiSelect v-model="form.ownerType" :options="ownerTypeOptions" />
        </UiField>
        <UiField :label="t('automations.fields.ownerId')">
          <UiSelect v-model="form.ownerId" :options="ownerOptions" />
        </UiField>
        <UiField :label="t('automations.fields.status')">
          <UiSelect v-model="form.status" :options="statusOptions" />
        </UiField>
        <UiField :label="t('automations.fields.cadence')">
          <UiInput v-model="form.cadence" />
        </UiField>
        <UiField :label="t('automations.fields.description')">
          <UiTextarea v-model="form.description" :rows="4" />
        </UiField>
        <UiField :label="t('automations.fields.output')">
          <UiTextarea v-model="form.output" :rows="4" />
        </UiField>
        <div class="flex gap-3">
          <UiButton @click="saveAutomation">{{ t('common.save') }}</UiButton>
          <UiButton variant="ghost" @click="applyAutomation()">{{ t('common.reset') }}</UiButton>
          <UiButton v-if="selectedAutomationId" variant="ghost" @click="removeAutomation">{{ t('common.delete') }}</UiButton>
        </div>
      </section>
    </div>
  </div>
</template>
