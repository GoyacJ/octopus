import type {
  AgentRecord,
  CreateProjectRequest,
  ProjectRecord,
  TeamRecord,
  UpdateProjectRequest,
  UpsertAgentInput,
  UpsertTeamInput,
  WorkspaceOverviewSnapshot,
} from '@octopus/schema'

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function avatarPreviewFromPayload(input?: { contentType: string, dataBase64: string } | null) {
  if (!input?.contentType || !input.dataBase64) {
    return undefined
  }
  return `data:${input.contentType};base64,${input.dataBase64}`
}

function summarizePrompt(prompt: string, fallback: string) {
  const trimmed = prompt.trim()
  if (!trimmed) {
    return fallback
  }
  return trimmed.length > 120 ? `${trimmed.slice(0, 117)}...` : trimmed
}

function createProjectId(name: string) {
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
  return `proj-${slug || Date.now()}`
}

export function normalizeAgentRecord(
  input: UpsertAgentInput,
  current: AgentRecord | undefined,
  id: string,
): AgentRecord {
  const nextAvatar = input.removeAvatar
    ? undefined
    : avatarPreviewFromPayload(input.avatar) ?? current?.avatar

  return {
    id,
    workspaceId: input.workspaceId,
    projectId: input.projectId,
    scope: input.scope,
    name: input.name,
    avatarPath: nextAvatar ? current?.avatarPath ?? `data/blobs/avatars/${id}.png` : undefined,
    avatar: nextAvatar,
    personality: input.personality,
    tags: [...input.tags],
    prompt: input.prompt,
    builtinToolKeys: [...input.builtinToolKeys],
    skillIds: [...input.skillIds],
    mcpServerNames: [...input.mcpServerNames],
    title: input.title,
    description: input.description || summarizePrompt(input.prompt, input.personality || input.title),
    status: input.status,
    updatedAt: Date.now(),
  }
}

export function normalizeTeamRecord(
  input: UpsertTeamInput,
  current: TeamRecord | undefined,
  id: string,
): TeamRecord {
  const nextAvatar = input.removeAvatar
    ? undefined
    : avatarPreviewFromPayload(input.avatar) ?? current?.avatar

  return {
    id,
    workspaceId: input.workspaceId,
    projectId: input.projectId,
    scope: input.scope,
    name: input.name,
    avatarPath: nextAvatar ? current?.avatarPath ?? `data/blobs/avatars/${id}.png` : undefined,
    avatar: nextAvatar,
    personality: input.personality,
    tags: [...input.tags],
    prompt: input.prompt,
    builtinToolKeys: [...input.builtinToolKeys],
    skillIds: [...input.skillIds],
    mcpServerNames: [...input.mcpServerNames],
    leaderAgentId: input.leaderAgentId,
    memberAgentIds: [...input.memberAgentIds],
    description: input.description || summarizePrompt(input.prompt, input.personality || input.name),
    status: input.status,
    updatedAt: Date.now(),
  }
}

export function syncWorkspaceProjectState(workspaceState: {
  overview: WorkspaceOverviewSnapshot
  workspace: WorkspaceOverviewSnapshot['workspace']
  projects: ProjectRecord[]
}) {
  workspaceState.overview = {
    ...workspaceState.overview,
    workspace: clone(workspaceState.workspace),
    projects: clone(workspaceState.projects),
    metrics: workspaceState.overview.metrics.map(metric =>
      metric.id === 'projects'
        ? { ...metric, value: String(workspaceState.projects.length) }
        : metric),
  }
}

export function updateDefaultProjectIfNeeded(workspaceState: {
  workspace: WorkspaceOverviewSnapshot['workspace']
  projects: ProjectRecord[]
}, archivedProjectId?: string) {
  const activeProjects = workspaceState.projects.filter(project => project.status === 'active')
  if (archivedProjectId && workspaceState.workspace.defaultProjectId !== archivedProjectId) {
    return
  }

  if (activeProjects[0]) {
    workspaceState.workspace = {
      ...workspaceState.workspace,
      defaultProjectId: activeProjects[0].id,
    }
  }
}

export function createProjectRecord(
  workspaceId: string,
  input: CreateProjectRequest,
): ProjectRecord {
  return {
    id: createProjectId(input.name),
    workspaceId,
    name: input.name.trim(),
    status: 'active',
    description: input.description.trim(),
    resourceDirectory: input.resourceDirectory.trim(),
    ownerUserId: input.ownerUserId?.trim() || 'user-owner',
    memberUserIds: [...new Set([input.ownerUserId?.trim() || 'user-owner', ...(input.memberUserIds ?? [])].filter(Boolean))],
    permissionOverrides: input.permissionOverrides ?? {
      agents: 'inherit',
      resources: 'inherit',
      tools: 'inherit',
      knowledge: 'inherit',
    },
    linkedWorkspaceAssets: input.linkedWorkspaceAssets ?? {
      agentIds: [],
      resourceIds: [],
      toolSourceKeys: [],
      knowledgeIds: [],
    },
    assignments: input.assignments ? clone(input.assignments) : undefined,
  }
}

export function updateProjectRecord(
  current: ProjectRecord,
  input: UpdateProjectRequest,
): ProjectRecord {
  return {
    ...current,
    name: input.name.trim(),
    description: input.description.trim(),
    resourceDirectory: input.resourceDirectory.trim(),
    status: input.status,
    ownerUserId: input.ownerUserId?.trim() || current.ownerUserId,
    memberUserIds: [...new Set(
      [
        input.ownerUserId?.trim() || current.ownerUserId,
        ...(input.memberUserIds ?? current.memberUserIds ?? []),
      ].filter(Boolean),
    )],
    permissionOverrides: input.permissionOverrides ? clone(input.permissionOverrides) : current.permissionOverrides,
    linkedWorkspaceAssets: input.linkedWorkspaceAssets ? clone(input.linkedWorkspaceAssets) : current.linkedWorkspaceAssets,
    assignments: input.assignments ? clone(input.assignments) : undefined,
  }
}
