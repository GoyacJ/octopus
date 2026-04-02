import { defineStore } from 'pinia'
import {
  type ActivityRecord,
  type Agent,
  type AgentAssetKind,
  type ConnectionProfile,
  type ConversationActorKind,
  type ConversationAttachment,
  type ConversationComposerDraft,
  type Conversation,
  type ConversationMemoryItem,
  type ConversationQueueItem,
  ConversationIntent,
  type DecisionAction,
  type InboxItem,
  type KnowledgeEntry,
  type Message,
  type MessageProcessEntry,
  type MessageToolCall,
  type MessageUsage,
  type MenuNode,
  type PermissionMode,
  type Project,
  type ProjectResource,
  type RbacPermission,
  type RbacRole,
  type RunDetail,
  type RunStatus,
  type Team,
  type TeamStructureEdge,
  type TeamStructureNode,
  TeamMode,
  type ToolCatalogItem,
  type ToolCatalogGroup,
  type TraceRecord,
  type UserAccount,
  type Workspace,
  type WorkspaceMembership,
  type DashboardSnapshot,
  type DashboardMetric,
  type DashboardHighlight,
  type Artifact,
  type ProjectDashboardProgress,
  type ProjectDashboardSnapshot,
  type UsageRankingItem,
  type WorkspaceOverviewSnapshot,
} from '@octopus/schema'

import { mockKey, resolveMockField, translate } from '@/i18n/copy'
import { createMockWorkbenchSeed } from '@/mock/data'
import { MENU_DEFINITIONS, USER_CENTER_MENU_IDS, buildWorkspaceMenuNodes, getAncestorMenuIds, getMenuDefinition } from '@/navigation/menuRegistry'

function cloneSeed<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

interface CreateProjectResourceOptions {
  name?: string
  location?: string
}

interface UpdateProjectResourcePatch {
  name?: string
  location?: string
}

interface CreateUserAccountInput {
  workspaceId?: string
  username?: string
  nickname?: string
  gender?: UserAccount['gender']
  avatar?: string
  phone?: string
  email?: string
  status?: UserAccount['status']
  passwordState?: UserAccount['passwordState']
  roleIds?: string[]
  scopeMode?: WorkspaceMembership['scopeMode']
  scopeProjectIds?: string[]
}

interface CreateRoleInput {
  workspaceId?: string
  name: string
  code: string
  description: string
  status?: RbacRole['status']
  permissionIds?: string[]
  menuIds?: string[]
}

interface CreatePermissionInput {
  workspaceId?: string
  name: string
  code: string
  description: string
  status?: RbacPermission['status']
  kind: RbacPermission['kind']
  targetType?: RbacPermission['targetType']
  targetIds?: string[]
  action?: string
  memberPermissionIds?: string[]
}

interface UpdateProjectDetailsPatch {
  name?: string
  goal?: string
  phase?: string
  summary?: string
}

function formatMetric(value: number): string {
  return value.toString()
}

function sumMessageTokens(messages: Message[]): number {
  return messages.reduce((total, message) => total + (message.usage?.totalTokens ?? 0), 0)
}

function sumToolCallCount(messages: Message[]): number {
  return messages.reduce((total, message) =>
    total + (message.toolCalls?.reduce((count, item) => count + item.count, 0) ?? 0), 0)
}

function countModelCalls(messages: Message[]): number {
  return messages.reduce((total, message) => total + (message.modelId ? 1 : 0), 0)
}

function countActorCalls(messages: Message[], actorKind: ConversationActorKind): number {
  return messages.reduce((total, message) => total + (message.actorKind === actorKind ? 1 : 0), 0)
}

function membershipCanAccessProject(membership: WorkspaceMembership, projectId: string): boolean {
  return membership.scopeMode === 'all-projects' || membership.scopeProjectIds.includes(projectId)
}

function takeLatestActivities(activities: ActivityRecord[], limit = 10): ActivityRecord[] {
  return [...activities]
    .sort((left, right) => right.timestamp - left.timestamp)
    .slice(0, limit)
}

function sortRanking(items: UsageRankingItem[], limit?: number): UsageRankingItem[] {
  const ranked = [...items].sort((left, right) =>
    right.value - left.value || left.label.localeCompare(right.label),
  )
  return typeof limit === 'number' ? ranked.slice(0, limit) : ranked
}

function buildConversationTokenRanking(
  conversations: Conversation[],
  messages: Message[],
  limit = 3,
): UsageRankingItem[] {
  return sortRanking(
    conversations.map((conversation) => {
      const conversationMessages = messages.filter((message) => message.conversationId === conversation.id)
      return {
        id: conversation.id,
        label: conversation.title,
        value: sumMessageTokens(conversationMessages),
        secondary: conversation.summary,
      }
    }),
    limit,
  )
}

function buildActorUsageRanking(
  messages: Message[],
  actorKind: ConversationActorKind,
  labelMap: Map<string, string>,
): UsageRankingItem[] {
  const usage = new Map<string, { count: number, tokens: number }>()
  for (const message of messages) {
    if (message.actorKind !== actorKind || !message.actorId) {
      continue
    }

    const current = usage.get(message.actorId) ?? { count: 0, tokens: 0 }
    current.count += 1
    current.tokens += message.usage?.totalTokens ?? 0
    usage.set(message.actorId, current)
  }

  return sortRanking(Array.from(usage.entries()).map(([id, metrics]) => ({
    id,
    label: labelMap.get(id) ?? id,
    value: metrics.count,
    secondary: `${metrics.tokens}`,
  })))
}

function buildToolUsageRanking(messages: Message[], labelMap: Map<string, string>): UsageRankingItem[] {
  const usage = new Map<string, number>()
  for (const message of messages) {
    for (const toolCall of message.toolCalls ?? []) {
      usage.set(toolCall.toolId, (usage.get(toolCall.toolId) ?? 0) + toolCall.count)
    }
  }

  return sortRanking(Array.from(usage.entries()).map(([id, value]) => ({
    id,
    label: labelMap.get(id) ?? id,
    value,
  })))
}

function buildModelUsageRanking(messages: Message[], labelMap: Map<string, string>): UsageRankingItem[] {
  const usage = new Map<string, { count: number, tokens: number }>()
  for (const message of messages) {
    if (!message.modelId) {
      continue
    }

    const current = usage.get(message.modelId) ?? { count: 0, tokens: 0 }
    current.count += 1
    current.tokens += message.usage?.totalTokens ?? 0
    usage.set(message.modelId, current)
  }

  return sortRanking(Array.from(usage.entries()).map(([id, metrics]) => ({
    id,
    label: labelMap.get(id) ?? id,
    value: metrics.tokens,
    secondary: `${metrics.count}`,
  })))
}

function teamNodePosition(index: number) {
  const column = index % 3
  const row = Math.floor(index / 3)

  return {
    x: 80 + column * 220,
    y: 80 + row * 160,
  }
}

function buildTeamStructure(teamId: string, members: string[]) {
  const structureNodes: TeamStructureNode[] = members.map((memberId, index) => ({
    id: `${teamId}-node-${index + 1}`,
    label: index === 0 ? '负责人' : `成员 ${index}`,
    role: index === 0 ? 'Lead' : 'Contributor',
    memberId,
    level: index,
    position: teamNodePosition(index),
  }))
  const structureEdges: TeamStructureEdge[] = members.length > 1
    ? members.slice(1).map((_, index) => ({
        id: `${teamId}-edge-${index + 1}`,
        source: `${teamId}-node-1`,
        target: `${teamId}-node-${index + 2}`,
        relation: 'coordinates',
      }))
    : []

  return {
    structureNodes,
    structureEdges,
  }
}

function uniqueValues(values: string[]): string[] {
  return Array.from(new Set(values))
}

function dedupeById<T extends { id: string }>(items: T[]): T[] {
  const seen = new Set<string>()
  return items.filter((item) => {
    if (seen.has(item.id)) {
      return false
    }

    seen.add(item.id)
    return true
  })
}

function buildMenuLookup(menus: MenuNode[], workspaceId: string) {
  return new Map(
    menus
      .filter((menu) => menu.workspaceId === workspaceId)
      .map((menu) => [menu.id, menu]),
  )
}

function findMembership(
  memberships: WorkspaceMembership[],
  workspaceId: string,
  userId: string,
): WorkspaceMembership | undefined {
  return memberships.find((membership) => membership.workspaceId === workspaceId && membership.userId === userId)
}

function expandPermissionIds(permissionIds: string[], permissions: RbacPermission[]): string[] {
  const permissionMap = new Map(permissions.map((permission) => [permission.id, permission]))
  const expanded = new Set<string>()

  for (const permissionId of permissionIds) {
    const permission = permissionMap.get(permissionId)
    if (!permission || permission.status !== 'active') {
      continue
    }

    if (permission.kind === 'bundle') {
      for (const memberId of permission.memberPermissionIds ?? []) {
        const member = permissionMap.get(memberId)
        if (member?.status === 'active' && member.kind === 'atomic') {
          expanded.add(member.id)
        }
      }
      continue
    }

    expanded.add(permission.id)
  }

  return Array.from(expanded)
}

function resolveEffectivePermissionIds(
  workspaceId: string,
  userId: string,
  memberships: WorkspaceMembership[],
  roles: RbacRole[],
  permissions: RbacPermission[],
): string[] {
  const membership = findMembership(memberships, workspaceId, userId)
  if (!membership) {
    return []
  }

  const workspacePermissions = permissions.filter((permission) => permission.workspaceId === workspaceId)
  const workspacePermissionMap = new Map(workspacePermissions.map((permission) => [permission.id, permission]))
  const activeRoleMap = new Map(
    roles
      .filter((role) => role.workspaceId === workspaceId && role.status === 'active')
      .map((role) => [role.id, role]),
  )

  return expandPermissionIds(
    membership.roleIds.flatMap((roleId) => activeRoleMap.get(roleId)?.permissionIds ?? []),
    workspacePermissions,
  ).filter((permissionId) => {
    const permission = workspacePermissionMap.get(permissionId)
    if (!permission) {
      return false
    }

    if (membership.scopeMode !== 'selected-projects' || permission.targetType !== 'project') {
      return true
    }

    return (permission.targetIds ?? []).some((targetId) => membership.scopeProjectIds.includes(targetId))
  })
}

function normalizeMenuIds(menuIds: string[]): string[] {
  return uniqueValues(menuIds.flatMap((menuId) => [...getAncestorMenuIds(menuId), menuId]))
}

function menuNodeBranchEnabled(menuId: string, menuLookup: Map<string, MenuNode>): boolean {
  let pointer = menuLookup.get(menuId)
  while (pointer) {
    if (pointer.status !== 'active') {
      return false
    }
    pointer = pointer.parentId ? menuLookup.get(pointer.parentId) : undefined
  }

  return true
}

function resolveEffectiveMenuIds(
  workspaceId: string,
  userId: string,
  memberships: WorkspaceMembership[],
  roles: RbacRole[],
  menus: MenuNode[],
): string[] {
  const membership = findMembership(memberships, workspaceId, userId)
  if (!membership) {
    return []
  }

  const menuLookup = buildMenuLookup(menus, workspaceId)
  const roleMap = new Map(
    roles
      .filter((role) => role.workspaceId === workspaceId && role.status === 'active')
      .map((role) => [role.id, role]),
  )

  return normalizeMenuIds(
    membership.roleIds.flatMap((roleId) => roleMap.get(roleId)?.menuIds ?? []),
  ).filter((menuId) => menuNodeBranchEnabled(menuId, menuLookup))
}

function syncWorkspaceMemberCounts(workspaces: Workspace[], memberships: WorkspaceMembership[]) {
  for (const workspace of workspaces) {
    workspace.memberCount = memberships.filter((membership) => membership.workspaceId === workspace.id).length
  }
}

function nextMockEntityId(prefix: string, items: { id: string }[]): string {
  const sequence = items.filter((item) => item.id.startsWith(prefix)).length + 1
  return `${prefix}-${sequence}`
}

function normalizeToolIds(toolIds: string[], permissionMode: PermissionMode): string[] {
  const deduped = uniqueValues(toolIds)
  if (permissionMode !== 'readonly') {
    return deduped
  }

  const readonlyTools = deduped.filter((toolId) => !/write|terminal|delete|send|command/i.test(toolId))
  return readonlyTools.length ? readonlyTools : ['read']
}

function resolveActorToolIds(
  agents: Agent[],
  teams: Team[],
  actorKind: 'agent' | 'team',
  actorId: string,
  permissionMode: PermissionMode,
): string[] {
  if (actorKind === 'team') {
    const team = teams.find((item) => item.id === actorId)
    const teamTools = uniqueValues(
      (team?.members ?? []).flatMap((memberId) =>
        agents.find((agent) => agent.id === memberId)?.capabilityPolicy.tools ?? [],
      ),
    )
    return normalizeToolIds(teamTools, permissionMode)
  }

  const agent = agents.find((item) => item.id === actorId)
  return normalizeToolIds(agent?.capabilityPolicy.tools ?? [], permissionMode)
}

function createMockRun(sequence: number, projectId: string, conversationId: string, title: string, timestamp: number): RunDetail {
  return {
    id: `run-mock-${sequence}`,
    conversationId,
    projectId,
    title,
    type: 'conversation_run',
    status: 'running',
    currentStep: 'Preparing the starter project context.',
    startedAt: timestamp,
    updatedAt: timestamp,
    ownerType: 'agent',
    ownerId: 'agent-architect',
    blockers: [],
    nextAction: 'Open the starter conversation and continue shaping the workspace.',
    timeline: [],
  }
}

function createMockConversation(sequence: number, projectId: string, title: string, timestamp: number, run: RunDetail): Conversation {
  return {
    id: `conv-mock-${sequence}`,
    projectId,
    title,
    intent: ConversationIntent.PLAN,
    activeAgentId: 'agent-architect',
    summary: 'A starter conversation created from the desktop shell mock controls.',
    currentGoal: 'Outline the first project steps and collect the initial requirements.',
    constraints: ['Mock-only workspace', 'Live integration deferred'],
    blockerIds: [],
    pendingInboxIds: [],
    resumePoints: [],
    branchLinks: [],
    artifactIds: [],
    recentRun: { ...run },
    stageProgress: 8,
    statusNote: 'Ready for the next command.',
  }
}

const QUEUE_BLOCKING_STATUSES = new Set<RunStatus>(['running', 'waiting_input', 'waiting_approval'])
const QUEUE_STOP_STATUSES = new Set<RunStatus>(['waiting_input', 'waiting_approval', 'blocked', 'paused'])
const BUILTIN_TOOL_MAP: Record<string, string> = {
  read: 'builtin-read',
  search: 'builtin-read',
  summarize: 'builtin-read',
  review: 'builtin-read',
  write: 'builtin-write',
  terminal: 'builtin-terminal',
}

interface ResolvedConversationActor {
  actorKind: ConversationActorKind
  actorId: string
  usedDefaultActor: boolean
}

function isRunBlockingForQueue(status?: RunStatus): boolean {
  return status ? QUEUE_STOP_STATUSES.has(status) : false
}

function isConversationBusy(status?: RunStatus): boolean {
  return status ? QUEUE_BLOCKING_STATUSES.has(status) : false
}

function findToolCatalogItem(toolCatalog: ToolCatalogGroup[], toolId: string): ToolCatalogItem | undefined {
  return toolCatalog.flatMap((group) => group.items).find((item) => item.id === toolId)
}

function mapAgentSkillTool(agent: Agent, toolCatalog: ToolCatalogGroup[]): MessageToolCall[] {
  if (!agent.skillTags.length) {
    return []
  }

  const preferredSkillId = agent.skillTags.some((tag) => /vue|tauri/i.test(tag))
    ? 'skill-vue'
    : 'skill-tdd'
  const skillItem = findToolCatalogItem(toolCatalog, preferredSkillId)

  return skillItem
    ? [{ toolId: skillItem.id, label: skillItem.name, kind: skillItem.kind, count: 1 }]
    : []
}

function mapAgentMcpTool(agent: Agent, toolCatalog: ToolCatalogGroup[]): MessageToolCall[] {
  if (!agent.mcpBindings.length) {
    return []
  }

  const preferredMcpId = agent.mcpBindings.some((binding) => /figma/i.test(binding))
    ? 'mcp-figma'
    : 'mcp-docs'
  const mcpItem = findToolCatalogItem(toolCatalog, preferredMcpId)

  return mcpItem
    ? [{ toolId: mcpItem.id, label: mcpItem.name, kind: mcpItem.kind, count: 1 }]
    : []
}

function aggregateToolCalls(toolCalls: MessageToolCall[]): MessageToolCall[] {
  const aggregated = new Map<string, MessageToolCall>()
  for (const tool of toolCalls) {
    const existing = aggregated.get(tool.toolId)
    if (existing) {
      existing.count += tool.count
      continue
    }

    aggregated.set(tool.toolId, { ...tool })
  }

  return Array.from(aggregated.values()).sort((left, right) => right.count - left.count)
}

function resolveActorToolCalls(
  agents: Agent[],
  teams: Team[],
  toolCatalog: ToolCatalogGroup[],
  actorKind: ConversationActorKind,
  actorId: string,
  permissionMode: PermissionMode,
): MessageToolCall[] {
  const builtinToolIds = resolveActorToolIds(agents, teams, actorKind, actorId, permissionMode)
    .map((toolId) => BUILTIN_TOOL_MAP[toolId] ?? BUILTIN_TOOL_MAP.read)
  const builtinCalls = builtinToolIds
    .map((toolId) => findToolCatalogItem(toolCatalog, toolId))
    .filter(Boolean)
    .map((item) => ({
      toolId: item!.id,
      label: item!.name,
      kind: item!.kind,
      count: 1,
    }))

  if (actorKind === 'team') {
    const team = teams.find((item) => item.id === actorId)
    const memberAgents = (team?.members ?? [])
      .map((memberId) => agents.find((agent) => agent.id === memberId))
      .filter(Boolean) as Agent[]

    return aggregateToolCalls([
      ...builtinCalls,
      ...memberAgents.flatMap((agent) => mapAgentSkillTool(agent, toolCatalog)),
      ...memberAgents.flatMap((agent) => mapAgentMcpTool(agent, toolCatalog)),
    ])
  }

  const agent = agents.find((item) => item.id === actorId)
  return aggregateToolCalls([
    ...builtinCalls,
    ...(agent ? mapAgentSkillTool(agent, toolCatalog) : []),
    ...(agent ? mapAgentMcpTool(agent, toolCatalog) : []),
  ])
}

function estimateMessageUsage(content: string, toolCalls: MessageToolCall[], isAgentMessage: boolean): MessageUsage {
  const inputTokens = Math.max(48, Math.ceil(content.length * 1.6))
  const toolCost = toolCalls.reduce((total, item) => total + item.count * 42, 0)
  const outputTokens = isAgentMessage ? Math.max(96, Math.ceil(content.length * 1.2) + toolCost) : 0

  return {
    inputTokens,
    outputTokens,
    totalTokens: inputTokens + outputTokens,
  }
}

function buildUserProcessEntries(
  content: string,
  timestamp: number,
  usedDefaultActor: boolean,
): MessageProcessEntry[] {
  return [
    {
      id: `process-user-${timestamp}`,
      type: 'execution',
      title: usedDefaultActor ? 'Queued for the default actor route' : 'Captured explicit actor route',
      detail: content,
      timestamp,
    },
  ]
}

function buildAgentProcessEntries(
  content: string,
  timestamp: number,
  toolCalls: MessageToolCall[],
): MessageProcessEntry[] {
  const baseEntries: MessageProcessEntry[] = [
    {
      id: `process-agent-thinking-${timestamp}`,
      type: 'thinking',
      title: 'Synthesized the latest request',
      detail: `Captured the current message and aligned it with the active conversation state: ${content}`,
      timestamp,
    },
  ]

  if (toolCalls[0]) {
    baseEntries.push({
      id: `process-agent-tool-${timestamp}`,
      type: 'tool',
      title: `Used ${toolCalls[0].label}`,
      detail: `The execution path touched ${toolCalls.reduce((total, tool) => total + tool.count, 0)} tool invocations across builtin, skill, and MCP scopes.`,
      timestamp: timestamp + 1,
      toolId: toolCalls[0].toolId,
    })
  }

  baseEntries.push({
    id: `process-agent-result-${timestamp}`,
    type: 'result',
    title: 'Recorded the execution response',
    detail: 'A new response, trace, and derived conversation data were attached to the active thread.',
    timestamp: timestamp + 2,
  })

  return baseEntries
}

function resolveConversationActorTarget(
  project: Project | undefined,
  conversation: Conversation | undefined,
  agents: Agent[],
  teams: Team[],
  requestedActorKind?: ConversationActorKind,
  requestedActorId?: string,
): ResolvedConversationActor | null {
  const requestedExists = requestedActorKind === 'team'
    ? teams.some((item) => item.id === requestedActorId)
    : agents.some((item) => item.id === requestedActorId)

  if (requestedActorKind && requestedActorId && requestedExists) {
    return {
      actorKind: requestedActorKind,
      actorId: requestedActorId,
      usedDefaultActor: false,
    }
  }

  if (conversation?.activeTeamId && teams.some((item) => item.id === conversation.activeTeamId)) {
    return {
      actorKind: 'team',
      actorId: conversation.activeTeamId,
      usedDefaultActor: true,
    }
  }

  if (conversation?.activeAgentId && agents.some((item) => item.id === conversation.activeAgentId)) {
    return {
      actorKind: 'agent',
      actorId: conversation.activeAgentId,
      usedDefaultActor: true,
    }
  }

  if (project?.defaultActorKind && project.defaultActorId) {
    const exists = project.defaultActorKind === 'team'
      ? teams.some((item) => item.id === project.defaultActorId)
      : agents.some((item) => item.id === project.defaultActorId)
    if (exists) {
      return {
        actorKind: project.defaultActorKind,
        actorId: project.defaultActorId,
        usedDefaultActor: true,
      }
    }
  }

  const fallbackTeam = project?.teamIds.find((teamId) => teams.some((team) => team.id === teamId))
  if (fallbackTeam) {
    return {
      actorKind: 'team',
      actorId: fallbackTeam,
      usedDefaultActor: true,
    }
  }

  const fallbackAgent = project?.agentIds.find((agentId) => agents.some((agent) => agent.id === agentId))
    ?? agents[0]?.id
  if (fallbackAgent) {
    return {
      actorKind: 'agent',
      actorId: fallbackAgent,
      usedDefaultActor: true,
    }
  }

  return null
}

function latestUserSummary(messages: Message[]): string {
  return messages
    .filter((message) => message.senderType === 'user')
    .at(-1)?.content ?? ''
}

function nextTimestampFrom(values: number[], fallback = Date.now()): number {
  if (!values.length) {
    return fallback
  }

  return Math.max(fallback, Math.max(...values) + 1)
}

function nextConversationTimestamp(messages: Message[], conversationId: string, fallback = Date.now()): number {
  return nextTimestampFrom(
    messages
      .filter((message) => message.conversationId === conversationId)
      .map((message) => message.timestamp),
    fallback,
  )
}

export const useWorkbenchStore = defineStore('workbench', {
  state: () => cloneSeed(createMockWorkbenchSeed()),
  getters: {
    activeWorkspace(state) {
      return state.workspaces.find((workspace) => workspace.id === state.currentWorkspaceId)
    },
    currentUser(state) {
      return state.users.find((user) => user.id === state.currentUserId)
    },
    workspaceUsers(state) {
      return state.users
        .filter((user) => findMembership(state.memberships, state.currentWorkspaceId, user.id))
        .sort((left, right) => left.nickname.localeCompare(right.nickname))
    },
    workspaceRoles(state) {
      return state.roles
        .filter((role) => role.workspaceId === state.currentWorkspaceId)
        .sort((left, right) => left.name.localeCompare(right.name))
    },
    workspacePermissions(state) {
      return state.permissions
        .filter((permission) => permission.workspaceId === state.currentWorkspaceId)
        .sort((left, right) => left.name.localeCompare(right.name))
    },
    workspaceMenus(state) {
      return state.menus
        .filter((menu) => menu.workspaceId === state.currentWorkspaceId)
        .sort((left, right) => left.order - right.order)
    },
    currentMembership(state) {
      return findMembership(state.memberships, state.currentWorkspaceId, state.currentUserId)
    },
    currentUserRoles(): RbacRole[] {
      const roleIds = new Set(this.currentMembership?.roleIds ?? [])
      return this.workspaceRoles.filter((role) => roleIds.has(role.id))
    },
    effectivePermissionIdsForWorkspace(state) {
      return (workspaceId: string, userId = state.currentUserId) => resolveEffectivePermissionIds(
        workspaceId,
        userId,
        state.memberships,
        state.roles,
        state.permissions,
      )
    },
    effectivePermissionIdsByUser(state) {
      return (userId: string) => resolveEffectivePermissionIds(
        state.currentWorkspaceId,
        userId,
        state.memberships,
        state.roles,
        state.permissions,
      )
    },
    currentEffectivePermissionIds(): string[] {
      return this.effectivePermissionIdsByUser(this.currentUserId)
    },
    effectiveMenuIdsForWorkspace(state) {
      return (workspaceId: string, userId = state.currentUserId) => resolveEffectiveMenuIds(
        workspaceId,
        userId,
        state.memberships,
        state.roles,
        state.menus,
      )
    },
    effectiveMenuIdsByUser(state) {
      return (userId: string) => resolveEffectiveMenuIds(
        state.currentWorkspaceId,
        userId,
        state.memberships,
        state.roles,
        state.menus,
      )
    },
    currentEffectiveMenuIds(): string[] {
      return this.effectiveMenuIdsByUser(this.currentUserId)
    },
    availableUserCenterMenus(): MenuNode[] {
      const accessibleMenuIds = new Set(this.currentEffectiveMenuIds)
      return this.workspaceMenus.filter((menu) => USER_CENTER_MENU_IDS.includes(menu.id) && accessibleMenuIds.has(menu.id))
    },
    firstAccessibleUserCenterRouteName(): string | undefined {
      return this.availableUserCenterMenus
        .map((menu) => getMenuDefinition(menu.id)?.routeName)
        .find((routeName): routeName is string => Boolean(routeName))
    },
    firstAccessibleUserCenterRouteForWorkspace(state) {
      return (workspaceId: string, userId = state.currentUserId) => {
        const accessibleMenuIds = new Set(resolveEffectiveMenuIds(
          workspaceId,
          userId,
          state.memberships,
          state.roles,
          state.menus,
        ))
        return USER_CENTER_MENU_IDS
          .filter((menuId) => accessibleMenuIds.has(menuId))
          .map((menuId) => getMenuDefinition(menuId)?.routeName)
          .find((routeName): routeName is string => Boolean(routeName))
      }
    },
    workspaceProjects(state) {
      return state.projects.filter((project) => project.workspaceId === state.currentWorkspaceId)
    },
    activeProject(state) {
      return state.projects.find((project) => project.id === state.currentProjectId)
    },
    firstConversationIdForProject(state) {
      return (projectId: string) => state.projects.find((project) => project.id === projectId)?.conversationIds[0] ?? ''
    },
    projectConversations(state) {
      return state.conversations.filter((conversation) => conversation.projectId === state.currentProjectId)
    },
    activeConversation(state) {
      return state.conversations.find((conversation) => conversation.id === state.currentConversationId)
    },
    activeConversationDefaultActor(state) {
      const project = state.projects.find((item) => item.id === state.currentProjectId)
      const conversation = state.conversations.find((item) => item.id === state.currentConversationId)
      return resolveConversationActorTarget(project, conversation, state.agents, state.teams)
    },
    conversationMessages(state) {
      return state.messages
        .filter((message) => message.conversationId === state.currentConversationId)
        .sort((left, right) => left.timestamp - right.timestamp)
    },
    activeConversationQueue(state) {
      return state.conversationQueue
        .filter((item) => item.conversationId === state.currentConversationId)
        .sort((left, right) => left.createdAt - right.createdAt)
    },
    activeConversationMemories(state) {
      return state.conversationMemories
        .filter((memory) => memory.conversationId === state.currentConversationId)
        .sort((left, right) => right.createdAt - left.createdAt)
    },
    activeConversationArtifacts(state) {
      const conversation = state.conversations.find((item) => item.id === state.currentConversationId)
      if (!conversation) {
        return []
      }

      return state.artifacts.filter((artifact) => conversation.artifactIds.includes(artifact.id))
    },
    projectResources(state): ProjectResource[] {
      const project = state.projects.find((item) => item.id === state.currentProjectId)
      if (!project) {
        return []
      }

      const explicitResources = state.resources.filter((resource) => project.resourceIds.includes(resource.id))
      const artifactResources = state.artifacts
        .filter((artifact) => project.artifactIds.includes(artifact.id))
        .map<ProjectResource>((artifact) => ({
          id: artifact.id,
          projectId: artifact.projectId,
          workspaceId: project.workspaceId,
          name: artifact.title,
          kind: 'artifact',
          sourceArtifactId: artifact.id,
          linkedConversationIds: [artifact.conversationId],
          createdAt: artifact.updatedAt,
          sizeLabel: `v${artifact.version}`,
          location: artifact.type,
          tags: artifact.tags,
        }))

      const seen = new Set<string>()
      return [...explicitResources, ...artifactResources]
        .filter((resource) => {
          if (seen.has(resource.id)) {
            return false
          }

          seen.add(resource.id)
          return true
        })
        .sort((left, right) => right.createdAt - left.createdAt)
    },
    workspaceInbox(state) {
      return state.inbox.filter((item) => item.workspaceId === state.currentWorkspaceId)
    },
    activeRun(state) {
      return state.runs.find((run) => run.id === state.currentRunId)
    },
    activeTrace(state) {
      const run = state.runs.find((item) => item.id === state.currentRunId)
      if (!run) {
        return []
      }

      return state.traces.filter((trace) => trace.runId === run.id)
    },
    activeConversationTimeline(state) {
      return state.traces
        .filter((trace) => trace.conversationId === state.currentConversationId)
        .sort((left, right) => right.timestamp - left.timestamp)
    },
    projectKnowledge(state) {
      return state.knowledge.filter((item) => item.projectId === state.currentProjectId)
    },
    activeConversationKnowledge(): KnowledgeEntry[] {
      const artifactIds = new Set(this.activeConversationArtifacts.map((artifact) => artifact.id))
      const runIds = new Set(this.runs
        .filter((run) => run.conversationId === this.currentConversationId)
        .map((run) => run.id))

      return this.projectKnowledge
        .filter((entry) =>
          entry.conversationId === this.currentConversationId
          || entry.sourceId === this.currentConversationId
          || artifactIds.has(entry.sourceId)
          || runIds.has(entry.sourceId)
          || entry.lineage.includes(this.currentConversationId),
        )
        .sort((left, right) => right.lastUsedAt - left.lastUsedAt)
    },
    activeConversationResources(): ProjectResource[] {
      const conversationId = this.currentConversationId
      const artifactIds = new Set(this.activeConversationArtifacts.map((artifact) => artifact.id))

      return this.projectResources.filter((resource) =>
        resource.linkedConversationIds.includes(conversationId)
        || (resource.kind === 'artifact' && artifactIds.has(resource.sourceArtifactId ?? resource.id)),
      )
    },
    activeConversationToolStats(): MessageToolCall[] {
      return aggregateToolCalls(
        this.conversationMessages.flatMap((message) => message.toolCalls ?? []),
      )
    },
    workspaceLevelAgents(state): Agent[] {
      return state.agents.filter((agent) =>
        agent.scope === 'workspace' && agent.owner === `workspace:${state.currentWorkspaceId}`,
      )
    },
    projectOwnedAgents(state): Agent[] {
      const activeProject = state.projects.find((project) => project.id === state.currentProjectId)
      if (!activeProject) {
        return []
      }

      return state.agents.filter((agent) =>
        agent.scope === 'project'
        && agent.owner === `project:${activeProject.id}`
        && activeProject.agentIds.includes(agent.id),
      )
    },
    projectReferencedAgents(state): Agent[] {
      const activeProject = state.projects.find((project) => project.id === state.currentProjectId)
      if (!activeProject) {
        return []
      }

      return state.agents.filter((agent) =>
        activeProject.agentIds.includes(agent.id)
        && agent.scope !== 'project'
        && (
          agent.scope !== 'workspace'
          || agent.owner === `workspace:${state.currentWorkspaceId}`
        ),
      )
    },
    projectAgents(state): Agent[] {
      const activeProject = state.projects.find((project) => project.id === state.currentProjectId)
      if (!activeProject) {
        return []
      }

      const referencedAgents = state.agents.filter((agent) =>
        activeProject.agentIds.includes(agent.id)
        && agent.scope !== 'project'
        && (
          agent.scope !== 'workspace'
          || agent.owner === `workspace:${state.currentWorkspaceId}`
        ),
      )
      const ownedAgents = state.agents.filter((agent) =>
        agent.scope === 'project'
        && agent.owner === `project:${activeProject.id}`
        && activeProject.agentIds.includes(agent.id),
      )

      return dedupeById([...referencedAgents, ...ownedAgents])
    },
    workspaceAgents(state): Agent[] {
      const activeProject = state.projects.find((project) => project.id === state.currentProjectId)
      const workspaceLevelAgents = state.agents.filter((agent) =>
        agent.scope === 'workspace' && agent.owner === `workspace:${state.currentWorkspaceId}`,
      )
      const projectAgents = activeProject
        ? dedupeById(state.agents.filter((agent) =>
          activeProject.agentIds.includes(agent.id)
          && (
            agent.scope === 'project'
              ? agent.owner === `project:${activeProject.id}`
              : agent.scope !== 'workspace' || agent.owner === `workspace:${state.currentWorkspaceId}`
          ),
        ))
        : []

      return dedupeById([
        ...projectAgents,
        ...workspaceLevelAgents,
        ...state.agents.filter((agent) => agent.scope === 'personal'),
      ])
    },
    workspaceLevelTeams(state): Team[] {
      return state.teams.filter((team) => team.workspaceId === state.currentWorkspaceId && team.useScope === 'workspace')
    },
    projectOwnedTeams(state): Team[] {
      const activeProject = state.projects.find((project) => project.id === state.currentProjectId)
      if (!activeProject) {
        return []
      }

      return state.teams.filter((team) =>
        team.projectId === activeProject.id
        && team.useScope === 'project'
        && activeProject.teamIds.includes(team.id),
      )
    },
    projectReferencedTeams(state): Team[] {
      const activeProject = state.projects.find((project) => project.id === state.currentProjectId)
      if (!activeProject) {
        return []
      }

      return state.teams.filter((team) =>
        team.workspaceId === state.currentWorkspaceId
        && team.useScope === 'workspace'
        && activeProject.teamIds.includes(team.id),
      )
    },
    projectTeams(state): Team[] {
      const activeProject = state.projects.find((project) => project.id === state.currentProjectId)
      if (!activeProject) {
        return []
      }

      const referencedTeams = state.teams.filter((team) =>
        team.workspaceId === state.currentWorkspaceId
        && team.useScope === 'workspace'
        && activeProject.teamIds.includes(team.id),
      )
      const ownedTeams = state.teams.filter((team) =>
        team.projectId === activeProject.id
        && team.useScope === 'project'
        && activeProject.teamIds.includes(team.id),
      )

      return dedupeById([...referencedTeams, ...ownedTeams])
    },
    workspaceTeams(state) {
      return state.teams.filter((team) => team.workspaceId === state.currentWorkspaceId)
    },
    workspaceAutomations(state) {
      return state.automations.filter((automation) => automation.workspaceId === state.currentWorkspaceId)
    },
    activeConnections(state) {
      return state.connections.filter((connection) => connection.workspaceId === state.currentWorkspaceId)
    },
    workspaceModelCatalog(state) {
      return state.modelCatalog
    },
    toolCatalogGroups(state): ToolCatalogGroup[] {
      return state.toolCatalog
    },
    workspaceOverview(): WorkspaceOverviewSnapshot {
      const workspaceId = this.currentWorkspaceId
      const project = this.activeProject ?? this.workspaceProjects[0]
      const workspaceProjectIds = new Set(this.workspaceProjects.map((item) => item.id))
      const workspaceConversationIds = new Set(
        this.conversations
          .filter((conversation) => workspaceProjectIds.has(conversation.projectId))
          .map((conversation) => conversation.id),
      )
      const workspaceMessages = this.messages.filter((message) => workspaceConversationIds.has(message.conversationId))
      const workspaceActivities = this.activities.filter((activity) => activity.workspaceId === workspaceId)
      const userActivities = takeLatestActivities(workspaceActivities.filter((activity) => activity.scope === 'user'))
      const loginCount = workspaceActivities.filter((activity) => activity.scope === 'user' && activity.type === 'login').length
      const projectMessages = project
        ? this.messages.filter((message) => project.conversationIds.includes(message.conversationId))
        : []
      const projectConversations = project
        ? this.conversations.filter((conversation) => conversation.projectId === project.id)
        : []
      const projectKnowledgeCount = project
        ? this.knowledge.filter((entry) => entry.projectId === project.id).length
        : 0
      const projectActivity = project
        ? takeLatestActivities(
            workspaceActivities.filter((activity) => activity.scope === 'project' && activity.projectId === project.id),
          )
        : []
      const projectConversationTokenTop = buildConversationTokenRanking(projectConversations, this.messages)
      const projectParticipants = project
        ? new Set(
            this.memberships
              .filter((membership) =>
                membership.workspaceId === workspaceId && membershipCanAccessProject(membership, project.id),
              )
              .map((membership) => membership.userId),
          ).size
        : 0
      const workspaceTokenTop = sortRanking(this.workspaceProjects.map((workspaceProject) => {
        const messages = this.messages.filter((message) => workspaceProject.conversationIds.includes(message.conversationId))
        return {
          id: workspaceProject.id,
          label: workspaceProject.name,
          value: sumMessageTokens(messages),
          secondary: workspaceProject.phase,
        }
      }))
      const projectMap = new Map(this.projects.map((item) => [item.id, item]))
      const workspaceAgents = this.agents.filter((agent) =>
        agent.owner === `workspace:${workspaceId}`
        || (agent.owner.startsWith('project:') && projectMap.get(agent.owner.replace('project:', ''))?.workspaceId === workspaceId),
      )
      const workspaceTeams = this.teams.filter((team) => team.workspaceId === workspaceId)
      const toolCatalogItems = this.toolCatalogGroups.flatMap((group) => group.items)
      const agentLabelMap = new Map(workspaceAgents.map((agent) => [agent.id, agent.name]))
      const teamLabelMap = new Map(workspaceTeams.map((team) => [team.id, team.name]))
      const toolLabelMap = new Map(toolCatalogItems.map((item) => [item.id, item.name]))
      const modelLabelMap = new Map(this.workspaceModelCatalog.map((item) => [item.id, item.label]))

      return {
        workspaceId,
        projectId: project?.id,
        userMetrics: [
          { label: 'overview.user.users', value: formatMetric(this.workspaceUsers.length) },
          { label: 'overview.user.logins', value: formatMetric(loginCount) },
          {
            label: 'overview.user.tokens',
            value: formatMetric(
              sumMessageTokens(workspaceMessages.filter((message) => message.senderType === 'user')),
            ),
          },
        ],
        userActivity: userActivities,
        projectSummary: {
          projectId: project?.id ?? '',
          metrics: [
            { label: 'projectDashboard.data.conversations', value: formatMetric(project?.conversationIds.length ?? 0) },
            { label: 'projectDashboard.data.participants', value: formatMetric(projectParticipants) },
            { label: 'projectDashboard.data.boundAgents', value: formatMetric(project?.agentIds.length ?? 0) },
            { label: 'projectDashboard.data.knowledge', value: formatMetric(projectKnowledgeCount) },
            { label: 'projectDashboard.data.agentCalls', value: formatMetric(countActorCalls(projectMessages, 'agent')) },
            { label: 'projectDashboard.data.toolCalls', value: formatMetric(sumToolCallCount(projectMessages)) },
            { label: 'projectDashboard.data.modelCalls', value: formatMetric(countModelCalls(projectMessages)) },
            { label: 'projectDashboard.data.tokens', value: formatMetric(sumMessageTokens(projectMessages)) },
            { label: 'projectDashboard.data.activity', value: formatMetric(projectActivity.length) },
          ],
          activity: projectActivity,
          conversationTokenTop: projectConversationTokenTop,
        },
        workspaceMetrics: [
          { label: 'overview.workspace.projects', value: formatMetric(this.workspaceProjects.length) },
          { label: 'overview.workspace.agents', value: formatMetric(workspaceAgents.length) },
          { label: 'overview.workspace.teams', value: formatMetric(workspaceTeams.length) },
          { label: 'overview.workspace.tools', value: formatMetric(toolCatalogItems.length) },
          { label: 'overview.workspace.models', value: formatMetric(this.workspaceModelCatalog.length) },
          {
            label: 'overview.workspace.knowledge',
            value: formatMetric(this.knowledge.filter((entry) => entry.workspaceId === workspaceId).length),
          },
          { label: 'overview.workspace.automations', value: formatMetric(this.workspaceAutomations.length) },
          { label: 'overview.workspace.tokens', value: formatMetric(sumMessageTokens(workspaceMessages)) },
        ],
        projectTokenTop: workspaceTokenTop,
        agentUsage: buildActorUsageRanking(workspaceMessages, 'agent', agentLabelMap),
        teamUsage: buildActorUsageRanking(workspaceMessages, 'team', teamLabelMap),
        toolUsage: buildToolUsageRanking(workspaceMessages, toolLabelMap),
        modelUsage: buildModelUsageRanking(workspaceMessages, modelLabelMap),
      }
    },
    projectDashboard(): ProjectDashboardSnapshot {
      const project = this.activeProject ?? this.workspaceProjects[0]
      if (!project) {
        throw new Error('No active project selected')
      }

      const projectConversations = this.conversations.filter((conversation) => conversation.projectId === project.id)
      const projectMessages = this.messages.filter((message) => project.conversationIds.includes(message.conversationId))
      const mainConversation = projectConversations.find((conversation) => conversation.id === project.conversationIds[0]) ?? projectConversations[0]
      const run = mainConversation?.recentRun ?? this.runs.find((item) => item.conversationId === mainConversation?.id)
      const pendingInboxCount = projectConversations.reduce((count, conversation) => count + conversation.pendingInboxIds.length, 0)
      const participants = new Set(
        this.memberships
          .filter((membership) =>
            membership.workspaceId === project.workspaceId && membershipCanAccessProject(membership, project.id),
          )
          .map((membership) => membership.userId),
      ).size
      const projectKnowledgeCount = this.knowledge.filter((entry) => entry.projectId === project.id).length
      const projectActivity = takeLatestActivities(
        this.activities.filter((activity) => activity.scope === 'project' && activity.projectId === project.id),
      )

      const progress: ProjectDashboardProgress = {
        phase: project.phase,
        progress: mainConversation?.stageProgress ?? 0,
        runStatus: run?.status,
        currentStep: run?.currentStep ?? '',
        blockerCount: project.blockerIds.length + (run?.blockers.length ?? 0),
        pendingInboxCount,
      }

      return {
        workspaceId: project.workspaceId,
        project,
        resourceMetrics: [
          { label: 'projectDashboard.resources.conversations', value: formatMetric(project.conversationIds.length) },
          { label: 'projectDashboard.resources.resources', value: formatMetric(this.projectResources.length) },
          { label: 'projectDashboard.resources.agents', value: formatMetric(project.agentIds.length) },
          { label: 'projectDashboard.resources.teams', value: formatMetric(project.teamIds.length) },
          { label: 'projectDashboard.resources.knowledge', value: formatMetric(projectKnowledgeCount) },
        ],
        progress,
        dataMetrics: [
          { label: 'projectDashboard.data.conversations', value: formatMetric(project.conversationIds.length) },
          { label: 'projectDashboard.data.participants', value: formatMetric(participants) },
          { label: 'projectDashboard.data.boundAgents', value: formatMetric(project.agentIds.length) },
          { label: 'projectDashboard.data.knowledge', value: formatMetric(projectKnowledgeCount) },
          { label: 'projectDashboard.data.agentCalls', value: formatMetric(countActorCalls(projectMessages, 'agent')) },
          { label: 'projectDashboard.data.toolCalls', value: formatMetric(sumToolCallCount(projectMessages)) },
          { label: 'projectDashboard.data.modelCalls', value: formatMetric(countModelCalls(projectMessages)) },
          { label: 'projectDashboard.data.tokens', value: formatMetric(sumMessageTokens(projectMessages)) },
          { label: 'projectDashboard.data.activity', value: formatMetric(projectActivity.length) },
        ],
        activity: projectActivity,
        conversationTokenTop: buildConversationTokenRanking(projectConversations, this.messages),
      }
    },
    workspaceDashboard(): DashboardSnapshot {
      const workspaceMetrics: DashboardMetric[] = [
        { label: 'dashboard.metrics.activeProjects', value: formatMetric(this.workspaceProjects.length) },
        {
          label: 'dashboard.metrics.activeConversations',
          value: formatMetric(this.workspaceProjects.reduce((count, project) => count + project.conversationIds.length, 0)),
        },
        {
          label: 'dashboard.metrics.pendingInbox',
          value: formatMetric(this.workspaceInbox.filter((item) => item.status === 'pending').length),
          tone: this.workspaceInbox.some((item) => item.priority === 'high' && item.status === 'pending') ? 'warning' : 'default',
        },
      ]

      const projectMetrics: DashboardMetric[] = this.activeProject
        ? [
            {
              label: 'dashboard.metrics.projectPhase',
              value: mockKey('project', this.activeProject.id, 'phase', this.activeProject.phase),
            },
            { label: 'dashboard.metrics.artifacts', value: formatMetric(this.activeProject.artifactIds.length) },
            { label: 'dashboard.metrics.teams', value: formatMetric(this.activeProject.teamIds.length) },
          ]
        : []

      const conversationMetrics: DashboardMetric[] = this.activeConversation
        ? [
            { label: 'dashboard.metrics.intent', value: `enum.conversationIntent.${this.activeConversation.intent}` },
            { label: 'dashboard.metrics.progress', value: `${this.activeConversation.stageProgress}%` },
            { label: 'dashboard.metrics.resumePoints', value: formatMetric(this.activeConversation.resumePoints.length) },
          ]
        : []

      const highlights: DashboardHighlight[] = [
        {
          id: 'highlight-conversation',
          title: this.activeConversation
            ? mockKey('conversation', this.activeConversation.id, 'title', this.activeConversation.title)
            : 'dashboard.highlights.conversationTitle',
          description: this.activeConversation
            ? mockKey('conversation', this.activeConversation.id, 'statusNote', this.activeConversation.statusNote)
            : 'dashboard.highlights.conversationDescription',
          route: this.currentConversationId
            ? `/workspaces/${this.currentWorkspaceId}/projects/${this.currentProjectId}/conversations/${this.currentConversationId}`
            : `/workspaces/${this.currentWorkspaceId}/projects/${this.currentProjectId}/conversations`,
          surface: 'conversation',
        },
        {
          id: 'highlight-artifact',
          title: this.activeConversationArtifacts[0]
            ? mockKey('artifact', this.activeConversationArtifacts[0].id, 'title', this.activeConversationArtifacts[0].title)
            : 'dashboard.highlights.artifactTitle',
          description: 'dashboard.highlights.artifactDescription',
          route: `/workspaces/${this.currentWorkspaceId}/projects/${this.currentProjectId}/conversations/${this.currentConversationId}?detail=resources`,
          surface: 'artifact',
        },
        {
          id: 'highlight-trace',
          title: this.activeRun
            ? mockKey('run', this.activeRun.id, 'title', this.activeRun.title)
            : 'dashboard.highlights.traceTitle',
          description: this.activeRun
            ? mockKey('run', this.activeRun.id, 'nextAction', this.activeRun.nextAction)
            : 'dashboard.highlights.traceDescription',
          route: `/workspaces/${this.currentWorkspaceId}/projects/${this.currentProjectId}/trace`,
          surface: 'trace',
        },
      ]

      return {
        workspaceId: this.currentWorkspaceId,
        projectId: this.currentProjectId,
        conversationId: this.currentConversationId,
        workspaceMetrics,
        projectMetrics,
        conversationMetrics,
        highlights,
      }
    },
  },
  actions: {
    switchCurrentUser(userId: string) {
      const user = this.users.find((item) => item.id === userId)
      if (!user) {
        return false
      }

      this.currentUserId = userId

      const currentWorkspaceMembership = findMembership(this.memberships, this.currentWorkspaceId, userId)
      if (!currentWorkspaceMembership) {
        const fallbackMembership = this.memberships.find((membership) => membership.userId === userId)
        if (fallbackMembership) {
          this.selectWorkspace(fallbackMembership.workspaceId)
        }
      }

      const activeMembership = findMembership(this.memberships, this.currentWorkspaceId, userId)
      if (
        activeMembership?.scopeMode === 'selected-projects'
        && activeMembership.scopeProjectIds.length
        && !activeMembership.scopeProjectIds.includes(this.currentProjectId)
      ) {
        const nextProjectId = activeMembership.scopeProjectIds.find((projectId) =>
          this.projects.some((project) => project.id === projectId && project.workspaceId === this.currentWorkspaceId),
        )

        if (nextProjectId) {
          this.selectProject(nextProjectId)
        }
      }

      return true
    },
    createUser(input: CreateUserAccountInput = {}) {
      const userId = nextMockEntityId('user-mock', this.users)
      const workspaceId = input.workspaceId ?? this.currentWorkspaceId
      const timestamp = Date.now()
      const fallbackSequence = this.users.length + 1
      const username = input.username?.trim() || `user${fallbackSequence}`
      const user: UserAccount = {
        id: userId,
        username,
        nickname: input.nickname?.trim() || `User ${fallbackSequence}`,
        gender: input.gender ?? 'unknown',
        avatar: input.avatar?.trim() || username.slice(0, 2).toUpperCase(),
        phone: input.phone?.trim() || '',
        email: input.email?.trim() || `${username}@octopus.local`,
        status: input.status ?? 'active',
        passwordState: input.passwordState ?? 'temporary',
        passwordUpdatedAt: timestamp,
      }

      this.users.push(user)
      this.upsertMembership({
        workspaceId,
        userId,
        roleIds: input.roleIds ?? [],
        scopeMode: input.scopeMode ?? 'all-projects',
        scopeProjectIds: input.scopeProjectIds ?? [],
      })

      return user
    },
    updateUser(userId: string, patch: Partial<Omit<UserAccount, 'id'>>) {
      const user = this.users.find((item) => item.id === userId)
      if (!user) {
        return undefined
      }

      Object.assign(user, patch)
      return user
    },
    deleteUser(userId: string) {
      if (this.users.length <= 1) {
        return false
      }

      const exists = this.users.some((user) => user.id === userId)
      if (!exists) {
        return false
      }

      this.users = this.users.filter((user) => user.id !== userId)
      this.memberships = this.memberships.filter((membership) => membership.userId !== userId)
      syncWorkspaceMemberCounts(this.workspaces, this.memberships)

      if (this.currentUserId === userId) {
        const fallbackMembership = this.memberships.find((membership) => membership.workspaceId === this.currentWorkspaceId)
          ?? this.memberships[0]
        const fallbackUserId = fallbackMembership?.userId ?? this.users[0]?.id
        if (fallbackUserId) {
          this.switchCurrentUser(fallbackUserId)
        }
      }

      return true
    },
    toggleUserStatus(userId: string) {
      const user = this.users.find((item) => item.id === userId)
      if (!user) {
        return undefined
      }

      user.status = user.status === 'active' ? 'disabled' : 'active'
      return user.status
    },
    resetUserPassword(userId: string) {
      const user = this.users.find((item) => item.id === userId)
      if (!user) {
        return undefined
      }

      user.passwordState = 'temporary'
      user.passwordUpdatedAt = Date.now()
      return user
    },
    upsertMembership(membership: WorkspaceMembership) {
      const normalized: WorkspaceMembership = {
        workspaceId: membership.workspaceId,
        userId: membership.userId,
        roleIds: uniqueValues(
          membership.roleIds.filter((roleId) =>
            this.roles.some((role) => role.id === roleId && role.workspaceId === membership.workspaceId),
          ),
        ),
        scopeMode: membership.scopeMode,
        scopeProjectIds: membership.scopeMode === 'selected-projects'
          ? uniqueValues(
              membership.scopeProjectIds.filter((projectId) =>
                this.projects.some((project) => project.id === projectId && project.workspaceId === membership.workspaceId),
              ),
            )
          : [],
      }

      const membershipIndex = this.memberships.findIndex((item) =>
        item.workspaceId === normalized.workspaceId && item.userId === normalized.userId,
      )

      if (membershipIndex === -1) {
        this.memberships.push(normalized)
      }
      else {
        this.memberships[membershipIndex] = normalized
      }

      syncWorkspaceMemberCounts(this.workspaces, this.memberships)
      return normalized
    },
    setUserRoles(userId: string, roleIds: string[], workspaceId?: string) {
      const targetWorkspaceId = workspaceId ?? this.currentWorkspaceId
      const membership = findMembership(this.memberships, targetWorkspaceId, userId) ?? {
        workspaceId: targetWorkspaceId,
        userId,
        roleIds: [],
        scopeMode: 'all-projects' as const,
        scopeProjectIds: [],
      }

      return this.upsertMembership({
        ...membership,
        roleIds,
      })
    },
    setMembershipScope(
      userId: string,
      scopeMode: WorkspaceMembership['scopeMode'],
      scopeProjectIds: string[],
      workspaceId?: string,
    ) {
      const targetWorkspaceId = workspaceId ?? this.currentWorkspaceId
      const membership = findMembership(this.memberships, targetWorkspaceId, userId) ?? {
        workspaceId: targetWorkspaceId,
        userId,
        roleIds: [],
        scopeMode,
        scopeProjectIds: [],
      }

      return this.upsertMembership({
        ...membership,
        scopeMode,
        scopeProjectIds,
      })
    },
    createRole(input: CreateRoleInput) {
      const workspaceId = input.workspaceId ?? this.currentWorkspaceId
      const role: RbacRole = {
        id: nextMockEntityId(`role-${workspaceId}`, this.roles),
        workspaceId,
        name: input.name.trim(),
        code: input.code.trim(),
        description: input.description.trim(),
        status: input.status ?? 'active',
        permissionIds: uniqueValues(
          (input.permissionIds ?? []).filter((permissionId) =>
            this.permissions.some((permission) => permission.id === permissionId && permission.workspaceId === workspaceId),
          ),
        ),
        menuIds: uniqueValues(
          normalizeMenuIds(
            (input.menuIds ?? []).filter((menuId) =>
              this.menus.some((menu) => menu.id === menuId && menu.workspaceId === workspaceId),
            ),
          ),
        ),
      }

      this.roles.push(role)
      return role
    },
    updateRole(roleId: string, patch: Partial<Omit<RbacRole, 'id' | 'workspaceId'>>) {
      const role = this.roles.find((item) => item.id === roleId)
      if (!role) {
        return undefined
      }

      if (patch.name !== undefined) {
        role.name = patch.name.trim()
      }
      if (patch.code !== undefined) {
        role.code = patch.code.trim()
      }
      if (patch.description !== undefined) {
        role.description = patch.description.trim()
      }
      if (patch.status !== undefined) {
        role.status = patch.status
      }
      if (patch.permissionIds !== undefined) {
        role.permissionIds = uniqueValues(
          patch.permissionIds.filter((permissionId) =>
            this.permissions.some((permission) => permission.id === permissionId && permission.workspaceId === role.workspaceId),
          ),
        )
      }
      if (patch.menuIds !== undefined) {
        role.menuIds = uniqueValues(
          normalizeMenuIds(
            patch.menuIds.filter((menuId) =>
              this.menus.some((menu) => menu.id === menuId && menu.workspaceId === role.workspaceId),
            ),
          ),
        )
      }

      return role
    },
    deleteRole(roleId: string) {
      if (this.memberships.some((membership) => membership.roleIds.includes(roleId))) {
        return false
      }

      const hasRole = this.roles.some((role) => role.id === roleId)
      if (!hasRole) {
        return false
      }

      this.roles = this.roles.filter((role) => role.id !== roleId)
      return true
    },
    toggleRoleStatus(roleId: string) {
      const role = this.roles.find((item) => item.id === roleId)
      if (!role) {
        return undefined
      }

      role.status = role.status === 'active' ? 'disabled' : 'active'
      return role.status
    },
    assignRolePermissions(roleId: string, permissionIds: string[]) {
      return this.updateRole(roleId, { permissionIds })
    },
    assignRoleMenus(roleId: string, menuIds: string[]) {
      return this.updateRole(roleId, { menuIds })
    },
    createPermission(input: CreatePermissionInput) {
      const workspaceId = input.workspaceId ?? this.currentWorkspaceId
      const permission: RbacPermission = {
        id: nextMockEntityId(`perm-${workspaceId}`, this.permissions),
        workspaceId,
        name: input.name.trim(),
        code: input.code.trim(),
        description: input.description.trim(),
        status: input.status ?? 'active',
        kind: input.kind,
        ...(input.kind === 'bundle'
          ? {
              memberPermissionIds: uniqueValues(
                (input.memberPermissionIds ?? []).filter((permissionId) =>
                  this.permissions.some((permission) =>
                    permission.id === permissionId
                    && permission.workspaceId === workspaceId
                    && permission.kind === 'atomic',
                  ),
                ),
              ),
            }
          : {
              targetType: input.targetType,
              targetIds: uniqueValues(input.targetIds ?? []),
              action: input.action?.trim() || 'use',
            }),
      }

      this.permissions.push(permission)
      return permission
    },
    updatePermission(permissionId: string, patch: Partial<Omit<RbacPermission, 'id' | 'workspaceId'>>) {
      const permission = this.permissions.find((item) => item.id === permissionId)
      if (!permission) {
        return undefined
      }

      if (patch.name !== undefined) {
        permission.name = patch.name.trim()
      }
      if (patch.code !== undefined) {
        permission.code = patch.code.trim()
      }
      if (patch.description !== undefined) {
        permission.description = patch.description.trim()
      }
      if (patch.status !== undefined) {
        permission.status = patch.status
      }
      if (patch.kind !== undefined) {
        permission.kind = patch.kind
      }

      if (permission.kind === 'bundle') {
        permission.memberPermissionIds = uniqueValues(
          (patch.memberPermissionIds ?? permission.memberPermissionIds ?? []).filter((memberId) =>
            this.permissions.some((candidate) =>
              candidate.id === memberId
              && candidate.workspaceId === permission.workspaceId
              && candidate.kind === 'atomic',
            ),
          ),
        )
        delete permission.targetType
        delete permission.targetIds
        delete permission.action
      }
      else {
        if (patch.targetType !== undefined) {
          permission.targetType = patch.targetType
        }
        if (patch.targetIds !== undefined) {
          permission.targetIds = uniqueValues(patch.targetIds)
        }
        if (patch.action !== undefined) {
          permission.action = patch.action.trim()
        }
        delete permission.memberPermissionIds
      }

      return permission
    },
    deletePermission(permissionId: string) {
      const exists = this.permissions.some((permission) => permission.id === permissionId)
      if (!exists) {
        return false
      }

      this.permissions = this.permissions.filter((permission) => permission.id !== permissionId)
      for (const role of this.roles) {
        role.permissionIds = role.permissionIds.filter((id) => id !== permissionId)
      }
      for (const permission of this.permissions) {
        if (permission.kind === 'bundle') {
          permission.memberPermissionIds = (permission.memberPermissionIds ?? []).filter((id) => id !== permissionId)
        }
      }

      return true
    },
    togglePermissionStatus(permissionId: string) {
      const permission = this.permissions.find((item) => item.id === permissionId)
      if (!permission) {
        return undefined
      }

      permission.status = permission.status === 'active' ? 'disabled' : 'active'
      return permission.status
    },
    updateMenu(menuId: string, patch: Partial<Pick<MenuNode, 'label' | 'order' | 'status'>>) {
      const menu = this.menus.find((item) => item.id === menuId && item.workspaceId === this.currentWorkspaceId)
      if (!menu) {
        return undefined
      }

      if (patch.label !== undefined) {
        menu.label = patch.label.trim()
      }
      if (patch.order !== undefined) {
        menu.order = patch.order
      }
      if (patch.status !== undefined) {
        menu.status = patch.status
      }

      return menu
    },
    toggleMenuStatus(menuId: string) {
      const menu = this.menus.find((item) => item.id === menuId && item.workspaceId === this.currentWorkspaceId)
      if (!menu) {
        return undefined
      }

      menu.status = menu.status === 'active' ? 'disabled' : 'active'
      return menu.status
    },
    selectWorkspace(workspaceId: string) {
      const workspace = this.workspaces.find((item) => item.id === workspaceId)
      if (!workspace) {
        return
      }

      this.currentWorkspaceId = workspaceId
      const nextProject = this.projects.find((project) => project.workspaceId === workspaceId)
      if (!nextProject) {
        this.currentProjectId = ''
        this.currentConversationId = ''
        this.currentRunId = ''
        return
      }

      this.currentProjectId = nextProject.id
      this.currentConversationId = nextProject.conversationIds[0] ?? ''
      if (this.currentConversationId) {
        this.selectRunByConversation(this.currentConversationId)
      }
      else {
        this.currentRunId = ''
      }
    },
    selectProject(projectId: string) {
      const project = this.projects.find((item) => item.id === projectId)
      if (!project) {
        return
      }

      this.currentProjectId = projectId
      this.currentWorkspaceId = project.workspaceId
      this.currentConversationId = project.conversationIds[0] ?? ''
      if (this.currentConversationId) {
        this.selectRunByConversation(this.currentConversationId)
      }
      else {
        this.currentRunId = ''
      }
    },
    selectConversation(conversationId: string) {
      const conversation = this.conversations.find((item) => item.id === conversationId)
      if (!conversation) {
        return
      }

      this.currentConversationId = conversationId
      this.currentProjectId = conversation.projectId
      const project = this.projects.find((item) => item.id === conversation.projectId)
      if (project) {
        this.currentWorkspaceId = project.workspaceId
      }
      this.selectRunByConversation(conversationId)
    },
    selectRunByConversation(conversationId: string) {
      const run = this.runs.find((item) => item.conversationId === conversationId)
      if (run) {
        this.currentRunId = run.id
        return
      }

      this.currentRunId = ''
    },
    resolveConversationActor(requestedActorKind?: ConversationActorKind, requestedActorId?: string) {
      return resolveConversationActorTarget(
        this.activeProject,
        this.activeConversation,
        this.agents,
        this.teams,
        requestedActorKind,
        requestedActorId,
      )
    },
    syncConversationRunSnapshot(conversationId: string) {
      const conversation = this.conversations.find((item) => item.id === conversationId)
      const run = this.runs.find((item) => item.conversationId === conversationId)
      if (!conversation || !run) {
        return
      }

      conversation.recentRun = { ...run }
    },
    ingestConversationPayload(
      conversationId: string,
      payload: ConversationComposerDraft,
      resolvedActor: ResolvedConversationActor,
      timestamp = Date.now(),
    ) {
      const conversation = this.conversations.find((item) => item.id === conversationId)
      const project = conversation
        ? this.projects.find((item) => item.id === conversation.projectId)
        : undefined
      if (!conversation || !project) {
        return
      }

      let run = this.runs.find((item) => item.conversationId === conversation.id)
      if (!run) {
        run = createMockRun(this.runs.length + 1, project.id, conversation.id, `${conversation.title} run`, timestamp)
        this.runs.push(run)
      }

      const resolvedToolIds = resolveActorToolIds(
        this.agents,
        this.teams,
        resolvedActor.actorKind,
        resolvedActor.actorId,
        payload.permissionMode,
      )
      const resolvedToolCalls = resolveActorToolCalls(
        this.agents,
        this.teams,
        this.toolCatalogGroups,
        resolvedActor.actorKind,
        resolvedActor.actorId,
        payload.permissionMode,
      )
      const attachmentArtifacts = payload.attachments
        .filter((attachment): attachment is ConversationAttachment & { kind: 'artifact' } => attachment.kind === 'artifact')
        .map((attachment) => attachment.id)

      if (resolvedActor.actorKind === 'team') {
        conversation.activeTeamId = resolvedActor.actorId
        conversation.activeAgentId = this.teams.find((team) => team.id === resolvedActor.actorId)?.members[0] ?? conversation.activeAgentId
      }
      else {
        conversation.activeAgentId = resolvedActor.actorId
        conversation.activeTeamId = undefined
      }

      const userMessage: Message = {
        id: `msg-user-${timestamp}`,
        conversationId: conversation.id,
        senderId: 'user-1',
        senderType: 'user',
        content: payload.content.trim(),
        modelId: payload.modelId,
        permissionMode: payload.permissionMode,
        actorKind: resolvedActor.actorKind,
        actorId: resolvedActor.actorId,
        requestedActorKind: payload.actorKind,
        requestedActorId: payload.actorId,
        usedDefaultActor: resolvedActor.usedDefaultActor,
        resourceIds: payload.resourceIds,
        toolIds: resolvedToolIds,
        toolCalls: resolvedToolCalls,
        usage: estimateMessageUsage(payload.content.trim(), resolvedToolCalls, false),
        processEntries: buildUserProcessEntries(payload.content.trim(), timestamp, resolvedActor.usedDefaultActor),
        attachments: payload.attachments,
        artifacts: attachmentArtifacts,
        timestamp,
      }
      const agentMessage: Message = {
        id: `msg-agent-${timestamp}`,
        conversationId: conversation.id,
        senderId: resolvedActor.actorId,
        senderType: 'agent',
        content: 'runtime.messages.requirementsRecorded',
        modelId: payload.modelId,
        permissionMode: payload.permissionMode,
        actorKind: resolvedActor.actorKind,
        actorId: resolvedActor.actorId,
        requestedActorKind: payload.actorKind,
        requestedActorId: payload.actorId,
        usedDefaultActor: resolvedActor.usedDefaultActor,
        resourceIds: payload.resourceIds,
        toolIds: resolvedToolIds,
        toolCalls: resolvedToolCalls,
        usage: estimateMessageUsage(payload.content.trim(), resolvedToolCalls, true),
        processEntries: buildAgentProcessEntries(payload.content.trim(), timestamp + 1, resolvedToolCalls),
        attachments: payload.attachments,
        artifacts: attachmentArtifacts.length ? attachmentArtifacts : undefined,
        timestamp: timestamp + 1,
      }

      const artifact: Artifact = {
        id: `art-generated-${timestamp}`,
        projectId: project.id,
        conversationId: conversation.id,
        type: 'execution-note',
        title: `Execution Note ${new Date(timestamp).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })}`,
        content: `# Execution Note\n\n${payload.content.trim()}\n\nResolved actor: ${resolvedActor.actorKind}:${resolvedActor.actorId}`,
        excerpt: payload.content.trim().slice(0, 140),
        tags: ['conversation', 'generated'],
        version: 1,
        status: 'draft',
        authorId: resolvedActor.actorId,
        updatedAt: timestamp + 1,
        createdByMessageId: agentMessage.id,
      }
      const resource: ProjectResource = {
        id: `res-generated-${timestamp}`,
        projectId: project.id,
        workspaceId: project.workspaceId,
        name: `${artifact.title}.md`,
        kind: 'file',
        linkedConversationIds: [conversation.id],
        createdAt: timestamp + 1,
        createdByMessageId: agentMessage.id,
        sizeLabel: `${Math.max(8, Math.ceil(artifact.content.length / 12))} KB`,
        location: `/mock/${project.id}/${artifact.id}.md`,
        tags: ['generated', 'conversation'],
      }
      const knowledgeEntry: KnowledgeEntry = {
        id: `knowledge-generated-${timestamp}`,
        workspaceId: project.workspaceId,
        projectId: project.id,
        conversationId: conversation.id,
        title: 'Conversation execution insight',
        kind: 'candidate',
        status: 'candidate',
        sourceType: 'conversation',
        sourceId: conversation.id,
        summary: payload.content.trim().slice(0, 160),
        ownerId: resolvedActor.actorId,
        lastUsedAt: timestamp + 1,
        trustLevel: 'medium',
        lineage: [conversation.id, agentMessage.id],
        createdByMessageId: agentMessage.id,
      }
      const conversationMemory: ConversationMemoryItem = {
        id: `memory-conversation-${timestamp}`,
        conversationId: conversation.id,
        title: 'Conversation memory',
        summary: `Latest retained instruction: ${payload.content.trim()}`,
        source: 'conversation',
        createdAt: timestamp + 1,
        createdByMessageId: agentMessage.id,
      }
      const trace: TraceRecord = {
        id: `trace-generated-${timestamp}`,
        runId: run.id,
        conversationId: conversation.id,
        kind: 'tool',
        title: 'Executed message payload',
        detail: `Processed ${resolvedToolCalls.reduce((total, item) => total + item.count, 0)} tool calls for ${resolvedActor.actorKind}:${resolvedActor.actorId}.`,
        status: 'info',
        timestamp: timestamp + 1,
        actor: resolvedActor.actorId,
        messageId: agentMessage.id,
        toolId: resolvedToolCalls[0]?.toolId,
        createdByMessageId: agentMessage.id,
        relatedArtifactId: artifact.id,
      }

      this.messages.push(userMessage, agentMessage)
      this.artifacts.unshift(artifact)
      this.resources.unshift(resource)
      this.knowledge.unshift(knowledgeEntry)
      this.conversationMemories.unshift(conversationMemory)
      this.traces.unshift(trace)

      if (!conversation.artifactIds.includes(artifact.id)) {
        conversation.artifactIds.unshift(artifact.id)
      }
      if (!project.artifactIds.includes(artifact.id)) {
        project.artifactIds.unshift(artifact.id)
      }
      if (!project.resourceIds.includes(resource.id)) {
        project.resourceIds.unshift(resource.id)
      }

      conversation.intent = ConversationIntent.CLARIFY
      conversation.summary = payload.content.trim()
      conversation.statusNote = resolvedActor.usedDefaultActor
        ? 'runtime.conversation.sentWithDefaultActor'
        : 'runtime.conversation.inputIngested'
      conversation.stageProgress = Math.min(100, conversation.stageProgress + 4)

      run.status = 'running'
      run.currentStep = 'runtime.run.ingestingConstraints'
      run.ownerType = resolvedActor.actorKind
      run.ownerId = resolvedActor.actorId
      run.updatedAt = timestamp
      run.nextAction = 'runtime.run.awaitingFollowUp'

      this.currentRunId = run.id
      this.syncConversationRunSnapshot(conversation.id)
    },
    removeQueuedMessage(queueItemId: string) {
      this.conversationQueue = this.conversationQueue.filter((item) => item.id !== queueItemId)
    },
    drainConversationQueue(conversationId: string) {
      const nextQueueItem = this.conversationQueue
        .filter((item) => item.conversationId === conversationId)
        .sort((left, right) => left.createdAt - right.createdAt)[0]

      if (!nextQueueItem) {
        return
      }

      this.conversationQueue = this.conversationQueue.filter((item) => item.id !== nextQueueItem.id)
      const timestamp = nextConversationTimestamp(this.messages, conversationId)
      this.ingestConversationPayload(
        conversationId,
        {
          content: nextQueueItem.content,
          modelId: nextQueueItem.modelId,
          permissionMode: nextQueueItem.permissionMode,
          actorKind: nextQueueItem.requestedActorKind,
          actorId: nextQueueItem.requestedActorId,
          resourceIds: nextQueueItem.resourceIds,
          attachments: nextQueueItem.attachments,
        },
        {
          actorKind: nextQueueItem.resolvedActorKind,
          actorId: nextQueueItem.resolvedActorId,
          usedDefaultActor: !nextQueueItem.requestedActorKind || !nextQueueItem.requestedActorId,
        },
        timestamp,
      )
    },
    completeActiveRun(status: RunStatus = 'completed') {
      const run = this.activeRun
      if (!run) {
        return
      }

      run.status = status
      run.updatedAt = nextTimestampFrom([run.updatedAt], Date.now())
      if (status === 'completed') {
        run.currentStep = 'runtime.run.completed'
      }

      this.syncConversationRunSnapshot(run.conversationId)
      if (!isRunBlockingForQueue(status)) {
        this.drainConversationQueue(run.conversationId)
      }
    },
    rollbackConversationToMessage(messageId: string) {
      const targetMessage = this.messages.find((item) => item.id === messageId && item.conversationId === this.currentConversationId)
      const conversation = this.activeConversation
      const project = this.activeProject
      const run = this.activeRun
      if (!targetMessage || !conversation || !project || !run) {
        return
      }

      const keptMessages = this.conversationMessages.filter((message) => message.timestamp <= targetMessage.timestamp)
      const removedMessages = this.conversationMessages.filter((message) => message.timestamp > targetMessage.timestamp)
      const removedMessageIds = new Set(removedMessages.map((message) => message.id))

      this.messages = this.messages.filter((message) =>
        message.conversationId !== conversation.id || message.timestamp <= targetMessage.timestamp,
      )
      this.conversationQueue = this.conversationQueue.filter((item) => item.conversationId !== conversation.id)

      const removedArtifactIds = new Set(
        this.artifacts
          .filter((artifact) => artifact.conversationId === conversation.id && removedMessageIds.has(artifact.createdByMessageId ?? ''))
          .map((artifact) => artifact.id),
      )
      this.artifacts = this.artifacts.filter((artifact) => !removedArtifactIds.has(artifact.id))
      conversation.artifactIds = conversation.artifactIds.filter((artifactId) => !removedArtifactIds.has(artifactId))
      project.artifactIds = project.artifactIds.filter((artifactId) => !removedArtifactIds.has(artifactId))

      const removableResourceIds = new Set(
        this.resources
          .filter((resource) => removedMessageIds.has(resource.createdByMessageId ?? '') && resource.linkedConversationIds.includes(conversation.id))
          .map((resource) => resource.id),
      )
      this.resources = this.resources.filter((resource) => !removableResourceIds.has(resource.id))
      project.resourceIds = project.resourceIds.filter((resourceId) => !removableResourceIds.has(resourceId))

      this.knowledge = this.knowledge.filter((entry) =>
        !(entry.conversationId === conversation.id && removedMessageIds.has(entry.createdByMessageId ?? '')),
      )
      this.conversationMemories = this.conversationMemories.filter((memory) =>
        !(memory.conversationId === conversation.id && removedMessageIds.has(memory.createdByMessageId ?? '')),
      )

      const removedTraceIds = new Set(
        this.traces
          .filter((trace) => trace.conversationId === conversation.id && removedMessageIds.has(trace.createdByMessageId ?? ''))
          .map((trace) => trace.id),
      )
      this.traces = this.traces.filter((trace) => !removedTraceIds.has(trace.id))
      this.inbox = this.inbox.filter((item) =>
        !(item.conversationId === conversation.id && (removedTraceIds.has(item.traceId ?? '') || removedArtifactIds.has(item.artifactId ?? ''))),
      )
      conversation.pendingInboxIds = conversation.pendingInboxIds.filter((inboxId) => this.inbox.some((item) => item.id === inboxId))

      const lastRetainedMessage = keptMessages.at(-1)
      conversation.activeTeamId = lastRetainedMessage?.actorKind === 'team' ? lastRetainedMessage.actorId : undefined
      conversation.activeAgentId = lastRetainedMessage?.actorKind === 'agent'
        ? lastRetainedMessage.actorId
        : conversation.activeTeamId
          ? this.teams.find((team) => team.id === conversation.activeTeamId)?.members[0] ?? conversation.activeAgentId
          : conversation.activeAgentId
      conversation.summary = latestUserSummary(keptMessages)
      conversation.statusNote = 'runtime.conversation.rolledBack'
      conversation.intent = keptMessages.length ? ConversationIntent.CLARIFY : ConversationIntent.PLAN
      conversation.stageProgress = Math.max(0, keptMessages.length ? conversation.stageProgress - removedMessages.length * 4 : 0)

      run.status = 'planned'
      run.currentStep = 'runtime.run.rolledBackToCheckpoint'
      run.updatedAt = nextTimestampFrom([run.updatedAt, targetMessage.timestamp], Date.now())
      run.ownerType = conversation.activeTeamId ? 'team' : 'agent'
      run.ownerId = conversation.activeTeamId ?? conversation.activeAgentId ?? run.ownerId
      this.syncConversationRunSnapshot(conversation.id)
    },
    createWorkspace() {
      const sequence = this.workspaces.length + 1
      const timestamp = Date.now()
      const projectId = `proj-mock-${sequence}`
      const conversationId = `conv-mock-${sequence}`
      const workspaceId = `ws-mock-${sequence}`
      const roleId = `role-${workspaceId}-admin`

      const workspace: Workspace = {
        id: workspaceId,
        name: `Workspace ${sequence}`,
        avatar: `W${sequence}`,
        isLocal: true,
        description: `Mock workspace ${sequence} for shell interaction demos.`,
        roleSummary: 'Owner · Mock Runtime',
        memberCount: 0,
        projectIds: [projectId],
      }

      const project: Project = {
        id: projectId,
        workspaceId: workspace.id,
        name: `Project ${sequence}`,
        status: 'active',
        goal: `Track mock project ${sequence} inside the desktop workbench shell.`,
        phase: 'Planning',
        summary: 'Fresh mock project created from the account workspace menu.',
        blockerIds: [],
        recentDecision: 'Created from the topbar account menu.',
        conversationIds: [conversationId],
        artifactIds: [],
        resourceIds: [],
        agentIds: ['agent-architect'],
        teamIds: [],
        defaultActorKind: 'agent',
        defaultActorId: 'agent-architect',
      }

      const run = createMockRun(sequence, project.id, conversationId, `Workspace ${sequence} bootstrap run`, timestamp)
      const conversation = createMockConversation(sequence, project.id, `Starter Conversation ${sequence}`, timestamp, run)

      const connection: ConnectionProfile = {
        id: `conn-mock-${sequence}`,
        mode: 'local',
        label: `Mock Workspace ${sequence}`,
        workspaceId: workspace.id,
        state: 'local-ready',
        lastSyncAt: timestamp,
      }
      const role: RbacRole = {
        id: roleId,
        workspaceId,
        name: 'Workspace Admin',
        code: `${workspaceId}_admin`,
        description: 'Auto-created administrator role for mock workspaces.',
        status: 'active',
        permissionIds: [],
        menuIds: MENU_DEFINITIONS.map((definition) => definition.id),
      }
      const membership: WorkspaceMembership = {
        workspaceId,
        userId: this.currentUserId,
        roleIds: [roleId],
        scopeMode: 'all-projects',
        scopeProjectIds: [],
      }

      this.workspaces.push(workspace)
      this.menus.push(...buildWorkspaceMenuNodes(workspace.id))
      this.roles.push(role)
      this.memberships.push(membership)
      this.projects.push(project)
      this.conversations.push(conversation)
      this.runs.push(run)
      this.connections.push(connection)
      syncWorkspaceMemberCounts(this.workspaces, this.memberships)

      this.currentWorkspaceId = workspace.id
      this.currentProjectId = project.id
      this.currentConversationId = conversation.id
      this.currentRunId = run.id

      return workspace
    },
    createProject(workspaceId?: string, projectName?: string) {
      const targetWorkspaceId = workspaceId ?? this.currentWorkspaceId
      const workspace = this.workspaces.find((item) => item.id === targetWorkspaceId)
      if (!workspace) {
        throw new Error(`Workspace ${targetWorkspaceId} not found`)
      }

      const projectSequence = this.projects.filter((item) => item.id.startsWith('proj-mock-')).length + 1
      const conversationSequence = this.conversations.filter((item) => item.id.startsWith('conv-mock-')).length + 1
      const timestamp = Date.now()
      const projectId = `proj-mock-${projectSequence}`
      const conversationId = `conv-mock-${conversationSequence}`
      const projectTitle = projectName?.trim() || `Project ${projectSequence}`
      const conversationTitle = `Starter Conversation ${conversationSequence}`

      const project: Project = {
        id: projectId,
        workspaceId: workspace.id,
        name: projectTitle,
        status: 'active',
        goal: `Track mock project ${projectSequence} inside the desktop workbench shell.`,
        phase: 'Planning',
        summary: 'Fresh mock project created from the left sidebar controls.',
        blockerIds: [],
        recentDecision: 'Created from the sidebar project controls.',
        conversationIds: [conversationId],
        artifactIds: [],
        resourceIds: [],
        agentIds: ['agent-architect'],
        teamIds: [],
        defaultActorKind: 'agent',
        defaultActorId: 'agent-architect',
      }

      const run = createMockRun(conversationSequence, project.id, conversationId, `${projectTitle} bootstrap run`, timestamp)
      const conversation = createMockConversation(conversationSequence, project.id, conversationTitle, timestamp, run)

      workspace.projectIds.push(project.id)
      this.projects.push(project)
      this.conversations.push(conversation)
      this.runs.push(run)

      this.currentWorkspaceId = workspace.id
      this.currentProjectId = project.id
      this.currentConversationId = conversation.id
      this.currentRunId = run.id

      return project
    },
    updateProjectDetails(projectId: string, patch: UpdateProjectDetailsPatch) {
      const project = this.projects.find((item) => item.id === projectId)
      if (!project) {
        return undefined
      }

      if (patch.name !== undefined) {
        const nextName = patch.name.trim()
        if (nextName) {
          project.name = nextName
        }
      }
      if (patch.goal !== undefined) {
        const nextGoal = patch.goal.trim()
        if (nextGoal) {
          project.goal = nextGoal
        }
      }
      if (patch.phase !== undefined) {
        const nextPhase = patch.phase.trim()
        if (nextPhase) {
          project.phase = nextPhase
        }
      }
      if (patch.summary !== undefined) {
        const nextSummary = patch.summary.trim()
        if (nextSummary) {
          project.summary = nextSummary
        }
      }

      return project
    },
    removeProject(projectId: string) {
      const projectIndex = this.projects.findIndex((item) => item.id === projectId)
      if (projectIndex === -1) {
        return this.currentProjectId || null
      }

      const project = this.projects[projectIndex]
      const workspace = this.workspaces.find((item) => item.id === project.workspaceId)
      if (!workspace) {
        return this.currentProjectId || null
      }

      const workspaceProjectIds = [...workspace.projectIds]
      if (workspaceProjectIds.length <= 1) {
        return null
      }

      const workspaceProjectIndex = workspaceProjectIds.findIndex((item) => item === projectId)
      const remainingWorkspaceProjectIds = workspaceProjectIds.filter((item) => item !== projectId)
      const removedConversationIds = this.conversations
        .filter((conversation) => conversation.projectId === projectId)
        .map((conversation) => conversation.id)
      const removedRunIds = this.runs
        .filter((run) => run.projectId === projectId || removedConversationIds.includes(run.conversationId))
        .map((run) => run.id)

      workspace.projectIds = remainingWorkspaceProjectIds
      this.projects = this.projects.filter((item) => item.id !== projectId)
      this.conversations = this.conversations.filter((item) => item.projectId !== projectId)
      this.messages = this.messages.filter((message) => !removedConversationIds.includes(message.conversationId))
      this.artifacts = this.artifacts.filter((artifact) => artifact.projectId !== projectId)
      this.resources = this.resources.filter((resource) => resource.projectId !== projectId)
      this.runs = this.runs.filter((run) => run.projectId !== projectId && !removedConversationIds.includes(run.conversationId))
      this.traces = this.traces.filter((trace) => !removedRunIds.includes(trace.runId))
      this.knowledge = this.knowledge.filter((entry) => entry.projectId !== projectId)
      this.conversationMemories = this.conversationMemories.filter((memory) => memory.conversationId && !removedConversationIds.includes(memory.conversationId))
      this.conversationQueue = this.conversationQueue.filter((item) => !removedConversationIds.includes(item.conversationId))
      this.inbox = this.inbox.filter((item) => item.projectId !== projectId && !removedConversationIds.includes(item.conversationId ?? ''))
      this.activities = this.activities.filter((activity) => activity.projectId !== projectId)
      this.teams = this.teams.filter((team) => team.projectId !== projectId)
      this.agents = this.agents.filter((agent) => !(agent.isProjectCopy && agent.owner === `project:${projectId}`))

      for (const conversation of this.conversations) {
        conversation.branchLinks = conversation.branchLinks.filter((link) => !removedConversationIds.includes(link.targetConversationId))
      }

      if (this.currentProjectId !== projectId) {
        return this.currentProjectId || null
      }

      const nextProjectId = remainingWorkspaceProjectIds[workspaceProjectIndex]
        ?? remainingWorkspaceProjectIds[workspaceProjectIndex - 1]
        ?? null

      this.currentWorkspaceId = workspace.id
      this.currentProjectId = nextProjectId ?? ''

      if (nextProjectId) {
        const nextProject = this.projects.find((item) => item.id === nextProjectId)
        this.currentConversationId = nextProject?.conversationIds[0] ?? ''
        if (this.currentConversationId) {
          this.selectRunByConversation(this.currentConversationId)
        }
        else {
          this.currentRunId = ''
        }
      }
      else {
        this.currentConversationId = ''
        this.currentRunId = ''
      }

      return nextProjectId
    },
    createConversation(projectId?: string) {
      const targetProjectId = projectId ?? this.currentProjectId
      const project = this.projects.find((item) => item.id === targetProjectId)
      if (!project) {
        throw new Error(`Project ${targetProjectId} not found`)
      }

      const sequence = this.conversations.filter((item) => item.id.startsWith('conv-mock-')).length + 1
      const timestamp = Date.now()
      const conversationTitle = `Starter Conversation ${sequence}`
      const run = createMockRun(sequence, project.id, `conv-mock-${sequence}`, `${conversationTitle} run`, timestamp)
      const conversation = createMockConversation(sequence, project.id, conversationTitle, timestamp, run)

      project.conversationIds.push(conversation.id)
      this.conversations.push(conversation)
      this.runs.push(run)

      this.currentWorkspaceId = project.workspaceId
      this.currentProjectId = project.id
      this.currentConversationId = conversation.id
      this.currentRunId = run.id

      return conversation
    },
    removeConversation(conversationId: string) {
      const conversationIndex = this.conversations.findIndex((item) => item.id === conversationId)
      if (conversationIndex === -1) {
        return this.currentConversationId || null
      }

      const conversation = this.conversations[conversationIndex]
      const project = this.projects.find((item) => item.id === conversation.projectId)
      if (!project) {
        return this.currentConversationId || null
      }

      const projectConversationIds = [...project.conversationIds]
      const projectConversationIndex = projectConversationIds.findIndex((item) => item === conversationId)
      const remainingProjectConversationIds = projectConversationIds.filter((item) => item !== conversationId)
      const removedRunIds = this.runs
        .filter((run) => run.conversationId === conversationId)
        .map((run) => run.id)
      const removedArtifactIds = this.artifacts
        .filter((artifact) => artifact.conversationId === conversationId)
        .map((artifact) => artifact.id)
      const removedResourceIds = this.resources
        .filter((resource) => resource.linkedConversationIds.includes(conversationId))
        .map((resource) => resource.id)

      project.conversationIds = remainingProjectConversationIds
      this.conversations = this.conversations.filter((item) => item.id !== conversationId)
      this.messages = this.messages.filter((message) => message.conversationId !== conversationId)
      this.inbox = this.inbox.filter((item) => item.conversationId !== conversationId)
      this.artifacts = this.artifacts.filter((artifact) => artifact.conversationId !== conversationId)
      this.resources = this.resources.filter((resource) => !resource.linkedConversationIds.includes(conversationId))
      this.runs = this.runs.filter((run) => run.conversationId !== conversationId)
      this.traces = this.traces.filter((trace) => !removedRunIds.includes(trace.runId))
      this.knowledge = this.knowledge.filter((entry) => entry.conversationId !== conversationId)
      this.conversationMemories = this.conversationMemories.filter((memory) => memory.conversationId !== conversationId)
      this.conversationQueue = this.conversationQueue.filter((item) => item.conversationId !== conversationId)
      project.artifactIds = project.artifactIds.filter((artifactId) => !removedArtifactIds.includes(artifactId))
      project.resourceIds = project.resourceIds.filter((resourceId) => !removedResourceIds.includes(resourceId))

      for (const item of this.conversations) {
        item.branchLinks = item.branchLinks.filter((link) => link.targetConversationId !== conversationId)
      }

      if (this.currentConversationId !== conversationId) {
        return this.currentConversationId || null
      }

      const nextConversationId = remainingProjectConversationIds[projectConversationIndex]
        ?? remainingProjectConversationIds[projectConversationIndex - 1]
        ?? null

      this.currentWorkspaceId = project.workspaceId
      this.currentProjectId = project.id
      this.currentConversationId = nextConversationId ?? ''

      if (nextConversationId) {
        this.selectRunByConversation(nextConversationId)
      }
      else {
        this.currentRunId = ''
      }

      return nextConversationId
    },
    removeWorkspace(workspaceId: string) {
      if (this.workspaces.length <= 1) {
        return null
      }

      const workspaceIndex = this.workspaces.findIndex((item) => item.id === workspaceId)
      if (workspaceIndex === -1) {
        return this.currentWorkspaceId
      }

      const removedWorkspace = this.workspaces[workspaceIndex]
      const removedProjectIds = this.projects.filter((project) => project.workspaceId === removedWorkspace.id).map((project) => project.id)
      const removedConversationIds = this.conversations
        .filter((conversation) => removedProjectIds.includes(conversation.projectId))
        .map((conversation) => conversation.id)
      const removedRunIds = this.runs
        .filter((run) => removedConversationIds.includes(run.conversationId) || removedProjectIds.includes(run.projectId))
        .map((run) => run.id)

      this.workspaces = this.workspaces.filter((workspace) => workspace.id !== removedWorkspace.id)
      this.projects = this.projects.filter((project) => project.workspaceId !== removedWorkspace.id)
      this.conversations = this.conversations.filter((conversation) => !removedConversationIds.includes(conversation.id))
      this.messages = this.messages.filter((message) => !removedConversationIds.includes(message.conversationId))
      this.artifacts = this.artifacts.filter((artifact) => !removedProjectIds.includes(artifact.projectId))
      this.resources = this.resources.filter((resource) => !removedProjectIds.includes(resource.projectId))
      this.runs = this.runs.filter((run) => !removedRunIds.includes(run.id))
      this.traces = this.traces.filter((trace) => !removedRunIds.includes(trace.runId))
      this.knowledge = this.knowledge.filter((entry) => entry.workspaceId !== removedWorkspace.id)
      this.conversationMemories = this.conversationMemories.filter((memory) => !removedConversationIds.includes(memory.conversationId))
      this.conversationQueue = this.conversationQueue.filter((item) => !removedConversationIds.includes(item.conversationId))
      this.inbox = this.inbox.filter((item) => item.workspaceId !== removedWorkspace.id)
      this.activities = this.activities.filter((activity) => activity.workspaceId !== removedWorkspace.id)
      this.automations = this.automations.filter((automation) => automation.workspaceId !== removedWorkspace.id)
      this.teams = this.teams.filter((team) => team.workspaceId !== removedWorkspace.id)
      this.connections = this.connections.filter((connection) => connection.workspaceId !== removedWorkspace.id)
      this.roles = this.roles.filter((role) => role.workspaceId !== removedWorkspace.id)
      this.permissions = this.permissions.filter((permission) => permission.workspaceId !== removedWorkspace.id)
      this.memberships = this.memberships.filter((membership) => membership.workspaceId !== removedWorkspace.id)
      this.menus = this.menus.filter((menu) => menu.workspaceId !== removedWorkspace.id)
      syncWorkspaceMemberCounts(this.workspaces, this.memberships)

      const targetWorkspace = this.workspaces[
        Math.min(workspaceIndex, this.workspaces.length - 1)
      ]

      if (!targetWorkspace) {
        return null
      }

      if (this.currentWorkspaceId === removedWorkspace.id || !this.workspaces.some((workspace) => workspace.id === this.currentWorkspaceId)) {
        this.selectWorkspace(targetWorkspace.id)
      }

      return this.currentWorkspaceId
    },
    sendMessage(payload: ConversationComposerDraft) {
      const trimmed = payload.content.trim()
      if (!trimmed) {
        return
      }

      const conversation = this.conversations.find((item) => item.id === this.currentConversationId)
      const project = this.activeProject
      if (!conversation || !project) {
        return
      }

      const resolvedActor = resolveConversationActorTarget(
        project,
        conversation,
        this.agents,
        this.teams,
        payload.actorKind,
        payload.actorId,
      )
      if (!resolvedActor) {
        return
      }

      const activeRun = this.activeRun ?? this.runs.find((item) => item.conversationId === conversation.id)
      if (isConversationBusy(activeRun?.status)) {
        const queueTimestamp = nextTimestampFrom(
          [
            ...this.conversationQueue
              .filter((item) => item.conversationId === conversation.id)
              .map((item) => item.createdAt),
            ...this.messages
              .filter((message) => message.conversationId === conversation.id)
              .map((message) => message.timestamp),
            activeRun?.updatedAt ?? 0,
          ],
          Date.now(),
        )
        this.conversationQueue.push({
          id: `queue-${queueTimestamp}`,
          conversationId: conversation.id,
          content: trimmed,
          modelId: payload.modelId,
          permissionMode: payload.permissionMode,
          requestedActorKind: payload.actorKind,
          requestedActorId: payload.actorId,
          resolvedActorKind: resolvedActor.actorKind,
          resolvedActorId: resolvedActor.actorId,
          resourceIds: [...payload.resourceIds],
          attachments: payload.attachments.map((attachment) => ({ ...attachment })),
          createdAt: queueTimestamp,
        })
        conversation.statusNote = 'runtime.conversation.messageQueued'
        this.syncConversationRunSnapshot(conversation.id)
        return
      }

      this.ingestConversationPayload(
        conversation.id,
        {
          ...payload,
          content: trimmed,
        },
        resolvedActor,
        nextConversationTimestamp(this.messages, conversation.id),
      )
      this.syncConversationRunSnapshot(conversation.id)
    },
    createProjectResource(kind: 'file' | 'folder' | 'url', options: CreateProjectResourceOptions = {}) {
      const project = this.activeProject
      if (!project) {
        throw new Error('No active project selected')
      }

      const sequence = this.resources.filter((resource) => resource.kind === kind).length + 1
      const trimmedName = options.name?.trim()
      const trimmedLocation = options.location?.trim()
      const createdAt = nextTimestampFrom(
        this.resources
          .filter((resource) => resource.projectId === project.id)
          .map((resource) => resource.createdAt),
        Date.now(),
      )
      const resource: ProjectResource = {
        id: `res-${kind}-mock-${sequence}`,
        projectId: project.id,
        workspaceId: project.workspaceId,
        name: trimmedName || (
          kind === 'file'
            ? `Mock File ${sequence}.md`
            : kind === 'folder'
              ? `Mock Folder ${sequence}`
              : `Mock URL ${sequence}`
        ),
        kind,
        linkedConversationIds: this.currentConversationId ? [this.currentConversationId] : [],
        createdAt,
        sizeLabel: kind === 'file' ? '16 KB' : kind === 'folder' ? '3 items' : 'URL',
        location: trimmedLocation || (
          kind === 'file'
            ? `/mock/${project.id}/mock-file-${sequence}.md`
            : kind === 'folder'
              ? `/mock/${project.id}/mock-folder-${sequence}`
              : `https://example.com/resource-${sequence}`
        ),
        tags: kind === 'url' ? ['mock-upload', 'link'] : ['mock-upload'],
      }

      this.resources.unshift(resource)
      project.resourceIds.unshift(resource.id)

      return resource
    },
    updateProjectResource(resourceId: string, patch: UpdateProjectResourcePatch) {
      const resource = this.resources.find((item) => item.id === resourceId)
      if (!resource || resource.kind === 'artifact') {
        return
      }

      const nextName = patch.name?.trim()
      if (nextName) {
        resource.name = nextName
      }

      if (resource.kind === 'url') {
        const nextLocation = patch.location?.trim()
        if (nextLocation) {
          resource.location = nextLocation
        }
      }
    },
    removeProjectResource(resourceId: string) {
      const project = this.projects.find((item) =>
        item.resourceIds.includes(resourceId) || item.artifactIds.includes(resourceId),
      )
      const artifact = this.artifacts.find((item) => item.id === resourceId)

      if (artifact) {
        this.artifacts = this.artifacts.filter((item) => item.id !== resourceId)
        this.projects.forEach((item) => {
          item.artifactIds = item.artifactIds.filter((artifactId) => artifactId !== resourceId)
          item.resourceIds = item.resourceIds.filter((resourceItemId) => resourceItemId !== resourceId)
        })
        this.conversations.forEach((conversation) => {
          conversation.artifactIds = conversation.artifactIds.filter((artifactId) => artifactId !== resourceId)
        })
      }
      else {
        this.resources = this.resources.filter((item) => item.id !== resourceId)
        if (project) {
          project.resourceIds = project.resourceIds.filter((item) => item !== resourceId)
        }
      }

      this.messages = this.messages.map((message) => ({
        ...message,
        resourceIds: message.resourceIds?.filter((item) => item !== resourceId),
        artifacts: message.artifacts?.filter((item) => item !== resourceId),
        attachments: message.attachments?.filter((attachment) => attachment.id !== resourceId),
      }))

      this.conversationQueue = this.conversationQueue.map((item) => ({
        ...item,
        resourceIds: item.resourceIds.filter((resourceItemId) => resourceItemId !== resourceId),
        attachments: item.attachments.filter((attachment) => attachment.id !== resourceId),
      }))
    },
    exportAgentAsset(kind: AgentAssetKind, id: string): string {
      const entity = kind === 'agent'
        ? this.agents.find((item) => item.id === id)
        : this.teams.find((item) => item.id === id)

      return JSON.stringify({
        kind,
        exportedAt: Date.now(),
        entity: entity ? cloneSeed(entity) : null,
      }, null, 2)
    },
    requestArtifactReview(artifactId: string) {
      const artifact = this.artifacts.find((item) => item.id === artifactId)
      const conversation = this.conversations.find((item) => item.id === this.currentConversationId)
      if (!artifact || !conversation) {
        return
      }

      const artifactUpdatedAt = nextTimestampFrom([artifact.updatedAt], Date.now())
      artifact.status = 'review'
      artifact.version += 1
      artifact.updatedAt = artifactUpdatedAt
      conversation.intent = ConversationIntent.REVIEW
      conversation.statusNote = 'runtime.conversation.reviewRequested'

      const inboxId = `inbox-review-${artifactId}`
      const existing = this.inbox.find((item) => item.id === inboxId)
      if (!existing) {
        const inboxCreatedAt = nextTimestampFrom(
          this.inbox
            .filter((item) => item.conversationId === conversation.id)
            .map((item) => item.createdAt),
          artifactUpdatedAt,
        )
        this.inbox.unshift({
          id: inboxId,
          workspaceId: this.currentWorkspaceId,
          projectId: this.currentProjectId,
          type: 'knowledge_promotion_approval',
          title: 'runtime.inbox.reviewArtifactTitle',
          description: 'runtime.inbox.reviewArtifactDescription',
          relatedId: artifact.id,
          status: 'pending',
          priority: 'medium',
          createdAt: inboxCreatedAt,
          impact: 'runtime.inbox.reviewArtifactImpact',
          riskNote: 'runtime.inbox.reviewArtifactRisk',
          recommendedAction: 'runtime.inbox.reviewArtifactAction',
          conversationId: conversation.id,
          artifactId: artifact.id,
        })
        if (!conversation.pendingInboxIds.includes(inboxId)) {
          conversation.pendingInboxIds.push(inboxId)
        }
      }
    },
    updateArtifactContent(artifactId: string, content: string) {
      const artifact = this.artifacts.find((item) => item.id === artifactId)
      if (!artifact) {
        return
      }

      artifact.content = content.trim()
      artifact.excerpt = content.trim().slice(0, 140)
      artifact.updatedAt = nextTimestampFrom([artifact.updatedAt], Date.now())
    },
    pauseConversation() {
      const conversation = this.activeConversation
      const run = this.activeRun
      if (!conversation || !run) {
        return
      }

      conversation.intent = ConversationIntent.PAUSED
      conversation.statusNote = 'runtime.conversation.paused'
      run.status = 'paused'
      run.currentStep = 'runtime.run.pausedByUser'
      run.updatedAt = nextTimestampFrom([run.updatedAt], Date.now())
    },
    resumeConversation() {
      const conversation = this.activeConversation
      const run = this.activeRun
      if (!conversation || !run) {
        return
      }

      conversation.intent = ConversationIntent.EXECUTE
      conversation.statusNote = 'runtime.conversation.resumed'
      run.status = 'running'
      run.currentStep = 'runtime.run.resumedFromCheckpoint'
      run.updatedAt = nextTimestampFrom([run.updatedAt], Date.now())
    },
    resolveInboxItem(inboxId: string, decision: DecisionAction) {
      const inboxItem = this.inbox.find((item) => item.id === inboxId)
      if (!inboxItem) {
        return
      }

      inboxItem.status = decision === 'approve' ? 'resolved' : 'dismissed'

      const linkedConversation = inboxItem.conversationId
        ? this.conversations.find((item) => item.id === inboxItem.conversationId)
        : this.activeConversation
      const linkedRun = inboxItem.relatedId
        ? this.runs.find((item) => item.id === inboxItem.relatedId)
        : this.activeRun

      if (linkedConversation) {
        linkedConversation.pendingInboxIds = linkedConversation.pendingInboxIds.filter((item) => item !== inboxId)
      }

      if (decision === 'approve') {
        if (linkedConversation) {
          linkedConversation.intent = ConversationIntent.EXECUTE
          linkedConversation.statusNote = 'runtime.conversation.approved'
        }
        if (linkedRun) {
          linkedRun.status = 'running'
          linkedRun.currentStep = 'runtime.run.approvalReceived'
          linkedRun.updatedAt = nextTimestampFrom([linkedRun.updatedAt], Date.now())
          this.currentRunId = linkedRun.id
        }
      } else {
        if (linkedConversation) {
          linkedConversation.intent = ConversationIntent.BLOCKED
          linkedConversation.statusNote = 'runtime.conversation.rejected'
        }
        if (linkedRun) {
          linkedRun.status = 'blocked'
          linkedRun.currentStep = 'runtime.run.reroutedAfterRejection'
          linkedRun.updatedAt = nextTimestampFrom([linkedRun.updatedAt], Date.now())
          this.currentRunId = linkedRun.id
        }
      }
    },
    updateAgent(agentId: string, patch: Partial<Agent>) {
      const agentIndex = this.agents.findIndex((item) => item.id === agentId)
      if (agentIndex === -1) {
        return
      }

      this.agents[agentIndex] = {
        ...this.agents[agentIndex],
        ...patch,
      }
    },
    createAgent(scope: 'workspace' | 'project') {
      const sequence = this.agents.filter((agent) => agent.id.startsWith('agent-mock-')).length + 1
      const isProjectScoped = scope === 'project'
      const agent: Agent = {
        id: `agent-mock-${sequence}`,
        name: `Mock Agent ${sequence}`,
        avatar: `M${sequence}`,
        role: isProjectScoped ? 'Project Specialist' : 'Workspace Specialist',
        summary: isProjectScoped
          ? '项目级智能体，可在当前项目内独立调整能力与知识范围。'
          : '工作空间级智能体，可被多个项目引用复用。',
        persona: ['结构化', '稳健', '协作'],
        skillTags: isProjectScoped ? ['项目执行', '上下文约束'] : ['跨项目复用', '能力模板'],
        mcpBindings: [],
        systemPrompt: isProjectScoped
          ? '聚焦当前项目目标，严格遵守项目上下文与权限边界。'
          : '作为工作空间通用智能体，提供稳定的一致性能力。',
        capabilityPolicy: {
          model: 'gpt-5.4',
          tools: ['read', 'search'],
          externalBindings: [],
          environment: [isProjectScoped ? 'project-sandbox' : 'workspace-readonly'],
          approvalRequired: [],
          forbiddenActions: [],
          defaultResultFormat: 'markdown',
          riskLevel: 'low',
        },
        knowledgeScope: {
          privateMemories: [],
          sharedSources: isProjectScoped ? ['project-brief'] : ['workspace-handbook'],
          accessibleProjects: isProjectScoped && this.currentProjectId ? [this.currentProjectId] : [],
        },
        executionProfile: {
          planningStyle: 'structured',
          verificationStyle: 'checklist',
          autonomyLevel: isProjectScoped ? 'guided' : 'review-first',
          interruptPreference: 'confirm-major-actions',
        },
        approvalPreferences: [],
        scope,
        owner: isProjectScoped ? `project:${this.currentProjectId}` : `workspace:${this.currentWorkspaceId}`,
        status: 'active',
        isProjectCopy: false,
      }

      this.agents.unshift(agent)

      if (isProjectScoped) {
        const project = this.activeProject
        if (project && !project.agentIds.includes(agent.id)) {
          project.agentIds.unshift(agent.id)
        }
      }

      return agent
    },
    createProjectAgentCopy(agentId: string) {
      const source = this.agents.find((item) => item.id === agentId)
      if (!source) {
        return
      }

      const copyId = `${source.id}-copy-${this.currentProjectId}`
      if (this.agents.some((agent) => agent.id === copyId)) {
        return this.agents.find((agent) => agent.id === copyId)
      }

      const copy: Agent = {
        ...cloneSeed(source),
        id: copyId,
        name: `${resolveMockField('agent', source.id, 'name', source.name)} · ${translate('runtime.copy.projectSuffix')}`,
        scope: 'project',
        owner: `project:${this.currentProjectId}`,
        isProjectCopy: true,
        sourceAgentId: source.id,
      }

      this.agents.unshift(copy)
      const project = this.activeProject
      if (project && !project.agentIds.includes(copyId)) {
        project.agentIds.unshift(copyId)
      }

      return copy
    },
    removeProjectAgentReference(agentId: string) {
      const project = this.activeProject
      if (!project) {
        return
      }

      project.agentIds = project.agentIds.filter((id) => id !== agentId)
    },
    deleteAgent(agentId: string) {
      this.agents = this.agents.filter((item) => item.id !== agentId)

      for (const project of this.projects) {
        project.agentIds = project.agentIds.filter((id) => id !== agentId)
      }

      for (const team of this.teams) {
        team.members = team.members.filter((memberId) => memberId !== agentId)
        const removedNodeIds = new Set(
          team.structureNodes
            .filter((node) => node.memberId === agentId)
            .map((node) => node.id),
        )
        team.structureNodes = team.structureNodes.filter((node) => node.memberId !== agentId)
        team.structureEdges = team.structureEdges.filter((edge) =>
          !removedNodeIds.has(edge.source) && !removedNodeIds.has(edge.target),
        )
      }

      for (const conversation of this.conversations) {
        if (conversation.activeAgentId === agentId) {
          conversation.activeAgentId = undefined
        }
      }
    },
    updateTeam(teamId: string, patch: Partial<Team>) {
      const teamIndex = this.teams.findIndex((item) => item.id === teamId)
      if (teamIndex === -1) {
        return
      }

      this.teams[teamIndex] = {
        ...this.teams[teamIndex],
        ...patch,
      }
    },
    createTeam(scope: 'workspace' | 'project') {
      const sequence = this.teams.filter((team) => team.id.startsWith('team-mock-')).length + 1
      const isProjectScoped = scope === 'project'
      const memberPool = isProjectScoped ? this.projectAgents : this.workspaceLevelAgents
      const members = memberPool.slice(0, 2).map((agent) => agent.id)
      const team: Team = {
        id: `team-mock-${sequence}`,
        workspaceId: this.currentWorkspaceId,
        projectId: isProjectScoped ? this.currentProjectId : undefined,
        name: `Mock Team ${sequence}`,
        description: isProjectScoped
          ? '项目级团队智能体，面向当前项目的分工与协作。'
          : '工作空间级团队智能体，可在多个项目中被引用。',
        summary: isProjectScoped
          ? '聚焦当前项目的团队智能体，支持本项目独立配置。'
          : '面向工作空间复用的团队智能体。',
        avatar: `T${sequence}`,
        mode: TeamMode.HYBRID,
        members,
        skillTags: isProjectScoped ? ['项目协作', '执行编排'] : ['跨项目协作', '组织协同'],
        mcpBindings: [],
        defaultKnowledgeScope: isProjectScoped ? ['project-brief'] : ['workspace-handbook'],
        defaultOutput: 'Team brief + next actions',
        useScope: scope,
        projectNotes: isProjectScoped ? '新建项目团队智能体。' : '新建工作空间团队智能体。',
        approvalPreferences: [],
        structureMode: 'flow',
        ...buildTeamStructure(`team-mock-${sequence}`, members),
        status: 'active',
        isProjectCopy: false,
      }

      this.teams.unshift(team)

      if (isProjectScoped) {
        const project = this.activeProject
        if (project && !project.teamIds.includes(team.id)) {
          project.teamIds.unshift(team.id)
        }
      }

      return team
    },
    createProjectTeamCopy(teamId: string) {
      const source = this.teams.find((item) => item.id === teamId)
      if (!source) {
        return
      }

      const copyId = `${teamId}-copy-${this.currentProjectId}`
      if (this.teams.some((team) => team.id === copyId)) {
        return
      }

      this.teams.unshift({
        ...cloneSeed(source),
        id: copyId,
        name: `${resolveMockField('team', source.id, 'name', source.name)} · ${translate('runtime.copy.projectSuffix')}`,
        workspaceId: this.currentWorkspaceId,
        projectId: this.currentProjectId,
        useScope: 'project',
        isProjectCopy: true,
        sourceTeamId: source.id,
      })
      const project = this.activeProject
      if (project && !project.teamIds.includes(copyId)) {
        project.teamIds.unshift(copyId)
      }
    },
    removeProjectTeamReference(teamId: string) {
      const project = this.activeProject
      if (!project) {
        return
      }

      project.teamIds = project.teamIds.filter((id) => id !== teamId)
    },
    deleteTeam(teamId: string) {
      this.teams = this.teams.filter((item) => item.id !== teamId)

      for (const project of this.projects) {
        project.teamIds = project.teamIds.filter((id) => id !== teamId)
      }

      for (const conversation of this.conversations) {
        if (conversation.activeTeamId === teamId) {
          conversation.activeTeamId = undefined
        }
      }
    },
  },
})
