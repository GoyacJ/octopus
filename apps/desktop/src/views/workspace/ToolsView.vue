<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { Plus, Search } from 'lucide-vue-next'
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
  UiTabs,
  UiTextarea,
} from '@octopus/ui'

import { usePagination } from '@/composables/usePagination'
import { useWorkbenchStore } from '@/stores/workbench'

type ToolTab = 'builtin' | 'skill' | 'mcp'

const PAGE_SIZE = 10

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
const canManageToolCatalog = computed(() =>
  workbench.hasPermission('tool:catalog:update', 'update'),
)
const canCreate = computed(() => activeTab.value !== 'builtin' && canManageToolCatalog.value)

const {
  currentPage,
  pageCount,
  pagedItems,
  setPage,
} = usePagination(filteredItems, {
  pageSize: PAGE_SIZE,
  resetOn: [activeTab, normalizedSearch],
})

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
  <div class="w-full flex flex-col gap-8 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0">
      <UiSectionHeading
        data-testid="tools-title"
        :eyebrow="t('tools.header.eyebrow')"
        :title="t('tools.header.title')"
      />
    </header>

    <!-- Unified Filter Bar -->
    <div class="px-2 flex flex-wrap items-center justify-between gap-6 border-b border-border-subtle dark:border-white/[0.05] pb-6">
      <UiTabs v-model="activeTab" data-testid="tools-tabs" :tabs="tabs" />
      
      <div class="flex items-center gap-3">
        <div class="relative w-80">
          <Search :size="14" class="absolute left-3 top-1/2 -translate-y-1/2 text-text-tertiary" />
          <UiInput
            v-model="searchQuery"
            data-testid="tools-search-input"
            class="pl-9 bg-subtle/30 h-10 text-sm"
            placeholder="Search tools..."
          />
        </div>
        <UiButton v-if="canCreate" data-testid="tools-create-button" variant="primary" class="h-10 px-4" @click="applyTool()">
          <Plus :size="18" />
          {{ activeTab === 'skill' ? t('tools.actions.createSkill') : t('tools.actions.createMcp') }}
        </UiButton>
      </div>
    </div>

    <!-- Main Split View (Extended) -->
    <div class="flex flex-1 min-h-0 gap-12 px-2">
      
      <!-- Left: Tool List (Wider for full screen) -->
      <aside class="flex flex-col w-[420px] shrink-0 border-r border-border-subtle dark:border-white/[0.05] pr-12 gap-6">
        <div class="flex items-center justify-between">
          <h3 class="text-[15px] font-bold text-text-primary">{{ activeGroup?.title ?? t('tools.header.title') }}</h3>
          <span class="text-[12px] text-text-tertiary font-medium">{{ filteredItems.length }} items</span>
        </div>

        <div data-testid="tools-record-list" class="flex-1 overflow-y-auto min-h-0 pb-4 space-y-4 pr-3">
          <template v-if="pagedItems.length">
            <UiRecordCard
              v-for="item in pagedItems"
              :key="item.id"
              :test-id="`tool-item-${item.id}`"
              :title="item.name"
              :description="item.description"
              :active="selectedToolId === item.id"
              interactive
              class="p-4"
              @click="applyTool(item.id)"
            >
              <template #badges>
                <UiBadge :label="t(`tools.permissions.${item.permissionMode}`)" subtle />
                <UiBadge :label="t(`tools.status.${item.status}`)" :tone="statusTone(item.status)" />
              </template>
              <template #meta>
                <span class="text-[11px] text-text-tertiary uppercase tracking-widest font-bold">{{ t(`tools.availability.${item.availability}`) }}</span>
              </template>
              <template #actions>
                <div class="opacity-0 group-hover:opacity-100 transition-opacity flex gap-1">
                  <UiButton v-if="canManageToolCatalog" :data-testid="`tool-toggle-${item.id}`" variant="ghost" size="sm" class="h-7 text-xs" @click.stop="toggleTool(item.id)">
                    {{ item.status === 'active' ? 'Disable' : 'Enable' }}
                  </UiButton>
                  <UiButton v-if="canManageToolCatalog && item.kind !== 'builtin'" :data-testid="`tool-delete-${item.id}`" variant="ghost" size="sm" class="h-7 text-xs text-destructive hover:bg-destructive/10" @click.stop="deleteSelectedTool(item.id)">
                    Delete
                  </UiButton>
                </div>
              </template>
            </UiRecordCard>
          </template>
          <UiEmptyState v-else :title="t('tools.emptyTitle')" :description="t('tools.emptyDescription')" />
        </div>
        
        <div v-if="pageCount > 1" class="pt-6 border-t border-border-subtle dark:border-white/[0.05] shrink-0">
          <div data-testid="tools-pagination-summary" class="mb-2 text-xs text-text-tertiary">第 {{ currentPage }} / {{ pageCount }} 页</div>
          <UiPagination :page="currentPage" :page-count="pageCount" @update:page="setPage" />
        </div>
      </aside>

      <!-- Right: Tool Editor (Expanded) -->
      <main class="flex-1 overflow-y-auto min-h-0 pr-6 pb-12 space-y-10">
        <header class="space-y-2">
          <h2 class="text-2xl font-bold text-text-primary">
            {{ t(selectedToolId ? 'tools.editor.editTitle' : 'tools.editor.createTitle') }}
          </h2>
          <p class="text-base text-text-secondary max-w-3xl leading-relaxed">{{ selectedTool?.description || t('tools.header.eyebrow') }}</p>
        </header>

        <div class="grid gap-x-12 gap-y-8 md:grid-cols-2 max-w-5xl">
          <UiField :label="t('tools.fields.name')">
            <UiInput v-model="form.name" data-testid="tools-form-name" :disabled="activeTab === 'builtin'" class="h-10" />
          </UiField>
          <UiField :label="t('tools.fields.permission')">
            <UiSelect v-model="form.permissionMode" :options="permissionOptions" class="h-10" />
          </UiField>
          <UiField :label="t('tools.fields.status')">
            <UiSelect v-model="form.status" :options="statusOptions" class="h-10" />
          </UiField>
          <UiField :label="t('tools.fields.availability')">
            <UiSelect v-model="form.availability" :options="availabilityOptions" class="h-10" />
          </UiField>
          <UiField class="md:col-span-2" :label="t('tools.fields.description')">
            <UiTextarea v-model="form.description" data-testid="tools-form-description" :rows="2" :disabled="activeTab === 'builtin'" />
          </UiField>

          <template v-if="activeTab === 'skill'">
            <UiField class="md:col-span-2" :label="t('tools.fields.skillContent')">
              <UiTextarea v-model="form.content" data-testid="tools-form-content" :rows="12" class="font-mono text-[13px] leading-relaxed" />
            </UiField>
          </template>

          <template v-if="activeTab === 'mcp'">
            <UiField :label="t('tools.fields.serverName')">
              <UiInput v-model="form.serverName" data-testid="tools-form-server-name" class="h-10" />
            </UiField>
            <UiField :label="t('tools.fields.endpoint')">
              <UiInput v-model="form.endpoint" data-testid="tools-form-endpoint" class="h-10" />
            </UiField>
            <UiField class="md:col-span-2" :label="t('tools.fields.toolNames')">
              <UiInput v-model="form.toolNames" data-testid="tools-form-tool-names" class="h-10" />
            </UiField>
            <UiField class="md:col-span-2" :label="t('tools.fields.notes')">
              <UiTextarea v-model="form.notes" :rows="5" />
            </UiField>
          </template>
        </div>

        <div class="pt-8 border-t border-border-subtle dark:border-white/[0.05] flex gap-4 max-w-5xl">
          <UiButton v-if="canManageToolCatalog" data-testid="tools-form-save" variant="primary" class="h-10 px-8" @click="saveTool">
            {{ t('common.save') }}
          </UiButton>
          <UiButton variant="ghost" class="h-10 px-6" @click="applyTool(filteredItems[0]?.id)">
            {{ t('common.cancel') }}
          </UiButton>
        </div>
      </main>
    </div>
  </div>
</template>
