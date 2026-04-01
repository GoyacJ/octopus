<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'
import { VueFlow } from '@vue-flow/core'
import { LayoutGrid, List } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import '@vue-flow/core/dist/style.css'

import { UiBadge, UiEmptyState, UiField, UiPagination } from '@octopus/ui'

import { enumLabel, resolveMockField, resolveMockList } from '@/i18n/copy'
import { usePagination } from '@/composables/usePagination'
import { useWorkbenchStore } from '@/stores/workbench'

type AgentCenterTab = 'agent' | 'team'
type AgentCenterView = 'icon' | 'list'
type DialogMode = 'create' | 'edit' | null
type DialogKind = 'agent' | 'team' | null

interface DraftState {
  name: string
  avatar: string
  role: string
  summary: string
  description: string
  skillTags: string
  mcpBindings: string
  permissions: string
  approvalPreferences: string
  model: string
  sharedSources: string
  autonomyLevel: string
  mode: string
  members: string
  defaultOutput: string
  projectNotes: string
}

interface MemberOption {
  value: string
  sourceId: string
  label: string
}

interface FlowNodeData {
  label: string
  role: string
  memberId?: string
}

interface FlowCanvasNode {
  id: string
  position: {
    x: number
    y: number
  }
  data: FlowNodeData
  draggable?: boolean
}

interface FlowCanvasEdge {
  id: string
  source: string
  target: string
  label?: string
}

const pageSize = 20

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workbench = useWorkbenchStore()

const isProjectScope = computed(() => route.name === 'project-agents')
const currentProjectId = computed(() => workbench.currentProjectId ?? '')
const searchQuery = ref('')
const createMenuOpen = ref(false)
const activeTab = ref<AgentCenterTab>(route.query.kind === 'team' ? 'team' : 'agent')
const viewMode = ref<AgentCenterView>('icon')
const dialogMode = ref<DialogMode>(null)
const dialogKind = ref<DialogKind>(null)
const editingId = ref('')
const exportPreview = ref('')
const pendingMemberId = ref('')
const memberPickerElement = ref<HTMLSelectElement>()
const flowNodes = shallowRef<FlowCanvasNode[]>([])
const flowEdges = shallowRef<FlowCanvasEdge[]>([])
const draft = ref<DraftState>(emptyDraft(activeTab.value))

const flowEnabled = typeof window !== 'undefined' && 'ResizeObserver' in window

const workspaceAgents = computed(() => isProjectScope.value ? workbench.projectReferencedAgents : workbench.workspaceLevelAgents)
const projectAgents = computed(() => isProjectScope.value ? workbench.projectOwnedAgents : [])
const workspaceTeams = computed(() => isProjectScope.value ? workbench.projectReferencedTeams : workbench.workspaceLevelTeams)
const projectTeams = computed(() => isProjectScope.value ? workbench.projectOwnedTeams : [])

const allAgents = computed(() => [...workspaceAgents.value, ...projectAgents.value])
const allTeams = computed(() => [...workspaceTeams.value, ...projectTeams.value])

const activeAgent = computed(() => editingId.value
  ? allAgents.value.find((agent) => agent.id === editingId.value)
  : undefined)
const activeTeam = computed(() => editingId.value
  ? allTeams.value.find((team) => team.id === editingId.value)
  : undefined)

const normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase())
const isDialogOpen = computed(() => dialogMode.value !== null && dialogKind.value !== null)
const isReferencedSelection = computed(() => {
  if (!isProjectScope.value) {
    return false
  }

  if (dialogKind.value === 'agent' && activeAgent.value) {
    return activeAgent.value.scope !== 'project'
  }

  if (dialogKind.value === 'team' && activeTeam.value) {
    return activeTeam.value.useScope !== 'project'
  }

  return false
})
const canCreateProjectCopy = computed(() => {
  if (!isProjectScope.value || dialogMode.value !== 'edit') {
    return false
  }

  return isReferencedSelection.value
})

const filteredAgents = computed(() => sortAgents(allAgents.value.filter((agent) =>
  matchesSearch([
    resolveMockField('agent', agent.id, 'name', agent.name),
    resolveMockField('agent', agent.id, 'role', agent.role),
    agent.summary,
    agent.skillTags.join(' '),
  ]),
)))
const filteredTeams = computed(() => sortTeams(allTeams.value.filter((team) =>
  matchesSearch([
    resolveMockField('team', team.id, 'name', team.name),
    resolveMockField('team', team.id, 'description', team.description),
    team.summary,
    team.skillTags.join(' '),
  ]),
)))

const {
  currentPage: agentPage,
  pageCount: agentPageCount,
  totalItems: agentTotalItems,
  pagedItems: pagedAgents,
  setPage: setAgentPage,
} = usePagination(filteredAgents, {
  pageSize,
  resetOn: [normalizedSearch],
})
const {
  currentPage: teamPage,
  pageCount: teamPageCount,
  totalItems: teamTotalItems,
  pagedItems: pagedTeams,
  setPage: setTeamPage,
} = usePagination(filteredTeams, {
  pageSize,
  resetOn: [normalizedSearch],
})
const paginationSizeLabel = computed(() => t('agentCenter.pagination.perPage', { count: pageSize }))
const agentPaginationLabel = computed(() =>
  t('agentCenter.pagination.summary', {
    page: agentPage.value,
    totalPages: agentPageCount.value,
    count: agentTotalItems.value,
  }),
)
const teamPaginationLabel = computed(() =>
  t('agentCenter.pagination.summary', {
    page: teamPage.value,
    totalPages: teamPageCount.value,
    count: teamTotalItems.value,
  }),
)
const agentPageInfoLabel = computed(() => `${agentPage.value} / ${agentPageCount.value}`)
const teamPageInfoLabel = computed(() => `${teamPage.value} / ${teamPageCount.value}`)

const memberIds = computed(() => splitList(draft.value.members))
const teamMembers = computed(() => memberIds.value.map((memberId) => {
  const agent = workbench.agents.find((item) => item.id === memberId)

  return {
    id: memberId,
    name: agent ? resolveMockField('agent', agent.id, 'name', agent.name) : memberId,
    role: agent ? resolveMockField('agent', agent.id, 'role', agent.role) : '',
  }
}))
const teamMemberOptions = computed<MemberOption[]>(() => {
  const selectedMembers = new Set(memberIds.value)
  const options: MemberOption[] = []
  const seenValues = new Set<string>()

  for (const agent of sortAgents(allAgents.value)) {
    let value = agent.id
    let sourceId = agent.id
    let label = `${resolveMockField('agent', agent.id, 'name', agent.name)} · ${resolveMockField('agent', agent.id, 'role', agent.role)}`

    if (isProjectScope.value && agent.scope !== 'project' && currentProjectId.value) {
      const projectCopyId = `${agent.id}-copy-${currentProjectId.value}`
      const existingCopy = workbench.agents.find((item) => item.id === projectCopyId)
      value = existingCopy?.id ?? projectCopyId
      sourceId = agent.id
      if (!existingCopy) {
        label = `${label} · ${t('agentCenter.actions.createProjectCopyInline')}`
      }
    }

    if (selectedMembers.has(value) || seenValues.has(value)) {
      continue
    }

    seenValues.add(value)
    options.push({
      value,
      sourceId,
      label,
    })
  }

  return options
})

watch(() => route.query.kind, (kind) => {
  activeTab.value = kind === 'team' ? 'team' : 'agent'
})

function emptyDraft(kind: AgentCenterTab): DraftState {
  return {
    name: '',
    avatar: kind === 'team' ? 'TM' : '',
    role: '',
    summary: '',
    description: '',
    skillTags: '',
    mcpBindings: '',
    permissions: '',
    approvalPreferences: '',
    model: 'gpt-5.4',
    sharedSources: '',
    autonomyLevel: '',
    mode: 'leadered',
    members: '',
    defaultOutput: '',
    projectNotes: '',
  }
}

function splitList(value: string): string[] {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

function matchesSearch(values: Array<string | undefined>) {
  if (!normalizedSearch.value) {
    return true
  }

  return values.some((value) => value?.toLowerCase().includes(normalizedSearch.value))
}

function sortAgents(agents: typeof allAgents.value) {
  return [...agents].sort((left, right) => {
    const mockWeight = Number(left.id.startsWith('agent-mock-')) - Number(right.id.startsWith('agent-mock-'))
    if (mockWeight !== 0) {
      return mockWeight
    }

    return resolveMockField('agent', left.id, 'name', left.name).localeCompare(
      resolveMockField('agent', right.id, 'name', right.name),
      undefined,
      { numeric: true },
    )
  })
}

function sortTeams(teams: typeof allTeams.value) {
  return [...teams].sort((left, right) =>
    resolveMockField('team', left.id, 'name', left.name).localeCompare(
      resolveMockField('team', right.id, 'name', right.name),
      undefined,
      { numeric: true },
    ),
  )
}

function nextNodePosition(index: number) {
  const column = index % 3
  const row = Math.floor(index / 3)

  return {
    x: 80 + column * 220,
    y: 72 + row * 150,
  }
}

function toFlowNodes(teamId: string, members: string[]) {
  if (activeTeam.value?.structureNodes.length) {
    return activeTeam.value.structureNodes.map<FlowCanvasNode>((node) => ({
      id: node.id,
      position: node.position,
      data: {
        label: node.label,
        role: node.role,
        memberId: node.memberId,
      },
      draggable: true,
    }))
  }

  return members.map<FlowCanvasNode>((memberId, index) => {
    const agent = workbench.agents.find((item) => item.id === memberId)

    return {
      id: `${teamId}-node-${index + 1}`,
      position: nextNodePosition(index),
      data: {
        label: agent ? resolveMockField('agent', agent.id, 'name', agent.name) : memberId,
        role: agent ? resolveMockField('agent', agent.id, 'role', agent.role) : t('agentCenter.structure.memberRole'),
        memberId,
      },
      draggable: true,
    }
  })
}

function toFlowEdges(teamId: string, members: string[]) {
  if (activeTeam.value?.structureEdges.length) {
    return activeTeam.value.structureEdges.map<FlowCanvasEdge>((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      label: edge.relation,
    }))
  }

  if (members.length <= 1) {
    return []
  }

  return members.slice(1).map<FlowCanvasEdge>((_, index) => ({
    id: `${teamId}-edge-${index + 1}`,
    source: `${teamId}-node-1`,
    target: `${teamId}-node-${index + 2}`,
    label: 'coordinates',
  }))
}

function syncDraftFromEntity() {
  if (dialogKind.value === 'agent' && activeAgent.value) {
    draft.value = {
      name: resolveMockField('agent', activeAgent.value.id, 'name', activeAgent.value.name),
      avatar: activeAgent.value.avatar,
      role: resolveMockField('agent', activeAgent.value.id, 'role', activeAgent.value.role),
      summary: activeAgent.value.summary,
      description: '',
      skillTags: activeAgent.value.skillTags.join(', '),
      mcpBindings: activeAgent.value.mcpBindings.join(', '),
      permissions: activeAgent.value.capabilityPolicy.tools.join(', '),
      approvalPreferences: activeAgent.value.approvalPreferences.join(', '),
      model: activeAgent.value.capabilityPolicy.model,
      sharedSources: resolveMockList('agent', activeAgent.value.id, 'knowledgeScope.sharedSources', activeAgent.value.knowledgeScope.sharedSources).join(', '),
      autonomyLevel: activeAgent.value.executionProfile.autonomyLevel,
      mode: 'leadered',
      members: '',
      defaultOutput: '',
      projectNotes: '',
    }
    flowNodes.value = []
    flowEdges.value = []
    pendingMemberId.value = ''
    return
  }

  if (dialogKind.value === 'team' && activeTeam.value) {
    draft.value = {
      name: resolveMockField('team', activeTeam.value.id, 'name', activeTeam.value.name),
      avatar: activeTeam.value.avatar,
      role: '',
      summary: activeTeam.value.summary,
      description: resolveMockField('team', activeTeam.value.id, 'description', activeTeam.value.description),
      skillTags: activeTeam.value.skillTags.join(', '),
      mcpBindings: activeTeam.value.mcpBindings.join(', '),
      permissions: activeTeam.value.members
        .flatMap((memberId) => workbench.agents.find((agent) => agent.id === memberId)?.capabilityPolicy.tools ?? [])
        .join(', '),
      approvalPreferences: activeTeam.value.approvalPreferences.join(', '),
      model: '',
      sharedSources: activeTeam.value.defaultKnowledgeScope.join(', '),
      autonomyLevel: '',
      mode: activeTeam.value.mode,
      members: activeTeam.value.members.join(', '),
      defaultOutput: resolveMockField('team', activeTeam.value.id, 'defaultOutput', activeTeam.value.defaultOutput),
      projectNotes: resolveMockField('team', activeTeam.value.id, 'projectNotes', activeTeam.value.projectNotes),
    }
    flowNodes.value = toFlowNodes(activeTeam.value.id, activeTeam.value.members)
    flowEdges.value = toFlowEdges(activeTeam.value.id, activeTeam.value.members)
    pendingMemberId.value = ''
    return
  }

  draft.value = emptyDraft(activeTab.value)
  flowNodes.value = []
  flowEdges.value = []
  pendingMemberId.value = ''
}

function closeDialog() {
  dialogMode.value = null
  dialogKind.value = null
  editingId.value = ''
  exportPreview.value = ''
  pendingMemberId.value = ''
  draft.value = emptyDraft(activeTab.value)
  flowNodes.value = []
  flowEdges.value = []
}

function openCreateDialog(kind: AgentCenterTab) {
  activeTab.value = kind
  dialogMode.value = 'create'
  dialogKind.value = kind
  editingId.value = ''
  exportPreview.value = ''
  pendingMemberId.value = ''
  draft.value = emptyDraft(kind)
  flowNodes.value = []
  flowEdges.value = []
  createMenuOpen.value = false
}

function openEditDialog(kind: AgentCenterTab, id: string) {
  activeTab.value = kind
  dialogMode.value = 'edit'
  dialogKind.value = kind
  editingId.value = id
  exportPreview.value = ''
  syncDraftFromEntity()
}

async function setActiveTab(tab: AgentCenterTab) {
  activeTab.value = tab
  exportPreview.value = ''
  await router.replace({
    query: {
      ...route.query,
      kind: tab === 'team' ? 'team' : undefined,
    },
  })
}

function setViewMode(mode: AgentCenterView) {
  viewMode.value = mode
}

function originLabel(isProjectOwned: boolean) {
  if (!isProjectScope.value) {
    return ''
  }

  return isProjectOwned
    ? t('agentCenter.origins.projectOwned')
    : t('agentCenter.origins.workspaceReference')
}

function avatarFallback(name: string, currentAvatar: string) {
  if (currentAvatar.startsWith('data:image/')) {
    return ''
  }

  if (currentAvatar.trim()) {
    return currentAvatar.slice(0, 2).toUpperCase()
  }

  return name.trim().slice(0, 2).toUpperCase() || 'AG'
}

function handleAvatarChange(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]

  if (!file) {
    return
  }

  const reader = new FileReader()
  reader.onload = () => {
    if (typeof reader.result === 'string') {
      draft.value.avatar = reader.result
    }
  }
  reader.readAsDataURL(file)
}

function addTeamMember() {
  const requestedMemberId = pendingMemberId.value || memberPickerElement.value?.value || ''
  if (!requestedMemberId) {
    return
  }

  const option = teamMemberOptions.value.find((item) => item.value === requestedMemberId)
  let memberId = option?.value ?? requestedMemberId
  let sourceId = option?.sourceId

  if (!sourceId && isProjectScope.value && currentProjectId.value && requestedMemberId.endsWith(`-copy-${currentProjectId.value}`)) {
    sourceId = requestedMemberId.replace(`-copy-${currentProjectId.value}`, '')
  }

  if (isProjectScope.value && sourceId && sourceId !== memberId && !workbench.agents.some((agent) => agent.id === memberId)) {
    memberId = workbench.createProjectAgentCopy(sourceId)?.id ?? memberId
  }

  const members = splitList(draft.value.members)
  if (!members.includes(memberId)) {
    members.push(memberId)
  }
  draft.value.members = members.join(', ')

  if (!flowNodes.value.some((node) => node.data?.memberId === memberId)) {
    const sourceAgent = workbench.agents.find((agent) => agent.id === memberId)
      ?? (sourceId ? workbench.agents.find((agent) => agent.id === sourceId) : undefined)
    const nextNodeId = activeTeam.value
      ? `${activeTeam.value.id}-node-${flowNodes.value.length + 1}`
      : `draft-node-${flowNodes.value.length + 1}`
    const nextNode: FlowCanvasNode = {
      id: nextNodeId,
      position: nextNodePosition(flowNodes.value.length),
      data: {
        label: sourceAgent ? resolveMockField('agent', sourceAgent.id, 'name', sourceAgent.name) : memberId,
        role: sourceAgent ? resolveMockField('agent', sourceAgent.id, 'role', sourceAgent.role) : t('agentCenter.structure.memberRole'),
        memberId,
      },
      draggable: true,
    }

    flowNodes.value = [...flowNodes.value, nextNode]

    const leadNodeId = flowNodes.value[0]?.id
    if (leadNodeId && leadNodeId !== nextNode.id) {
      flowEdges.value = [...flowEdges.value, {
        id: `${nextNode.id}-edge`,
        source: leadNodeId,
        target: nextNode.id,
        label: 'collaborates-with',
      }]
    }
  }

  pendingMemberId.value = ''
}

function removeTeamMember(memberId: string) {
  const members = splitList(draft.value.members).filter((item) => item !== memberId)
  draft.value.members = members.join(', ')

  const removedNodeIds = new Set(
    flowNodes.value
      .filter((node) => node.data?.memberId === memberId)
      .map((node) => node.id),
  )
  flowNodes.value = flowNodes.value.filter((node) => node.data?.memberId !== memberId)
  flowEdges.value = flowEdges.value.filter((edge) =>
    !removedNodeIds.has(edge.source) && !removedNodeIds.has(edge.target),
  )
}

function serializeFlowNodes() {
  return flowNodes.value.map((node, index) => ({
    id: node.id,
    label: node.data?.label ?? (draft.value.name || `Node ${index + 1}`),
    role: node.data?.role ?? t('agentCenter.structure.memberRole'),
    memberId: node.data?.memberId,
    level: index,
    position: {
      x: Math.round(node.position.x),
      y: Math.round(node.position.y),
    },
  }))
}

function serializeFlowEdges() {
  return flowEdges.value.map((edge) => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
    relation: typeof edge.label === 'string' && edge.label.trim() ? edge.label : 'collaborates-with',
  }))
}

function saveAgent(agentId: string) {
  const source = workbench.agents.find((agent) => agent.id === agentId)
  if (!source) {
    return
  }

  workbench.updateAgent(agentId, {
    name: draft.value.name,
    avatar: draft.value.avatar || source.avatar,
    role: draft.value.role,
    summary: draft.value.summary,
    skillTags: splitList(draft.value.skillTags),
    mcpBindings: splitList(draft.value.mcpBindings),
    capabilityPolicy: {
      ...source.capabilityPolicy,
      model: draft.value.model,
      tools: splitList(draft.value.permissions),
    },
    knowledgeScope: {
      ...source.knowledgeScope,
      sharedSources: splitList(draft.value.sharedSources),
    },
    executionProfile: {
      ...source.executionProfile,
      autonomyLevel: draft.value.autonomyLevel,
    },
    approvalPreferences: splitList(draft.value.approvalPreferences),
  })
}

function saveTeam(teamId: string) {
  const source = workbench.teams.find((team) => team.id === teamId)
  if (!source) {
    return
  }

  workbench.updateTeam(teamId, {
    name: draft.value.name,
    avatar: draft.value.avatar || source.avatar,
    description: draft.value.description,
    summary: draft.value.summary,
    skillTags: splitList(draft.value.skillTags),
    mcpBindings: splitList(draft.value.mcpBindings),
    defaultKnowledgeScope: splitList(draft.value.sharedSources),
    defaultOutput: draft.value.defaultOutput,
    mode: draft.value.mode as typeof source.mode,
    members: splitList(draft.value.members),
    projectNotes: draft.value.projectNotes,
    approvalPreferences: splitList(draft.value.approvalPreferences),
    structureMode: 'flow',
    structureNodes: serializeFlowNodes(),
    structureEdges: serializeFlowEdges(),
  })
}

function confirmDialog() {
  if (!draft.value.name.trim()) {
    return
  }

  const scope = isProjectScope.value ? 'project' : 'workspace'

  if (dialogKind.value === 'agent') {
    const agent = workbench.createAgent(scope)
    saveAgent(agent.id)
    closeDialog()
    return
  }

  if (dialogKind.value === 'team') {
    const team = workbench.createTeam(scope)
    editingId.value = team.id
    saveTeam(team.id)
    closeDialog()
    activeTab.value = 'team'
  }
}

function saveDialog() {
  if (!draft.value.name.trim()) {
    return
  }

  if (dialogKind.value === 'agent' && activeAgent.value) {
    saveAgent(activeAgent.value.id)
    closeDialog()
    return
  }

  if (dialogKind.value === 'team' && activeTeam.value) {
    saveTeam(activeTeam.value.id)
    closeDialog()
  }
}

function createProjectCopy() {
  if (dialogKind.value === 'agent' && activeAgent.value) {
    workbench.createProjectAgentCopy(activeAgent.value.id)
    closeDialog()
    return
  }

  if (dialogKind.value === 'team' && activeTeam.value) {
    workbench.createProjectTeamCopy(activeTeam.value.id)
    closeDialog()
  }
}

function exportSelection() {
  if (dialogKind.value === 'agent' && activeAgent.value) {
    exportPreview.value = workbench.exportAgentAsset('agent', activeAgent.value.id)
    return
  }

  if (dialogKind.value === 'team' && activeTeam.value) {
    exportPreview.value = workbench.exportAgentAsset('team', activeTeam.value.id)
  }
}

function removeReference() {
  if (dialogKind.value === 'agent' && activeAgent.value) {
    workbench.removeProjectAgentReference(activeAgent.value.id)
    closeDialog()
    return
  }

  if (dialogKind.value === 'team' && activeTeam.value) {
    workbench.removeProjectTeamReference(activeTeam.value.id)
    closeDialog()
  }
}

function deleteSelection() {
  const confirmMessage = dialogKind.value === 'agent'
    ? t('agentCenter.actions.deleteAgentConfirm')
    : t('agentCenter.actions.deleteTeamConfirm')

  if (!window.confirm(confirmMessage)) {
    return
  }

  if (dialogKind.value === 'agent' && activeAgent.value) {
    workbench.deleteAgent(activeAgent.value.id)
    closeDialog()
    return
  }

  if (dialogKind.value === 'team' && activeTeam.value) {
    workbench.deleteTeam(activeTeam.value.id)
    closeDialog()
  }
}
</script>

<template>
  <section class="section-stack" data-testid="agent-center-page">
    <div class="agent-page-header">
      <div class="header-top-row">
        <h1 class="page-title" data-testid="agent-center-title">{{ t('agentCenter.header.title') }}</h1>
        <div class="mini-tabs" role="tablist" :aria-label="t('agentCenter.header.title')">
          <button
            type="button"
            class="mini-tab"
            :class="{ active: activeTab === 'agent' }"
            data-testid="agent-center-tab-agent"
            @click="setActiveTab('agent')"
          >
            {{ t('agentCenter.tabs.agent') }}
          </button>
          <button
            type="button"
            class="mini-tab"
            :class="{ active: activeTab === 'team' }"
            data-testid="agent-center-tab-team"
            @click="setActiveTab('team')"
          >
            {{ t('agentCenter.tabs.team') }}
          </button>
        </div>
        <div class="header-spacer" aria-hidden="true" />
      </div>

      <div class="agent-center-toolbar" data-testid="agent-center-toolbar">
        <input
          v-model="searchQuery"
          :placeholder="t('agentCenter.actions.searchPlaceholder')"
          type="search"
          class="search-input"
          data-testid="agent-center-search"
        >

        <div class="toolbar-actions">
          <div class="view-switcher">
            <button
              type="button"
              class="view-button"
              :class="{ active: viewMode === 'icon' }"
              data-testid="agent-center-view-icon"
              :title="t('agentCenter.views.icon')"
              :aria-label="t('agentCenter.views.icon')"
              @click="setViewMode('icon')"
            >
              <LayoutGrid :size="15" />
            </button>
            <button
              type="button"
              class="view-button"
              :class="{ active: viewMode === 'list' }"
              data-testid="agent-center-view-list"
              :title="t('agentCenter.views.list')"
              :aria-label="t('agentCenter.views.list')"
              @click="setViewMode('list')"
            >
              <List :size="15" />
            </button>
          </div>

          <div class="create-menu">
            <button
              type="button"
              class="primary-button"
              data-testid="agent-center-create-trigger"
              @click="createMenuOpen = !createMenuOpen"
            >
              {{ t('agentCenter.actions.create') }}
            </button>
            <div v-if="createMenuOpen" class="create-menu-panel">
              <button
                type="button"
                class="create-menu-item"
                data-testid="agent-center-create-agent"
                @click="openCreateDialog('agent')"
              >
                {{ t('agentCenter.actions.createAgent') }}
              </button>
              <button
                type="button"
                class="create-menu-item"
                data-testid="agent-center-create-team"
                @click="openCreateDialog('team')"
              >
                {{ t('agentCenter.actions.createTeam') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="activeTab === 'agent'" class="agent-collection-shell">
      <div
        v-if="pagedAgents.length && viewMode === 'icon'"
        class="asset-grid"
        data-testid="agent-center-icon-view-agent"
      >
        <button
          v-for="agent in pagedAgents"
          :key="agent.id"
          type="button"
          class="asset-icon-item"
          :data-testid="`agent-center-item-agent-${agent.id}`"
          @click="openEditDialog('agent', agent.id)"
        >
          <div class="tile-avatar">
            <img v-if="agent.avatar.startsWith('data:image/')" :src="agent.avatar" alt="">
            <span v-else>{{ avatarFallback(agent.name, agent.avatar) }}</span>
          </div>
          <strong>{{ resolveMockField('agent', agent.id, 'name', agent.name) }}</strong>
          <small>{{ resolveMockField('agent', agent.id, 'role', agent.role) }}</small>
          <p>{{ agent.summary }}</p>
          <UiBadge
            v-if="isProjectScope"
            :label="originLabel(agent.scope === 'project')"
            subtle
          />
        </button>
      </div>

      <div
        v-else-if="pagedAgents.length"
        class="table-shell"
        data-testid="agent-center-list-view-agent"
      >
        <table class="asset-table">
          <thead>
            <tr>
              <th>{{ t('agentCenter.table.name') }}</th>
              <th>{{ t('agentCenter.table.role') }}</th>
              <th>{{ t('agentCenter.table.summary') }}</th>
              <th>{{ t('agentCenter.table.origin') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="agent in pagedAgents"
              :key="agent.id"
              class="table-row"
              :data-testid="`agent-center-item-agent-${agent.id}`"
              @click="openEditDialog('agent', agent.id)"
            >
              <td>
                <div class="table-identity">
                  <div class="tile-avatar table-avatar">
                    <img v-if="agent.avatar.startsWith('data:image/')" :src="agent.avatar" alt="">
                    <span v-else>{{ avatarFallback(agent.name, agent.avatar) }}</span>
                  </div>
                  <div class="list-main">
                    <span class="table-body-text table-name">{{ resolveMockField('agent', agent.id, 'name', agent.name) }}</span>
                  </div>
                </div>
              </td>
              <td class="table-body-text">{{ resolveMockField('agent', agent.id, 'role', agent.role) }}</td>
              <td class="table-body-text table-summary">{{ agent.summary }}</td>
              <td class="table-origin-cell">
                <span
                  class="table-body-text table-origin"
                  :data-testid="`agent-center-origin-agent-${agent.id}`"
                >
                  {{ isProjectScope ? originLabel(agent.scope === 'project') : '-' }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <UiEmptyState
        v-else
        :title="t('agentCenter.sections.agents.emptyTitle')"
        :description="t('agentCenter.sections.agents.emptyDescription')"
      />

      <UiPagination
        v-if="filteredAgents.length"
        :page="agentPage"
        :page-count="agentPageCount"
        :meta-label="paginationSizeLabel"
        :summary-label="agentPaginationLabel"
        :page-info-label="agentPageInfoLabel"
        :previous-label="t('agentCenter.pagination.previous')"
        :next-label="t('agentCenter.pagination.next')"
        root-test-id="agent-center-pagination-agent"
        previous-button-test-id="agent-center-page-prev-agent"
        next-button-test-id="agent-center-page-next-agent"
        page-info-test-id="agent-center-page-info-agent"
        summary-test-id="agent-center-page-summary-agent"
        @update:page="setAgentPage"
      />
    </div>

    <div v-else class="agent-collection-shell">
      <div
        v-if="pagedTeams.length && viewMode === 'icon'"
        class="asset-grid"
        data-testid="agent-center-icon-view-team"
      >
        <button
          v-for="team in pagedTeams"
          :key="team.id"
          type="button"
          class="asset-icon-item"
          :data-testid="`agent-center-item-team-${team.id}`"
          @click="openEditDialog('team', team.id)"
        >
          <div class="tile-avatar">
            <span>{{ avatarFallback(team.name, team.avatar) }}</span>
          </div>
          <strong>{{ resolveMockField('team', team.id, 'name', team.name) }}</strong>
          <small>{{ enumLabel('teamMode', team.mode) }}</small>
          <p>{{ team.summary }}</p>
          <div class="tile-meta">
            <span>{{ t('common.members', { count: team.members.length }) }}</span>
            <UiBadge
              v-if="isProjectScope"
              :label="originLabel(team.useScope === 'project')"
              subtle
            />
          </div>
        </button>
      </div>

      <div
        v-else-if="pagedTeams.length"
        class="table-shell"
        data-testid="agent-center-list-view-team"
      >
        <table class="asset-table">
          <thead>
            <tr>
              <th>{{ t('agentCenter.table.name') }}</th>
              <th>{{ t('agentCenter.table.mode') }}</th>
              <th>{{ t('agentCenter.table.members') }}</th>
              <th>{{ t('agentCenter.table.summary') }}</th>
              <th>{{ t('agentCenter.table.origin') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="team in pagedTeams"
              :key="team.id"
              class="table-row"
              :data-testid="`agent-center-item-team-${team.id}`"
              @click="openEditDialog('team', team.id)"
            >
              <td>
                <div class="table-identity">
                  <div class="tile-avatar table-avatar">
                    <span>{{ avatarFallback(team.name, team.avatar) }}</span>
                  </div>
                  <div class="list-main">
                    <span class="table-body-text table-name">{{ resolveMockField('team', team.id, 'name', team.name) }}</span>
                  </div>
                </div>
              </td>
              <td class="table-body-text">{{ enumLabel('teamMode', team.mode) }}</td>
              <td class="table-body-text">{{ team.members.length }}</td>
              <td class="table-body-text table-summary">{{ team.summary }}</td>
              <td class="table-origin-cell">
                <span
                  class="table-body-text table-origin"
                  :data-testid="`agent-center-origin-team-${team.id}`"
                >
                  {{ isProjectScope ? originLabel(team.useScope === 'project') : '-' }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <UiEmptyState
        v-else
        :title="t('agentCenter.sections.teams.emptyTitle')"
        :description="t('agentCenter.sections.teams.emptyDescription')"
      />

      <UiPagination
        v-if="filteredTeams.length"
        :page="teamPage"
        :page-count="teamPageCount"
        :meta-label="paginationSizeLabel"
        :summary-label="teamPaginationLabel"
        :page-info-label="teamPageInfoLabel"
        :previous-label="t('agentCenter.pagination.previous')"
        :next-label="t('agentCenter.pagination.next')"
        root-test-id="agent-center-pagination-team"
        previous-button-test-id="agent-center-page-prev-team"
        next-button-test-id="agent-center-page-next-team"
        page-info-test-id="agent-center-page-info-team"
        summary-test-id="agent-center-page-summary-team"
        @update:page="setTeamPage"
      />
    </div>

    <div v-if="isDialogOpen" class="agent-dialog-shell" :data-testid="`agent-center-dialog-${dialogKind}-${dialogMode}`">
      <button type="button" class="agent-dialog-backdrop" @click="closeDialog" />
      <section class="agent-dialog">
        <div class="agent-dialog-copy">
          <strong>
            {{
              dialogMode === 'create'
                ? (dialogKind === 'agent' ? t('agentCenter.dialog.createAgent') : t('agentCenter.dialog.createTeam'))
                : (dialogKind === 'agent' ? t('agentCenter.dialog.editAgent') : t('agentCenter.dialog.editTeam'))
            }}
          </strong>
          <p>{{ dialogMode === 'create' ? t('agentCenter.dialog.createDescription') : t('agentCenter.dialog.editDescription') }}</p>
        </div>

        <div v-if="dialogKind === 'agent'" class="detail-stack">
          <div class="avatar-editor">
            <div class="avatar-preview" data-testid="agent-center-avatar-preview">
              <img v-if="draft.avatar.startsWith('data:image/')" :src="draft.avatar" alt="">
              <span v-else>{{ avatarFallback(draft.name, draft.avatar) }}</span>
            </div>
            <label class="secondary-button upload-label">
              {{ t('agentCenter.actions.uploadAvatar') }}
              <input
                type="file"
                accept="image/*"
                class="visually-hidden"
                data-testid="agent-center-avatar-input"
                @change="handleAvatarChange"
              >
            </label>
          </div>

          <div class="field-grid">
            <UiField :label="t('agentCenter.form.name')">
              <input v-model="draft.name" type="text" data-testid="agent-center-dialog-name">
            </UiField>
            <UiField :label="t('agentCenter.form.role')">
              <input v-model="draft.role" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.summary')">
              <input v-model="draft.summary" type="text" data-testid="agent-center-dialog-summary">
            </UiField>
            <UiField :label="t('agentCenter.form.skillTags')">
              <input v-model="draft.skillTags" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.mcpBindings')">
              <input v-model="draft.mcpBindings" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.permissions')">
              <input v-model="draft.permissions" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.approvalPreferences')">
              <input v-model="draft.approvalPreferences" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.sharedSources')">
              <input v-model="draft.sharedSources" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.model')">
              <input v-model="draft.model" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.autonomy')">
              <input v-model="draft.autonomyLevel" type="text">
            </UiField>
          </div>
        </div>

        <div v-else class="detail-stack">
          <div class="field-grid">
            <UiField :label="t('agentCenter.form.name')">
              <input v-model="draft.name" type="text" data-testid="agent-center-dialog-name">
            </UiField>
            <UiField :label="t('agentCenter.form.mode')">
              <select v-model="draft.mode">
                <option value="leadered">{{ enumLabel('teamMode', 'leadered') }}</option>
                <option value="hybrid">{{ enumLabel('teamMode', 'hybrid') }}</option>
                <option value="mesh">{{ enumLabel('teamMode', 'mesh') }}</option>
              </select>
            </UiField>
            <UiField :label="t('agentCenter.form.summary')">
              <input v-model="draft.summary" type="text" data-testid="agent-center-dialog-summary">
            </UiField>
            <UiField :label="t('agentCenter.form.skillTags')">
              <input v-model="draft.skillTags" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.mcpBindings')">
              <input v-model="draft.mcpBindings" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.permissions')">
              <input v-model="draft.permissions" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.approvalPreferences')">
              <input v-model="draft.approvalPreferences" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.sharedSources')">
              <input v-model="draft.sharedSources" type="text">
            </UiField>
            <UiField :label="t('agentCenter.form.defaultOutput')">
              <input v-model="draft.defaultOutput" type="text">
            </UiField>
          </div>

          <UiField :label="t('agentCenter.form.description')">
            <textarea v-model="draft.description" rows="3" />
          </UiField>

          <UiField :label="t('agentCenter.form.projectNotes')">
            <textarea v-model="draft.projectNotes" rows="3" />
          </UiField>

          <div class="member-section">
            <div class="member-toolbar">
              <UiField :label="t('agentCenter.form.memberPicker')" class="member-picker-field">
                <select ref="memberPickerElement" v-model="pendingMemberId" data-testid="agent-center-member-picker">
                  <option value="">{{ t('agentCenter.actions.selectMember') }}</option>
                  <option v-for="option in teamMemberOptions" :key="option.value" :value="option.value">
                    {{ option.label }}
                  </option>
                </select>
              </UiField>
              <button
                type="button"
                class="secondary-button"
                data-testid="agent-center-member-add"
                @click="addTeamMember"
              >
                {{ t('agentCenter.actions.addMember') }}
              </button>
            </div>

            <div class="member-chip-list">
              <button
                v-for="member in teamMembers"
                :key="member.id"
                type="button"
                class="member-chip"
                @click="removeTeamMember(member.id)"
              >
                <strong>{{ member.name }}</strong>
                <small>{{ member.role }}</small>
              </button>
            </div>
          </div>

          <div class="structure-canvas" data-testid="agent-center-structure-canvas">
            <div class="structure-copy">
              <strong>{{ t('agentCenter.structure.title') }}</strong>
              <p>{{ t('agentCenter.structure.description') }}</p>
            </div>
            <div class="flow-shell" data-testid="agent-center-flow-canvas">
              <VueFlow
                v-if="flowEnabled"
                v-model:nodes="flowNodes"
                v-model:edges="flowEdges"
                :fit-view-on-init="true"
                class="flow-pane"
              >
                <template #node-default="nodeProps">
                  <div class="flow-node-card" :data-testid="`agent-center-flow-node-${nodeProps.id}`">
                    <strong>{{ nodeProps.data.label }}</strong>
                    <small>{{ nodeProps.data.role }}</small>
                  </div>
                </template>
              </VueFlow>
              <div v-else class="flow-fallback">
                <div
                  v-for="node in flowNodes"
                  :key="node.id"
                  class="flow-node-card fallback-node"
                  :style="{ left: `${node.position.x}px`, top: `${node.position.y}px` }"
                  :data-testid="`agent-center-flow-node-${node.id}`"
                >
                  <strong>{{ node.data?.label }}</strong>
                  <small>{{ node.data?.role }}</small>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="action-row">
          <button type="button" class="ghost-button" @click="closeDialog">
            {{ t('common.cancel') }}
          </button>
          <button
            v-if="dialogMode === 'create'"
            type="button"
            class="primary-button"
            data-testid="agent-center-dialog-confirm"
            @click="confirmDialog"
          >
            {{ t('common.confirm') }}
          </button>
          <button
            v-else
            type="button"
            class="primary-button"
            data-testid="agent-center-dialog-save"
            @click="saveDialog"
          >
            {{ t('common.mockSave') }}
          </button>
          <button
            v-if="dialogMode === 'edit' && canCreateProjectCopy"
            type="button"
            class="secondary-button"
            data-testid="agent-center-create-project-copy"
            @click="createProjectCopy"
          >
            {{ t('common.createProjectCopy') }}
          </button>
          <button
            v-if="dialogMode === 'edit'"
            type="button"
            class="ghost-button"
            data-testid="agent-center-export"
            @click="exportSelection"
          >
            {{ t('agentCenter.actions.export') }}
          </button>
        </div>

        <div v-if="dialogMode === 'edit'" class="danger-zone">
          <strong>{{ t('agentCenter.actions.dangerTitle') }}</strong>
          <p>{{ t('agentCenter.actions.dangerDescription') }}</p>
          <div class="action-row">
            <button
              v-if="isReferencedSelection"
              type="button"
              class="ghost-button"
              data-testid="agent-center-remove-reference"
              @click="removeReference"
            >
              {{ t('agentCenter.actions.removeReference') }}
            </button>
            <button
              v-else
              type="button"
              class="danger-button"
              data-testid="agent-center-delete-asset"
              @click="deleteSelection"
            >
              {{ t('agentCenter.actions.deleteAsset') }}
            </button>
          </div>
        </div>

        <pre v-if="exportPreview" class="export-preview" data-testid="agent-center-export-preview">{{ exportPreview }}</pre>
      </section>
    </div>
  </section>
</template>

<style scoped>
.agent-page-header,
.agent-collection-shell,
.agent-center-toolbar,
.toolbar-actions,
.mini-tabs,
.view-switcher,
.create-menu,
.create-menu-panel,
.asset-icon-item,
.tile-meta,
.table-identity,
.list-main,
.agent-dialog-copy,
.detail-stack,
.action-row,
.danger-zone,
.member-section,
.member-toolbar,
.member-chip-list,
.structure-canvas,
.structure-copy,
.avatar-editor,
.pagination-row,
.pagination-meta,
.pagination-actions {
  display: flex;
}

.agent-page-header,
.agent-collection-shell,
.detail-stack,
.danger-zone,
.member-section,
.structure-canvas,
.structure-copy,
.agent-dialog-copy {
  flex-direction: column;
}

.agent-page-header,
.agent-collection-shell,
.detail-stack,
.danger-zone,
.structure-canvas,
.structure-copy {
  gap: 0.9rem;
}

.agent-page-header {
  padding-bottom: 0.2rem;
}

.header-top-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 1rem;
}

.page-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.header-spacer {
  min-height: 1px;
}

.mini-tabs {
  justify-self: center;
  gap: 0;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  overflow: hidden;
}

.mini-tab {
  padding: 0.28rem 0.78rem;
  border-radius: 0;
  background: transparent;
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.mini-tab + .mini-tab {
  border-left: 1px solid color-mix(in srgb, var(--border-subtle) 72%, transparent);
}

.mini-tab.active {
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
  color: var(--text-primary);
}

.agent-center-toolbar,
.toolbar-actions,
.view-switcher,
.member-toolbar,
.pagination-row,
.pagination-meta,
.pagination-actions {
  align-items: center;
}

.agent-center-toolbar,
.pagination-row {
  justify-content: space-between;
}

.agent-center-toolbar {
  gap: 1rem;
}

.toolbar-actions,
.view-switcher,
.member-chip-list,
.action-row,
.pagination-meta,
.pagination-actions {
  gap: 0.65rem;
}

.search-input {
  width: min(28rem, 100%);
}

.view-button {
  width: 2rem;
  height: 2rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: transparent;
  color: var(--text-secondary);
}

.view-button.active {
  border-color: color-mix(in srgb, var(--brand-primary) 42%, transparent);
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
  color: var(--text-primary);
}

.create-menu {
  position: relative;
}

.create-menu-panel {
  position: absolute;
  top: calc(100% + 0.45rem);
  right: 0;
  z-index: 6;
  min-width: 12rem;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.4rem;
  border-radius: 0.9rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: var(--bg-surface);
  box-shadow: 0 18px 42px color-mix(in srgb, var(--bg-main) 18%, transparent);
}

.create-menu-item {
  width: 100%;
  padding: 0.8rem 0.9rem;
  border-radius: 0.75rem;
  text-align: left;
}

.create-menu-item:hover {
  background: color-mix(in srgb, var(--bg-subtle) 85%, transparent);
}

.asset-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
  gap: 1rem;
  align-items: start;
}

.asset-icon-item {
  flex-direction: column;
  align-items: center;
  gap: 0.42rem;
  min-width: 0;
  padding: 0.45rem 0.5rem 0.65rem;
  border-radius: 0.85rem;
  text-align: center;
  background: transparent;
}

.asset-icon-item:hover {
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.tile-avatar,
.avatar-preview {
  width: 2.6rem;
  height: 2.6rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.9rem;
  background: color-mix(in srgb, var(--brand-primary) 14%, var(--bg-subtle));
  color: var(--text-primary);
  font-weight: 700;
  overflow: hidden;
}

.tile-avatar img,
.avatar-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.asset-icon-item .tile-avatar {
  width: 3rem;
  height: 3rem;
}

.asset-icon-item strong,
.asset-icon-item small,
.asset-icon-item p {
  width: 100%;
  display: block;
}

.asset-icon-item p,
.agent-dialog-copy p,
.structure-copy p,
.danger-zone p {
  margin: 0;
  color: var(--text-secondary);
  line-height: 1.45;
}

.asset-icon-item p {
  font-size: 0.82rem;
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.asset-icon-item strong,
.asset-icon-item small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tile-meta {
  align-items: center;
  justify-content: center;
  gap: 0.55rem;
  flex-wrap: wrap;
}

.table-shell {
  overflow-x: auto;
}

.asset-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
  table-layout: fixed;
}

.asset-table th,
.asset-table td {
  padding: 0.78rem 0.75rem;
  text-align: left;
  vertical-align: middle;
}

.asset-table td {
  font-size: 0.95rem;
  line-height: 1.45;
}

.asset-table thead tr,
.table-row {
  background-image: linear-gradient(
    to right,
    color-mix(in srgb, var(--border-subtle) 72%, transparent),
    color-mix(in srgb, var(--border-subtle) 72%, transparent)
  );
  background-position: left bottom;
  background-repeat: no-repeat;
  background-size: 100% 1px;
}

.asset-table th {
  padding-top: 0.45rem;
  padding-bottom: 0.55rem;
  font-size: 0.76rem;
  font-weight: 600;
  color: var(--text-muted);
  white-space: nowrap;
}

.table-row {
  cursor: pointer;
}

.table-row:hover {
  background-color: color-mix(in srgb, var(--bg-subtle) 74%, transparent);
}

.table-identity {
  align-items: center;
  gap: 0.7rem;
  min-width: 0;
}

.table-avatar {
  flex-shrink: 0;
  width: 2.35rem;
  height: 2.35rem;
  border-radius: 0.78rem;
}

.list-main {
  min-width: 0;
  flex-direction: column;
  gap: 0.16rem;
}

.list-main strong,
.list-main small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.list-main small,
.pagination-summary,
.pagination-size {
  color: var(--text-muted);
}

.table-body-text {
  color: var(--text-secondary);
  font-size: inherit;
  line-height: inherit;
  font-weight: 400;
}

.table-name {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.table-summary,
.table-origin {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.table-summary {
  color: var(--text-secondary);
}

.table-origin-cell {
  white-space: nowrap;
}

.table-origin {
  color: var(--text-secondary);
}

.pagination-row {
  margin-top: 0.1rem;
  padding-top: 0.25rem;
}

.pagination-meta {
  flex-wrap: wrap;
  font-size: 0.82rem;
}

.agent-dialog-shell {
  position: fixed;
  inset: 0;
  z-index: 30;
}

.agent-dialog-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.42);
}

.agent-dialog {
  position: relative;
  z-index: 1;
  width: min(64rem, calc(100vw - 2rem));
  max-height: calc(100vh - 3rem);
  margin: 4vh auto 0;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  overflow: auto;
  padding: 1.1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 86%, transparent);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
  box-shadow: var(--shadow-lg);
}

.field-grid {
  display: grid;
  gap: 0.85rem;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.avatar-editor {
  align-items: center;
  gap: 0.9rem;
}

.upload-label {
  cursor: pointer;
}

.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  border: 0;
}

.member-toolbar {
  gap: 0.85rem;
}

.member-picker-field {
  flex: 1;
}

.member-chip {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  align-items: flex-start;
  padding: 0.65rem 0.8rem;
  border-radius: 0.85rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 86%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 80%, transparent);
}

.member-chip small {
  color: var(--text-muted);
}

.structure-canvas {
  padding: 1rem;
  border-radius: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 68%, transparent);
}

.flow-shell {
  min-height: 28rem;
  border-radius: 0.9rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 82%, transparent);
  background:
    linear-gradient(90deg, color-mix(in srgb, var(--border-subtle) 18%, transparent) 1px, transparent 1px),
    linear-gradient(color-mix(in srgb, var(--border-subtle) 18%, transparent) 1px, transparent 1px),
    var(--bg-surface);
  background-size: 24px 24px;
  overflow: hidden;
}

.flow-pane,
.flow-fallback {
  width: 100%;
  height: 100%;
  min-height: 28rem;
}

.flow-fallback {
  position: relative;
}

.flow-node-card {
  min-width: 10rem;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.7rem 0.85rem;
  border-radius: 0.85rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 82%, transparent);
  background: color-mix(in srgb, var(--bg-surface) 98%, transparent);
  box-shadow: 0 10px 24px color-mix(in srgb, var(--bg-main) 14%, transparent);
}

.flow-node-card small {
  color: var(--text-muted);
}

.fallback-node {
  position: absolute;
}

.danger-zone {
  padding: 1rem;
  border-radius: 1rem;
  border: 1px solid color-mix(in srgb, var(--status-error) 24%, var(--border-subtle));
  background: color-mix(in srgb, var(--status-error) 8%, transparent);
}

.export-preview {
  margin: 0;
  padding: 0.95rem;
  border-radius: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 76%, transparent);
  color: var(--text-secondary);
  font-size: 0.78rem;
  line-height: 1.55;
  overflow: auto;
}

@media (max-width: 900px) {
  .header-top-row,
  .agent-center-toolbar,
  .member-toolbar,
  .field-grid {
    display: grid;
  }

  .search-input,
  .toolbar-actions,
  .view-switcher,
  .create-menu,
  .create-menu > button,
  .member-picker-field {
    width: 100%;
  }

  .header-top-row {
    grid-template-columns: 1fr;
    justify-items: center;
  }

  .page-title {
    justify-self: start;
  }

  .create-menu-panel {
    left: 0;
    right: auto;
    width: 100%;
  }

  .field-grid {
    grid-template-columns: 1fr;
  }

  .agent-dialog {
    width: calc(100vw - 1rem);
    margin-top: 1rem;
  }

  .asset-icon-item {
    width: min(8.75rem, 100%);
  }

  .asset-table {
    min-width: 42rem;
  }

  .pagination-row {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
