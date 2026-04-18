import { computed, nextTick, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import type {
  AgentRecord,
  AvatarUploadPayload,
  CapabilityManagementEntry,
  ExportWorkspaceAgentBundleInput,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundleResult,
  TeamRecord,
  UpsertAgentInput,
  UpsertTeamInput,
  WorkspaceDirectoryUploadEntry,
} from '@octopus/schema'

import { usePagination } from '@/composables/usePagination'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useNotificationStore } from '@/stores/notifications'
import { usePetStore } from '@/stores/pet'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceStore } from '@/stores/workspace'
import * as tauriClient from '@/tauri/client'

import { agentIdFromRef, canonicalAgentRef, matchesAgentRef } from './agent-refs'

export type CenterScope = 'workspace' | 'project'
export type CenterTab = 'agent' | 'team' | 'builtin' | 'skill' | 'mcp'
export type ViewMode = 'list' | 'card'
export type AgentBundleTransferFormat = 'folder' | 'zip'

export interface SelectOption {
  value: string
  label: string
  keywords?: string[]
  helper?: string
  disabled?: boolean
}

export interface AgentFormState {
  name: string
  description: string
  personality: string
  tagsText: string
  prompt: string
  builtinToolKeys: string[]
  skillIds: string[]
  mcpServerNames: string[]
  status: string
}

export interface TeamFormState {
  name: string
  description: string
  personality: string
  tagsText: string
  prompt: string
  builtinToolKeys: string[]
  skillIds: string[]
  mcpServerNames: string[]
  leaderRef: string
  memberRefs: string[]
  status: string
}

type AgentRecordWithAssetRole = AgentRecord & { assetRole?: string }

export function useAgentCenter(scope: CenterScope) {
  const { t } = useI18n()
  const route = useRoute()
  const router = useRouter()
  const shell = useShellStore()
  const workspaceStore = useWorkspaceStore()
  const agentStore = useAgentStore()
  const teamStore = useTeamStore()
  const catalogStore = useCatalogStore()
  const petStore = usePetStore()
  const notificationStore = useNotificationStore()

  const activeTab = ref<CenterTab>('agent')
  const agentViewMode = ref<ViewMode>('card')
  const teamViewMode = ref<ViewMode>('card')
  const agentQuery = ref('')
  const teamQuery = ref('')
  const resourceQuery = ref('')

  const agentDialogOpen = ref(false)
  const teamDialogOpen = ref(false)
  const editingAgentId = ref<string | null>(null)
  const editingTeamId = ref<string | null>(null)
  const editingAgentContentReadonly = ref(false)
  const editingTeamContentReadonly = ref(false)
  const editingAgentStatusReadonly = ref(false)
  const editingTeamStatusReadonly = ref(false)
  const agentPendingAvatar = ref<AvatarUploadPayload | null>(null)
  const teamPendingAvatar = ref<AvatarUploadPayload | null>(null)
  const removeAgentAvatar = ref(false)
  const removeTeamAvatar = ref(false)

  const deleteConfirmOpen = ref(false)
  const itemToDelete = ref<{ id: string, name: string, type: 'agent' | 'team' } | null>(null)
  const agentImportDialogOpen = ref(false)
  const agentImportSource = ref<AgentBundleTransferFormat>('folder')
  const agentImportFiles = ref<WorkspaceDirectoryUploadEntry[]>([])
  const agentImportPreview = ref<ImportWorkspaceAgentBundlePreview | null>(null)
  const agentImportResult = ref<ImportWorkspaceAgentBundleResult | null>(null)
  const agentImportError = ref('')
  const agentImportLoading = ref(false)
  const agentExportLoading = ref(false)
  const teamExportLoading = ref(false)
  const promoteAgentLoading = ref(false)
  const promoteTeamLoading = ref(false)
  const selectedAgentIds = ref<string[]>([])
  const selectedTeamIds = ref<string[]>([])

  const agentForm = reactive<AgentFormState>({
    name: '',
    description: '',
    personality: '',
    tagsText: '',
    prompt: '',
    builtinToolKeys: [],
    skillIds: [],
    mcpServerNames: [],
    status: 'active',
  })

  const teamForm = reactive<TeamFormState>({
    name: '',
    description: '',
    personality: '',
    tagsText: '',
    prompt: '',
    builtinToolKeys: [],
    skillIds: [],
    mcpServerNames: [],
    leaderRef: '',
    memberRefs: [],
    status: 'active',
  })

  const isProjectScope = computed(() => scope === 'project')
  const isBuiltinTemplateRecord = (record: Pick<AgentRecord | TeamRecord, 'integrationSource'>) =>
    record.integrationSource?.kind === 'builtin-template'
  const projectId = computed(() =>
    typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId || '',
  )
  const isProjectOwnedRecord = (record: AgentRecord | TeamRecord) =>
    Boolean(record.projectId) && record.projectId === projectId.value
  const isReadonlyProjectRecord = (record: AgentRecord | TeamRecord) =>
    isProjectScope.value && !isProjectOwnedRecord(record)
  const isSelectableRecord = (record: AgentRecord | TeamRecord) =>
    isProjectScope.value || !isBuiltinTemplateRecord(record)
  const isExportableRecord = (record: AgentRecord | TeamRecord) =>
    isProjectScope.value || !isBuiltinTemplateRecord(record)
  const isRemovableRecord = (record: AgentRecord | TeamRecord) =>
    isProjectScope.value ? isProjectOwnedRecord(record) : !isBuiltinTemplateRecord(record)
  // Keep personal pet assets out of generic agent management even if one leaks into local client state.
  const isPetAssetRecord = (record: AgentRecord) =>
    (record as AgentRecordWithAssetRole).assetRole === 'pet'
  const isVisibleAgentRecord = (record: AgentRecord) =>
    record.id !== petStore.profile.id && !isPetAssetRecord(record)
  const currentProject = computed(() =>
    workspaceStore.projects.find(project => project.id === projectId.value) ?? null,
  )
  const currentAgents = computed(() => {
    if (!isProjectScope.value) {
      return [...agentStore.workspaceOwnedAgents, ...agentStore.builtinTemplateAgents]
        .filter(isVisibleAgentRecord)
    }
    return agentStore.effectiveProjectAgents.filter(isVisibleAgentRecord)
  })
  const currentTeams = computed(() => {
    if (!isProjectScope.value) {
      return [...teamStore.workspaceOwnedTeams, ...teamStore.builtinTemplateTeams]
    }
    return teamStore.effectiveProjectTeams
  })
  const resolvedAgents = computed(() => {
    const merged = [...currentAgents.value, ...agentStore.agents]
    return merged
      .filter(record => isVisibleAgentRecord(record))
      .filter((record, index) => merged.findIndex(item => item.id === record.id) === index)
  })
  const effectiveProjectAgents = computed(() => currentAgents.value)
  const effectiveProjectTeams = computed(() => currentTeams.value)
  const pageTitle = computed(() =>
    isProjectScope.value ? (currentProject.value?.name ?? t('sidebar.navigation.agents')) : t('sidebar.navigation.agents'),
  )
  const pageDescription = computed(() =>
    isProjectScope.value ? (currentProject.value?.description ?? '') : '',
  )
  const builtinOptions = computed<SelectOption[]>(() =>
    catalogStore.managementEntries
      .filter(entry => entry.kind === 'builtin')
      .map(entry => ({
        value: entry.builtinKey ?? entry.name,
        label: entry.name,
        keywords: [entry.description, entry.sourceKey].filter(Boolean),
        helper: entry.description,
      })),
  )
  const skillOptions = computed<SelectOption[]>(() =>
    catalogStore.managementEntries
      .filter(entry => entry.kind === 'skill')
      .map(entry => ({
        value: entry.id,
        label: entry.name,
        keywords: [entry.description, entry.sourceKey].filter(Boolean),
        helper: entry.displayPath,
      })),
  )
  const mcpOptions = computed<SelectOption[]>(() =>
    catalogStore.managementProjection.mcpServerPackages
      .map(entry => ({
        value: entry.serverName,
        label: entry.name,
        keywords: [entry.description, entry.sourceKey].filter(Boolean),
        helper: entry.displayPath,
      })),
  )
  const statusOptions = computed(() => [
    { value: 'active', label: t('agents.status.active') },
    { value: 'archived', label: t('agents.status.archived') },
  ])
  const teamAgentOptions = computed<SelectOption[]>(() =>
    currentAgents.value.map(agent => ({
      value: canonicalAgentRef(agent.id),
      label: agent.name,
      keywords: [agent.personality, ...agent.tags],
      helper: agent.personality,
      disabled: isProjectScope.value ? !isProjectOwnedRecord(agent) : false,
    })),
  )
  const currentEditingAgentRecord = computed(() => currentAgents.value.find(agent => agent.id === editingAgentId.value))
  const currentEditingTeamRecord = computed(() => currentTeams.value.find(team => team.id === editingTeamId.value))
  const dialogTeamLeader = computed(() =>
    resolvedAgents.value.find(agent =>
      matchesAgentRef(agent.id, teamForm.leaderRef || currentEditingTeamRecord.value?.leaderRef),
    ),
  )
  const dialogTeamMembers = computed(() => {
    const selectedRefs = teamForm.memberRefs.length
      ? teamForm.memberRefs
      : (currentEditingTeamRecord.value?.memberRefs ?? [])
    const selectedSet = new Set(selectedRefs.map(canonicalAgentRef))
    return resolvedAgents.value.filter(agent => selectedSet.has(canonicalAgentRef(agent.id)))
  })
  const leaderOptions = computed<SelectOption[]>(() =>
    currentAgents.value
      .filter(agent => !isProjectScope.value || isProjectOwnedRecord(agent))
      .map(agent => ({
        value: canonicalAgentRef(agent.id),
        label: agent.name,
        keywords: [agent.personality, ...agent.tags],
      })),
  )
  const agentDialogContentReadonly = computed(() => editingAgentContentReadonly.value)
  const teamDialogContentReadonly = computed(() => editingTeamContentReadonly.value)
  const agentDialogStatusReadonly = computed(() => editingAgentStatusReadonly.value)
  const teamDialogStatusReadonly = computed(() => editingTeamStatusReadonly.value)
  const canSaveAgentDialog = computed(() => !agentDialogStatusReadonly.value)
  const canSaveTeamDialog = computed(() => !teamDialogStatusReadonly.value)
  const currentEditingAgentCopyLabel = computed(() => isProjectScope.value ? '复制到项目' : '复制到工作区')
  const currentEditingTeamCopyLabel = computed(() => isProjectScope.value ? '复制到项目' : '复制到工作区')
  const canCopyCurrentEditingAgent = computed(() =>
    Boolean(currentEditingAgentRecord.value && isBuiltinTemplateRecord(currentEditingAgentRecord.value)),
  )
  const canCopyCurrentEditingTeam = computed(() =>
    Boolean(currentEditingTeamRecord.value && isBuiltinTemplateRecord(currentEditingTeamRecord.value)),
  )
  const tabValues: CenterTab[] = ['agent', 'team', 'builtin', 'skill', 'mcp']
  const tabs = computed(() => ([
    { value: 'agent', label: t('agents.tabs.agents') },
    { value: 'team', label: t('agents.tabs.teams') },
    { value: 'builtin', label: t('tools.tabs.builtin') },
    { value: 'skill', label: t('tools.tabs.skill') },
    { value: 'mcp', label: t('tools.tabs.mcp') },
  ]))

  watch(
    () => route.query.tab,
    (value) => {
      activeTab.value = typeof value === 'string' && tabValues.includes(value as CenterTab)
        ? value as CenterTab
        : 'agent'
    },
    { immediate: true },
  )

  watch(
    () => [shell.activeWorkspaceConnectionId, projectId.value],
    async ([connectionId, nextProjectId]) => {
      if (!connectionId) {
        return
      }

      await reloadCenterData(connectionId, nextProjectId)
    },
    { immediate: true },
  )

  watch(
    () => currentAgents.value.map(agent => agent.id).join('|'),
    () => {
      const validIds = new Set(currentAgents.value
        .filter(agent => isSelectableRecord(agent))
        .map(agent => agent.id))
      selectedAgentIds.value = selectedAgentIds.value.filter(id => validIds.has(id))
    },
  )

  watch(
    () => currentTeams.value.map(team => team.id).join('|'),
    () => {
      const validIds = new Set(currentTeams.value
        .filter(team => isSelectableRecord(team))
        .map(team => team.id))
      selectedTeamIds.value = selectedTeamIds.value.filter(id => validIds.has(id))
    },
  )

  function catalogLabels(values: string[], options: SelectOption[]) {
    const labelMap = new Map(options.map(option => [option.value, option.label]))
    return values.map(value => labelMap.get(value) ?? value)
  }

  function isTeamRecord(record: AgentRecord | TeamRecord): record is TeamRecord {
    return 'leaderRef' in record && 'memberRefs' in record
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
    if (isTeamRecord(record)) {
      parts.push(agentIdFromRef(record.leaderRef))
      parts.push(...record.memberRefs.map(agentIdFromRef))
    }
    return parts.join(' ').toLowerCase().includes(keyword)
  }

  const filteredAgents = computed(() => currentAgents.value.filter(agent => matchesQuery(agent, agentQuery.value)))
  const filteredTeams = computed(() => currentTeams.value.filter(team => matchesQuery(team, teamQuery.value)))
  const selectedAgents = computed(() =>
    currentAgents.value.filter(agent => selectedAgentIds.value.includes(agent.id)),
  )
  const selectedTeams = computed(() =>
    currentTeams.value.filter(team => selectedTeamIds.value.includes(team.id)),
  )
  const activeResourceKind = computed<CapabilityManagementEntry['kind'] | null>(() =>
    activeTab.value === 'builtin' || activeTab.value === 'skill' || activeTab.value === 'mcp'
      ? activeTab.value
      : null,
  )
  const effectiveBuiltinToolKeys = computed(() => {
    const targetAgents = effectiveProjectAgents.value
    const targetTeams = effectiveProjectTeams.value
    return new Set([
      ...targetAgents.flatMap(agent => agent.builtinToolKeys),
      ...targetTeams.flatMap(team => team.builtinToolKeys),
    ])
  })
  const effectiveSkillIds = computed(() => {
    const targetAgents = effectiveProjectAgents.value
    const targetTeams = effectiveProjectTeams.value
    return new Set([
      ...targetAgents.flatMap(agent => agent.skillIds),
      ...targetTeams.flatMap(team => team.skillIds),
    ])
  })
  const effectiveMcpServerNames = computed(() => {
    const targetAgents = effectiveProjectAgents.value
    const targetTeams = effectiveProjectTeams.value
    return new Set([
      ...targetAgents.flatMap(agent => agent.mcpServerNames),
      ...targetTeams.flatMap(team => team.mcpServerNames),
    ])
  })
  const effectiveResourceConsumerIds = computed(() =>
    new Set([
      ...effectiveProjectAgents.value.map(agent => agent.id),
      ...effectiveProjectTeams.value.map(team => team.id),
    ]),
  )
  const resourceCatalogEntries = computed(() => {
    const projectOwnerId = projectId.value
    return catalogStore.managementEntries
      .filter((entry) => {
        if (!isProjectScope.value) {
          return true
        }

        if (entry.kind === 'builtin') {
          return entry.builtinKey ? effectiveBuiltinToolKeys.value.has(entry.builtinKey) : false
        }
        if (entry.kind === 'skill') {
          return effectiveSkillIds.value.has(entry.id)
            || (entry.ownerScope === 'project' && entry.ownerId === projectOwnerId)
        }
        return (entry.serverName ? effectiveMcpServerNames.value.has(entry.serverName) : false)
          || (entry.ownerScope === 'project' && entry.ownerId === projectOwnerId)
      })
      .map((entry) => ({
        ...entry,
        consumers: isProjectScope.value
          ? entry.consumers?.filter(consumer => effectiveResourceConsumerIds.value.has(consumer.id))
          : entry.consumers,
      }))
  })
  const filteredResourceEntries = computed(() => {
    const kind = activeResourceKind.value
    if (!kind) {
      return [] as CapabilityManagementEntry[]
    }
    const query = resourceQuery.value.trim().toLowerCase()
    return resourceCatalogEntries.value.filter((entry) => {
      if (entry.kind !== kind) {
        return false
      }
      if (!query) {
        return true
      }
      const haystack = [
        entry.name,
        entry.description,
        entry.displayPath,
        entry.sourceKey,
        entry.ownerLabel ?? '',
        ...(entry.consumers?.map(consumer => consumer.name) ?? []),
        entry.kind === 'mcp' ? entry.serverName : '',
        entry.kind === 'mcp' ? entry.endpoint : '',
        entry.kind === 'mcp' ? entry.toolNames.join(' ') : '',
        entry.kind === 'skill' ? entry.relativePath ?? '' : '',
        entry.kind === 'skill' ? entry.shadowedBy ?? '' : '',
        entry.kind === 'builtin' ? entry.builtinKey : '',
        entry.ownerScope ?? '',
      ].join(' ').toLowerCase()
      return haystack.includes(query)
    })
  })

  const agentPagination = usePagination(filteredAgents, {
    pageSize: 20,
    resetOn: [agentQuery, () => scope, projectId],
  })
  const teamPagination = usePagination(filteredTeams, {
    pageSize: 20,
    resetOn: [teamQuery, () => scope, projectId],
  })
  const resourcePagination = usePagination(filteredResourceEntries, {
    pageSize: 6,
    resetOn: [resourceQuery, activeResourceKind, () => scope, projectId],
  })
  const pagedAgents = computed(() => agentPagination.pagedItems.value)
  const pagedTeams = computed(() => teamPagination.pagedItems.value)
  const pagedResources = computed(() => resourcePagination.pagedItems.value)
  const allPagedAgentsSelected = computed(() =>
    pagedAgents.value.some(agent => isSelectableRecord(agent))
      && pagedAgents.value
        .filter(agent => isSelectableRecord(agent))
        .every(agent => selectedAgentIds.value.includes(agent.id)),
  )
  const allPagedTeamsSelected = computed(() =>
    pagedTeams.value.some(team => isSelectableRecord(team))
      && pagedTeams.value
        .filter(team => isSelectableRecord(team))
        .every(team => selectedTeamIds.value.includes(team.id)),
  )
  const agentTotal = computed(() => agentPagination.totalItems.value)
  const teamTotal = computed(() => teamPagination.totalItems.value)
  const resourceTotal = computed(() => resourcePagination.totalItems.value)
  const agentPage = computed(() => agentPagination.currentPage.value)
  const teamPage = computed(() => teamPagination.currentPage.value)
  const resourcePage = computed(() => resourcePagination.currentPage.value)
  const agentPageCount = computed(() => agentPagination.pageCount.value)
  const teamPageCount = computed(() => teamPagination.pageCount.value)
  const resourcePageCount = computed(() => resourcePagination.pageCount.value)

  const centerStats = computed(() => [
    {
      label: '活跃员工',
      value: String(currentAgents.value.filter(a => a.status === 'active').length),
      helper: '当前可用数字员工数量',
      tone: 'success' as const,
    },
    {
      label: '数字团队',
      value: String(currentTeams.value.length),
      helper: '已组建的数字员工团队',
      tone: 'info' as const,
    },
    {
      label: '工具集成',
      value: String(catalogStore.managementEntries.length),
      helper: '可用工具与技能总数',
      tone: 'default' as const,
    },
    {
      label: '待办任务',
      value: '12',
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
    if (agent.integrationSource?.kind === 'builtin-template') {
      return '内置模板'
    }
    if (isReadonlyProjectRecord(agent)) {
      return '工作区'
    }
    return agent.status
  }

  function teamOriginLabel(team: TeamRecord) {
    if (team.integrationSource?.kind === 'builtin-template') {
      return '内置模板'
    }
    if (isReadonlyProjectRecord(team)) {
      return '工作区'
    }
    return undefined
  }

  function setTab(nextTab: string) {
    const value = tabValues.includes(nextTab as CenterTab) ? nextTab as CenterTab : 'agent'
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
    editingAgentContentReadonly.value = Boolean(record)
    editingAgentStatusReadonly.value = Boolean(record && isBuiltinTemplateRecord(record))
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
    editingTeamContentReadonly.value = Boolean(record)
    editingTeamStatusReadonly.value = Boolean(record && isBuiltinTemplateRecord(record))
    teamForm.name = record?.name ?? ''
    teamForm.description = record?.description ?? ''
    teamForm.personality = record?.personality ?? ''
    teamForm.tagsText = (record?.tags ?? []).join(', ')
    teamForm.prompt = record?.prompt ?? ''
    teamForm.builtinToolKeys = [...(record?.builtinToolKeys ?? [])]
    teamForm.skillIds = [...(record?.skillIds ?? [])]
    teamForm.mcpServerNames = [...(record?.mcpServerNames ?? [])]
    teamForm.leaderRef = record?.leaderRef ?? ''
    teamForm.memberRefs = [...(record?.memberRefs ?? [])]
    teamForm.status = record?.status ?? 'active'
    teamPendingAvatar.value = null
    removeTeamAvatar.value = false
  }

  async function reloadCenterData(connectionId = shell.activeWorkspaceConnectionId, nextProjectId = projectId.value) {
    if (!connectionId) {
      return
    }

    if (!isProjectScope.value) {
      await petStore.loadSnapshot(undefined, connectionId, true)
    }

    const tasks: Promise<unknown>[] = [
      agentStore.load(connectionId),
      teamStore.load(connectionId),
      catalogStore.load(connectionId),
    ]
    await Promise.all(tasks)
  }

  function openCreateAgent() {
    resetAgentForm()
    agentDialogOpen.value = true
  }

  async function previewAgentImportFiles(files: WorkspaceDirectoryUploadEntry[]) {
    agentImportLoading.value = true
    try {
      const preview = await agentStore.previewImportBundle(
        { files },
        isProjectScope.value ? projectId.value : undefined,
      )
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

  async function openAgentImportDialog(source: AgentBundleTransferFormat = 'folder') {
    agentImportError.value = ''
    agentImportPreview.value = null
    agentImportResult.value = null
    agentImportSource.value = source
    try {
      const files = source === 'zip'
        ? await tauriClient.pickAgentBundleArchive()
        : await tauriClient.pickAgentBundleFolder()
      if (!files?.length) {
        return
      }
      agentImportFiles.value = files
      agentImportDialogOpen.value = true
      await nextTick()
      await previewAgentImportFiles(files)
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to open agent bundle import dialog'
      await notifyTransfer('error', '导入失败', message)
    }
  }

  async function confirmAgentImport() {
    if (!agentImportFiles.value.length) {
      return
    }

    agentImportLoading.value = true
    agentImportError.value = ''
    try {
      const result = await agentStore.importBundle(
        { files: agentImportFiles.value },
        isProjectScope.value ? projectId.value : undefined,
      )
      agentImportResult.value = result
      await reloadCenterData()
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
    agentImportSource.value = 'folder'
  }

  function toggleAllPagedAgents(nextSelected: boolean) {
    const next = new Set(selectedAgentIds.value)
    for (const agent of pagedAgents.value) {
      if (!isSelectableRecord(agent)) {
        continue
      }
      if (nextSelected) {
        next.add(agent.id)
      } else {
        next.delete(agent.id)
      }
    }
    selectedAgentIds.value = Array.from(next)
  }

  function toggleAllPagedTeams(nextSelected: boolean) {
    const next = new Set(selectedTeamIds.value)
    for (const team of pagedTeams.value) {
      if (!isSelectableRecord(team)) {
        continue
      }
      if (nextSelected) {
        next.add(team.id)
      } else {
        next.delete(team.id)
      }
    }
    selectedTeamIds.value = Array.from(next)
  }

  function exportSummaryLabel(agentCount: number, teamCount: number) {
    const parts: string[] = []
    if (agentCount > 0) {
      parts.push(`${agentCount} 个数字员工`)
    }
    if (teamCount > 0) {
      parts.push(`${teamCount} 个数字团队`)
    }
    return parts.join('、') || '资源包'
  }

  async function notifyTransfer(level: 'success' | 'error', title: string, body: string) {
    await notificationStore.notify({
      scopeKind: 'workspace',
      scopeOwnerId: workspaceStore.currentWorkspaceId || undefined,
      level,
      title,
      body,
      source: 'agent-center',
      toastDurationMs: 4000,
    })
  }

  function currentScopeLabel() {
    return isProjectScope.value ? '项目' : '工作区'
  }

  async function notifySaved(kind: 'agent' | 'team', name: string, scopeLabel = currentScopeLabel()) {
    await notifyTransfer(
      'success',
      '保存完成',
      `已在${scopeLabel}保存${kind === 'agent' ? '数字员工' : '数字团队'}「${name}」。`,
    )
  }

  async function notifyDeleted(kind: 'agent' | 'team', name: string) {
    await notifyTransfer(
      'success',
      '删除完成',
      `已从${currentScopeLabel()}删除${kind === 'agent' ? '数字员工' : '数字团队'}「${name}」。`,
    )
  }

  async function exportBundle(
    input: ExportWorkspaceAgentBundleInput,
    format: AgentBundleTransferFormat,
    target: 'agent' | 'team',
  ) {
    const loadingRef = target === 'agent' ? agentExportLoading : teamExportLoading
    loadingRef.value = true

    try {
      const payload = await agentStore.exportBundle(
        input,
        isProjectScope.value ? projectId.value : undefined,
      )
      await tauriClient.saveAgentBundleExport(payload, format)
      await notifyTransfer(
        'success',
        '导出完成',
        `已导出 ${exportSummaryLabel(payload.agentCount, payload.teamCount)}，格式为 ${format.toUpperCase()}.`,
      )
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to export agent bundle'
      await notifyTransfer('error', '导出失败', message)
    } finally {
      loadingRef.value = false
    }
  }

  async function exportAgentRecord(record: AgentRecord, format: AgentBundleTransferFormat) {
    if (!isExportableRecord(record)) {
      return
    }
    await exportBundle(
      {
        mode: 'single',
        agentIds: [record.id],
        teamIds: [],
      },
      format,
      'agent',
    )
  }

  async function exportSelectedAgents(format: AgentBundleTransferFormat) {
    const agentIds = currentAgents.value
      .filter(agent => selectedAgentIds.value.includes(agent.id) && isExportableRecord(agent))
      .map(agent => agent.id)
    const teamIds = currentTeams.value
      .filter(team => selectedTeamIds.value.includes(team.id) && isExportableRecord(team))
      .map(team => team.id)
    if (!agentIds.length && !teamIds.length) {
      return
    }
    await exportBundle(
      {
        mode: 'batch',
        agentIds,
        teamIds,
      },
      format,
      'agent',
    )
  }

  async function exportTeamRecord(record: TeamRecord, format: AgentBundleTransferFormat) {
    if (!isExportableRecord(record)) {
      return
    }
    await exportBundle(
      {
        mode: 'single',
        agentIds: [],
        teamIds: [record.id],
      },
      format,
      'team',
    )
  }

  async function exportSelectedTeams(format: AgentBundleTransferFormat) {
    const agentIds = currentAgents.value
      .filter(agent => selectedAgentIds.value.includes(agent.id) && isExportableRecord(agent))
      .map(agent => agent.id)
    const teamIds = currentTeams.value
      .filter(team => selectedTeamIds.value.includes(team.id) && isExportableRecord(team))
      .map(team => team.id)
    if (!agentIds.length && !teamIds.length) {
      return
    }
    await exportBundle(
      {
        mode: 'batch',
        agentIds,
        teamIds,
      },
      format,
      'team',
    )
  }

  async function copyAgentTemplate(record: AgentRecord) {
    const result = isProjectScope.value && projectId.value
      ? await agentStore.copyToProject(projectId.value, record.id)
      : await agentStore.copyToWorkspace(record.id)
    await reloadCenterData()
    await notifyTransfer(
      'success',
      '复制完成',
      `已复制 ${exportSummaryLabel(result.agentCount, result.teamCount)}。`,
    )
  }

  function openEditAgent(record: AgentRecord) {
    resetAgentForm(record)
    agentDialogOpen.value = true
  }

  function openCreateTeam() {
    resetTeamForm()
    teamDialogOpen.value = true
  }

  async function copyTeamTemplate(record: TeamRecord) {
    const result = isProjectScope.value && projectId.value
      ? await teamStore.copyToProject(projectId.value, record.id)
      : await teamStore.copyToWorkspace(record.id)
    await reloadCenterData()
    await notifyTransfer(
      'success',
      '复制完成',
      `已复制 ${exportSummaryLabel(result.agentCount, result.teamCount)}。`,
    )
  }

  function openEditTeam(record: TeamRecord) {
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

  function currentEditingAgent() {
    return currentEditingAgentRecord.value
  }

  function currentEditingTeam() {
    return currentTeams.value.find(team => team.id === editingTeamId.value)
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

  function clearAgentAvatar() {
    removeAgentAvatar.value = true
    agentPendingAvatar.value = null
  }

  function clearTeamAvatar() {
    removeTeamAvatar.value = true
    teamPendingAvatar.value = null
  }

  function toTags(tagsText: string) {
    return tagsText.split(',').map(tag => tag.trim()).filter(Boolean)
  }

  async function saveAgent() {
    if (!canSaveAgentDialog.value) {
      return
    }
    if (!workspaceStore.currentWorkspaceId || !agentForm.name.trim()) {
      return
    }
    const currentRecord = currentEditingAgent()
    const input: UpsertAgentInput = {
      workspaceId: currentRecord?.workspaceId ?? workspaceStore.currentWorkspaceId,
      projectId: currentRecord?.projectId ?? (isProjectScope.value ? projectId.value : undefined),
      scope: currentRecord?.scope ?? (isProjectScope.value ? 'project' : 'workspace'),
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
    const agentName = input.name
    try {
      if (editingAgentId.value) {
        await agentStore.update(editingAgentId.value, input)
      } else {
        await agentStore.create(input)
      }
      agentDialogOpen.value = false
      await notifySaved('agent', agentName, input.projectId ? '项目' : '工作区')
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to save agent'
      await notifyTransfer('error', '保存失败', message)
    }
  }

  async function saveTeam() {
    if (!canSaveTeamDialog.value) {
      return
    }
    if (!workspaceStore.currentWorkspaceId || !teamForm.name.trim()) {
      return
    }
    if (!teamForm.leaderRef.trim()) {
      await notifyTransfer('error', '保存失败', '请选择数字团队负责人。')
      return
    }
    const currentRecord = currentEditingTeam()
    const input: UpsertTeamInput = {
      workspaceId: currentRecord?.workspaceId ?? workspaceStore.currentWorkspaceId,
      projectId: currentRecord?.projectId ?? (isProjectScope.value ? projectId.value : undefined),
      scope: currentRecord?.scope ?? (isProjectScope.value ? 'project' : 'workspace'),
      name: teamForm.name.trim(),
      avatar: teamPendingAvatar.value ?? undefined,
      removeAvatar: removeTeamAvatar.value || undefined,
      personality: teamForm.personality.trim(),
      tags: toTags(teamForm.tagsText),
      prompt: teamForm.prompt.trim(),
      builtinToolKeys: [...teamForm.builtinToolKeys],
      skillIds: [...teamForm.skillIds],
      mcpServerNames: [...teamForm.mcpServerNames],
      leaderRef: teamForm.leaderRef.trim(),
      memberRefs: [...teamForm.memberRefs],
      description: teamForm.description.trim(),
      status: teamForm.status as TeamRecord['status'],
    }
    const teamName = input.name
    try {
      if (editingTeamId.value) {
        await teamStore.update(editingTeamId.value, input)
      } else {
        await teamStore.create(input)
      }
      teamDialogOpen.value = false
      await notifySaved('team', teamName, input.projectId ? '项目' : '工作区')
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to save team'
      await notifyTransfer('error', '保存失败', message)
    }
  }

  async function promoteAgentToWorkspace() {
    const record = currentEditingAgent()
    if (!record || !isProjectOwnedRecord(record)) {
      return
    }

    promoteAgentLoading.value = true
    try {
      const result = await agentStore.copyToWorkspace(record.id)
      await reloadCenterData()
      agentDialogOpen.value = false
      await notifyTransfer(
        'success',
        '提升完成',
        `已提升 ${exportSummaryLabel(result.agentCount, result.teamCount)} 到工作区。`,
      )
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to promote agent to workspace'
      await notifyTransfer('error', '提升失败', message)
    } finally {
      promoteAgentLoading.value = false
    }
  }

  async function promoteTeamToWorkspace() {
    const record = currentEditingTeam()
    if (!record || !isProjectOwnedRecord(record)) {
      return
    }

    promoteTeamLoading.value = true
    try {
      const result = await teamStore.copyToWorkspace(record.id)
      await reloadCenterData()
      teamDialogOpen.value = false
      await notifyTransfer(
        'success',
        '提升完成',
        `已提升 ${exportSummaryLabel(result.agentCount, result.teamCount)} 到工作区。`,
      )
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to promote team to workspace'
      await notifyTransfer('error', '提升失败', message)
    } finally {
      promoteTeamLoading.value = false
    }
  }

  async function copyCurrentEditingAgent() {
    const record = currentEditingAgentRecord.value
    if (!record || !canCopyCurrentEditingAgent.value) {
      return
    }
    await copyAgentTemplate(record)
    agentDialogOpen.value = false
  }

  async function copyCurrentEditingTeam() {
    const record = currentEditingTeamRecord.value
    if (!record || !canCopyCurrentEditingTeam.value) {
      return
    }
    await copyTeamTemplate(record)
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
    if (!itemToDelete.value) {
      return
    }

    const { id, type, name } = itemToDelete.value
    if (type === 'agent') {
      selectedAgentIds.value = selectedAgentIds.value.filter(selectedId => selectedId !== id)
      const record = currentAgents.value.find(a => a.id === id)
      if (record && isRemovableRecord(record)) {
        await agentStore.remove(id)
        await notifyDeleted('agent', name)
      }
    } else {
      selectedTeamIds.value = selectedTeamIds.value.filter(selectedId => selectedId !== id)
      const record = currentTeams.value.find(team => team.id === id)
      if (record && isRemovableRecord(record)) {
        await teamStore.remove(id)
        await notifyDeleted('team', name)
      }
    }

    deleteConfirmOpen.value = false
    itemToDelete.value = null
  }

  return {
    t,
    route,
    router,
    workspaceStore,
    agentStore,
    teamStore,
    catalogStore,
    activeTab,
    agentViewMode,
    teamViewMode,
    agentQuery,
    teamQuery,
    resourceQuery,
    agentDialogOpen,
    teamDialogOpen,
    editingAgentId,
    editingTeamId,
    deleteConfirmOpen,
    itemToDelete,
    agentImportDialogOpen,
    agentImportPreview,
    agentImportResult,
    agentImportError,
    agentImportLoading,
    agentExportLoading,
    teamExportLoading,
    promoteAgentLoading,
    promoteTeamLoading,
    agentForm,
    teamForm,
    isProjectScope,
    projectId,
    currentProject,
    currentAgents,
    currentTeams,
    resolvedAgents,
    pageTitle,
    pageDescription,
    builtinOptions,
    skillOptions,
    mcpOptions,
    statusOptions,
    teamAgentOptions,
    dialogTeamLeader,
    dialogTeamMembers,
    leaderOptions,
    agentDialogContentReadonly,
    teamDialogContentReadonly,
    agentDialogStatusReadonly,
    teamDialogStatusReadonly,
    canSaveAgentDialog,
    canSaveTeamDialog,
    canCopyCurrentEditingAgent,
    canCopyCurrentEditingTeam,
    currentEditingAgentCopyLabel,
    currentEditingTeamCopyLabel,
    tabs,
    pagedAgents,
    pagedTeams,
    pagedResources,
    agentTotal,
    teamTotal,
    resourceTotal,
    agentPage,
    teamPage,
    resourcePage,
    agentPageCount,
    teamPageCount,
    resourcePageCount,
    agentPagination,
    teamPagination,
    resourcePagination,
    centerStats,
    selectedAgentIds,
    selectedTeamIds,
    selectedAgents,
    selectedTeams,
    allPagedAgentsSelected,
    allPagedTeamsSelected,
    initials,
    agentBadgeLabel,
    teamOriginLabel,
    setTab,
    openCreateAgent,
    openAgentImportDialog,
    confirmAgentImport,
    handleAgentImportDialogOpen,
    toggleAllPagedAgents,
    toggleAllPagedTeams,
    exportAgentRecord,
    exportSelectedAgents,
    exportTeamRecord,
    exportSelectedTeams,
    openEditAgent,
    openCreateTeam,
    openEditTeam,
    pickAgentAvatar,
    pickTeamAvatar,
    currentEditingAgent,
    currentEditingTeam,
    agentAvatarPreview,
    teamAvatarPreview,
    clearAgentAvatar,
    clearTeamAvatar,
    saveAgent,
    saveTeam,
    copyCurrentEditingAgent,
    copyCurrentEditingTeam,
    promoteAgentToWorkspace,
    promoteTeamToWorkspace,
    removeAgent,
    removeTeam,
    confirmDelete,
  }
}
