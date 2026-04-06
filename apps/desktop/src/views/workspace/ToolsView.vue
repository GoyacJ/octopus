<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { ToolRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiEmptyState, UiField, UiInput, UiRecordCard, UiSectionHeading, UiSelect, UiTextarea } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useCatalogStore } from '@/stores/catalog'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const catalogStore = useCatalogStore()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()

const selectedToolId = ref('')
const form = reactive({
  name: '',
  kind: 'builtin',
  description: '',
  status: 'active',
  permissionMode: 'ask',
})

const kindOptions = [
  { value: 'builtin', label: 'builtin' },
  { value: 'skill', label: 'skill' },
  { value: 'mcp', label: 'mcp' },
]

const statusOptions = [
  { value: 'active', label: 'active' },
  { value: 'disabled', label: 'disabled' },
]

const permissionOptions = [
  { value: 'ask', label: 'ask' },
  { value: 'readonly', label: 'readonly' },
  { value: 'allow', label: 'allow' },
  { value: 'deny', label: 'deny' },
]

watch(
  () => shell.activeWorkspaceConnectionId,
  (connectionId) => {
    if (connectionId) {
      void catalogStore.load(connectionId)
    }
  },
  { immediate: true },
)

watch(
  () => catalogStore.tools.map(tool => tool.id).join('|'),
  () => {
    if (!selectedToolId.value || !catalogStore.tools.some(tool => tool.id === selectedToolId.value)) {
      applyTool(catalogStore.tools[0]?.id)
      return
    }
    applyTool(selectedToolId.value)
  },
  { immediate: true },
)

function applyTool(toolId?: string) {
  const tool = catalogStore.tools.find(item => item.id === toolId)
  selectedToolId.value = tool?.id ?? ''
  form.name = tool?.name ?? ''
  form.kind = tool?.kind ?? 'builtin'
  form.description = tool?.description ?? ''
  form.status = tool?.status ?? 'active'
  form.permissionMode = tool?.permissionMode ?? 'ask'
}

async function saveTool() {
  if (!workspaceStore.currentWorkspaceId || !form.name.trim()) {
    return
  }

  const record: ToolRecord = {
    id: selectedToolId.value || `tool-${Date.now()}`,
    workspaceId: workspaceStore.currentWorkspaceId,
    kind: form.kind as ToolRecord['kind'],
    name: form.name.trim(),
    description: form.description.trim(),
    status: form.status as ToolRecord['status'],
    permissionMode: form.permissionMode as ToolRecord['permissionMode'],
    updatedAt: Date.now(),
  }

  if (selectedToolId.value) {
    await catalogStore.updateTool(selectedToolId.value, record)
  } else {
    const created = await catalogStore.createTool(record)
    selectedToolId.value = created.id
  }
}

async function removeTool() {
  if (!selectedToolId.value) {
    return
  }
  await catalogStore.removeTool(selectedToolId.value)
  applyTool(catalogStore.tools[0]?.id)
}
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="px-2">
      <UiSectionHeading :eyebrow="t('tools.header.eyebrow')" :title="t('sidebar.navigation.tools')" :subtitle="catalogStore.error || t('tools.header.subtitle')" />
    </header>

    <div class="grid gap-6 px-2 xl:grid-cols-[minmax(0,1fr)_360px]">
      <section class="space-y-3">
        <UiRecordCard
          v-for="tool in catalogStore.tools"
          :key="tool.id"
          :title="tool.name"
          :description="tool.description"
          interactive
          class="cursor-pointer"
          :class="selectedToolId === tool.id ? 'ring-1 ring-primary' : ''"
          @click="applyTool(tool.id)"
        >
          <template #badges>
            <UiBadge :label="tool.kind" subtle />
            <UiBadge :label="tool.permissionMode" subtle />
          </template>
          <template #meta>
            <span class="text-xs text-text-tertiary">{{ formatDateTime(tool.updatedAt) }}</span>
          </template>
        </UiRecordCard>
        <UiEmptyState v-if="!catalogStore.tools.length" :title="t('tools.empty.title')" :description="t('tools.empty.description')" />
      </section>

      <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
        <h3 class="text-base font-semibold text-text-primary">{{ selectedToolId ? t('tools.actions.edit') : t('tools.actions.create') }}</h3>
        <UiField :label="t('tools.fields.name')">
          <UiInput v-model="form.name" />
        </UiField>
        <UiField :label="t('tools.fields.kind')">
          <UiSelect v-model="form.kind" :options="kindOptions" />
        </UiField>
        <UiField :label="t('common.status')">
          <UiSelect v-model="form.status" :options="statusOptions" />
        </UiField>
        <UiField :label="t('tools.fields.permissionMode')">
          <UiSelect v-model="form.permissionMode" :options="permissionOptions" />
        </UiField>
        <UiField :label="t('tools.fields.description')">
          <UiTextarea v-model="form.description" :rows="6" />
        </UiField>
        <div class="flex gap-3">
          <UiButton @click="saveTool">{{ t('common.save') }}</UiButton>
          <UiButton variant="ghost" @click="applyTool()">{{ t('common.reset') }}</UiButton>
          <UiButton v-if="selectedToolId" variant="ghost" @click="removeTool">{{ t('common.delete') }}</UiButton>
        </div>
      </section>
    </div>
  </div>
</template>
