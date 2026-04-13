<script setup lang="ts">
import { computed, ref } from 'vue'
import { ChevronDown, Download, LayoutGrid, List, Trash2, Upload, UsersRound } from 'lucide-vue-next'

import type { AgentRecord, TeamRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiCheckbox, UiDropdownMenu, UiEmptyState, UiInput, UiPagination, UiRecordCard, UiToolbarRow } from '@octopus/ui'

import type { AgentBundleTransferFormat, ViewMode } from './useAgentCenter'
import TeamUnitCard from './TeamUnitCard.vue'

const props = defineProps<{
  query: string
  viewMode: ViewMode
  total: number
  page: number
  pageCount: number
  pagedTeams: TeamRecord[]
  currentAgents: AgentRecord[]
  isProjectScope: boolean
  importLoading: boolean
  exportLoading: boolean
  selectedTeamIds: string[]
  allPagedSelected: boolean
}>()

const emit = defineEmits<{
  'update:query': [value: string]
  'update:viewMode': [value: ViewMode]
  'update:page': [value: number]
  'update:selectedTeamIds': [value: string[]]
  'create-team': []
  'open-import-dialog': [format: AgentBundleTransferFormat]
  'toggle-all-paged': [value: boolean]
  'export-selected': [format: AgentBundleTransferFormat]
  'export-team': [team: TeamRecord, format: AgentBundleTransferFormat]
  'open-team': [team: TeamRecord]
  'remove-team': [team: TeamRecord]
}>()

const queryModel = computed({
  get: () => props.query,
  set: value => emit('update:query', value),
})

function teamOriginLabel(team: TeamRecord) {
  if (team.integrationSource?.kind === 'builtin-template') {
    return '内置模板'
  }
  if (props.isProjectScope && !isProjectOwnedTeam(team)) {
    return '工作区'
  }
  return undefined
}

function isBuiltinTemplateTeam(team: TeamRecord) {
  return team.integrationSource?.kind === 'builtin-template'
}

function isWorkspaceLinkedTeam(team: TeamRecord) {
  return team.integrationSource?.kind === 'workspace-link'
}

function isProjectOwnedTeam(team: TeamRecord) {
  return Boolean(props.isProjectScope && team.projectId)
}

function canSelectTeam(team: TeamRecord) {
  return props.isProjectScope || !isBuiltinTemplateTeam(team)
}

function canExportTeam(team: TeamRecord) {
  return props.isProjectScope || !isBuiltinTemplateTeam(team)
}

function canRemoveTeam(team: TeamRecord) {
  return props.isProjectScope ? isProjectOwnedTeam(team) : !isBuiltinTemplateTeam(team)
}

function openLabel(team: TeamRecord) {
  if (props.isProjectScope && !isProjectOwnedTeam(team)) {
    return '复制到项目'
  }
  if (isBuiltinTemplateTeam(team)) {
    return props.isProjectScope ? '复制到项目' : '复制到工作区'
  }
  return isWorkspaceLinkedTeam(team) ? '查看' : '编辑'
}

function resolveAgentName(agentId?: string) {
  if (!agentId) {
    return '未设置负责人'
  }
  return props.currentAgents.find(agent => agent.id === agentId)?.name ?? agentId
}

const importMenuItems = [
  { key: 'import-folder', label: '导入文件夹' },
  { key: 'import-zip', label: '导入 ZIP' },
]

const exportMenuItems = computed(() => [
  { key: 'export-folder', label: '导出为文件夹', disabled: props.selectedTeamIds.length === 0 },
  { key: 'export-zip', label: '导出为 ZIP', disabled: props.selectedTeamIds.length === 0 },
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

function handleExportTeam(team: TeamRecord, key: string) {
  emit('export-team', team, key === 'export-zip' ? 'zip' : 'folder')
}

function updateSelectedTeams(teamId: string, nextSelected: boolean) {
  const next = new Set(props.selectedTeamIds)
  if (nextSelected) {
    next.add(teamId)
  } else {
    next.delete(teamId)
  }
  emit('update:selectedTeamIds', Array.from(next))
}
</script>

<template>
  <section class="space-y-4">
    <UiToolbarRow>
      <template #search>
        <UiInput
          v-model="queryModel"
          placeholder="搜索数字团队名称、摘要或成员"
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
            已选 {{ selectedTeamIds.length }} / {{ total }}
          </span>
          <UiButton size="sm" @click="emit('create-team')">
            新建数字团队
          </UiButton>
          <UiDropdownMenu :open="importMenuOpen" :items="importMenuItems" @update:open="importMenuOpen = $event" @select="handleImportSelect">
            <template #trigger>
              <UiButton
                variant="outline"
                size="sm"
              :loading="importLoading"
              loading-label="Previewing"
              data-testid="agent-center-import-teams-trigger"
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
              :disabled="selectedTeamIds.length === 0"
              :loading="exportLoading"
              loading-label="Exporting"
              data-testid="agent-center-export-teams-trigger"
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
      <template v-for="team in pagedTeams" :key="team.id">
        <TeamUnitCard
          v-if="viewMode === 'card'"
          :id="team.id"
          :name="team.name"
          :title="team.personality || 'Digital Team'"
          :description="team.description"
          :lead-label="resolveAgentName(team.leaderAgentId)"
          :members="team.memberAgentIds.map(resolveAgentName)"
          :workflow="team.tags.slice(0, 3)"
          :recent-outcome="team.prompt || team.description"
          :origin-label="teamOriginLabel(team)"
          :open-label="openLabel(team)"
          :remove-label="isWorkspaceLinkedTeam(team) ? '移除接入' : '删除'"
          :open-test-id="`agent-center-open-team-${team.id}`"
          :remove-test-id="`agent-center-remove-team-${team.id}`"
          :selected="selectedTeamIds.includes(team.id)"
          :selection-test-id="`agent-center-select-team-${team.id}`"
          :selectable="canSelectTeam(team)"
          :exportable="canExportTeam(team)"
          :removable="canRemoveTeam(team)"
          @open="emit('open-team', team)"
          @update:selected="updateSelectedTeams(team.id, $event)"
          @export="handleExportTeam(team, $event)"
          @remove="emit('remove-team', team)"
        />

        <UiRecordCard
          v-else
          :title="team.name"
          interactive
          class="hover:bg-subtle/60"
          @click="emit('open-team', team)"
        >
          <template #leading>
            <div class="flex size-10 items-center justify-center overflow-hidden rounded-[var(--radius-m)] border border-border bg-subtle text-text-secondary">
              <UsersRound :size="18" />
            </div>
          </template>
          <template #badges>
            <div class="flex items-center gap-1.5">
              <div
                class="size-2 rounded-full"
                :class="team.status === 'active' ? 'bg-status-success' : 'bg-text-tertiary'"
              />
              <UiBadge v-if="teamOriginLabel(team)" :label="teamOriginLabel(team) ?? ''" subtle />
            </div>
          </template>
          <div class="flex w-full items-center gap-8 overflow-hidden">
            <div class="flex min-w-0 flex-[2] flex-col gap-0.5">
              <span class="truncate text-[11px] font-semibold uppercase tracking-[0.12em] text-text-tertiary">{{ team.personality || '数字团队' }}</span>
              <p class="truncate text-sm text-text-secondary">
                {{ team.description }}
              </p>
            </div>
            <div class="hidden flex-1 shrink-0 items-center gap-1 overflow-hidden lg:flex">
              <span v-for="tag in team.tags.slice(0, 3)" :key="tag" class="truncate rounded-full border border-border bg-subtle px-2 py-0.5 text-[10px] font-medium text-text-tertiary">
                #{{ tag }}
              </span>
            </div>
            <div class="hidden shrink-0 items-center gap-6 md:flex">
              <div class="flex flex-col items-end">
                <span class="text-[9px] font-bold uppercase tracking-tighter text-text-tertiary/40">负责人</span>
                <span class="max-w-[80px] truncate text-xs font-bold text-text-primary/70">
                  {{ resolveAgentName(team.leaderAgentId) || '未设置' }}
                </span>
              </div>
              <div class="flex flex-col items-end">
                <span class="text-[9px] font-bold uppercase tracking-tighter text-text-tertiary/40">成员</span>
                <span class="text-xs font-bold tabular-nums text-text-primary/70">{{ team.memberAgentIds.length }}</span>
              </div>
            </div>
          </div>
          <template #actions>
            <div class="flex items-center gap-1" @click.stop @keydown.stop>
              <UiCheckbox
                v-if="canSelectTeam(team)"
                :model-value="selectedTeamIds"
                :value="team.id"
                :data-testid="`agent-center-select-team-${team.id}`"
                @update:model-value="emit('update:selectedTeamIds', $event as string[])"
              />
              <UiButton size="sm" variant="ghost" class="h-8 px-3 text-[11px] font-semibold" @click.stop="emit('open-team', team)">
                {{ openLabel(team) }}
              </UiButton>
              <UiDropdownMenu v-if="canExportTeam(team)" :items="rowExportMenuItems" @select="handleExportTeam(team, $event)">
                <template #trigger>
                  <UiButton
                    size="sm"
                    variant="ghost"
                    class="h-8 px-3 text-[11px] font-semibold"
                    :aria-label="`导出 ${team.name}`"
                  >
                    导出
                    <ChevronDown :size="12" />
                  </UiButton>
                </template>
              </UiDropdownMenu>
              <UiButton
                v-if="canRemoveTeam(team)"
                variant="ghost"
                size="icon"
                class="size-8 rounded-full text-text-tertiary/40 hover:bg-error/10 hover:text-error"
                @click.stop="emit('remove-team', team)"
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
      title="暂无数字团队"
      description="创建工作区或项目数字团队。"
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
