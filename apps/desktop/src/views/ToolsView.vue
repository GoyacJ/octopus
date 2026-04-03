<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ChevronLeft, ChevronRight, Plus, Power, Trash2 } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiInput,
  UiPagination,
  UiRecordCard,
  UiSectionHeading,
  UiSelect,
  UiSurface,
  UiTabs,
  UiTextarea,
  UiToolbarRow,
} from '@octopus/ui'

import { usePagination } from '@/composables/usePagination'
import { useWorkbenchStore } from '@/stores/workbench'

type ToolTab = 'builtin' | 'skill' | 'mcp'

const PAGE_SIZE = 5

const { t } = useI18n()
const workbench = useWorkbenchStore()
const activeTab = ref<ToolTab>('builtin')
const selectedToolId = ref<string>('')
const searchQuery = ref('')

const form = reactive({
  name: '',
  description: '',
  availability: 'healthy' as 'healthy' | 'configured' | 'attention',
  status: 'active' as 'active' | 'disabled',
  permissionMode: 'allow' as 'allow' | 'deny' | 'ask' | 'readonly',
  content: '',
  serverName: '',
  endpoint: '',
  toolNames: '',
  notes: '',
})

const tabs = computed(() => [
  { value: 'builtin', label: t('tools.tabs.builtin') },
  { value: 'skill', label: t('tools.tabs.skill') },
  { value: 'mcp', label: t('tools.tabs.mcp') },
])

const normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase())
const activeGroup = computed(() =>
  workbench.toolCatalogGroups.find((group) => group.id === activeTab.value),
)
const activeGroupSubtitle = computed(() => {
  if (activeTab.value === 'builtin') {
    return t('tools.tabs.builtin')
  }

  if (activeTab.value === 'skill') {
    return t('tools.tabs.skill')
  }

  return t('tools.tabs.mcp')
})
const filteredItems = computed(() => {
  const items = activeGroup.value?.items ?? []
  if (!normalizedSearch.value) {
    return items
  }

  return items.filter((item) => [
    item.name,
    item.kind,
    item.description,
    item.permissionMode,
    item.status,
    item.availability,
    item.content ?? '',
    item.serverName ?? '',
    item.endpoint ?? '',
    item.toolNames?.join(' ') ?? '',
    item.notes ?? '',
  ].join(' ').toLowerCase().includes(normalizedSearch.value))
})
const selectedTool = computed(() =>
  filteredItems.value.find((item) => item.id === selectedToolId.value)
  ?? activeGroup.value?.items.find((item) => item.id === selectedToolId.value),
)
const canCreate = computed(() => activeTab.value !== 'builtin')

const {
  currentPage,
  pageCount,
  totalItems,
  pagedItems,
  setPage,
} = usePagination(filteredItems, {
  pageSize: PAGE_SIZE,
  resetOn: [activeTab, normalizedSearch],
})

const paginationMetaLabel = computed(() => t('tools.pagination.perPage', { count: PAGE_SIZE }))
const paginationSummaryLabel = computed(() => t('tools.pagination.summary', {
  page: currentPage.value,
  totalPages: pageCount.value,
  count: totalItems.value,
}))
const pageInfoLabel = computed(() => `${currentPage.value} / ${pageCount.value}`)

const statusOptions = computed(() => [
  { value: 'active', label: t('tools.status.active') },
  { value: 'disabled', label: t('tools.status.disabled') },
])

const permissionOptions = computed(() => [
  { value: 'allow', label: t('tools.permissions.allow') },
  { value: 'deny', label: t('tools.permissions.deny') },
  { value: 'ask', label: t('tools.permissions.ask') },
  { value: 'readonly', label: t('tools.permissions.readonly') },
])

const availabilityOptions = computed(() => [
  { value: 'healthy', label: t('tools.availability.healthy') },
  { value: 'configured', label: t('tools.availability.configured') },
  { value: 'attention', label: t('tools.availability.attention') },
])

function resetForm() {
  form.name = ''
  form.description = ''
  form.availability = 'healthy'
  form.status = 'active'
  form.permissionMode = 'allow'
  form.content = ''
  form.serverName = ''
  form.endpoint = ''
  form.toolNames = ''
  form.notes = ''
}

function applyTool(toolId?: string) {
  if (!toolId) {
    selectedToolId.value = ''
    resetForm()
    form.availability = activeTab.value === 'mcp' ? 'configured' : 'healthy'
    form.permissionMode = activeTab.value === 'mcp' ? 'ask' : 'allow'
    return
  }

  const tool = activeGroup.value?.items.find((item) => item.id === toolId)
  if (!tool) {
    applyTool()
    return
  }

  selectedToolId.value = tool.id
  form.name = tool.name
  form.description = tool.description
  form.availability = tool.availability
  form.status = tool.status
  form.permissionMode = tool.permissionMode
  form.content = tool.content ?? ''
  form.serverName = tool.serverName ?? ''
  form.endpoint = tool.endpoint ?? ''
  form.toolNames = tool.toolNames?.join(', ') ?? ''
  form.notes = tool.notes ?? ''
}

watch(
  () => [activeTab.value, workbench.currentWorkspaceId, activeGroup.value?.items.map((item) => item.id).join('|') ?? '', normalizedSearch.value],
  () => {
    const currentItems = filteredItems.value
    if (!selectedToolId.value || !currentItems.some((item) => item.id === selectedToolId.value)) {
      applyTool(currentItems[0]?.id)
      return
    }

    applyTool(selectedToolId.value)
  },
  { immediate: true },
)

function saveTool() {
  if (activeTab.value === 'builtin' && selectedToolId.value) {
    workbench.updateBuiltinTool(selectedToolId.value, {
      availability: form.availability,
      status: form.status,
      permissionMode: form.permissionMode,
    })
    return
  }

  if (activeTab.value === 'skill') {
    if (selectedToolId.value) {
      workbench.updateSkillTool(selectedToolId.value, {
        name: form.name,
        availability: form.availability,
        status: form.status,
        permissionMode: form.permissionMode,
        content: form.content,
      })
      return
    }

    const tool = workbench.createSkillTool({
      name: form.name,
      description: form.description,
      availability: form.availability,
      status: form.status,
      permissionMode: form.permissionMode,
      content: form.content,
    })
    applyTool(tool.id)
    return
  }

  if (selectedToolId.value) {
    workbench.updateMcpTool(selectedToolId.value, {
      name: form.name,
      availability: form.availability,
      status: form.status,
      permissionMode: form.permissionMode,
      serverName: form.serverName,
      endpoint: form.endpoint,
      toolNames: form.toolNames.split(',').map((item) => item.trim()).filter(Boolean),
      notes: form.notes,
    })
    return
  }

  const tool = workbench.createMcpTool({
    name: form.name,
    description: form.description,
    availability: form.availability,
    status: form.status,
    permissionMode: form.permissionMode,
    serverName: form.serverName,
    endpoint: form.endpoint,
    toolNames: form.toolNames.split(',').map((item) => item.trim()).filter(Boolean),
    notes: form.notes,
  })
  applyTool(tool.id)
}

function toggleTool(toolId: string) {
  if (activeTab.value === 'builtin') {
    workbench.toggleBuiltinToolStatus(toolId)
    return
  }

  if (activeTab.value === 'skill') {
    workbench.toggleSkillToolStatus(toolId)
    return
  }

  workbench.toggleMcpToolStatus(toolId)
}

function deleteSelectedTool(toolId: string) {
  if (activeTab.value === 'skill') {
    workbench.deleteSkillTool(toolId)
  }
  else if (activeTab.value === 'mcp') {
    workbench.deleteMcpTool(toolId)
  }
}

function statusTone(status: 'active' | 'disabled') {
  return status === 'active' ? 'success' : 'warning'
}

function availabilityTone(availability: 'healthy' | 'configured' | 'attention') {
  if (availability === 'healthy') {
    return 'success'
  }

  return availability === 'configured' ? 'info' : 'warning'
}
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('tools.header.eyebrow')"
      :title="t('tools.header.title')"
    />

    <div class="space-y-4">
      <div class="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        <h1 data-testid="tools-title" class="text-[clamp(1.28rem,1.8vw,1.9rem)] font-bold leading-tight tracking-[-0.03em] text-text-primary">
          {{ t('tools.header.title') }}
        </h1>
        <UiTabs
          v-model="activeTab"
          test-id="tools-tabs"
          variant="pill"
          :tabs="tabs"
        />
      </div>

      <UiToolbarRow test-id="tools-toolbar">
        <template #search>
          <UiField :label="t('common.search')">
            <UiInput
              v-model="searchQuery"
              data-testid="tools-search-input"
              :placeholder="t('tools.search.placeholder')"
            />
          </UiField>
        </template>
        <template #actions>
          <UiButton v-if="canCreate" size="sm" data-testid="tools-create-button" @click="applyTool()">
            <Plus :size="16" />
            {{ activeTab === 'skill' ? t('tools.actions.createSkill') : t('tools.actions.createMcp') }}
          </UiButton>
        </template>
      </UiToolbarRow>
    </div>

    <div class="grid gap-4 xl:grid-cols-[minmax(22rem,30rem)_minmax(0,1fr)]">
      <UiSurface :title="activeGroup?.title ?? t('tools.header.title')" :subtitle="activeGroupSubtitle">
        <div v-if="pagedItems.length" data-testid="tools-record-list" class="space-y-3">
          <UiRecordCard
            v-for="item in pagedItems"
            :key="item.id"
            :test-id="`tool-item-${item.id}`"
            :title="item.name"
            :description="item.description"
            :active="selectedToolId === item.id"
            interactive
            @click="applyTool(item.id)"
          >
            <template #badges>
              <UiBadge :label="t(`tools.permissions.${item.permissionMode}`)" subtle />
              <UiBadge :label="t(`tools.status.${item.status}`)" :tone="statusTone(item.status)" />
            </template>
            <template #meta>
              <UiBadge :label="t(`tools.availability.${item.availability}`)" :tone="availabilityTone(item.availability)" subtle />
            </template>
            <template #actions>
              <UiButton variant="ghost" size="sm" :data-testid="`tool-toggle-${item.id}`" @click.stop="toggleTool(item.id)">
                <Power :size="14" />
                {{ item.status === 'active' ? t('tools.actions.disable') : t('tools.actions.enable') }}
              </UiButton>
              <UiButton
                v-if="item.kind !== 'builtin'"
                variant="ghost"
                size="sm"
                :data-testid="`tool-delete-${item.id}`"
                @click.stop="deleteSelectedTool(item.id)"
              >
                <Trash2 :size="14" />
                {{ t('tools.actions.delete') }}
              </UiButton>
            </template>
          </UiRecordCard>
        </div>
        <UiEmptyState v-else :title="t('tools.emptyTitle')" :description="t('tools.emptyDescription')" />

        <UiPagination
          v-if="filteredItems.length"
          class="mt-4"
          :page="currentPage"
          :page-count="pageCount"
          :meta-label="paginationMetaLabel"
          :summary-label="paginationSummaryLabel"
          :page-info-label="pageInfoLabel"
          :previous-label="t('tools.pagination.previous')"
          :next-label="t('tools.pagination.next')"
          root-test-id="tools-pagination"
          previous-button-test-id="tools-pagination-prev"
          next-button-test-id="tools-pagination-next"
          page-info-test-id="tools-pagination-page-info"
          summary-test-id="tools-pagination-summary"
          @update:page="setPage"
        >
          <template #previousIcon>
            <ChevronLeft :size="16" />
          </template>
          <template #nextIcon>
            <ChevronRight :size="16" />
          </template>
        </UiPagination>
      </UiSurface>

      <UiSurface
        :title="t(selectedToolId ? 'tools.editor.editTitle' : 'tools.editor.createTitle')"
        :subtitle="selectedTool?.description || t('tools.header.eyebrow')"
      >
        <div class="grid gap-4 md:grid-cols-2">
          <UiField :label="t('tools.fields.name')">
            <UiInput v-model="form.name" :disabled="activeTab === 'builtin'" data-testid="tools-form-name" />
          </UiField>
          <UiField :label="t('tools.fields.permission')">
            <UiSelect v-model="form.permissionMode" :options="permissionOptions" data-testid="tools-form-permission" />
          </UiField>
          <UiField :label="t('tools.fields.status')">
            <UiSelect v-model="form.status" :options="statusOptions" data-testid="tools-form-status" />
          </UiField>
          <UiField :label="t('tools.fields.availability')">
            <UiSelect v-model="form.availability" :options="availabilityOptions" data-testid="tools-form-availability" />
          </UiField>
          <UiField class="md:col-span-2" :label="t('tools.fields.description')">
            <UiTextarea v-model="form.description" :rows="3" disabled data-testid="tools-form-description" />
          </UiField>
          <UiField v-if="activeTab === 'skill'" class="md:col-span-2" :label="t('tools.fields.skillContent')">
            <UiTextarea v-model="form.content" :rows="8" data-testid="tools-form-content" />
          </UiField>
          <template v-if="activeTab === 'mcp'">
            <UiField :label="t('tools.fields.serverName')">
              <UiInput v-model="form.serverName" data-testid="tools-form-server-name" />
            </UiField>
            <UiField :label="t('tools.fields.endpoint')">
              <UiInput v-model="form.endpoint" data-testid="tools-form-endpoint" />
            </UiField>
            <UiField class="md:col-span-2" :label="t('tools.fields.toolNames')">
              <UiInput v-model="form.toolNames" data-testid="tools-form-tool-names" />
            </UiField>
            <UiField class="md:col-span-2" :label="t('tools.fields.notes')">
              <UiTextarea v-model="form.notes" :rows="5" data-testid="tools-form-notes" />
            </UiField>
          </template>
        </div>

        <div class="mt-4 flex flex-wrap justify-end gap-3">
          <UiButton variant="ghost" data-testid="tools-form-cancel" @click="applyTool(filteredItems[0]?.id)">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton data-testid="tools-form-save" @click="saveTool">
            {{ t('common.save') }}
          </UiButton>
        </div>
      </UiSurface>
    </div>
  </section>
</template>
