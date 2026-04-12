import { computed, reactive, ref, watch } from 'vue'
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
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceStore } from '@/stores/workspace'
import * as tauriClient from '@/tauri/client'

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
  leaderAgentId: string
  memberAgentIds: string[]
  status: string
}

export function useAgentCenter(scope: CenterScope) {
  const { t } = useI18n()
  const route = useRoute()
  const router = useRouter()
  const shell = useShellStore()
  const workspaceStore = useWorkspaceStore()
  const agentStore = useAgentStore()
  const teamStore = useTeamStore()
  const catalogStore = useCatalogStore()
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
    leaderAgentId: '',
    memberAgentIds: [],
    status: 'active',
  })

  const isProjectScope = computed(() => scope === 'project')
  const isBuiltinTemplateRecord = (record: Pick<AgentRecord | TeamRecord, 'integrationSource'>) =>
    record.integrationSource?.kind === 'builtin-template'
  const isWorkspaceLinkedRecord = (record: Pick<AgentRecord | TeamRecord, 'integrationSource'>) =>
    record.integrationSource?.kind === 'workspace-link'
  const projectId = computed(() =>
    typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId || '',
  )
  const currentProject = computed(() =>
    workspaceStore.projects.find(project => project.id === projectId.value) ?? null,
  )
  const builtinTemplateAgents = computed(() =>
    agentStore.workspaceAgents.filter(agent => isBuiltinTemplateRecord(agent)),
  )
  const builtinTemplateTeams = computed(() =>
    teamStore.workspaceTeams.filter(team => isBuiltinTemplateRecord(team)),
  )
  const currentAgents = computed(() => {
    if (!isProjectScope.value) {
      return agentStore.workspaceAgents
    }
    const merged = [...agentStore.projectAgents, ...builtinTemplateAgents.value]
    return merged.filter((record, index) => merged.findIndex(item => item.id === record.id) === index)
  })
  const currentTeams = computed(() => {
    if (!isProjectScope.value) {
      return teamStore.workspaceTeams
    }
    const merged = [...teamStore.projectTeams, ...builtinTemplateTeams.value]
    return merged.filter((record, index) => merged.findIndex(item => item.id === record.id) === index)
  })
  const effectiveProjectAgents = computed(() =>
    isProjectScope.value
      ? currentAgents.value.filter(agent => !isBuiltinTemplateRecord(agent))
      : currentAgents.value,
  )
  const effectiveProjectTeams = computed(() =>
    isProjectScope.value
      ? currentTeams.value.filter(team => !isBuiltinTemplateRecord(team))
      : currentTeams.value,
  )
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
        .filter(agent => !isBuiltinTemplateRecord(agent))
        .map(agent => agent.id))
      selectedAgentIds.value = selectedAgentIds.value.filter(id => validIds.has(id))
    },
  )

  watch(
    () => currentTeams.value.map(team => team.id).join('|'),
    () => {
      const validIds = new Set(currentTeams.value
        .filter(team => !isBuiltinTemplateRecord(team))
        .map(team => team.id))
      selectedTeamIds.value = selectedTeamIds.value.filter(id => validIds.has(id))
    },
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
    pageSize: 6,
    resetOn: [agentQuery, () => scope, projectId],
  })
  const teamPagination = usePagination(filteredTeams, {
    pageSize: 6,
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
    pagedAgents.value.some(agent => !isBuiltinTemplateRecord(agent))
      && pagedAgents.value
        .filter(agent => !isBuiltinTemplateRecord(agent))
        .every(agent => selectedAgentIds.value.includes(agent.id)),
  )
  const allPagedTeamsSelected = computed(() =>
    pagedTeams.value.some(team => !isBuiltinTemplateRecord(team))
      && pagedTeams.value
        .filter(team => !isBuiltinTemplateRecord(team))
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
    if (agent.integrationSource?.kind === 'workspace-link') {
      return '工作区接入'
    }
    return agent.status
  }

  function teamOriginLabel(team: TeamRecord) {
    if (team.integrationSource?.kind === 'builtin-template') {
      return '内置模板'
    }
    if (team.integrationSource?.kind === 'workspace-link') {
      return '工作区接入'
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

  async function reloadCenterData(connectionId = shell.activeWorkspaceConnectionId, nextProjectId = projectId.value) {
    if (!connectionId) {
      return
    }

    const tasks: Promise<unknown>[] = [
      agentStore.load(connectionId),
      teamStore.load(connectionId),
      catalogStore.load(connectionId),
    ]
    if (isProjectScope.value && nextProjectId) {
      tasks.push(
        agentStore.loadProjectLinks(nextProjectId, connectionId),
        teamStore.loadProjectLinks(nextProjectId, connectionId),
      )
    }
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
    agentImportResult.value = null
    agentImportSource.value = source
    const files = source === 'zip'
      ? await tauriClient.pickAgentBundleArchive()
      : await tauriClient.pickAgentBundleFolder()
    if (!files?.length) {
      return
    }
    await previewAgentImportFiles(files)
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
      if (isBuiltinTemplateRecord(agent)) {
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
      if (isBuiltinTemplateRecord(team)) {
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
    })
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
    if (isBuiltinTemplateRecord(record)) {
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
      .filter(agent => selectedAgentIds.value.includes(agent.id) && !isBuiltinTemplateRecord(agent))
      .map(agent => agent.id)
    const teamIds = currentTeams.value
      .filter(team => selectedTeamIds.value.includes(team.id) && !isBuiltinTemplateRecord(team))
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
    if (isBuiltinTemplateRecord(record)) {
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
      .filter(agent => selectedAgentIds.value.includes(agent.id) && !isBuiltinTemplateRecord(agent))
      .map(agent => agent.id)
    const teamIds = currentTeams.value
      .filter(team => selectedTeamIds.value.includes(team.id) && !isBuiltinTemplateRecord(team))
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
    if (record.integrationSource?.kind === 'builtin-template') {
      void copyAgentTemplate(record)
      return
    }
    if (record.integrationSource?.kind === 'workspace-link' && isProjectScope.value) {
      void router.push({
        name: 'workspace-console-agents',
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
    if (record.integrationSource?.kind === 'builtin-template') {
      void copyTeamTemplate(record)
      return
    }
    if (record.integrationSource?.kind === 'workspace-link' && isProjectScope.value) {
      void router.push({
        name: 'workspace-console-agents',
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

  function currentEditingAgent() {
    return currentAgents.value.find(agent => agent.id === editingAgentId.value)
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
    if (!itemToDelete.value) {
      return
    }

    const { id, type } = itemToDelete.value
    if (type === 'agent') {
      selectedAgentIds.value = selectedAgentIds.value.filter(selectedId => selectedId !== id)
      const record = currentAgents.value.find(a => a.id === id)
      if (record?.integrationSource?.kind === 'workspace-link' && isProjectScope.value && projectId.value) {
        await agentStore.unlinkProject(projectId.value, id)
      } else {
        await agentStore.remove(id)
      }
    } else {
      selectedTeamIds.value = selectedTeamIds.value.filter(selectedId => selectedId !== id)
      const record = currentTeams.value.find(team => team.id === id)
      if (record?.integrationSource?.kind === 'workspace-link' && isProjectScope.value && projectId.value) {
        await teamStore.unlinkProject(projectId.value, id)
      } else {
        await teamStore.remove(id)
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
    agentForm,
    teamForm,
    isProjectScope,
    projectId,
    currentProject,
    currentAgents,
    currentTeams,
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
    removeAgent,
    removeTeam,
    confirmDelete,
  }
}
