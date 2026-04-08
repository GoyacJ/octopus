<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import {
  LayoutGrid,
  List,
  Network,
  Search,
  Trash2,
  UserCheck,
  Users,
  UsersRound,
  Wand2,
} from 'lucide-vue-next'

import type {
  AgentRecord,
  AvatarUploadPayload,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundleResult,
  TeamRecord,
  WorkspaceDirectoryUploadEntry,
  UpsertAgentInput,
  UpsertTeamInput,
} from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCombobox,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInput,
  UiPagination,
  UiRecordCard,
  UiSearchableMultiSelect,
  UiSectionHeading,
  UiSelect,
  UiSurface,
  UiTabs,
  UiTextarea,
  UiToolbarRow,
} from '@octopus/ui'

import { usePagination } from '@/composables/usePagination'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceStore } from '@/stores/workspace'
import * as tauriClient from '@/tauri/client'
import AgentEmployeeCard from './AgentEmployeeCard.vue'
import AgentBundleImportDialog from './AgentBundleImportDialog.vue'
import AgentsStatsStrip from './AgentsStatsStrip.vue'
import TeamUnitCard from './TeamUnitCard.vue'

type CenterScope = 'workspace' | 'project'
type CenterTab = 'agent' | 'team'
type ViewMode = 'list' | 'card'

interface SelectOption {
  value: string
  label: string
  keywords?: string[]
  helper?: string
  disabled?: boolean
}

const props = defineProps<{
  scope: CenterScope
}>()

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const agentStore = useAgentStore()
const teamStore = useTeamStore()
const catalogStore = useCatalogStore()

const activeTab = ref<CenterTab>('agent')
const agentViewMode = ref<ViewMode>('card')
const teamViewMode = ref<ViewMode>('card')
const agentQuery = ref('')
const teamQuery = ref('')

const agentDialogOpen = ref(false)
const teamDialogOpen = ref(false)
const editingAgentId = ref<string | null>(null)
const editingTeamId = ref<string | null>(null)
const agentPendingAvatar = ref<AvatarUploadPayload | null>(null)
const teamPendingAvatar = ref<AvatarUploadPayload | null>(null)
const removeAgentAvatar = ref(false)
const removeTeamAvatar = ref(false)

const deleteConfirmOpen = ref(false)
const itemToDelete = ref<{ id: string; name: string; type: 'agent' | 'team' } | null>(null)
const agentImportDialogOpen = ref(false)
const agentImportFiles = ref<WorkspaceDirectoryUploadEntry[]>([])
const agentImportPreview = ref<ImportWorkspaceAgentBundlePreview | null>(null)
const agentImportResult = ref<ImportWorkspaceAgentBundleResult | null>(null)
const agentImportError = ref('')
const agentImportLoading = ref(false)

const agentForm = reactive({
  name: '',
  description: '',
  personality: '',
  tagsText: '',
  prompt: '',
  builtinToolKeys: [] as string[],
  skillIds: [] as string[],
  mcpServerNames: [] as string[],
  status: 'active',
})

const teamForm = reactive({
  name: '',
  description: '',
  personality: '',
  tagsText: '',
  prompt: '',
  builtinToolKeys: [] as string[],
  skillIds: [] as string[],
  mcpServerNames: [] as string[],
  leaderAgentId: '',
  memberAgentIds: [] as string[],
  status: 'active',
})

const isProjectScope = computed(() => props.scope === 'project')
const projectId = computed(() => typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId || '')
const currentAgents = computed(() => isProjectScope.value ? agentStore.projectAgents : agentStore.workspaceAgents)
const currentTeams = computed(() => isProjectScope.value ? teamStore.projectTeams : teamStore.workspaceTeams)
const builtinOptions = computed<SelectOption[]>(() =>
  catalogStore.toolCatalogEntries
    .filter(entry => entry.kind === 'builtin')
    .map(entry => ({
      value: entry.builtinKey ?? entry.name,
      label: entry.name,
      keywords: [entry.description, entry.sourceKey].filter(Boolean),
      helper: entry.description,
    })),
)
const skillOptions = computed<SelectOption[]>(() =>
  catalogStore.toolCatalogEntries
    .filter(entry => entry.kind === 'skill')
    .map(entry => ({
      value: entry.id,
      label: entry.name,
      keywords: [entry.description, entry.sourceKey].filter(Boolean),
      helper: entry.displayPath,
    })),
)
const mcpOptions = computed<SelectOption[]>(() =>
  catalogStore.toolCatalogEntries
    .filter(entry => entry.kind === 'mcp')
    .map(entry => ({
      value: entry.serverName ?? entry.name,
      label: entry.name,
      keywords: [entry.description, entry.sourceKey].filter(Boolean),
      helper: entry.displayPath,
    })),
)
const statusOptions = [
  { value: 'active', label: 'Active' },
  { value: 'archived', label: 'Archived' },
]
const teamAgentOptions = computed<SelectOption[]>(() =>
  currentAgents.value.map(agent => ({
    value: agent.id,
    label: agent.name,
    keywords: [agent.personality, ...agent.tags],
    helper: agent.personality,
    disabled: Boolean(agent.integrationSource),
  })),
)
const currentEditingTeamRecord = computed(() => currentTeams.value.find(team => team.id === editingTeamId.value))
const dialogTeamLeader = computed(() =>
  currentAgents.value.find(agent => agent.id === (teamForm.leaderAgentId || currentEditingTeamRecord.value?.leaderAgentId)),
)
const dialogTeamMembers = computed(() => {
  const selectedIds = teamForm.memberAgentIds.length
    ? teamForm.memberAgentIds
    : (currentEditingTeamRecord.value?.memberAgentIds ?? [])
  return currentAgents.value.filter(agent => selectedIds.includes(agent.id))
})
const leaderOptions = computed<SelectOption[]>(() =>
  currentAgents.value
    .filter(agent => !agent.integrationSource)
    .map(agent => ({
      value: agent.id,
      label: agent.name,
      keywords: [agent.personality, ...agent.tags],
    })),
)
const tabs = [
  { value: 'agent', label: t('agents.tabs.agents') },
  { value: 'team', label: t('agents.tabs.teams') },
]

watch(
  () => route.query.tab,
  (value) => {
    activeTab.value = value === 'team' ? 'team' : 'agent'
  },
  { immediate: true },
)

watch(
  () => [shell.activeWorkspaceConnectionId, projectId.value],
  async ([connectionId, nextProjectId]) => {
    if (!connectionId) {
      return
    }

    await Promise.all([
      agentStore.load(connectionId),
      teamStore.load(connectionId),
      catalogStore.load(connectionId),
    ])

  },
  { immediate: true },
)

function catalogLabels(values: string[], options: SelectOption[]) {
  const labelMap = new Map(options.map(option => [option.value, option.label]))
  return values.map(value => labelMap.get(value) ?? value)
}

function matchesQuery(record: AgentRecord | TeamRecord, query: string) {
  if (!query.trim()) {
    return true
  }
  const keyword = query.trim().toLowerCase()
  const parts = [
    record.name,
    record.description,
    record.personality,
    record.prompt,
    ...record.tags,
    ...catalogLabels(record.builtinToolKeys, builtinOptions.value),
    ...catalogLabels(record.skillIds, skillOptions.value),
    ...catalogLabels(record.mcpServerNames, mcpOptions.value),
  ]
  if ('leaderAgentId' in record) {
    parts.push(record.leaderAgentId ?? '')
    parts.push(...record.memberAgentIds)
  }
  return parts.join(' ').toLowerCase().includes(keyword)
}

const filteredAgents = computed(() => currentAgents.value.filter(agent => matchesQuery(agent, agentQuery.value)))
const filteredTeams = computed(() => currentTeams.value.filter(team => matchesQuery(team, teamQuery.value)))

const agentPagination = usePagination(filteredAgents, {
  pageSize: 6,
  resetOn: [agentQuery, () => props.scope, projectId],
})
const teamPagination = usePagination(filteredTeams, {
  pageSize: 6,
  resetOn: [teamQuery, () => props.scope, projectId],
})
const pagedAgents = computed(() => agentPagination.pagedItems.value)
const pagedTeams = computed(() => teamPagination.pagedItems.value)
const agentTotal = computed(() => agentPagination.totalItems.value)
const teamTotal = computed(() => teamPagination.totalItems.value)
const agentPage = computed(() => agentPagination.currentPage.value)
const teamPage = computed(() => teamPagination.currentPage.value)
const agentPageCount = computed(() => agentPagination.pageCount.value)
const teamPageCount = computed(() => teamPagination.pageCount.value)

const centerStats = computed(() => [
  {
    label: '活跃员工',
    value: String(currentAgents.value.filter(a => a.status === 'active').length),
    helper: '当前可用数字员工数量',
    tone: 'success' as const,
  },
  {
    label: '协作团队',
    value: String(currentTeams.value.length),
    helper: '已组建的数字员工团队',
    tone: 'info' as const,
  },
  {
    label: '工具集成',
    value: String(catalogStore.toolCatalogEntries.length),
    helper: '可用工具与技能总数',
    tone: 'default' as const,
  },
  {
    label: '待办任务',
    value: '12', // Mock for now or based on real data if available
    helper: '进行中的数字员工任务',
    tone: 'warning' as const,
  },
])

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
  return agent.integrationSource ? 'Workspace Link' : agent.status
}

function teamOriginLabel(team: TeamRecord) {
  return team.integrationSource ? 'Workspace Link' : undefined
}

function setTab(nextTab: string) {
  const value = nextTab === 'team' ? 'team' : 'agent'
  activeTab.value = value
  void router.replace({
    query: {
      ...route.query,
      tab: value,
    },
  })
}

function resetAgentForm(record?: AgentRecord) {
  editingAgentId.value = record?.id ?? null
  agentForm.name = record?.name ?? ''
  agentForm.description = record?.description ?? ''
  agentForm.personality = record?.personality ?? ''
  agentForm.tagsText = (record?.tags ?? []).join(', ')
  agentForm.prompt = record?.prompt ?? ''
  agentForm.builtinToolKeys = [...(record?.builtinToolKeys ?? [])]
  agentForm.skillIds = [...(record?.skillIds ?? [])]
  agentForm.mcpServerNames = [...(record?.mcpServerNames ?? [])]
  agentForm.status = record?.status ?? 'active'
  agentPendingAvatar.value = null
  removeAgentAvatar.value = false
}

function resetTeamForm(record?: TeamRecord) {
  editingTeamId.value = record?.id ?? null
  teamForm.name = record?.name ?? ''
  teamForm.description = record?.description ?? ''
  teamForm.personality = record?.personality ?? ''
  teamForm.tagsText = (record?.tags ?? []).join(', ')
  teamForm.prompt = record?.prompt ?? ''
  teamForm.builtinToolKeys = [...(record?.builtinToolKeys ?? [])]
  teamForm.skillIds = [...(record?.skillIds ?? [])]
  teamForm.mcpServerNames = [...(record?.mcpServerNames ?? [])]
  teamForm.leaderAgentId = record?.leaderAgentId ?? ''
  teamForm.memberAgentIds = [...(record?.memberAgentIds ?? [])]
  teamForm.status = record?.status ?? 'active'
  teamPendingAvatar.value = null
  removeTeamAvatar.value = false
}

function openCreateAgent() {
  resetAgentForm()
  agentDialogOpen.value = true
}

async function openAgentImportDialog() {
  agentImportError.value = ''
  agentImportResult.value = null
  const files = await tauriClient.pickAgentBundleFolder()
  if (!files?.length) {
    return
  }

  agentImportLoading.value = true
  try {
    const preview = await agentStore.previewImportBundle({ files })
    agentImportFiles.value = files
    agentImportPreview.value = preview
    agentImportDialogOpen.value = true
  } catch (error) {
    agentImportPreview.value = null
    agentImportFiles.value = []
    agentImportError.value = error instanceof Error ? error.message : 'Failed to preview agent bundle import'
    agentImportDialogOpen.value = true
  } finally {
    agentImportLoading.value = false
  }
}

async function confirmAgentImport() {
  if (!agentImportFiles.value.length) {
    return
  }

  agentImportLoading.value = true
  agentImportError.value = ''
  try {
    const result = await agentStore.importBundle({ files: agentImportFiles.value })
    agentImportResult.value = result
    await catalogStore.load()
  } catch (error) {
    agentImportError.value = error instanceof Error ? error.message : 'Failed to import agent bundle'
  } finally {
    agentImportLoading.value = false
  }
}

function handleAgentImportDialogOpen(nextOpen: boolean) {
  agentImportDialogOpen.value = nextOpen
  if (nextOpen) {
    return
  }

  agentImportFiles.value = []
  agentImportPreview.value = null
  agentImportResult.value = null
  agentImportError.value = ''
}

function openEditAgent(record: AgentRecord) {
  if (record.integrationSource && isProjectScope.value) {
    void router.push({
      name: 'workspace-agents',
      params: { workspaceId: workspaceStore.currentWorkspaceId },
      query: { tab: 'agent' },
    })
    return
  }
  resetAgentForm(record)
  agentDialogOpen.value = true
}

function openCreateTeam() {
  resetTeamForm()
  teamDialogOpen.value = true
}

function openEditTeam(record: TeamRecord) {
  if (record.integrationSource && isProjectScope.value) {
    void router.push({
      name: 'workspace-agents',
      params: { workspaceId: workspaceStore.currentWorkspaceId },
      query: { tab: 'team' },
    })
    return
  }
  resetTeamForm(record)
  teamDialogOpen.value = true
}

async function pickAgentAvatar() {
  const picked = await tauriClient.pickAvatarImage()
  if (!picked) {
    return
  }
  agentPendingAvatar.value = picked
  removeAgentAvatar.value = false
}

async function pickTeamAvatar() {
  const picked = await tauriClient.pickAvatarImage()
  if (!picked) {
    return
  }
  teamPendingAvatar.value = picked
  removeTeamAvatar.value = false
}

function agentAvatarPreview(record?: AgentRecord) {
  if (agentPendingAvatar.value) {
    return `data:${agentPendingAvatar.value.contentType};base64,${agentPendingAvatar.value.dataBase64}`
  }
  if (removeAgentAvatar.value) {
    return ''
  }
  return record?.avatar ?? ''
}

function teamAvatarPreview(record?: TeamRecord) {
  if (teamPendingAvatar.value) {
    return `data:${teamPendingAvatar.value.contentType};base64,${teamPendingAvatar.value.dataBase64}`
  }
  if (removeTeamAvatar.value) {
    return ''
  }
  return record?.avatar ?? ''
}

function currentEditingAgent() {
  return currentAgents.value.find(agent => agent.id === editingAgentId.value)
}

function currentEditingTeam() {
  return currentTeams.value.find(team => team.id === editingTeamId.value)
}

function toTags(tagsText: string) {
  return tagsText.split(',').map(tag => tag.trim()).filter(Boolean)
}

async function saveAgent() {
  if (!workspaceStore.currentWorkspaceId || !agentForm.name.trim()) {
    return
  }
  const input: UpsertAgentInput = {
    workspaceId: workspaceStore.currentWorkspaceId,
    projectId: isProjectScope.value ? projectId.value : undefined,
    scope: isProjectScope.value ? 'project' : 'workspace',
    name: agentForm.name.trim(),
    avatar: agentPendingAvatar.value ?? undefined,
    removeAvatar: removeAgentAvatar.value || undefined,
    personality: agentForm.personality.trim(),
    tags: toTags(agentForm.tagsText),
    prompt: agentForm.prompt.trim(),
    builtinToolKeys: [...agentForm.builtinToolKeys],
    skillIds: [...agentForm.skillIds],
    mcpServerNames: [...agentForm.mcpServerNames],
    description: agentForm.description.trim(),
    status: agentForm.status as AgentRecord['status'],
  }
  if (editingAgentId.value) {
    await agentStore.update(editingAgentId.value, input)
  } else {
    await agentStore.create(input)
  }
  agentDialogOpen.value = false
}

async function saveTeam() {
  if (!workspaceStore.currentWorkspaceId || !teamForm.name.trim()) {
    return
  }
  const input: UpsertTeamInput = {
    workspaceId: workspaceStore.currentWorkspaceId,
    projectId: isProjectScope.value ? projectId.value : undefined,
    scope: isProjectScope.value ? 'project' : 'workspace',
    name: teamForm.name.trim(),
    avatar: teamPendingAvatar.value ?? undefined,
    removeAvatar: removeTeamAvatar.value || undefined,
    personality: teamForm.personality.trim(),
    tags: toTags(teamForm.tagsText),
    prompt: teamForm.prompt.trim(),
    builtinToolKeys: [...teamForm.builtinToolKeys],
    skillIds: [...teamForm.skillIds],
    mcpServerNames: [...teamForm.mcpServerNames],
    leaderAgentId: teamForm.leaderAgentId || undefined,
    memberAgentIds: [...teamForm.memberAgentIds],
    description: teamForm.description.trim(),
    status: teamForm.status as TeamRecord['status'],
  }
  if (editingTeamId.value) {
    await teamStore.update(editingTeamId.value, input)
  } else {
    await teamStore.create(input)
  }
  teamDialogOpen.value = false
}

function removeAgent(record: AgentRecord) {
  itemToDelete.value = { id: record.id, name: record.name, type: 'agent' }
  deleteConfirmOpen.value = true
}

function removeTeam(record: TeamRecord) {
  itemToDelete.value = { id: record.id, name: record.name, type: 'team' }
  deleteConfirmOpen.value = true
}

async function confirmDelete() {
  if (!itemToDelete.value) return
  
  const { id, type } = itemToDelete.value
  if (type === 'agent') {
    const record = currentAgents.value.find(a => a.id === id)
    if (record?.integrationSource && isProjectScope.value && projectId.value) {
      await agentStore.unlinkProject(projectId.value, id)
    } else {
      await agentStore.remove(id)
    }
  } else {
    await teamStore.remove(id)
  }
  
  deleteConfirmOpen.value = false
  itemToDelete.value = null
}

</script>

<template>
  <div class="flex h-full min-h-0 w-full flex-col gap-6 pb-20">
    <AgentsStatsStrip :stats="centerStats" class="px-2" />

    <div class="border-b border-border-subtle px-2 pb-1 dark:border-white/[0.05]">
      <UiTabs
        v-model="activeTab"
        :tabs="tabs"
        @update:model-value="setTab"
      />
    </div>

    <section v-show="activeTab === 'agent'" class="space-y-4 px-2">
      <UiToolbarRow>
        <template #search>
          <div class="relative flex max-w-md items-center group/search">
            <Search :size="15" class="absolute left-0 text-text-tertiary transition-colors group-focus-within/search:text-primary" />
            <UiInput
              v-model="agentQuery"
              placeholder="搜索员工、性格或工具"
              class="border-none bg-transparent pl-7 pr-0 text-sm placeholder:text-text-tertiary/60 focus-visible:ring-0"
            />
          </div>
        </template>
        <template #views>
          <UiButton
            variant="ghost"
            size="sm"
            :class="agentViewMode === 'list' ? 'bg-accent text-text-primary' : ''"
            @click="agentViewMode = 'list'"
          >
            <List :size="14" />
            列表
          </UiButton>
          <UiButton
            variant="ghost"
            size="sm"
            :class="agentViewMode === 'card' ? 'bg-accent text-text-primary' : ''"
            @click="agentViewMode = 'card'"
          >
            <LayoutGrid :size="14" />
            卡片
          </UiButton>
        </template>
        <template #actions>
          <UiButton
            v-if="!isProjectScope"
            variant="outline"
            size="sm"
            :loading="agentImportLoading && !agentImportDialogOpen"
            loading-label="Previewing"
            @click="openAgentImportDialog"
          >
            Import Agent
          </UiButton>
        </template>
      </UiToolbarRow>

      <div v-if="agentTotal" :class="agentViewMode === 'card' ? 'grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4' : 'space-y-2'">
        <template v-for="agent in pagedAgents" :key="agent.id">
          <AgentEmployeeCard
            v-if="agentViewMode === 'card'"
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
            :origin-label="agent.integrationSource ? 'Workspace' : undefined"
            :open-label="agent.integrationSource ? '查看' : '编辑'"
            :remove-label="agent.integrationSource ? '移除接入' : '删除'"
            :open-test-id="`agent-center-open-agent-${agent.id}`"
            :remove-test-id="`agent-center-remove-agent-${agent.id}`"
            @open="openEditAgent(agent)"
            @remove="removeAgent(agent)"
          />

          <UiRecordCard
            v-else
            :title="agent.name"
            interactive
            class="transition-all duration-300 hover:bg-accent/40 group/item"
            @click="openEditAgent(agent)"
          >
            <template #leading>
              <div class="flex size-10 items-center justify-center overflow-hidden rounded-xl bg-primary/5 text-xs font-bold text-primary/60 shadow-inner">
                <img v-if="agent.avatar" :src="agent.avatar" alt="" class="size-full object-cover opacity-90 transition-opacity group-hover/item:opacity-100">
                <span v-else>{{ initials(agent.name) }}</span>
              </div>
            </template>
            <template #badges>
              <div class="flex items-center gap-1.5">
                <div 
                  class="size-2 rounded-full shadow-[0_0_8px_rgba(0,0,0,0.1)]"
                  :class="agent.status === 'active' ? 'bg-emerald-500 shadow-emerald-500/20' : 'bg-slate-400'"
                />
                <UiBadge v-if="agent.integrationSource" label="Workspace" variant="outline" class="h-4 px-1 text-[8px] font-bold bg-primary/5 border-primary/20" />
              </div>
            </template>
            <div class="flex items-center gap-8 w-full overflow-hidden">
              <div class="flex min-w-0 flex-[2] flex-col gap-0.5">
                <span class="truncate text-[11px] font-bold uppercase tracking-wider text-primary/70">{{ agent.personality }}</span>
                <p class="truncate text-sm text-text-secondary/80">
                  {{ agent.description }}
                </p>
              </div>
              <div class="hidden flex-1 shrink-0 items-center gap-1 overflow-hidden lg:flex">
                <span v-for="tag in agent.tags.slice(0, 3)" :key="tag" class="truncate rounded-md bg-accent/30 px-2 py-0.5 text-[10px] font-medium text-text-tertiary">
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
              <div class="flex items-center gap-1 opacity-0 transition-opacity group-hover/item:opacity-100">
                <UiButton size="sm" variant="ghost" class="h-8 px-3 text-[11px] font-bold text-primary hover:bg-primary/5" @click.stop="openEditAgent(agent)">
                  配置
                </UiButton>
                <UiButton
                  variant="ghost"
                  size="icon"
                  class="size-8 rounded-full text-text-tertiary/40 hover:bg-error/10 hover:text-error"
                  @click.stop="removeAgent(agent)"
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
        v-if="agentTotal > 6"
        :page="agentPage"
        :page-count="agentPageCount"
        :meta-label="`共 ${agentTotal} 项`"
        @update:page="agentPagination.setPage"
      />
    </section>

    <section v-show="activeTab === 'team'" class="space-y-4 px-2">
      <UiToolbarRow>
        <template #search>
          <div class="relative flex max-w-md items-center group/search">
            <Search :size="15" class="absolute left-0 text-text-tertiary transition-colors group-focus-within/search:text-primary" />
            <UiInput
              v-model="teamQuery"
              placeholder="搜索团队名称、摘要或成员"
              class="border-none bg-transparent pl-7 pr-0 text-sm placeholder:text-text-tertiary/60 focus-visible:ring-0"
            />
          </div>
        </template>
        <template #views>
          <UiButton
            variant="ghost"
            size="sm"
            :class="teamViewMode === 'list' ? 'bg-accent text-text-primary' : ''"
            @click="teamViewMode = 'list'"
          >
            <List :size="14" />
            列表
          </UiButton>
          <UiButton
            variant="ghost"
            size="sm"
            :class="teamViewMode === 'card' ? 'bg-accent text-text-primary' : ''"
            @click="teamViewMode = 'card'"
          >
            <LayoutGrid :size="14" />
            卡片
          </UiButton>
        </template>
      </UiToolbarRow>

      <div v-if="teamTotal" :class="teamViewMode === 'card' ? 'grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4' : 'space-y-2'">
        <template v-for="team in pagedTeams" :key="team.id">
          <TeamUnitCard
            v-if="teamViewMode === 'card'"
            :id="team.id"
            :name="team.name"
            :title="team.personality || 'Team'"
            :description="team.description"
            :lead-label="currentAgents.find(agent => agent.id === team.leaderAgentId)?.name ?? '未设置负责人'"
            :members="team.memberAgentIds.map(memberId => currentAgents.find(agent => agent.id === memberId)?.name ?? memberId)"
            :workflow="team.tags.slice(0, 3)"
            :recent-outcome="team.prompt || team.description"
            :origin-label="teamOriginLabel(team)"
            :open-label="team.integrationSource ? '查看' : '编辑'"
            :remove-label="team.integrationSource ? '移除接入' : '删除'"
            :open-test-id="`agent-center-open-team-${team.id}`"
            :remove-test-id="`agent-center-remove-team-${team.id}`"
            @open="openEditTeam(team)"
            @remove="removeTeam(team)"
          />

          <UiRecordCard
            v-else
            :title="team.name"
            interactive
            class="transition-all duration-300 hover:bg-accent/40 group/item"
            @click="openEditTeam(team)"
          >
            <template #leading>
              <div class="flex size-10 items-center justify-center overflow-hidden rounded-xl bg-indigo-500/5 text-xs font-bold text-indigo-600/70 shadow-inner">
                <UsersRound :size="18" />
              </div>
            </template>
            <template #badges>
              <div class="flex items-center gap-1.5">
                <div 
                  class="size-2 rounded-full shadow-[0_0_8px_rgba(0,0,0,0.1)]"
                  :class="team.status === 'active' ? 'bg-emerald-500 shadow-emerald-500/20' : 'bg-slate-400'"
                />
                <UiBadge v-if="team.integrationSource" label="Workspace" variant="outline" class="h-4 px-1 text-[8px] font-bold bg-indigo-500/5 border-indigo-500/20 text-indigo-600" />
              </div>
            </template>
            <div class="flex items-center gap-8 w-full overflow-hidden">
              <div class="flex min-w-0 flex-[2] flex-col gap-0.5">
                <span class="truncate text-[11px] font-bold uppercase tracking-wider text-indigo-600/70">{{ team.personality || '数字员工团队' }}</span>
                <p class="truncate text-sm text-text-secondary/80">
                  {{ team.description }}
                </p>
              </div>
              <div class="hidden flex-1 shrink-0 items-center gap-1 overflow-hidden lg:flex">
                <span v-for="tag in team.tags.slice(0, 3)" :key="tag" class="truncate rounded-md bg-indigo-500/5 px-2 py-0.5 text-[10px] font-medium text-indigo-600/70">
                  #{{ tag }}
                </span>
              </div>
              <div class="hidden shrink-0 items-center gap-6 md:flex">
                <div class="flex flex-col items-end">
                  <span class="text-[9px] font-bold uppercase tracking-tighter text-text-tertiary/40">负责人</span>
                  <span class="text-xs font-bold text-text-primary/70 truncate max-w-[80px]">
                    {{ currentAgents.find(agent => agent.id === team.leaderAgentId)?.name ?? '未设置' }}
                  </span>
                </div>
                <div class="flex flex-col items-end">
                  <span class="text-[9px] font-bold uppercase tracking-tighter text-text-tertiary/40">成员</span>
                  <span class="text-xs font-bold tabular-nums text-text-primary/70">{{ team.memberAgentIds.length }}</span>
                </div>
              </div>
            </div>
            <template #actions>
              <div class="flex items-center gap-1 opacity-0 transition-opacity group-hover/item:opacity-100">
                <UiButton size="sm" variant="ghost" class="h-8 px-3 text-[11px] font-bold text-indigo-600 hover:bg-indigo-500/5" @click.stop="openEditTeam(team)">
                  配置
                </UiButton>
                <UiButton
                  variant="ghost"
                  size="icon"
                  class="size-8 rounded-full text-text-tertiary/40 hover:bg-error/10 hover:text-error"
                  @click.stop="removeTeam(team)"
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
        title="暂无团队"
        description="创建工作区或项目团队。"
      />

      <UiPagination
        v-if="teamTotal > 6"
        :page="teamPage"
        :page-count="teamPageCount"
        :meta-label="`共 ${teamTotal} 项`"
        @update:page="teamPagination.setPage"
      />
    </section>

    <UiDialog
      :open="agentDialogOpen"
      title="员工配置"
      description="配置数字员工的基本信息、核心性格与工具能力。"
      content-class="max-w-3xl"
      body-class="max-h-[75vh] overflow-y-auto pr-1"
      @update:open="agentDialogOpen = $event"
    >
      <div class="flex flex-col gap-8 py-2">
        <!-- Basic Info Section -->
        <section class="space-y-4">
          <UiSectionHeading title="基本信息" description="设置员工的展示名称、状态和视觉标识。">
            <template #icon><UserCheck :size="16" class="text-primary" /></template>
          </UiSectionHeading>
          
          <div class="grid gap-4 md:grid-cols-2">
            <UiField label="员工名称">
              <UiInput v-model="agentForm.name" placeholder="例如: 研发专家" />
            </UiField>
            <UiField label="状态">
              <UiSelect v-model="agentForm.status" :options="statusOptions" />
            </UiField>
            <UiField class="md:col-span-2" label="头像标识">
              <div class="flex items-center gap-4 rounded-2xl border border-border/40 bg-accent/20 p-4 dark:border-white/[0.08]">
                <div class="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-primary/[0.08] text-lg font-bold text-primary shadow-inner">
                  <img v-if="agentAvatarPreview(currentEditingAgent())" :src="agentAvatarPreview(currentEditingAgent())" alt="" class="size-full object-cover">
                  <span v-else>{{ initials(agentForm.name || 'A') }}</span>
                </div>
                <div class="flex flex-col gap-2">
                  <div class="flex gap-2">
                    <UiButton variant="outline" size="sm" class="h-8" @click="pickAgentAvatar">上传新头像</UiButton>
                    <UiButton v-if="agentAvatarPreview(currentEditingAgent())" variant="ghost" size="sm" class="h-8 text-text-tertiary" @click="removeAgentAvatar = true; agentPendingAvatar = null">移除</UiButton>
                  </div>
                  <p class="text-xs text-text-tertiary">建议使用正方形图片，支持 PNG/JPG 格式。</p>
                </div>
              </div>
            </UiField>
            <UiField class="md:col-span-2" label="核心摘要">
              <UiTextarea v-model="agentForm.description" :rows="2" placeholder="简要描述该员工的主要职责..." />
            </UiField>
          </div>
        </section>

        <!-- Personality & Prompt Section -->
        <section class="space-y-4">
          <UiSectionHeading title="性格与提示词" description="定义员工的行事风格、专业背景和核心工作指令。">
            <template #icon><Wand2 :size="16" class="text-primary" /></template>
          </UiSectionHeading>

          <div class="grid gap-4 md:grid-cols-1">
            <UiField label="性格设定">
              <UiInput v-model="agentForm.personality" placeholder="例如: 严谨、专业、富有逻辑" />
            </UiField>
            <UiField label="系统提示词 (System Prompt)">
              <UiTextarea v-model="agentForm.prompt" :rows="6" class="font-mono text-sm" placeholder="编写核心指令..." />
            </UiField>
            <UiField label="分类标签">
              <UiInput v-model="agentForm.tagsText" placeholder="用逗号分隔，例如: 开发, 代码审查, Rust" />
            </UiField>
          </div>
        </section>

        <!-- Capabilities Section -->
        <section class="space-y-4">
          <UiSectionHeading title="能力与工具" description="通过内置工具、扩展技能和 MCP 服务增强员工能力。">
            <template #icon><LayoutGrid :size="16" class="text-primary" /></template>
          </UiSectionHeading>

          <div class="grid gap-4 md:grid-cols-2">
            <UiField label="内置工具">
              <UiSearchableMultiSelect
                v-model="agentForm.builtinToolKeys"
                :options="builtinOptions"
                placeholder="搜索并选择工具"
              />
            </UiField>
            <UiField label="技能插件 (Skills)">
              <UiSearchableMultiSelect
                v-model="agentForm.skillIds"
                :options="skillOptions"
                placeholder="搜索并选择技能"
              />
            </UiField>
            <UiField class="md:col-span-2" label="MCP 服务集成">
              <UiSearchableMultiSelect
                v-model="agentForm.mcpServerNames"
                :options="mcpOptions"
                placeholder="搜索并选择 MCP 服务器"
              />
            </UiField>
          </div>
        </section>
      </div>
      <template #footer>
        <div class="flex w-full items-center justify-between">
          <span class="text-xs text-text-tertiary">所有更改将立即同步至 {{ props.scope }}</span>
          <div class="flex items-center gap-2">
            <UiButton variant="ghost" @click="agentDialogOpen = false">取消</UiButton>
            <UiButton class="px-6" @click="saveAgent">保存配置</UiButton>
          </div>
        </div>
      </template>
    </UiDialog>

    <UiDialog
      :open="teamDialogOpen"
      title="团队配置"
      description="配置团队负责人、核心成员以及协作背景。"
      content-class="max-w-3xl"
      body-class="max-h-[75vh] overflow-y-auto pr-1"
      @update:open="teamDialogOpen = $event"
    >
      <div class="flex flex-col gap-8 py-2">
        <!-- Basic info -->
        <section class="space-y-4">
          <UiSectionHeading title="基础信息" description="设置团队名称、头像和运行状态。">
            <template #icon><Users :size="16" class="text-primary" /></template>
          </UiSectionHeading>
          
          <div class="grid gap-4 md:grid-cols-2">
            <UiField label="团队名称">
              <UiInput v-model="teamForm.name" placeholder="例如: 核心研发组" />
            </UiField>
            <UiField label="状态">
              <UiSelect v-model="teamForm.status" :options="statusOptions" />
            </UiField>
            <UiField class="md:col-span-2" label="团队头像">
              <div class="flex items-center gap-4 rounded-2xl border border-border/40 bg-accent/20 p-4 dark:border-white/[0.08]">
                <div class="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-primary/[0.08] text-lg font-bold text-primary shadow-inner">
                  <img v-if="teamAvatarPreview(currentEditingTeam())" :src="teamAvatarPreview(currentEditingTeam())" alt="" class="size-full object-cover">
                  <span v-else>{{ initials(teamForm.name || 'T') }}</span>
                </div>
                <div class="flex flex-col gap-2">
                  <div class="flex gap-2">
                    <UiButton variant="outline" size="sm" class="h-8" @click="pickTeamAvatar">上传新头像</UiButton>
                    <UiButton v-if="teamAvatarPreview(currentEditingTeam())" variant="ghost" size="sm" class="h-8 text-text-tertiary" @click="removeTeamAvatar = true; teamPendingAvatar = null">移除</UiButton>
                  </div>
                  <p class="text-xs text-text-tertiary">建议使用正方形图片，支持 PNG/JPG 格式。</p>
                </div>
              </div>
            </UiField>
            <UiField class="md:col-span-2" label="核心愿景">
              <UiInput v-model="teamForm.personality" placeholder="定义团队的协作风格和长期目标" />
            </UiField>
          </div>
        </section>

        <!-- Composition -->
        <section class="space-y-4">
          <UiSectionHeading title="编组管理" description="指定团队负责人 (Leader) 并选择参与协作的成员。">
            <template #icon><Network :size="16" class="text-primary" /></template>
          </UiSectionHeading>

          <div class="grid gap-4 md:grid-cols-2">
            <UiField label="负责人 (Leader)">
              <UiCombobox
                v-model="teamForm.leaderAgentId"
                :options="leaderOptions"
                placeholder="选择负责人"
              />
            </UiField>
            <UiField class="md:col-span-2" label="协作成员">
              <UiSearchableMultiSelect
                v-model="teamForm.memberAgentIds"
                :options="teamAgentOptions"
                placeholder="搜索并添加数字员工成员"
              />
            </UiField>

            <!-- Relationship Visualization -->
            <div class="md:col-span-2">
              <UiSurface class="overflow-hidden border-border/40 bg-[linear-gradient(180deg,color-mix(in_srgb,var(--bg-subtle)_72%,transparent),transparent)] p-5 dark:border-white/[0.08]">
                <div class="mb-5 flex items-center justify-between">
                  <div class="flex items-center gap-2 text-sm font-semibold text-text-primary">
                    <Network :size="15" class="text-primary" />
                    组织结构预览
                  </div>
                  <UiBadge :label="`${dialogTeamMembers.length + (dialogTeamLeader ? 1 : 0)} 节点`" subtle />
                </div>
                
                <div class="flex flex-col items-center">
                  <div v-if="dialogTeamLeader" class="relative">
                    <div class="min-w-[12rem] rounded-2xl border border-primary/20 bg-background px-4 py-3 text-center shadow-sm">
                      <div class="text-[10px] font-bold uppercase tracking-[0.16em] text-primary">Leader</div>
                      <div class="mt-1 text-sm font-bold text-text-primary">{{ dialogTeamLeader.name }}</div>
                      <div class="mt-1 line-clamp-1 text-[11px] text-text-tertiary">{{ dialogTeamLeader.personality }}</div>
                    </div>
                    <div class="mx-auto h-8 w-px bg-gradient-to-b from-primary/30 to-border/60" />
                  </div>
                  <div v-else class="py-6 text-sm text-text-tertiary italic">
                    尚未指定团队负责人
                  </div>

                  <div v-if="dialogTeamMembers.length > 0" class="grid w-full gap-3 pt-2 sm:grid-cols-2 lg:grid-cols-3">
                    <div
                      v-for="member in dialogTeamMembers"
                      :key="member.id"
                      class="group relative rounded-xl border border-border/40 bg-background p-3 transition-colors hover:border-primary/30"
                    >
                      <div class="flex items-center justify-between gap-2">
                        <strong class="truncate text-sm font-semibold text-text-primary group-hover:text-primary">{{ member.name }}</strong>
                        <UiBadge v-if="member.integrationSource" label="Linked" variant="outline" class="scale-90" />
                      </div>
                      <div class="mt-1 line-clamp-1 text-[11px] text-text-tertiary">{{ member.personality }}</div>
                    </div>
                  </div>
                  <div v-else-if="dialogTeamLeader" class="py-4 text-xs text-text-tertiary italic">
                    暂未添加其他成员
                  </div>
                </div>
              </UiSurface>
            </div>
          </div>
        </section>

        <!-- Configuration -->
        <section class="space-y-4">
          <UiSectionHeading title="协作上下文" description="定义团队的工作流程标签、协作摘要及核心指令。">
            <template #icon><Wand2 :size="16" class="text-primary" /></template>
          </UiSectionHeading>

          <div class="grid gap-4">
            <UiField label="团队摘要">
              <UiTextarea v-model="teamForm.description" :rows="2" placeholder="简述团队的主要工作职责和产出目标..." />
            </UiField>
            <UiField label="协作提示词 (Team Prompt)">
              <UiTextarea v-model="teamForm.prompt" :rows="5" class="font-mono text-sm" placeholder="定义团队的协作 SOP 和核心指令..." />
            </UiField>
            <UiField label="工作流标签">
              <UiInput v-model="teamForm.tagsText" placeholder="用逗号分隔，例如: 并行研发, 自动化测试" />
            </UiField>
          </div>
        </section>

        <!-- Tools Integration -->
        <section class="space-y-4">
          <UiSectionHeading title="扩展能力" description="为整个团队集成工具能力。">
            <template #icon><LayoutGrid :size="16" class="text-primary" /></template>
          </UiSectionHeading>

          <div class="grid gap-4 md:grid-cols-2">
            <UiField label="内置工具">
              <UiSearchableMultiSelect v-model="teamForm.builtinToolKeys" :options="builtinOptions" placeholder="选择内置工具" />
            </UiField>
            <UiField label="技能插件 (Skills)">
              <UiSearchableMultiSelect v-model="teamForm.skillIds" :options="skillOptions" placeholder="选择 Skill" />
            </UiField>
            <UiField class="md:col-span-2" label="MCP 服务集成">
              <UiSearchableMultiSelect v-model="teamForm.mcpServerNames" :options="mcpOptions" placeholder="选择 MCP 服务器" />
            </UiField>
          </div>
        </section>
      </div>
      <template #footer>
        <div class="flex w-full items-center justify-between">
          <span class="text-xs text-text-tertiary">团队更改将应用于所有成员的协作上下文</span>
          <div class="flex items-center gap-2">
            <UiButton variant="ghost" @click="teamDialogOpen = false">取消</UiButton>
            <UiButton class="px-6" @click="saveTeam">保存配置</UiButton>
          </div>
        </div>
      </template>
    </UiDialog>

    <UiDialog
      :open="deleteConfirmOpen"
      title="确认删除"
      :description="`您确定要删除「${itemToDelete?.name}」吗？此操作无法撤销。`"
      content-class="max-w-md"
      @update:open="deleteConfirmOpen = $event"
    >
      <div class="py-4 text-sm text-text-secondary">
        删除此项将永久移除相关配置及协作关系，且无法恢复。
      </div>
      <template #footer>
        <div class="flex w-full items-center justify-end gap-2">
          <UiButton variant="ghost" @click="deleteConfirmOpen = false">取消</UiButton>
          <UiButton variant="outline" class="border-error/20 text-error hover:bg-error/10 hover:border-error/40" @click="confirmDelete">确认删除</UiButton>
        </div>
      </template>
    </UiDialog>

    <AgentBundleImportDialog
      :open="agentImportDialogOpen"
      :preview="agentImportPreview"
      :result="agentImportResult"
      :loading="agentImportLoading"
      :error-message="agentImportError"
      @update:open="handleAgentImportDialogOpen"
      @confirm="confirmAgentImport"
    />
  </div>
</template>
