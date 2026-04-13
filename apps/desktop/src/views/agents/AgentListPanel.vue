<script setup lang="ts">
import { computed, ref } from 'vue'
import { ChevronDown, Download, LayoutGrid, List, Trash2, Upload } from 'lucide-vue-next'

import type { AgentRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiCheckbox, UiDropdownMenu, UiEmptyState, UiInput, UiPagination, UiRecordCard, UiToolbarRow } from '@octopus/ui'

import type { AgentBundleTransferFormat, ViewMode } from './useAgentCenter'
import AgentEmployeeCard from './AgentEmployeeCard.vue'

const props = defineProps<{
  query: string
  viewMode: ViewMode
  total: number
  page: number
  pageCount: number
  pagedAgents: AgentRecord[]
  isProjectScope: boolean
  importLoading: boolean
  exportLoading: boolean
  selectedAgentIds: string[]
  allPagedSelected: boolean
}>()

const emit = defineEmits<{
  'update:query': [value: string]
  'update:viewMode': [value: ViewMode]
  'update:page': [value: number]
  'update:selectedAgentIds': [value: string[]]
  'create-agent': []
  'open-import-dialog': [format: AgentBundleTransferFormat]
  'toggle-all-paged': [value: boolean]
  'export-selected': [format: AgentBundleTransferFormat]
  'export-agent': [agent: AgentRecord, format: AgentBundleTransferFormat]
  'open-agent': [agent: AgentRecord]
  'remove-agent': [agent: AgentRecord]
}>()

const queryModel = computed({
  get: () => props.query,
  set: value => emit('update:query', value),
})

function initials(name: string) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map(part => part[0])
    .join('')
    .toUpperCase()
}

function agentBadgeLabel(agent: AgentRecord) {
  if (agent.integrationSource?.kind === 'builtin-template') {
    return '内置模板'
  }
  if (agent.integrationSource?.kind === 'workspace-link') {
    return '工作区接入'
  }
  return agent.status
}

function isBuiltinTemplateAgent(agent: AgentRecord) {
  return agent.integrationSource?.kind === 'builtin-template'
}

function isWorkspaceLinkedAgent(agent: AgentRecord) {
  return agent.integrationSource?.kind === 'workspace-link'
}

function isProjectOwnedAgent(agent: AgentRecord) {
  return Boolean(props.isProjectScope && agent.projectId)
}

function canSelectAgent(agent: AgentRecord) {
  return props.isProjectScope || !isBuiltinTemplateAgent(agent)
}

function canExportAgent(agent: AgentRecord) {
  return props.isProjectScope || !isBuiltinTemplateAgent(agent)
}

function canRemoveAgent(agent: AgentRecord) {
  return props.isProjectScope ? isProjectOwnedAgent(agent) : !isBuiltinTemplateAgent(agent)
}

function openLabel(agent: AgentRecord) {
  if (props.isProjectScope && !isProjectOwnedAgent(agent)) {
    return '复制到项目'
  }
  if (isBuiltinTemplateAgent(agent)) {
    return props.isProjectScope ? '复制到项目' : '复制到工作区'
  }
  return isWorkspaceLinkedAgent(agent) ? '查看' : '编辑'
}

function originLabel(agent: AgentRecord) {
  if (isBuiltinTemplateAgent(agent)) {
    return '内置模板'
  }
  return props.isProjectScope && !isProjectOwnedAgent(agent) ? '工作区' : undefined
}

const importMenuItems = [
  { key: 'import-folder', label: '导入文件夹' },
  { key: 'import-zip', label: '导入 ZIP' },
]

const exportMenuItems = computed(() => [
  { key: 'export-folder', label: '导出为文件夹', disabled: props.selectedAgentIds.length === 0 },
  { key: 'export-zip', label: '导出为 ZIP', disabled: props.selectedAgentIds.length === 0 },
])

const rowExportMenuItems = [
  { key: 'export-folder', label: '导出为文件夹' },
  { key: 'export-zip', label: '导出为 ZIP' },
]

const importMenuOpen = ref(false)
const exportMenuOpen = ref(false)

function handleImportSelect(key: string) {
  importMenuOpen.value = false
  emit('open-import-dialog', key === 'import-zip' ? 'zip' : 'folder')
}

function handleExportSelected(key: string) {
  exportMenuOpen.value = false
  emit('export-selected', key === 'export-zip' ? 'zip' : 'folder')
}

function handleExportAgent(agent: AgentRecord, key: string) {
  emit('export-agent', agent, key === 'export-zip' ? 'zip' : 'folder')
}

function updateSelectedAgents(agentId: string, nextSelected: boolean) {
  const next = new Set(props.selectedAgentIds)
  if (nextSelected) {
    next.add(agentId)
  } else {
    next.delete(agentId)
  }
  emit('update:selectedAgentIds', Array.from(next))
}
</script>

<template>
  <section class="space-y-4">
    <UiToolbarRow>
      <template #search>
        <UiInput
          v-model="queryModel"
          placeholder="搜索员工、性格或工具"
          class="max-w-md"
        />
      </template>
      <template #views>
        <UiButton
          variant="ghost"
          size="sm"
          :class="viewMode === 'list' ? 'bg-subtle text-text-primary' : ''"
          @click="emit('update:viewMode', 'list')"
        >
          <List :size="14" />
          列表
        </UiButton>
        <UiButton
          variant="ghost"
          size="sm"
          :class="viewMode === 'card' ? 'bg-subtle text-text-primary' : ''"
          @click="emit('update:viewMode', 'card')"
        >
          <LayoutGrid :size="14" />
          卡片
        </UiButton>
      </template>
      <template #actions>
        <div class="flex flex-wrap items-center justify-end gap-2">
          <span class="text-[12px] text-text-tertiary">
            已选 {{ selectedAgentIds.length }} / {{ total }}
          </span>
          <UiButton size="sm" @click="emit('create-agent')">
            新建数字员工
          </UiButton>
          <UiDropdownMenu :open="importMenuOpen" :items="importMenuItems" @update:open="importMenuOpen = $event" @select="handleImportSelect">
            <template #trigger>
              <UiButton
                variant="outline"
                size="sm"
              :loading="importLoading"
              loading-label="Previewing"
              data-testid="agent-center-import-agents-trigger"
            >
                <Upload :size="14" />
                导入
                <ChevronDown :size="14" />
              </UiButton>
            </template>
          </UiDropdownMenu>
          <UiDropdownMenu :open="exportMenuOpen" :items="exportMenuItems" @update:open="exportMenuOpen = $event" @select="handleExportSelected">
            <template #trigger>
              <UiButton
                variant="outline"
                size="sm"
              :disabled="selectedAgentIds.length === 0"
              :loading="exportLoading"
              loading-label="Exporting"
              data-testid="agent-center-export-agents-trigger"
            >
                <Download :size="14" />
                批量导出
                <ChevronDown :size="14" />
              </UiButton>
            </template>
          </UiDropdownMenu>
        </div>
      </template>
      <template #filters>
        <UiCheckbox
          :model-value="allPagedSelected"
          label="全选当前页"
          @update:model-value="emit('toggle-all-paged', Boolean($event))"
        />
      </template>
    </UiToolbarRow>

    <div v-if="total" :class="viewMode === 'card' ? 'grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4' : 'space-y-2'">
      <template v-for="agent in pagedAgents" :key="agent.id">
        <AgentEmployeeCard
          v-if="viewMode === 'card'"
          :id="agent.id"
          :name="agent.name"
          :role="agent.personality"
          :summary="agent.description"
          :recent-task="agent.prompt || agent.description"
          :avatar="agent.avatar || initials(agent.name)"
          :status-label="agentBadgeLabel(agent)"
          :status-tone="agent.status === 'active' ? 'success' : 'default'"
          :skills="agent.tags.slice(0, 3)"
          :metrics="[
            { label: 'Tools', value: String(agent.builtinToolKeys.length) },
            { label: 'Skills', value: String(agent.skillIds.length) },
          ]"
          :origin-label="originLabel(agent)"
          :open-label="openLabel(agent)"
          :remove-label="isWorkspaceLinkedAgent(agent) ? '移除接入' : '删除'"
          :open-test-id="`agent-center-open-agent-${agent.id}`"
          :remove-test-id="`agent-center-remove-agent-${agent.id}`"
          :selected="selectedAgentIds.includes(agent.id)"
          :selection-test-id="`agent-center-select-agent-${agent.id}`"
          :selectable="canSelectAgent(agent)"
          :exportable="canExportAgent(agent)"
          :removable="canRemoveAgent(agent)"
          @open="emit('open-agent', agent)"
          @update:selected="updateSelectedAgents(agent.id, $event)"
          @export="handleExportAgent(agent, $event)"
          @remove="emit('remove-agent', agent)"
        />

        <UiRecordCard
          v-else
          :title="agent.name"
          interactive
          class="hover:bg-subtle/60"
          @click="emit('open-agent', agent)"
        >
          <template #leading>
            <div class="flex size-10 items-center justify-center overflow-hidden rounded-[var(--radius-m)] border border-border bg-subtle text-xs font-semibold text-text-secondary">
              <img v-if="agent.avatar" :src="agent.avatar" alt="" class="size-full object-cover">
              <span v-else>{{ initials(agent.name) }}</span>
            </div>
          </template>
          <template #badges>
            <div class="flex items-center gap-1.5">
              <div
                class="size-2 rounded-full"
                :class="agent.status === 'active' ? 'bg-status-success' : 'bg-text-tertiary'"
              />
              <UiBadge v-if="originLabel(agent)" :label="originLabel(agent) ?? ''" subtle />
            </div>
          </template>
          <div class="flex w-full items-center gap-8 overflow-hidden">
            <div class="flex min-w-0 flex-[2] flex-col gap-0.5">
              <span class="truncate text-[11px] font-semibold uppercase tracking-[0.12em] text-text-tertiary">{{ agent.personality }}</span>
              <p class="truncate text-sm text-text-secondary">
                {{ agent.description }}
              </p>
            </div>
            <div class="hidden flex-1 shrink-0 items-center gap-1 overflow-hidden lg:flex">
              <span v-for="tag in agent.tags.slice(0, 3)" :key="tag" class="truncate rounded-full border border-border bg-subtle px-2 py-0.5 text-[10px] font-medium text-text-tertiary">
                #{{ tag }}
              </span>
            </div>
            <div class="hidden shrink-0 items-center gap-6 md:flex">
              <div class="flex flex-col items-end">
                <span class="text-[9px] font-bold uppercase tracking-tighter text-text-tertiary/40">工具</span>
                <span class="text-xs font-bold tabular-nums text-text-primary/70">{{ agent.builtinToolKeys.length }}</span>
              </div>
              <div class="flex flex-col items-end">
                <span class="text-[9px] font-bold uppercase tracking-tighter text-text-tertiary/40">技能</span>
                <span class="text-xs font-bold tabular-nums text-text-primary/70">{{ agent.skillIds.length }}</span>
              </div>
            </div>
          </div>
          <template #actions>
            <div class="flex items-center gap-1" @click.stop @keydown.stop>
              <UiCheckbox
                v-if="canSelectAgent(agent)"
                :model-value="selectedAgentIds"
                :value="agent.id"
                :data-testid="`agent-center-select-agent-${agent.id}`"
                @update:model-value="emit('update:selectedAgentIds', $event as string[])"
              />
              <UiButton size="sm" variant="ghost" class="h-8 px-3 text-[11px] font-semibold" @click.stop="emit('open-agent', agent)">
                {{ openLabel(agent) }}
              </UiButton>
              <UiDropdownMenu v-if="canExportAgent(agent)" :items="rowExportMenuItems" @select="handleExportAgent(agent, $event)">
                <template #trigger>
                  <UiButton
                    size="sm"
                    variant="ghost"
                    class="h-8 px-3 text-[11px] font-semibold"
                    :aria-label="`导出 ${agent.name}`"
                  >
                    导出
                    <ChevronDown :size="12" />
                  </UiButton>
                </template>
              </UiDropdownMenu>
              <UiButton
                v-if="canRemoveAgent(agent)"
                variant="ghost"
                size="icon"
                class="size-8 rounded-full text-text-tertiary/40 hover:bg-error/10 hover:text-error"
                @click.stop="emit('remove-agent', agent)"
              >
                <Trash2 :size="14" />
              </UiButton>
            </div>
          </template>
        </UiRecordCard>
      </template>
    </div>

    <UiEmptyState
      v-else
      title="暂无数字员工"
      description="创建工作区或项目数字员工。"
    />

    <UiPagination
      v-if="total > 6"
      :page="page"
      :page-count="pageCount"
      :meta-label="`共 ${total} 项`"
      @update:page="emit('update:page', $event)"
    />
  </section>
</template>
