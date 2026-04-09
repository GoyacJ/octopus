import { vi } from 'vitest'

import type {
  AgentRecord,
  AutomationRecord,
  ChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse,
  CopyWorkspaceSkillToManagedInput,
  CreateWorkspaceSkillInput,
  CreateProjectRequest,
  CreateWorkspaceUserRequest,
  CredentialBinding,
  ImportWorkspaceSkillArchiveInput,
  ImportWorkspaceSkillFolderInput,
  JsonValue,
  KnowledgeRecord,
  LoginResponse,
  MenuRecord,
  ModelCatalogSnapshot,
  PermissionRecord,
  ProjectAgentLinkInput,
  ProjectAgentLinkRecord,
  ProjectDashboardSnapshot,
  ProjectRecord,
  ProjectTeamLinkInput,
  ProjectTeamLinkRecord,
  RegisterWorkspaceOwnerRequest,
  RegisterWorkspaceOwnerResponse,
  RoleRecord,
  BindPetConversationInput,
  PetConversationBinding,
  PetMessage,
  PetPresenceState,
  PetProfile,
  PetWorkspaceSnapshot,
  SavePetPresenceInput,
  RuntimeApprovalRequest,
  RuntimeBootstrap,
  RuntimeConfigPatch,
  RuntimeConfigValidationResult,
  RuntimeEventEnvelope,
  RuntimeEffectiveConfig,
  RuntimeMessage,
  ArtifactRecord,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  RuntimeTraceItem,
  ShellBootstrap,
  SystemBootstrapStatus,
  TeamRecord,
  ToolRecord,
  UpdateCurrentUserProfileRequest,
  UpdateProjectRequest,
  UpsertAgentInput,
  UpsertTeamInput,
  UpdateWorkspaceUserRequest,
  UpdateWorkspaceSkillFileInput,
  UpdateWorkspaceSkillInput,
  UpsertWorkspaceMcpServerInput,
  UserCenterOverviewSnapshot,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceDirectoryUploadEntry,
  WorkspaceMcpServerDocument,
  WorkspaceOverviewSnapshot,
  WorkspaceResourceRecord,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
  WorkspaceSkillTreeNode,
  WorkspaceToolCatalogEntry,
  WorkspaceToolCatalogSnapshot,
  WorkspaceToolDisablePatch,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'
import { resolveRuntimePermissionMode } from '@octopus/schema'

import type { WorkspaceClient } from '@/tauri/workspace-client'
import { WorkspaceApiError } from '@/tauri/shared'
import * as tauriClient from '@/tauri/client'

interface FixtureOptions {
  preloadConversationMessages?: boolean
  localRuntimeConfigTransform?: (config: RuntimeEffectiveConfig) => RuntimeEffectiveConfig
  localOwnerReady?: boolean
  localSetupRequired?: boolean
  preloadWorkspaceSessions?: boolean
  localSessionValid?: boolean
}

interface RuntimeSessionState {
  detail: RuntimeSessionDetail
  events: RuntimeEventEnvelope[]
  nextSequence: number
}

interface WorkspaceFixtureState {
  systemBootstrap: SystemBootstrapStatus
  workspace: WorkspaceOverviewSnapshot['workspace']
  overview: WorkspaceOverviewSnapshot
  projects: ProjectRecord[]
  dashboards: Record<string, ProjectDashboardSnapshot>
  workspaceResources: WorkspaceResourceRecord[]
  projectResources: Record<string, WorkspaceResourceRecord[]>
  artifacts: ArtifactRecord[]
  workspaceKnowledge: KnowledgeRecord[]
  projectKnowledge: Record<string, KnowledgeRecord[]>
  agents: AgentRecord[]
  projectAgentLinks: Record<string, ProjectAgentLinkRecord[]>
  teams: TeamRecord[]
  projectTeamLinks: Record<string, ProjectTeamLinkRecord[]>
  catalog: ModelCatalogSnapshot
  toolCatalog: WorkspaceToolCatalogSnapshot
  skillDocuments: Record<string, WorkspaceSkillDocument>
  skillFiles: Record<string, Record<string, WorkspaceSkillFileDocument>>
  mcpDocuments: Record<string, WorkspaceMcpServerDocument>
  tools: ToolRecord[]
  automations: AutomationRecord[]
  userCenterOverview: UserCenterOverviewSnapshot
  users: UserRecordSummary[]
  userPasswords: Record<string, string>
  roles: RoleRecord[]
  permissions: PermissionRecord[]
  menus: MenuRecord[]
  runtimeSessions: Map<string, RuntimeSessionState>
  runtimeWorkspaceConfig: RuntimeEffectiveConfig
  runtimeProjectConfigs: Record<string, RuntimeEffectiveConfig>
  runtimeUserConfig: RuntimeEffectiveConfig
  petProfile: PetProfile
  workspacePetPresence: PetPresenceState
  projectPetPresences: Record<string, PetPresenceState>
  workspacePetBinding?: PetConversationBinding
  projectPetBindings: Record<string, PetConversationBinding>
}

const WORKSPACE_CONNECTIONS: WorkspaceConnectionRecord[] = [
  {
    workspaceConnectionId: 'conn-local',
    workspaceId: 'ws-local',
    label: 'Local Workspace',
    baseUrl: 'http://127.0.0.1:43127',
    transportSecurity: 'loopback',
    authMode: 'session-token',
    status: 'connected',
  },
  {
    workspaceConnectionId: 'conn-enterprise',
    workspaceId: 'ws-enterprise',
    label: 'Enterprise Workspace',
    baseUrl: 'https://enterprise.example.test',
    transportSecurity: 'trusted',
    authMode: 'session-token',
    status: 'connected',
  },
]

const WORKSPACE_SESSIONS: WorkspaceSessionTokenEnvelope[] = WORKSPACE_CONNECTIONS.map(connection => ({
  workspaceConnectionId: connection.workspaceConnectionId,
  token: `token-${connection.workspaceId}`,
  issuedAt: 1,
  session: {
    id: `sess-${connection.workspaceId}`,
    workspaceId: connection.workspaceId,
    userId: 'user-owner',
    clientAppId: 'octopus-desktop',
    token: `token-${connection.workspaceId}`,
    status: 'active',
    createdAt: 1,
    expiresAt: undefined,
    roleIds: ['role-owner'],
    scopeProjectIds: [],
  },
}))

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

function normalizeAgentRecord(
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

function normalizeTeamRecord(
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

function createManagementCapabilities(canDisable: boolean, canEdit: boolean, canDelete: boolean) {
  return {
    canDisable,
    canEdit,
    canDelete,
  }
}

function skillSlugFromRelativePath(relativePath?: string): string {
  if (!relativePath) {
    return ''
  }
  const match = relativePath.match(/^(?:data\/skills|\.codex\/skills|\.claude\/skills)\/([^/]+)\/SKILL\.md$/)
  return match?.[1] ?? ''
}

function skillNameFromContent(content: string, fallback: string): string {
  const nameLine = content
    .split(/\r?\n/)
    .find(line => line.trimStart().startsWith('name:'))
  return nameLine?.split(':').slice(1).join(':').trim() || fallback
}

function skillDescriptionFromContent(content: string, fallback: string): string {
  const descriptionLine = content
    .split(/\r?\n/)
    .find(line => line.trimStart().startsWith('description:'))
  return descriptionLine?.split(':').slice(1).join(':').trim() || fallback
}

function createSkillTemplate(slug: string) {
  return [
    '---',
    `name: ${slug}`,
    `description: Workspace skill ${slug}.`,
    '---',
    '',
    '# Overview',
  ].join('\n')
}

function createSkillTree(paths: string[]): WorkspaceSkillTreeNode[] {
  const root: WorkspaceSkillTreeNode[] = []

  for (const path of paths) {
    const segments = path.split('/').filter(Boolean)
    let currentNodes = root
    let currentPath = ''

    for (const [index, segment] of segments.entries()) {
      currentPath = currentPath ? `${currentPath}/${segment}` : segment
      const isFile = index === segments.length - 1
      let node = currentNodes.find(item => item.name === segment)
      if (!node) {
        node = isFile
          ? {
              path: currentPath,
              name: segment,
              kind: 'file',
              byteSize: 0,
              isText: true,
            }
          : {
              path: currentPath,
              name: segment,
              kind: 'directory',
              children: [],
            }
        currentNodes.push(node)
      }

      if (!isFile) {
        node.children ||= []
        currentNodes = node.children
      }
    }
  }

  const sortNodes = (nodes: WorkspaceSkillTreeNode[]): WorkspaceSkillTreeNode[] => nodes
    .map(node => node.kind === 'directory' && node.children
      ? { ...node, children: sortNodes(node.children) }
      : node)
    .sort((left, right) => {
      if (left.kind !== right.kind) {
        return left.kind === 'directory' ? -1 : 1
      }
      return left.path.localeCompare(right.path)
    })

  return sortNodes(root)
}

function createSkillDocument(
  id: string,
  sourceKey: string,
  name: string,
  description: string,
  displayPath: string,
  workspaceOwned: boolean,
  files: Record<string, WorkspaceSkillFileDocument>,
  relativePath?: string,
): WorkspaceSkillDocument {
  const rootPath = displayPath.replace(/\/SKILL\.md$/, '')
  const content = files['SKILL.md']?.content ?? ''
  return {
    id,
    sourceKey,
    name,
    description,
    content,
    displayPath,
    rootPath,
    tree: createSkillTree(Object.keys(files)),
    sourceOrigin: 'skills_dir',
    workspaceOwned,
    relativePath,
  }
}

function inferContentType(path: string, isText: boolean) {
  if (!isText) {
    return 'application/octet-stream'
  }
  if (path.endsWith('.md')) {
    return 'text/markdown'
  }
  if (path.endsWith('.json')) {
    return 'application/json'
  }
  if (path.endsWith('.txt')) {
    return 'text/plain'
  }
  return 'text/plain'
}

function inferLanguage(path: string) {
  if (path.endsWith('.md')) {
    return 'markdown'
  }
  if (path.endsWith('.json')) {
    return 'json'
  }
  if (path.endsWith('.ts')) {
    return 'typescript'
  }
  if (path.endsWith('.js')) {
    return 'javascript'
  }
  if (path.endsWith('.yml') || path.endsWith('.yaml')) {
    return 'yaml'
  }
  if (path.endsWith('.txt')) {
    return 'text'
  }
  return undefined
}

function createSkillFileDocument(
  skillId: string,
  sourceKey: string,
  rootPath: string,
  path: string,
  options: {
    content?: string
    isText?: boolean
    readonly?: boolean
    contentType?: string
    language?: string
    byteSize?: number
  } = {},
): WorkspaceSkillFileDocument {
  const isText = options.isText ?? true
  const content = isText ? (options.content ?? '') : null
  return {
    skillId,
    sourceKey,
    path,
    displayPath: `${rootPath}/${path}`,
    byteSize: options.byteSize ?? (content?.length ?? 0),
    isText,
    content,
    contentType: options.contentType ?? inferContentType(path, isText),
    language: options.language ?? inferLanguage(path),
    readonly: options.readonly ?? false,
  }
}

function cloneSkillFiles(files: Record<string, WorkspaceSkillFileDocument>) {
  return Object.fromEntries(Object.entries(files).map(([path, document]) => [path, clone(document)]))
}

function createSkillAsset(input: {
  id: string
  sourceKey: string
  name: string
  description: string
  displayPath: string
  workspaceOwned: boolean
  files: Record<string, WorkspaceSkillFileDocument>
  relativePath?: string
}): { document: WorkspaceSkillDocument, files: Record<string, WorkspaceSkillFileDocument> } {
  const document = createSkillDocument(
    input.id,
    input.sourceKey,
    input.name,
    input.description,
    input.displayPath,
    input.workspaceOwned,
    input.files,
    input.relativePath,
  )
  return {
    document,
    files: cloneSkillFiles(input.files),
  }
}

function createImportedSkillFiles(
  skillId: string,
  sourceKey: string,
  rootPath: string,
  slug: string,
  uploadedFiles?: WorkspaceDirectoryUploadEntry[],
) {
  const normalizedFiles = normalizeImportedFiles(uploadedFiles)
  if (!normalizedFiles?.length) {
    return {
      'SKILL.md': createSkillFileDocument(skillId, sourceKey, rootPath, 'SKILL.md', {
        content: [
          '---',
          `name: ${slug}`,
          `description: Imported ${slug} skill.`,
          '---',
          '',
          '# Overview',
        ].join('\n'),
      }),
    }
  }

  return Object.fromEntries(normalizedFiles.map((file) => {
    const isText = file.contentType.startsWith('text/')
      || /\.(md|txt|json|ya?ml|ts|js|mts|cts)$/i.test(file.relativePath)
    const decoded = isText ? atob(file.dataBase64) : undefined
    const content = file.relativePath === 'SKILL.md' && decoded
      ? normalizeSkillFrontmatterName(decoded, slug)
      : decoded
    return [file.relativePath, createSkillFileDocument(skillId, sourceKey, rootPath, file.relativePath, {
      content,
      isText,
      byteSize: file.byteSize,
      contentType: file.contentType,
    })]
  }))
}

function normalizeSkillFrontmatterName(content: string, slug: string) {
  const endsWithNewline = content.endsWith('\n')
  const lines = content.replace(/\r\n/g, '\n').split('\n')
  if (endsWithNewline && lines.at(-1) === '') {
    lines.pop()
  }
  if (lines[0]?.trim() !== '---') {
    return content
  }

  const closingIndex = lines.findIndex((line, index) => index > 0 && line.trim() === '---')
  if (closingIndex === -1) {
    return content
  }

  const nameIndex = lines.findIndex((line, index) => index > 0 && index < closingIndex && line.trimStart().startsWith('name:'))
  const normalizedName = `name: ${slug}`
  if (nameIndex >= 0) {
    lines[nameIndex] = normalizedName
  } else {
    lines.splice(closingIndex, 0, normalizedName)
  }

  const updated = lines.join('\n')
  return endsWithNewline ? `${updated}\n` : updated
}

function normalizeImportedFiles(files?: WorkspaceDirectoryUploadEntry[]) {
  if (!files?.length) {
    return null
  }

  const normalized = files.map((file) => ({
    ...file,
    relativePath: file.relativePath.replace(/^\/+/, '').replace(/\\/g, '/'),
  }))
  const direct = normalized.some(file => file.relativePath === 'SKILL.md')
  if (direct) {
    return normalized
  }

  const firstSegments = new Set(normalized.map(file => file.relativePath.split('/')[0]).filter(Boolean))
  if (firstSegments.size !== 1) {
    return normalized
  }

  const prefix = [...firstSegments][0]
  return normalized.map((file) => ({
    ...file,
    relativePath: file.relativePath.startsWith(`${prefix}/`)
      ? file.relativePath.slice(prefix.length + 1)
      : file.relativePath,
  }))
}

function mcpEndpointFromConfig(config: Record<string, JsonValue>): string {
  const url = config.url
  if (typeof url === 'string' && url) {
    return url
  }

  const command = config.command
  if (typeof command === 'string' && command) {
    return command
  }

  return 'configured'
}

function createSkillCatalogEntry(
  workspaceId: string,
  document: WorkspaceSkillDocument,
  disabled = false,
): WorkspaceToolCatalogEntry {
  return {
    id: document.id,
    workspaceId,
    kind: 'skill',
    name: document.name,
    description: document.description,
    availability: 'healthy',
    requiredPermission: null,
    sourceKey: document.sourceKey,
    displayPath: document.displayPath,
    disabled,
    management: createManagementCapabilities(true, document.workspaceOwned, document.workspaceOwned),
    active: true,
    sourceOrigin: document.sourceOrigin,
    workspaceOwned: document.workspaceOwned,
    relativePath: document.relativePath,
  }
}

function createMcpCatalogEntry(
  workspaceId: string,
  document: WorkspaceMcpServerDocument,
  disabled = false,
): WorkspaceToolCatalogEntry {
  return {
    id: `mcp-${document.serverName}`,
    workspaceId,
    kind: 'mcp',
    name: document.serverName,
    description: 'Configured MCP server.',
    availability: 'configured',
    requiredPermission: null,
    sourceKey: document.sourceKey,
    displayPath: document.displayPath,
    disabled,
    management: createManagementCapabilities(true, true, true),
    serverName: document.serverName,
    endpoint: mcpEndpointFromConfig(document.config),
    toolNames: [],
    scope: document.scope,
  }
}

function createProjectId(name: string) {
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
  return `proj-${slug || Date.now()}`
}

function syncWorkspaceProjectState(workspaceState: WorkspaceFixtureState) {
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

function updateDefaultProjectIfNeeded(workspaceState: WorkspaceFixtureState, archivedProjectId?: string) {
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

function createProjectRecord(
  workspaceId: string,
  input: CreateProjectRequest,
): ProjectRecord {
  return {
    id: createProjectId(input.name),
    workspaceId,
    name: input.name.trim(),
    status: 'active',
    description: input.description.trim(),
    assignments: input.assignments ? clone(input.assignments) : undefined,
  }
}

const MAIN_WORKSPACE_MENU_IDS = [
  'menu-workspace-overview',
  'menu-workspace-projects',
  'menu-workspace-knowledge',
  'menu-workspace-resources',
  'menu-workspace-agents',
  'menu-workspace-teams',
  'menu-workspace-models',
  'menu-workspace-tools',
  'menu-workspace-automations',
  'menu-workspace-user-center',
] as const

function updateProjectRecord(
  current: ProjectRecord,
  input: UpdateProjectRequest,
): ProjectRecord {
  return {
    ...current,
    name: input.name.trim(),
    description: input.description.trim(),
    status: input.status,
    assignments: input.assignments ? clone(input.assignments) : undefined,
  }
}

function createRuntimeConfigSource(
  scope: 'workspace' | 'project' | 'user',
  workspaceId: string,
  ownerId?: string,
): RuntimeEffectiveConfig['sources'][number] {
  if (scope === 'workspace') {
    return {
      scope,
      displayPath: 'config/runtime/workspace.json',
      sourceKey: 'workspace',
      exists: true,
      loaded: true,
      contentHash: `${workspaceId}-workspace-runtime-source-hash`,
      document: {
        model: 'claude-sonnet-4-5',
        permissions: {
          defaultMode: 'plan',
        },
        toolCatalog: {
          disabledSourceKeys: [],
        },
        mcpServers: {
          ops: {
            type: 'http',
            url: 'https://ops.example.test/mcp',
          },
        },
      },
    }
  }

  if (scope === 'project') {
    return {
      scope,
      ownerId,
      displayPath: `config/runtime/projects/${ownerId}.json`,
      sourceKey: `project:${ownerId}`,
      exists: true,
      loaded: true,
      contentHash: `${workspaceId}-${ownerId}-project-runtime-source-hash`,
      document: {
        approvals: {
          defaultMode: 'manual',
        },
      },
    }
  }

  return {
    scope,
    ownerId,
    displayPath: `config/runtime/users/${ownerId}.json`,
    sourceKey: `user:${ownerId}`,
    exists: true,
    loaded: true,
    contentHash: `${workspaceId}-${ownerId}-user-runtime-source-hash`,
    document: {
      provider: {
        defaultModel: 'claude-sonnet-4-5',
      },
    },
  }
}

function createHostBootstrap(): ShellBootstrap {
  return {
    hostState: {
      platform: 'tauri',
      mode: 'local',
      appVersion: '0.1.0-test',
      cargoWorkspace: true,
      shell: 'tauri2',
    },
    preferences: {
      theme: 'system',
      locale: 'zh-CN',
      compactSidebar: false,
      leftSidebarCollapsed: false,
      rightSidebarCollapsed: false,
      updateChannel: 'formal',
      fontSize: 16,
      fontFamily: 'Inter, sans-serif',
      fontStyle: 'sans',
      defaultWorkspaceId: 'ws-local',
      lastVisitedRoute: '/workspaces/ws-local/overview?project=proj-redesign',
    },
    connections: [
      {
        id: 'conn-local',
        mode: 'local',
        label: 'Local Workspace',
        workspaceId: 'ws-local',
        state: 'local-ready',
      },
      {
        id: 'conn-enterprise',
        mode: 'shared',
        label: 'Enterprise Workspace',
        workspaceId: 'ws-enterprise',
        state: 'connected',
        baseUrl: 'https://enterprise.example.test',
      },
    ],
    backend: {
      baseUrl: 'http://127.0.0.1:43127',
      authToken: 'desktop-test-token',
      state: 'ready',
      transport: 'http',
    },
    workspaceConnections: clone(WORKSPACE_CONNECTIONS),
  }
}

function createWorkspaceFixtureState(
  connection: WorkspaceConnectionRecord,
  options: FixtureOptions = {},
): WorkspaceFixtureState {
  const local = connection.workspaceId === 'ws-local'
  const ownerReady = local ? options.localOwnerReady ?? true : true
  const setupRequired = local ? options.localSetupRequired ?? false : false
  const workspace = {
    id: connection.workspaceId,
    name: local ? 'Local Workspace' : 'Enterprise Workspace',
    slug: local ? 'local-workspace' : 'enterprise-workspace',
    deployment: local ? 'local' : 'remote',
    bootstrapStatus: setupRequired ? 'setup_required' : 'ready',
    ownerUserId: ownerReady ? 'user-owner' : undefined,
    host: local ? '127.0.0.1' : 'enterprise.example.test',
    listenAddress: connection.baseUrl,
    defaultProjectId: local ? 'proj-redesign' : 'proj-launch',
  } as const

  const projects: ProjectRecord[] = local
    ? [
        {
          id: 'proj-redesign',
          workspaceId: workspace.id,
          name: 'Desktop Redesign',
          status: 'active',
          description: 'Real workspace API migration for the desktop surface.',
          assignments: {
            models: {
              configuredModelIds: ['anthropic-primary', 'anthropic-alt'],
              defaultConfiguredModelId: 'anthropic-primary',
            },
            tools: {
              sourceKeys: ['builtin:bash', 'mcp:ops'],
            },
            agents: {
              agentIds: ['agent-architect'],
              teamIds: ['team-studio'],
            },
          },
        },
        {
          id: 'proj-governance',
          workspaceId: workspace.id,
          name: 'Workspace Governance',
          status: 'active',
          description: 'RBAC, menu policies, and audit automation.',
        },
      ]
    : [
        {
          id: 'proj-launch',
          workspaceId: workspace.id,
          name: 'Launch Readiness',
          status: 'active',
          description: 'Enterprise launch planning and cutover execution.',
        },
      ]

  const recentConversations = local
    ? [
        {
          id: 'conv-redesign',
          workspaceId: workspace.id,
          projectId: 'proj-redesign',
          sessionId: 'rt-conv-redesign',
          title: 'Conversation Redesign',
          status: 'completed',
          updatedAt: 100,
          lastMessagePreview: 'Runtime-only conversation state is active.',
        },
        {
          id: 'conv-governance',
          workspaceId: workspace.id,
          projectId: 'proj-governance',
          sessionId: 'rt-conv-governance',
          title: 'Governance Checklist',
          status: 'draft',
          updatedAt: 90,
          lastMessagePreview: 'Define workspace menu policy.',
        },
      ]
    : [
        {
          id: 'conv-launch',
          workspaceId: workspace.id,
          projectId: 'proj-launch',
          sessionId: 'rt-conv-launch',
          title: 'Launch Cutover',
          status: 'running',
          updatedAt: 120,
          lastMessagePreview: 'Cutover checklist is in review.',
        },
      ]

  const recentActivity = local
    ? [
        { id: 'activity-sync', title: 'Workspace synced', description: 'Bootstrap and projections loaded.', timestamp: 100 },
        { id: 'activity-runtime', title: 'Runtime event replay', description: 'Recovered session stream after reconnect.', timestamp: 96 },
      ]
    : [
        { id: 'activity-launch', title: 'Launch dashboard refreshed', description: 'Enterprise projection rebuilt.', timestamp: 120 },
      ]

  const overview: WorkspaceOverviewSnapshot = {
    workspace,
    metrics: [
      { id: 'projects', label: 'Projects', value: String(projects.length), tone: 'accent' },
      { id: 'conversations', label: 'Conversations', value: String(recentConversations.length), tone: 'info' },
      { id: 'automations', label: 'Automations', value: local ? '1' : '0', tone: local ? 'success' : 'default' },
      { id: 'alerts', label: 'Alerts', value: local ? '0' : '1', tone: local ? 'default' : 'warning' },
    ],
    projects,
    recentConversations,
    recentActivity,
  }

  const dashboards: Record<string, ProjectDashboardSnapshot> = Object.fromEntries(projects.map(project => [
    project.id,
    {
      project,
      metrics: [
        { id: 'sessions', label: 'Sessions', value: String(recentConversations.filter(item => item.projectId === project.id).length), tone: 'accent' },
        { id: 'resources', label: 'Resources', value: local ? '2' : '1', tone: 'info' },
      ],
      recentConversations: recentConversations.filter(item => item.projectId === project.id),
      recentActivity: recentActivity,
    },
  ]))

  const workspaceResources: WorkspaceResourceRecord[] = [
    {
      id: `${workspace.id}-res-workspace-1`,
      workspaceId: workspace.id,
      kind: 'folder',
      name: local ? 'Shared Specs' : 'Launch Runbooks',
      location: local ? '/workspace/specs' : 's3://launch/runbooks',
      origin: 'source',
      status: 'healthy',
      updatedAt: 100,
      tags: ['docs', 'shared'],
    },
  ]

  const projectResources: Record<string, WorkspaceResourceRecord[]> = Object.fromEntries(projects.map(project => [
    project.id,
    [
      {
        id: `${project.id}-res-1`,
        workspaceId: workspace.id,
        projectId: project.id,
        kind: 'file',
        name: `${project.name} Brief`,
        location: `/projects/${project.id}/brief.md`,
        origin: 'source',
        status: 'healthy',
        updatedAt: 101,
        tags: ['brief'],
      },
      {
        id: `${project.id}-res-2`,
        workspaceId: workspace.id,
        projectId: project.id,
        kind: 'url',
        name: `${project.name} API`,
        location: `https://example.test/${project.id}/api`,
        origin: 'generated',
        sourceArtifactId: project.id === 'proj-redesign' ? 'artifact-run-conv-redesign' : undefined,
        status: 'configured',
        updatedAt: 102,
        tags: ['api'],
      },
    ],
  ]))

  const artifacts: ArtifactRecord[] = [
    {
      id: 'artifact-run-conv-redesign',
      workspaceId: workspace.id,
      projectId: 'proj-redesign',
      title: 'Runtime Delivery Summary',
      status: 'review',
      latestVersion: 3,
      updatedAt: 103,
      contentType: 'text/markdown',
    },
    {
      id: 'artifact-run-conv-approval',
      workspaceId: workspace.id,
      projectId: 'proj-redesign',
      title: 'Approval Command Output',
      status: 'draft',
      latestVersion: 1,
      updatedAt: 104,
      contentType: 'text/plain',
    },
    {
      id: 'artifact-1',
      workspaceId: workspace.id,
      projectId: 'proj-redesign',
      title: 'Workspace Protocol Baseline',
      status: 'approved',
      latestVersion: 5,
      updatedAt: 100,
      contentType: 'text/markdown',
    },
  ]

  const workspaceKnowledge: KnowledgeRecord[] = [
    {
      id: `${workspace.id}-knowledge-1`,
      workspaceId: workspace.id,
      title: local ? 'Workspace Protocol Baseline' : 'Enterprise Release Policy',
      summary: 'Projection snapshot used by the desktop shell.',
      kind: 'shared',
      status: 'shared',
      sourceType: 'artifact',
      sourceRef: 'artifact-1',
      updatedAt: 100,
    },
  ]

  const projectKnowledge: Record<string, KnowledgeRecord[]> = Object.fromEntries(projects.map(project => [
    project.id,
    [
      {
        id: `${project.id}-knowledge-1`,
        workspaceId: workspace.id,
        projectId: project.id,
        title: `${project.name} Notes`,
        summary: `Knowledge entries scoped to ${project.name}.`,
        kind: 'shared',
        status: 'reviewed',
        sourceType: 'conversation',
        sourceRef: `conv-${project.id}`,
        updatedAt: 101,
      },
    ],
  ]))

  const agents: AgentRecord[] = local
    ? [
        {
          id: 'agent-architect',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Architect Agent',
          avatarPath: 'data/blobs/avatars/agent-architect.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Calm systems thinker',
          tags: ['architecture', 'platform'],
          prompt: 'Drive architecture reviews and schema decisions.',
          builtinToolKeys: ['bash'],
          skillIds: ['skill-workspace-help'],
          mcpServerNames: ['ops'],
          title: 'System architect',
          description: 'Owns protocol, schema, and platform integration decisions.',
          status: 'active',
          updatedAt: 100,
        },
        {
          id: 'agent-coder',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Coder Agent',
          avatarPath: 'data/blobs/avatars/agent-coder.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Fast implementation closer',
          tags: ['frontend', 'delivery'],
          prompt: 'Implement scoped frontend and backend tasks quickly.',
          builtinToolKeys: ['bash'],
          skillIds: ['skill-external-checks'],
          mcpServerNames: [],
          title: 'Implementation lead',
          description: 'Delivers frontend and backend implementation changes.',
          status: 'active',
          updatedAt: 99,
        },
        {
          id: 'agent-redesign',
          workspaceId: workspace.id,
          projectId: 'proj-redesign',
          scope: 'project',
          name: 'Redesign Copilot',
          avatarPath: 'data/blobs/avatars/agent-redesign.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Product-focused collaborator',
          tags: ['redesign', 'ux'],
          prompt: 'Track the desktop redesign migration plan and unblock delivery.',
          builtinToolKeys: ['bash'],
          skillIds: ['skill-workspace-help'],
          mcpServerNames: ['ops'],
          title: 'Project agent',
          description: 'Tracks the redesign migration work.',
          status: 'active',
          updatedAt: 98,
        },
      ]
    : [
        {
          id: 'agent-gov',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Governance Agent',
          avatarPath: 'data/blobs/avatars/agent-gov.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Compliance reviewer',
          tags: ['governance'],
          prompt: 'Review launch and compliance readiness.',
          builtinToolKeys: ['bash'],
          skillIds: [],
          mcpServerNames: [],
          title: 'Compliance lead',
          description: 'Reviews launch and compliance readiness.',
          status: 'active',
          updatedAt: 120,
        },
      ]

  const projectAgentLinks: Record<string, ProjectAgentLinkRecord[]> = local
    ? {
        'proj-redesign': [
          {
            workspaceId: workspace.id,
            projectId: 'proj-redesign',
            agentId: 'agent-architect',
            linkedAt: 97,
          },
        ],
      }
    : {}

  const teams: TeamRecord[] = local
    ? [
        {
          id: 'team-studio',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Studio Direction Team',
          avatarPath: 'data/blobs/avatars/team-studio.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Cross-functional design leadership',
          tags: ['ux', 'direction'],
          prompt: 'Coordinate shell direction and experience consistency.',
          builtinToolKeys: ['bash'],
          skillIds: ['skill-workspace-help'],
          mcpServerNames: ['ops'],
          leaderAgentId: 'agent-architect',
          memberAgentIds: ['agent-architect', 'agent-coder'],
          description: 'Owns shared UX and shell direction.',
          status: 'active',
          updatedAt: 100,
        },
        {
          id: 'team-redesign',
          workspaceId: workspace.id,
          projectId: 'proj-redesign',
          scope: 'project',
          name: 'Redesign Tiger Team',
          avatarPath: 'data/blobs/avatars/team-redesign.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Delivery-focused strike team',
          tags: ['migration', 'desktop'],
          prompt: 'Execute the desktop redesign migration.',
          builtinToolKeys: ['bash'],
          skillIds: ['skill-external-checks'],
          mcpServerNames: [],
          leaderAgentId: 'agent-redesign',
          memberAgentIds: ['agent-redesign'],
          description: 'Executes the desktop migration.',
          status: 'active',
          updatedAt: 99,
        },
      ]
    : [
        {
          id: 'team-launch',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Launch Readiness Team',
          avatarPath: 'data/blobs/avatars/team-launch.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Enterprise rollout coordinators',
          tags: ['launch', 'operations'],
          prompt: 'Coordinate enterprise rollout readiness.',
          builtinToolKeys: ['bash'],
          skillIds: [],
          mcpServerNames: [],
          leaderAgentId: 'agent-gov',
          memberAgentIds: ['agent-gov'],
          description: 'Coordinates enterprise rollout.',
          status: 'active',
          updatedAt: 120,
        },
      ]

  const projectTeamLinks: Record<string, ProjectTeamLinkRecord[]> = local
    ? {
        'proj-redesign': [
          {
            workspaceId: workspace.id,
            projectId: 'proj-redesign',
            teamId: 'team-studio',
            linkedAt: 96,
          },
        ],
      }
    : {}

  const credentialBindings: CredentialBinding[] = [
    {
      credentialRef: 'env:ANTHROPIC_API_KEY',
      providerId: 'anthropic',
      label: 'Anthropic Primary',
      baseUrl: 'https://api.anthropic.com',
      status: 'healthy',
      configured: true,
      source: 'workspace',
    },
    {
      credentialRef: 'env:OPENAI_API_KEY',
      providerId: 'openai',
      label: 'OpenAI Primary',
      baseUrl: 'https://api.openai.com/v1',
      status: 'healthy',
      configured: true,
      source: 'workspace',
    },
  ]

  const catalog: ModelCatalogSnapshot = {
    providers: [
      {
        providerId: 'anthropic',
        label: 'Anthropic',
        enabled: true,
        surfaces: [
          {
            surface: 'conversation',
            protocolFamily: 'anthropic_messages',
            transport: ['https'],
            authStrategy: 'x_api_key',
            baseUrl: 'https://api.anthropic.com',
            baseUrlPolicy: 'allow_override',
            enabled: true,
            capabilities: [
              { capabilityId: 'streaming', label: 'Streaming' },
              { capabilityId: 'tool_calling', label: 'Tool Calling' },
            ],
          },
        ],
        metadata: {},
      },
      {
        providerId: 'openai',
        label: 'OpenAI',
        enabled: true,
        surfaces: [
          {
            surface: 'conversation',
            protocolFamily: 'openai_chat',
            transport: ['https'],
            authStrategy: 'bearer',
            baseUrl: 'https://api.openai.com/v1',
            baseUrlPolicy: 'allow_override',
            enabled: true,
            capabilities: [
              { capabilityId: 'streaming', label: 'Streaming' },
              { capabilityId: 'tool_calling', label: 'Tool Calling' },
            ],
          },
        ],
        metadata: {},
      },
    ],
    models: [
      {
        modelId: 'gpt-4o',
        label: 'GPT-4o',
        providerId: 'openai',
        description: 'Balanced model for interactive work.',
        family: 'gpt-4o',
        track: 'general',
        enabled: true,
        recommendedFor: 'General desktop orchestration',
        availability: 'healthy',
        defaultPermission: 'auto',
        surfaceBindings: [
          {
            surface: 'conversation',
            protocolFamily: 'openai_chat',
            enabled: true,
          },
        ],
        capabilities: [
          { capabilityId: 'streaming', label: 'Streaming' },
          { capabilityId: 'tool_calling', label: 'Tool Calling' },
        ],
        metadata: {},
      },
      {
        modelId: 'claude-sonnet-4-5',
        label: 'Claude Sonnet 4.5',
        providerId: 'anthropic',
        description: 'Runtime-heavy work and reasoning.',
        family: 'claude-sonnet',
        track: 'default',
        enabled: true,
        recommendedFor: 'Runtime sessions',
        availability: 'configured',
        defaultPermission: 'readonly',
        surfaceBindings: [
          {
            surface: 'conversation',
            protocolFamily: 'anthropic_messages',
            enabled: true,
          },
        ],
        capabilities: [
          { capabilityId: 'streaming', label: 'Streaming' },
          { capabilityId: 'tool_calling', label: 'Tool Calling' },
        ],
        metadata: {},
      },
    ],
    configuredModels: [
      {
        configuredModelId: 'openai-primary',
        name: 'GPT-4o',
        providerId: 'openai',
        modelId: 'gpt-4o',
        credentialRef: 'env:OPENAI_API_KEY',
        tokenUsage: {
          usedTokens: 0,
          exhausted: false,
        },
        enabled: true,
        source: 'workspace',
        status: 'configured',
        configured: true,
      },
      {
        configuredModelId: 'anthropic-primary',
        name: 'Claude Primary',
        providerId: 'anthropic',
        modelId: 'claude-sonnet-4-5',
        credentialRef: 'env:ANTHROPIC_API_KEY',
        tokenUsage: {
          usedTokens: 0,
          exhausted: false,
        },
        enabled: true,
        source: 'workspace',
        status: 'configured',
        configured: true,
      },
      {
        configuredModelId: 'anthropic-alt',
        name: 'Claude Alt',
        providerId: 'anthropic',
        modelId: 'claude-sonnet-4-5',
        credentialRef: 'env:ANTHROPIC_ALT_API_KEY',
        tokenUsage: {
          usedTokens: 0,
          exhausted: false,
        },
        enabled: true,
        source: 'workspace',
        status: 'configured',
        configured: true,
      },
    ],
    credentialBindings,
    defaultSelections: {
      conversation: {
        configuredModelId: 'anthropic-primary',
        providerId: 'anthropic',
        modelId: 'claude-sonnet-4-5',
        surface: 'conversation',
      },
    },
    diagnostics: {
      warnings: [],
      errors: [],
    },
  }

  const tools: ToolRecord[] = [
    {
      id: 'tool-read',
      workspaceId: workspace.id,
      kind: 'builtin',
      name: 'Read',
      description: 'Read files from the workspace.',
      status: 'active',
      permissionMode: 'readonly',
      updatedAt: 100,
    },
    {
      id: 'tool-terminal',
      workspaceId: workspace.id,
      kind: 'builtin',
      name: 'Terminal',
      description: 'Execute commands in the workspace terminal.',
      status: 'active',
      permissionMode: 'ask',
      updatedAt: 100,
    },
  ]

  const managedHelpSkill = createSkillAsset({
    id: 'skill-workspace-help',
    sourceKey: 'skill:data/skills/help/SKILL.md',
    name: 'help',
    description: 'Helpful local skill.',
    displayPath: 'data/skills/help/SKILL.md',
    workspaceOwned: true,
    relativePath: 'data/skills/help/SKILL.md',
    files: {
      'SKILL.md': createSkillFileDocument(
        'skill-workspace-help',
        'skill:data/skills/help/SKILL.md',
        'data/skills/help',
        'SKILL.md',
        {
          content: [
            '---',
            'name: help',
            'description: Helpful local skill.',
            '---',
            '',
            '# Help',
            '',
            'Useful local workspace instructions.',
          ].join('\n'),
        },
      ),
      'notes/guide.md': createSkillFileDocument(
        'skill-workspace-help',
        'skill:data/skills/help/SKILL.md',
        'data/skills/help',
        'notes/guide.md',
        {
          content: '# Guide\n\nReview the workspace guide before use.',
        },
      ),
      'assets/logo.png': createSkillFileDocument(
        'skill-workspace-help',
        'skill:data/skills/help/SKILL.md',
        'data/skills/help',
        'assets/logo.png',
        {
          isText: false,
          byteSize: 2048,
          contentType: 'image/png',
        },
      ),
    },
  })

  const externalClaudeSkill = createSkillAsset({
    id: 'skill-external-help',
    sourceKey: 'skill:.claude/skills/external-help/SKILL.md',
    name: 'external-help',
    description: 'Helpful external skill.',
    displayPath: '.claude/skills/external-help/SKILL.md',
    workspaceOwned: false,
    relativePath: '.claude/skills/external-help/SKILL.md',
    files: {
      'SKILL.md': createSkillFileDocument(
        'skill-external-help',
        'skill:.claude/skills/external-help/SKILL.md',
        '.claude/skills/external-help',
        'SKILL.md',
        {
          content: [
            '---',
            'name: external-help',
            'description: Helpful external skill.',
            '---',
            '',
            '# External',
          ].join('\n'),
          readonly: true,
        },
      ),
      'examples/prompt.txt': createSkillFileDocument(
        'skill-external-help',
        'skill:.claude/skills/external-help/SKILL.md',
        '.claude/skills/external-help',
        'examples/prompt.txt',
        {
          content: 'Use this skill when you need external guidance.',
          readonly: true,
          language: 'text',
        },
      ),
    },
  })

  const externalCodexSkill = createSkillAsset({
    id: 'skill-external-checks',
    sourceKey: 'skill:.codex/skills/external-checks/SKILL.md',
    name: 'external-checks',
    description: 'Helpful external checks skill.',
    displayPath: '.codex/skills/external-checks/SKILL.md',
    workspaceOwned: false,
    relativePath: '.codex/skills/external-checks/SKILL.md',
    files: {
      'SKILL.md': createSkillFileDocument(
        'skill-external-checks',
        'skill:.codex/skills/external-checks/SKILL.md',
        '.codex/skills/external-checks',
        'SKILL.md',
        {
          content: [
            '---',
            'name: external-checks',
            'description: Helpful external checks skill.',
            '---',
            '',
            '# Checks',
          ].join('\n'),
          readonly: true,
        },
      ),
      'templates/checklist.md': createSkillFileDocument(
        'skill-external-checks',
        'skill:.codex/skills/external-checks/SKILL.md',
        '.codex/skills/external-checks',
        'templates/checklist.md',
        {
          content: '# Checklist\n\n- Inspect the current workspace state.',
          readonly: true,
        },
      ),
    },
  })

  const skillDocuments: Record<string, WorkspaceSkillDocument> = {
    [managedHelpSkill.document.id]: managedHelpSkill.document,
    [externalClaudeSkill.document.id]: externalClaudeSkill.document,
    [externalCodexSkill.document.id]: externalCodexSkill.document,
  }

  const skillFiles: Record<string, Record<string, WorkspaceSkillFileDocument>> = {
    [managedHelpSkill.document.id]: managedHelpSkill.files,
    [externalClaudeSkill.document.id]: externalClaudeSkill.files,
    [externalCodexSkill.document.id]: externalCodexSkill.files,
  }

  const mcpDocuments: Record<string, WorkspaceMcpServerDocument> = {
    ops: {
      serverName: 'ops',
      sourceKey: 'mcp:ops',
      displayPath: 'config/runtime/workspace.json',
      scope: 'workspace',
      config: {
        type: 'http',
        url: 'https://ops.example.test/mcp',
      },
    },
  }

  const toolCatalog = {
    entries: [
      {
        id: 'builtin-bash',
        workspaceId: workspace.id,
        kind: 'builtin',
        name: 'bash',
        description: 'Execute a shell command in the current workspace.',
        availability: 'healthy',
        requiredPermission: 'danger-full-access',
        sourceKey: 'builtin:bash',
        displayPath: 'runtime builtin registry',
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        builtinKey: 'bash',
      },
      {
        id: managedHelpSkill.document.id,
        workspaceId: workspace.id,
        kind: 'skill',
        name: managedHelpSkill.document.name,
        description: managedHelpSkill.document.description,
        availability: 'healthy',
        requiredPermission: null,
        sourceKey: managedHelpSkill.document.sourceKey,
        displayPath: managedHelpSkill.document.displayPath,
        disabled: false,
        management: createManagementCapabilities(true, true, true),
        active: true,
        sourceOrigin: 'skills_dir',
        workspaceOwned: true,
        relativePath: managedHelpSkill.document.relativePath,
      },
      {
        id: externalClaudeSkill.document.id,
        workspaceId: workspace.id,
        kind: 'skill',
        name: externalClaudeSkill.document.name,
        description: externalClaudeSkill.document.description,
        availability: 'healthy',
        requiredPermission: null,
        sourceKey: externalClaudeSkill.document.sourceKey,
        displayPath: externalClaudeSkill.document.displayPath,
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        active: true,
        sourceOrigin: 'skills_dir',
        workspaceOwned: false,
        relativePath: externalClaudeSkill.document.relativePath,
      },
      {
        id: externalCodexSkill.document.id,
        workspaceId: workspace.id,
        kind: 'skill',
        name: externalCodexSkill.document.name,
        description: externalCodexSkill.document.description,
        availability: 'healthy',
        requiredPermission: null,
        sourceKey: externalCodexSkill.document.sourceKey,
        displayPath: externalCodexSkill.document.displayPath,
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        active: true,
        sourceOrigin: 'skills_dir',
        workspaceOwned: false,
        relativePath: externalCodexSkill.document.relativePath,
      },
      {
        id: 'mcp-ops',
        workspaceId: workspace.id,
        kind: 'mcp',
        name: 'ops',
        description: 'Configured MCP server.',
        availability: 'attention',
        requiredPermission: null,
        sourceKey: 'mcp:ops',
        displayPath: 'config/runtime/workspace.json',
        disabled: false,
        management: createManagementCapabilities(true, true, true),
        serverName: 'ops',
        endpoint: 'https://ops.example.test/mcp',
        toolNames: ['mcp__ops__tail_logs'],
        statusDetail: 'MCP handshake timed out',
        scope: 'workspace',
      },
    ] satisfies WorkspaceToolCatalogEntry[],
  }

  const automations: AutomationRecord[] = local
    ? [
        {
          id: 'automation-sync',
          workspaceId: workspace.id,
          title: 'Daily Runtime Sync',
          description: 'Refresh runtime projections every morning.',
          cadence: 'Every day 09:00',
          ownerType: 'agent',
          ownerId: 'agent-architect',
          status: 'active',
          nextRunAt: 110,
          lastRunAt: 90,
          output: 'Update overview and dashboard projections.',
        },
      ]
    : []

  const users: UserRecordSummary[] = [
    ...(ownerReady
      ? [{
          id: 'user-owner',
          username: 'owner',
          displayName: local ? 'Octopus Owner' : 'Enterprise Owner',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          status: 'active',
          passwordState: 'set' as const,
          roleIds: ['role-owner'],
          scopeProjectIds: [],
        }]
      : []),
    {
      id: 'user-operator',
      username: 'operator',
      displayName: 'Lin Zhou',
      avatar: 'data:image/png;base64,iVBORw0KGgo=',
      status: 'active',
      passwordState: 'set',
      roleIds: ['role-operator'],
      scopeProjectIds: projects.map(project => project.id),
    },
  ]

  const roles: RoleRecord[] = [
    {
      id: 'role-owner',
      workspaceId: workspace.id,
      name: 'Owner',
      code: 'owner',
      description: 'Full workspace access.',
      status: 'active',
      permissionIds: ['perm-manage-users', 'perm-manage-roles', 'perm-manage-tools'],
      menuIds: [
        ...MAIN_WORKSPACE_MENU_IDS,
        'menu-workspace-user-center-profile',
        'menu-workspace-user-center-pet',
        'menu-workspace-user-center-users',
        'menu-workspace-user-center-roles',
        'menu-workspace-user-center-permissions',
        'menu-workspace-user-center-menus',
      ],
    },
    {
      id: 'role-operator',
      workspaceId: workspace.id,
      name: 'Operator',
      code: 'operator',
      description: 'Daily operations access.',
      status: 'active',
      permissionIds: ['perm-manage-tools'],
      menuIds: [
        'menu-workspace-overview',
        'menu-workspace-projects',
        'menu-workspace-user-center-profile',
        'menu-workspace-user-center-pet',
        'menu-workspace-user-center-users',
      ],
    },
  ]

  const permissions: PermissionRecord[] = [
    {
      id: 'perm-manage-users',
      workspaceId: workspace.id,
      name: 'Manage users',
      code: 'workspace.users',
      description: 'Create and update workspace users.',
      status: 'active',
      kind: 'atomic',
      targetType: 'user',
      targetIds: [],
      action: 'manage',
      memberPermissionIds: [],
    },
    {
      id: 'perm-manage-roles',
      workspaceId: workspace.id,
      name: 'Manage roles',
      code: 'workspace.roles',
      description: 'Create and update roles.',
      status: 'active',
      kind: 'atomic',
      targetType: 'role',
      targetIds: [],
      action: 'manage',
      memberPermissionIds: [],
    },
    {
      id: 'perm-manage-tools',
      workspaceId: workspace.id,
      name: 'Manage tools',
      code: 'workspace.tools',
      description: 'Create and update tools.',
      status: 'active',
      kind: 'atomic',
      targetType: 'tool',
      targetIds: [],
      action: 'manage',
      memberPermissionIds: [],
    },
  ]

  const menus: MenuRecord[] = [
    {
      id: 'menu-workspace-user-center-profile',
      workspaceId: workspace.id,
      parentId: 'menu-workspace-user-center',
      source: 'user-center',
      label: 'Profile',
      routeName: 'workspace-user-center-profile',
      status: 'active',
      order: 120,
    },
    {
      id: 'menu-workspace-user-center-pet',
      workspaceId: workspace.id,
      parentId: 'menu-workspace-user-center',
      source: 'user-center',
      label: 'Pet',
      routeName: 'workspace-user-center-pet',
      status: 'active',
      order: 125,
    },
    {
      id: 'menu-workspace-user-center-users',
      workspaceId: workspace.id,
      parentId: 'menu-workspace-user-center',
      source: 'user-center',
      label: 'Users',
      routeName: 'workspace-user-center-users',
      status: 'active',
      order: 130,
    },
    {
      id: 'menu-workspace-user-center-roles',
      workspaceId: workspace.id,
      parentId: 'menu-workspace-user-center',
      source: 'user-center',
      label: 'Roles',
      routeName: 'workspace-user-center-roles',
      status: 'active',
      order: 140,
    },
    {
      id: 'menu-workspace-user-center-permissions',
      workspaceId: workspace.id,
      parentId: 'menu-workspace-user-center',
      source: 'user-center',
      label: 'Permissions',
      routeName: 'workspace-user-center-permissions',
      status: 'active',
      order: 150,
    },
    {
      id: 'menu-workspace-user-center-menus',
      workspaceId: workspace.id,
      parentId: 'menu-workspace-user-center',
      source: 'user-center',
      label: 'Menus',
      routeName: 'workspace-user-center-menus',
      status: 'active',
      order: 160,
    },
  ]

  const userCenterOverview: UserCenterOverviewSnapshot = {
    workspaceId: workspace.id,
    currentUser: users[0],
    roleNames: ownerReady ? ['Owner'] : ['Operator'],
    metrics: [
      { id: 'users', label: 'Users', value: String(users.length), tone: 'accent' },
      { id: 'roles', label: 'Roles', value: String(roles.length), tone: 'info' },
    ],
    alerts: [],
    quickLinks: menus.slice(0, 2),
  }

  const runtimeWorkspaceConfig: RuntimeEffectiveConfig = {
    effectiveConfig: {
      model: 'claude-sonnet-4-5',
      permissions: {
        defaultMode: 'plan',
      },
      toolCatalog: {
        disabledSourceKeys: [],
      },
      mcpServers: {
        ops: clone(mcpDocuments.ops.config),
      },
    },
    effectiveConfigHash: `${workspace.id}-cfg-hash-1`,
    sources: [
      createRuntimeConfigSource('workspace', workspace.id),
    ],
    validation: {
      valid: true,
      errors: [],
      warnings: [],
    },
    secretReferences: [],
  }

  const runtimeProjectConfigs = Object.fromEntries(projects.map(project => [
    project.id,
    {
      effectiveConfig: {
        provider: {
          defaultModel: 'claude-sonnet-4-5',
        },
        ...clone(runtimeWorkspaceConfig.effectiveConfig),
        approvals: {
          defaultMode: 'manual',
        },
      },
      effectiveConfigHash: `${workspace.id}-${project.id}-project-cfg-hash-1`,
      sources: (() => {
        const projectSource = createRuntimeConfigSource('project', workspace.id, project.id)
        if (project.id === 'proj-redesign') {
          projectSource.document = {
            approvals: {
              defaultMode: 'manual',
            },
            projectSettings: {
              models: {
                allowedConfiguredModelIds: ['anthropic-primary'],
                defaultConfiguredModelId: 'anthropic-primary',
              },
              tools: {
                enabledSourceKeys: ['builtin:bash'],
                overrides: {
                  'builtin:bash': {
                    permissionMode: 'readonly',
                  },
                  'mcp:ops': {
                    permissionMode: 'deny',
                  },
                },
              },
              agents: {
                enabledAgentIds: ['agent-architect'],
                enabledTeamIds: ['team-studio'],
              },
            },
          }
        }

        return [
          createRuntimeConfigSource('user', workspace.id, 'user-owner'),
          createRuntimeConfigSource('workspace', workspace.id),
          projectSource,
        ]
      })(),
      validation: {
        valid: true,
        errors: [],
        warnings: [],
      },
      secretReferences: [],
    } satisfies RuntimeEffectiveConfig,
  ]))

  const runtimeUserConfig: RuntimeEffectiveConfig = {
    effectiveConfig: {
      provider: {
        defaultModel: 'claude-sonnet-4-5',
      },
      ...clone(runtimeWorkspaceConfig.effectiveConfig),
    },
    effectiveConfigHash: `${workspace.id}-user-owner-runtime-cfg-hash-1`,
    sources: [
      createRuntimeConfigSource('user', workspace.id, 'user-owner'),
      createRuntimeConfigSource('workspace', workspace.id),
    ],
    validation: {
      valid: true,
      errors: [],
      warnings: [],
    },
    secretReferences: [],
  }

  const petProfile = createPetProfile()
  const workspacePetPresence = createPetPresenceState(petProfile.id)
  const projectPetPresences = Object.fromEntries(projects.map(project => [project.id, createPetPresenceState(petProfile.id)]))

  return {
    systemBootstrap: {
      workspace,
      setupRequired,
      ownerReady,
      registeredApps: [],
      protocolVersion: '1.0.0-test',
      apiBasePath: '/api/v1',
      transportSecurity: connection.transportSecurity,
      authMode: 'session-token',
      capabilities: {
        runtime: true,
        approvals: true,
        polling: true,
        sse: true,
      },
    },
    workspace,
    overview,
    projects,
    dashboards,
    workspaceResources,
    projectResources,
    artifacts,
    workspaceKnowledge,
    projectKnowledge,
    agents,
    projectAgentLinks,
    teams,
    projectTeamLinks,
    catalog,
    toolCatalog,
    skillDocuments,
    skillFiles,
    mcpDocuments,
    tools,
    automations,
    userCenterOverview,
    users,
    userPasswords: ownerReady ? { 'user-owner': 'owner-owner', 'user-operator': 'operator-operator' } : { 'user-operator': 'operator-operator' },
    roles,
    permissions,
    menus,
    runtimeSessions: new Map(),
    runtimeWorkspaceConfig,
    runtimeProjectConfigs,
    runtimeUserConfig,
    petProfile,
    workspacePetPresence,
    projectPetPresences,
    workspacePetBinding: undefined,
    projectPetBindings: {},
  }
}

function createPetProfile(): PetProfile {
  return {
    id: 'pet-octopus',
    species: 'octopus',
    displayName: '小章',
    ownerUserId: 'user-owner',
    avatarLabel: 'Octopus mascot',
    summary: 'Octopus 首席吉祥物，负责卖萌和加油。',
    greeting: '嗨！我是小章，今天也要加油哦！',
    mood: 'happy',
    favoriteSnack: '新鲜小虾',
    promptHints: ['最近有什么好消息？', '给我讲个冷笑话', '我们要加油呀！'],
    fallbackAsset: 'octopus',
    riveAsset: undefined,
    stateMachine: undefined,
  }
}

function createPetPresenceState(petId = 'pet-octopus'): PetPresenceState {
  return {
    petId,
    isVisible: true,
    chatOpen: false,
    motionState: 'idle',
    unreadCount: 0,
    lastInteractionAt: 0,
    position: { x: 0, y: 0 },
  }
}

function mapRuntimeMessageToPetMessage(message: RuntimeMessage, petId: string): PetMessage {
  return {
    id: message.id,
    petId,
    sender: message.senderType === 'assistant' ? 'pet' : 'user',
    content: message.content,
    timestamp: message.timestamp,
  }
}

function createPetSnapshot(
  workspaceState: WorkspaceFixtureState,
  projectId?: string,
): PetWorkspaceSnapshot {
  const binding = projectId
    ? workspaceState.projectPetBindings[projectId]
    : workspaceState.workspacePetBinding
  const presence = projectId
    ? workspaceState.projectPetPresences[projectId] ?? createPetPresenceState(workspaceState.petProfile.id)
    : workspaceState.workspacePetPresence
  const runtimeMessages = binding
    ? [...workspaceState.runtimeSessions.values()]
      .find(state => state.detail.summary.conversationId === binding.conversationId)
      ?.detail.messages
      .map(message => mapRuntimeMessageToPetMessage(message, workspaceState.petProfile.id)) ?? []
    : []
  return {
    profile: clone(workspaceState.petProfile),
    presence: clone(presence),
    binding: binding ? clone(binding) : undefined,
    messages: runtimeMessages,
  }
}

function createSessionDetail(conversationId: string, projectId: string, title: string, sessionKind: 'project' | 'pet' = 'project'): RuntimeSessionDetail {
  const sessionId = `rt-${conversationId}`
  return {
    summary: {
      id: sessionId,
      conversationId,
      projectId,
      title,
      sessionKind,
      status: 'draft',
      updatedAt: 1,
      lastMessagePreview: undefined,
      configSnapshotId: 'cfgsnap-fixture',
      effectiveConfigHash: 'cfg-hash-fixture',
      startedFromScopeSet: ['project'],
    },
    run: {
      id: `run-${conversationId}`,
      sessionId,
      conversationId,
      status: 'draft',
      currentStep: 'runtime.run.idle',
      startedAt: 1,
      updatedAt: 1,
      configuredModelId: 'anthropic-primary',
      configuredModelName: 'Claude Sonnet 4.5',
      modelId: 'claude-sonnet-4-5',
      nextAction: 'runtime.run.awaitingInput',
      configSnapshotId: 'cfgsnap-fixture',
      effectiveConfigHash: 'cfg-hash-fixture',
      startedFromScopeSet: ['project'],
    },
    messages: [],
    trace: [],
    pendingApproval: undefined,
  }
}

function applyJsonMergePatch(
  target: Record<string, any>,
  patch: Record<string, any>,
): Record<string, any> {
  const next = structuredClone(target)
  for (const [key, value] of Object.entries(patch)) {
    if (value === null) {
      delete next[key]
      continue
    }
    if (Array.isArray(value)) {
      next[key] = structuredClone(value)
      continue
    }
    if (typeof value === 'object') {
      const current = typeof next[key] === 'object' && next[key] && !Array.isArray(next[key])
        ? next[key]
        : {}
      next[key] = applyJsonMergePatch(current, value as Record<string, any>)
      continue
    }
    next[key] = value
  }
  return next
}

function createRuntimeMessage(
  state: RuntimeSessionState,
  senderType: RuntimeMessage['senderType'],
  senderLabel: string,
  content: string,
  modelId = 'claude-sonnet-4-5',
  configuredModelId = modelId,
  configuredModelName = modelId === 'claude-sonnet-4-5' ? 'Claude Sonnet 4.5' : 'GPT-4o',
  actorKind: RuntimeMessage['resolvedActorKind'] = 'agent',
  actorId = 'agent-architect',
): RuntimeMessage {
  const timestamp = state.nextSequence * 10
  return {
    id: `msg-${state.detail.summary.id}-${state.nextSequence}`,
    sessionId: state.detail.summary.id,
    conversationId: state.detail.summary.conversationId,
    senderType,
    senderLabel,
    content,
    timestamp,
    configuredModelId,
    configuredModelName,
    modelId,
    status: state.detail.run.status,
    requestedActorKind: actorKind,
    requestedActorId: actorId,
    resolvedActorKind: actorKind,
    resolvedActorId: actorId,
    resolvedActorLabel: senderType === 'assistant' ? senderLabel : 'You',
    usedDefaultActor: false,
    resourceIds: senderType === 'assistant' ? [`${state.detail.summary.projectId}-res-2`] : [],
    attachments: [],
    artifacts: senderType === 'assistant' ? [`artifact-${state.detail.run.id}`] : [],
    usage: senderType === 'assistant'
      ? {
          inputTokens: 320,
          outputTokens: 180,
          totalTokens: 500,
        }
      : undefined,
    processEntries: senderType === 'assistant'
      ? [
          {
            id: `process-${state.detail.summary.id}-${state.nextSequence}`,
            type: 'execution',
            title: 'Runtime execution',
            detail: `Resolved ${actorKind}:${actorId} and produced a conversation response.`,
            timestamp,
          },
        ]
      : [],
    toolCalls: senderType === 'assistant'
      ? [
          {
            toolId: 'workspace-api',
            label: 'Workspace API',
            kind: 'builtin',
            count: 1,
          },
        ]
      : [],
  }
}

function createTraceItem(
  state: RuntimeSessionState,
  detail: string,
  tone: RuntimeTraceItem['tone'] = 'info',
  actorKind: RuntimeTraceItem['actorKind'] = 'agent',
  actorId = 'agent-architect',
  actor = 'Octopus Runtime',
): RuntimeTraceItem {
  const timestamp = state.nextSequence * 10
  return {
    id: `trace-${state.detail.summary.id}-${state.nextSequence}`,
    sessionId: state.detail.summary.id,
    runId: state.detail.run.id,
    conversationId: state.detail.summary.conversationId,
    kind: 'reasoning',
    title: 'Runtime Trace',
    detail,
    tone,
    timestamp,
    actor,
    actorKind,
    actorId,
  }
}

function createApproval(state: RuntimeSessionState): RuntimeApprovalRequest {
  return {
    id: `approval-${state.detail.summary.id}`,
    sessionId: state.detail.summary.id,
    conversationId: state.detail.summary.conversationId,
    runId: state.detail.run.id,
    toolName: 'bash',
    summary: 'Approve workspace command execution',
    detail: 'Run pwd in the workspace terminal.',
    riskLevel: 'medium',
    createdAt: state.nextSequence * 10,
    status: 'pending',
  }
}

function createEvent(
  state: RuntimeSessionState,
  workspaceId: string,
  eventType: RuntimeEventEnvelope['eventType'],
  patch: Partial<RuntimeEventEnvelope> = {},
): RuntimeEventEnvelope {
  const sequence = state.nextSequence++
  return {
    id: `event-${state.detail.summary.id}-${sequence}`,
    eventType,
    kind: eventType,
    workspaceId,
    projectId: state.detail.summary.projectId,
    sessionId: state.detail.summary.id,
    conversationId: state.detail.summary.conversationId,
    runId: state.detail.run.id,
    emittedAt: sequence * 10,
    sequence,
    ...patch,
  }
}

function eventsAfter(state: RuntimeSessionState, after?: string): RuntimeEventEnvelope[] {
  if (!after) {
    return state.events
  }

  const index = state.events.findIndex(event => event.id === after)
  return index === -1 ? state.events : state.events.slice(index + 1)
}

function createWorkspaceClientFixture(
  connection: WorkspaceConnectionRecord,
  workspaceState: WorkspaceFixtureState,
  options: FixtureOptions = {},
): WorkspaceClient {
  const ensureRuntimeState = (sessionId: string): RuntimeSessionState => {
    const state = workspaceState.runtimeSessions.get(sessionId)
    if (!state) {
      throw new Error(`Unknown runtime session ${sessionId}`)
    }
    return state
  }

  const defaultSession = clone(WORKSPACE_SESSIONS.find(item => item.workspaceConnectionId === connection.workspaceConnectionId)!)

  const buildSession = (userId: string, token = defaultSession.token): WorkspaceSessionTokenEnvelope['session'] => ({
    ...clone(defaultSession.session),
    userId,
    token,
  })

  const getWorkspaceRuntimeDocument = () => {
    const source = workspaceState.runtimeWorkspaceConfig.sources.find(item => item.scope === 'workspace')
    const current = source?.document && typeof source.document === 'object' && !Array.isArray(source.document)
      ? clone(source.document)
      : {}
    return {
      source,
      current,
    }
  }

  const updateWorkspaceRuntimeDocument = (
    transform: (current: Record<string, any>) => Record<string, any>,
  ) => {
    const { source, current } = getWorkspaceRuntimeDocument()
    const next = transform(current)
    if (source) {
      source.document = clone(next)
      source.exists = true
      source.loaded = true
    }
    workspaceState.runtimeWorkspaceConfig = {
      ...workspaceState.runtimeWorkspaceConfig,
      effectiveConfig: clone(next),
      effectiveConfigHash: `${workspaceState.workspace.id}-cfg-hash-${Date.now()}`,
      validation: {
        valid: true,
        errors: [],
        warnings: [],
      },
    }
  }

  const syncManagedToolConfig = () => {
    updateWorkspaceRuntimeDocument((current) => {
      const currentToolCatalog = typeof current.toolCatalog === 'object' && current.toolCatalog && !Array.isArray(current.toolCatalog)
        ? current.toolCatalog
        : {}
      return {
        ...current,
        toolCatalog: {
          ...currentToolCatalog,
          disabledSourceKeys: workspaceState.toolCatalog.entries
            .filter(entry => entry.disabled)
            .map(entry => entry.sourceKey),
        },
        mcpServers: Object.fromEntries(
          Object.values(workspaceState.mcpDocuments).map(document => [document.serverName, clone(document.config)]),
        ),
      }
    })
  }

  const findToolCatalogEntry = (sourceKey: string) =>
    workspaceState.toolCatalog.entries.find(entry => entry.sourceKey === sourceKey)

  const replaceToolCatalogEntry = (entry: WorkspaceToolCatalogEntry) => {
    const index = workspaceState.toolCatalog.entries.findIndex(item => item.id === entry.id)
    if (index === -1) {
      workspaceState.toolCatalog.entries = [...workspaceState.toolCatalog.entries, entry]
      return
    }
    workspaceState.toolCatalog.entries = workspaceState.toolCatalog.entries.map((item, itemIndex) => itemIndex === index ? entry : item)
  }

  const ensureSkillDocument = (skillId: string) => {
    const document = workspaceState.skillDocuments[skillId]
    if (!document) {
      throw new WorkspaceApiError({
        message: 'skill not found',
        status: 404,
        requestId: 'req-skill-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }
    return document
  }

  const ensureSkillFiles = (skillId: string) => {
    const files = workspaceState.skillFiles[skillId]
    if (!files) {
      throw new WorkspaceApiError({
        message: 'skill files not found',
        status: 404,
        requestId: 'req-skill-files-not-found',
        retryable: false,
        code: 'NOT_FOUND',
      })
    }
    return files
  }

  const replaceSkillState = (
    document: WorkspaceSkillDocument,
    files: Record<string, WorkspaceSkillFileDocument>,
    disabled?: boolean,
  ) => {
    workspaceState.skillDocuments = {
      ...workspaceState.skillDocuments,
      [document.id]: clone(document),
    }
    workspaceState.skillFiles = {
      ...workspaceState.skillFiles,
      [document.id]: cloneSkillFiles(files),
    }
    const currentEntry = findToolCatalogEntry(document.sourceKey)
    replaceToolCatalogEntry(
      createSkillCatalogEntry(
        workspaceState.workspace.id,
        document,
        disabled ?? currentEntry?.disabled ?? false,
      ),
    )
  }

  const createManagedSkill = (
    slug: string,
    files?: Record<string, WorkspaceSkillFileDocument>,
  ) => {
    const normalizedSlug = slug.trim()
    const relativePath = `data/skills/${normalizedSlug}/SKILL.md`
    const sourceKey = `skill:${relativePath}`
    if (!normalizedSlug) {
      throw new WorkspaceApiError({
        message: 'skill slug is required',
        status: 400,
        requestId: 'req-skill-slug-required',
        retryable: false,
        code: 'INVALID_INPUT',
      })
    }
    if (findToolCatalogEntry(sourceKey)) {
      throw new WorkspaceApiError({
        message: 'skill already exists',
        status: 409,
        requestId: 'req-skill-exists',
        retryable: false,
        code: 'CONFLICT',
      })
    }

    const skillId = `skill-workspace-${normalizedSlug}`
    const rootPath = `data/skills/${normalizedSlug}`
    const nextFiles = files
      ? cloneSkillFiles(files)
      : {
          'SKILL.md': createSkillFileDocument(skillId, sourceKey, rootPath, 'SKILL.md', {
            content: createSkillTemplate(normalizedSlug),
          }),
        }

    const document = createSkillDocument(
      skillId,
      sourceKey,
      skillNameFromContent(nextFiles['SKILL.md']?.content ?? '', normalizedSlug),
      skillDescriptionFromContent(nextFiles['SKILL.md']?.content ?? '', `Workspace skill ${normalizedSlug}`),
      relativePath,
      true,
      nextFiles,
      relativePath,
    )
    replaceSkillState(document, nextFiles, false)
    syncManagedToolConfig()
    return document
  }

  const ensureWorkspaceOwnedSkillDocument = (skillId: string) => {
    const document = ensureSkillDocument(skillId)
    if (!document.workspaceOwned) {
      throw new WorkspaceApiError({
        message: 'external skills are read-only',
        status: 400,
        requestId: 'req-skill-readonly',
        retryable: false,
        code: 'INVALID_INPUT',
      })
    }
    return document
  }

  return {
    system: {
      async bootstrap() {
        return clone(workspaceState.systemBootstrap)
      },
    },
    auth: {
      async login() {
        const user = workspaceState.users.find(record => record.id === workspaceState.workspace.ownerUserId) ?? workspaceState.users[0]
        return {
          session: buildSession(user?.id ?? 'user-owner'),
          workspace: clone(workspaceState.workspace),
        }
      },
      async registerOwner(request: RegisterWorkspaceOwnerRequest): Promise<RegisterWorkspaceOwnerResponse> {
        const ownerId = 'user-owner'
        const ownerAvatar = `data:${request.avatar.contentType};base64,${request.avatar.dataBase64}`
        const ownerRecord: UserRecordSummary = {
          id: ownerId,
          username: request.username,
          displayName: request.displayName,
          avatar: ownerAvatar,
          status: 'active',
          passwordState: 'set',
          roleIds: ['role-owner'],
          scopeProjectIds: [],
        }

        workspaceState.workspace = {
          ...workspaceState.workspace,
          bootstrapStatus: 'ready',
          ownerUserId: ownerId,
        }
        workspaceState.systemBootstrap = {
          ...workspaceState.systemBootstrap,
          workspace: clone(workspaceState.workspace),
          setupRequired: false,
          ownerReady: true,
        }
        workspaceState.overview = {
          ...workspaceState.overview,
          workspace: clone(workspaceState.workspace),
        }
        workspaceState.users = [
          ownerRecord,
          ...workspaceState.users.filter(record => record.id !== ownerId),
        ]
        workspaceState.userCenterOverview = {
          ...workspaceState.userCenterOverview,
          currentUser: clone(ownerRecord),
          roleNames: ['Owner'],
          metrics: workspaceState.userCenterOverview.metrics.map(metric =>
            metric.id === 'users'
              ? { ...metric, value: String(workspaceState.users.length) }
              : metric),
        }

        return {
          session: buildSession(ownerId, 'token-owner'),
          workspace: clone(workspaceState.workspace),
        }
      },
      async logout() {},
      async session() {
        if (connection.workspaceId === 'ws-local' && options.localSessionValid === false) {
          throw new WorkspaceApiError({
            message: 'session expired',
            status: 401,
            requestId: 'req-fixture-session-expired',
            retryable: false,
            code: 'SESSION_EXPIRED',
          })
        }

        const user = workspaceState.users.find(record => record.id === workspaceState.workspace.ownerUserId) ?? workspaceState.users[0]
        return buildSession(user?.id ?? 'user-owner')
      },
    },
    workspace: {
      async get() {
        return clone(workspaceState.workspace)
      },
      async getOverview() {
        return clone(workspaceState.overview)
      },
    },
    projects: {
      async list() {
        return clone(workspaceState.projects)
      },
      async create(input) {
        const project = createProjectRecord(workspaceState.workspace.id, input)
        workspaceState.projects = [...workspaceState.projects, project]
        workspaceState.dashboards[project.id] = {
          project: clone(project),
          metrics: [],
          recentConversations: [],
          recentActivity: [],
        }
        workspaceState.projectResources[project.id] = []
        workspaceState.projectKnowledge[project.id] = []
        if (!workspaceState.projects.some(item => item.status === 'active' && item.id !== project.id)) {
          workspaceState.workspace = {
            ...workspaceState.workspace,
            defaultProjectId: project.id,
          }
        }
        syncWorkspaceProjectState(workspaceState)
        return clone(project)
      },
      async update(projectId, input) {
        const current = workspaceState.projects.find(project => project.id === projectId)
        if (!current) {
          throw new WorkspaceApiError({
            message: 'project not found',
            status: 404,
            requestId: 'req-project-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        if (!input.name.trim()) {
          throw new WorkspaceApiError({
            message: 'project name is required',
            status: 400,
            requestId: 'req-project-name-required',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        if (current.status === 'active' && input.status === 'archived' && workspaceState.projects.filter(project => project.status === 'active').length <= 1) {
          throw new WorkspaceApiError({
            message: 'cannot archive the last active project',
            status: 400,
            requestId: 'req-project-last-active',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        const updated = updateProjectRecord(current, input)
        workspaceState.projects = workspaceState.projects.map(project => project.id === projectId ? updated : project)
        if (workspaceState.dashboards[projectId]) {
          workspaceState.dashboards[projectId] = {
            ...workspaceState.dashboards[projectId],
            project: clone(updated),
          }
        }
        if (updated.status === 'archived') {
          updateDefaultProjectIfNeeded(workspaceState, projectId)
        }
        syncWorkspaceProjectState(workspaceState)
        return clone(updated)
      },
      async getDashboard(projectId) {
        return clone(workspaceState.dashboards[projectId])
      },
    },
    resources: {
      async listWorkspace() {
        return clone(workspaceState.workspaceResources)
      },
      async listProject(projectId) {
        return clone(workspaceState.projectResources[projectId] ?? [])
      },
    },
    artifacts: {
      async listWorkspace() {
        return clone(workspaceState.artifacts)
      },
    },
    knowledge: {
      async listWorkspace() {
        return clone(workspaceState.workspaceKnowledge)
      },
      async listProject(projectId) {
        return clone(workspaceState.projectKnowledge[projectId] ?? [])
      },
    },
    pet: {
      async getSnapshot(projectId) {
        return clone(createPetSnapshot(workspaceState, projectId))
      },
      async savePresence(input: SavePetPresenceInput, projectId) {
        const current = projectId
          ? workspaceState.projectPetPresences[projectId] ?? createPetPresenceState(input.petId || workspaceState.petProfile.id)
          : workspaceState.workspacePetPresence
        const next = {
          ...current,
          petId: input.petId || current.petId,
          isVisible: input.isVisible ?? current.isVisible,
          chatOpen: input.chatOpen ?? current.chatOpen,
          motionState: input.motionState ?? current.motionState,
          unreadCount: input.unreadCount ?? current.unreadCount,
          lastInteractionAt: input.lastInteractionAt ?? current.lastInteractionAt,
          position: input.position ?? current.position,
        }
        if (projectId) {
          workspaceState.projectPetPresences[projectId] = next
        } else {
          workspaceState.workspacePetPresence = next
        }
        return clone(next)
      },
      async bindConversation(input: BindPetConversationInput, projectId) {
        const binding: PetConversationBinding = {
          petId: input.petId,
          workspaceId: workspaceState.workspace.id,
          projectId: projectId ?? '',
          conversationId: input.conversationId,
          sessionId: input.sessionId,
          updatedAt: Date.now(),
        }
        if (projectId) {
          workspaceState.projectPetBindings[projectId] = binding
        } else {
          workspaceState.workspacePetBinding = binding
        }
        return clone(binding)
      },
    },
    agents: {
      async list() {
        return clone(workspaceState.agents)
      },
      async create(input) {
        const id = `agent-${Date.now()}`
        const record = normalizeAgentRecord(input as UpsertAgentInput, undefined, id)
        workspaceState.agents = [...workspaceState.agents, record]
        return clone(record)
      },
      async update(agentId, input) {
        const current = workspaceState.agents.find(item => item.id === agentId)
        const record = normalizeAgentRecord(input as UpsertAgentInput, current, agentId)
        workspaceState.agents = workspaceState.agents.map(item => item.id === agentId ? record : item)
        return clone(record)
      },
      async delete(agentId) {
        workspaceState.agents = workspaceState.agents.filter(item => item.id !== agentId)
        workspaceState.projectAgentLinks = Object.fromEntries(
          Object.entries(workspaceState.projectAgentLinks).map(([projectId, links]) => [
            projectId,
            links.filter(link => link.agentId !== agentId),
          ]),
        )
      },
      async listProjectLinks(projectId) {
        return clone(workspaceState.projectAgentLinks[projectId] ?? [])
      },
      async linkProject(input) {
        const created: ProjectAgentLinkRecord = {
          workspaceId: workspaceState.workspace.id,
          projectId: input.projectId,
          agentId: input.agentId,
          linkedAt: Date.now(),
        }
        workspaceState.projectAgentLinks[input.projectId] = [
          ...(workspaceState.projectAgentLinks[input.projectId] ?? []).filter(link => link.agentId !== input.agentId),
          created,
        ]
        return clone(created)
      },
      async unlinkProject(projectId, agentId) {
        workspaceState.projectAgentLinks[projectId] = (workspaceState.projectAgentLinks[projectId] ?? [])
          .filter(link => link.agentId !== agentId)
      },
    },
    teams: {
      async list() {
        return clone(workspaceState.teams)
      },
      async create(input) {
        const id = `team-${Date.now()}`
        const record = normalizeTeamRecord(input as UpsertTeamInput, undefined, id)
        workspaceState.teams = [...workspaceState.teams, record]
        return clone(record)
      },
      async update(teamId, input) {
        const current = workspaceState.teams.find(item => item.id === teamId)
        const record = normalizeTeamRecord(input as UpsertTeamInput, current, teamId)
        workspaceState.teams = workspaceState.teams.map(item => item.id === teamId ? record : item)
        return clone(record)
      },
      async delete(teamId) {
        workspaceState.teams = workspaceState.teams.filter(item => item.id !== teamId)
        workspaceState.projectTeamLinks = Object.fromEntries(
          Object.entries(workspaceState.projectTeamLinks).map(([projectId, links]) => [
            projectId,
            links.filter(link => link.teamId !== teamId),
          ]),
        )
      },
      async listProjectLinks(projectId) {
        return clone(workspaceState.projectTeamLinks[projectId] ?? [])
      },
      async linkProject(input) {
        const created: ProjectTeamLinkRecord = {
          workspaceId: workspaceState.workspace.id,
          projectId: input.projectId,
          teamId: input.teamId,
          linkedAt: Date.now(),
        }
        workspaceState.projectTeamLinks[input.projectId] = [
          ...(workspaceState.projectTeamLinks[input.projectId] ?? []).filter(link => link.teamId !== input.teamId),
          created,
        ]
        return clone(created)
      },
      async unlinkProject(projectId, teamId) {
        workspaceState.projectTeamLinks[projectId] = (workspaceState.projectTeamLinks[projectId] ?? [])
          .filter(link => link.teamId !== teamId)
      },
    },
    catalog: {
      async getSnapshot() {
        return clone(workspaceState.catalog)
      },
      async getToolCatalog() {
        return clone(workspaceState.toolCatalog)
      },
      async setToolDisabled(patch: WorkspaceToolDisablePatch) {
        const current = findToolCatalogEntry(patch.sourceKey)
        if (!current) {
          throw new WorkspaceApiError({
            message: 'tool source not found',
            status: 404,
            requestId: 'req-tool-source-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        replaceToolCatalogEntry({
          ...current,
          disabled: patch.disabled,
        })
        syncManagedToolConfig()
        return clone(workspaceState.toolCatalog)
      },
      async getSkill(skillId) {
        return clone(ensureSkillDocument(skillId))
      },
      async getSkillTree(skillId) {
        const document = ensureSkillDocument(skillId)
        return {
          skillId,
          sourceKey: document.sourceKey,
          displayPath: document.rootPath,
          rootPath: document.rootPath,
          tree: clone(document.tree),
        }
      },
      async getSkillFile(skillId: string, relativePath: string) {
        const file = ensureSkillFiles(skillId)[relativePath]
        if (!file) {
          throw new WorkspaceApiError({
            message: 'skill file not found',
            status: 404,
            requestId: 'req-skill-file-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        return clone(file)
      },
      async createSkill(input: CreateWorkspaceSkillInput) {
        const document = createManagedSkill(input.slug, {
          'SKILL.md': createSkillFileDocument(
            `skill-workspace-${input.slug.trim()}`,
            `skill:data/skills/${input.slug.trim()}/SKILL.md`,
            `data/skills/${input.slug.trim()}`,
            'SKILL.md',
            {
              content: input.content,
            },
          ),
        })
        return clone(document)
      },
      async updateSkill(skillId: string, input: UpdateWorkspaceSkillInput) {
        const current = ensureWorkspaceOwnedSkillDocument(skillId)
        const currentFiles = ensureSkillFiles(skillId)
        const nextFiles = cloneSkillFiles(currentFiles)
        nextFiles['SKILL.md'] = {
          ...nextFiles['SKILL.md'],
          content: input.content,
          byteSize: input.content.length,
        }
        const updated = createSkillDocument(
          current.id,
          current.sourceKey,
          skillNameFromContent(input.content, current.name || skillSlugFromRelativePath(current.relativePath) || 'skill'),
          skillDescriptionFromContent(input.content, current.description || 'Workspace skill'),
          current.displayPath,
          true,
          nextFiles,
          current.relativePath,
        )
        replaceSkillState(updated, nextFiles)
        return clone(updated)
      },
      async updateSkillFile(skillId: string, relativePath: string, input: UpdateWorkspaceSkillFileInput) {
        const current = ensureWorkspaceOwnedSkillDocument(skillId)
        const currentFiles = ensureSkillFiles(skillId)
        const file = currentFiles[relativePath]
        if (!file) {
          throw new WorkspaceApiError({
            message: 'skill file not found',
            status: 404,
            requestId: 'req-skill-file-update-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        if (!file.isText || file.readonly) {
          throw new WorkspaceApiError({
            message: 'skill file is read-only',
            status: 400,
            requestId: 'req-skill-file-readonly',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        const nextFiles = cloneSkillFiles(currentFiles)
        nextFiles[relativePath] = {
          ...nextFiles[relativePath],
          content: input.content,
          byteSize: input.content.length,
        }

        const updated = createSkillDocument(
          current.id,
          current.sourceKey,
          skillNameFromContent(nextFiles['SKILL.md']?.content ?? current.content, current.name || skillSlugFromRelativePath(current.relativePath) || 'skill'),
          skillDescriptionFromContent(nextFiles['SKILL.md']?.content ?? current.content, current.description || 'Workspace skill'),
          current.displayPath,
          true,
          nextFiles,
          current.relativePath,
        )
        replaceSkillState(updated, nextFiles)
        return clone(nextFiles[relativePath])
      },
      async importSkillArchive(input: ImportWorkspaceSkillArchiveInput) {
        const skillId = `skill-workspace-${input.slug.trim()}`
        const sourceKey = `skill:data/skills/${input.slug.trim()}/SKILL.md`
        const rootPath = `data/skills/${input.slug.trim()}`
        const files = createImportedSkillFiles(skillId, sourceKey, rootPath, input.slug.trim())
        return clone(createManagedSkill(input.slug, files))
      },
      async importSkillFolder(input: ImportWorkspaceSkillFolderInput) {
        const skillId = `skill-workspace-${input.slug.trim()}`
        const sourceKey = `skill:data/skills/${input.slug.trim()}/SKILL.md`
        const rootPath = `data/skills/${input.slug.trim()}`
        const files = createImportedSkillFiles(skillId, sourceKey, rootPath, input.slug.trim(), input.files)
        return clone(createManagedSkill(input.slug, files))
      },
      async copySkillToManaged(skillId: string, input: CopyWorkspaceSkillToManagedInput) {
        const sourceDocument = ensureSkillDocument(skillId)
        const sourceFiles = ensureSkillFiles(skillId)
        const nextSlug = input.slug.trim()
        const nextSkillId = `skill-workspace-${input.slug.trim()}`
        const nextSourceKey = `skill:data/skills/${input.slug.trim()}/SKILL.md`
        const nextRootPath = `data/skills/${input.slug.trim()}`
        const copiedFiles = Object.fromEntries(
          Object.entries(sourceFiles).map(([path, file]) => {
            const cloned = clone(file)
            return [path, {
              ...cloned,
              skillId: nextSkillId,
              sourceKey: nextSourceKey,
              displayPath: `${nextRootPath}/${path}`,
              content: path === 'SKILL.md' && cloned.content
                ? normalizeSkillFrontmatterName(cloned.content, nextSlug)
                : cloned.content,
              readonly: false,
            } satisfies WorkspaceSkillFileDocument]
          }),
        )
        const document = createManagedSkill(input.slug, copiedFiles)
        if (copiedFiles['SKILL.md']?.content) {
          const nextFiles = ensureSkillFiles(document.id)
          nextFiles['SKILL.md'] = {
            ...nextFiles['SKILL.md'],
            content: copiedFiles['SKILL.md'].content,
            byteSize: copiedFiles['SKILL.md'].byteSize,
          }
          const updated = createSkillDocument(
            document.id,
            document.sourceKey,
            skillNameFromContent(nextFiles['SKILL.md'].content ?? '', sourceDocument.name),
            skillDescriptionFromContent(nextFiles['SKILL.md'].content ?? '', sourceDocument.description),
            document.displayPath,
            true,
            nextFiles,
            document.relativePath,
          )
          replaceSkillState(updated, nextFiles)
          return clone(updated)
        }
        return clone(document)
      },
      async deleteSkill(skillId: string) {
        ensureWorkspaceOwnedSkillDocument(skillId)

        const nextDocuments = { ...workspaceState.skillDocuments }
        delete nextDocuments[skillId]
        const nextFiles = { ...workspaceState.skillFiles }
        delete nextFiles[skillId]
        workspaceState.skillDocuments = nextDocuments
        workspaceState.skillFiles = nextFiles
        workspaceState.toolCatalog.entries = workspaceState.toolCatalog.entries.filter(entry => entry.id !== skillId)
        syncManagedToolConfig()
      },
      async getMcpServer(serverName: string) {
        const document = workspaceState.mcpDocuments[serverName]
        if (!document) {
          throw new WorkspaceApiError({
            message: 'MCP server not found',
            status: 404,
            requestId: 'req-mcp-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        return clone(document)
      },
      async createMcpServer(input: UpsertWorkspaceMcpServerInput) {
        const serverName = input.serverName.trim()
        if (!serverName) {
          throw new WorkspaceApiError({
            message: 'server name is required',
            status: 400,
            requestId: 'req-mcp-name-required',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }
        if (workspaceState.mcpDocuments[serverName]) {
          throw new WorkspaceApiError({
            message: 'MCP server already exists',
            status: 409,
            requestId: 'req-mcp-exists',
            retryable: false,
            code: 'CONFLICT',
          })
        }

        const document: WorkspaceMcpServerDocument = {
          serverName,
          sourceKey: `mcp:${serverName}`,
          displayPath: 'config/runtime/workspace.json',
          scope: 'workspace',
          config: clone(input.config),
        }
        workspaceState.mcpDocuments = {
          ...workspaceState.mcpDocuments,
          [serverName]: document,
        }
        replaceToolCatalogEntry(createMcpCatalogEntry(workspaceState.workspace.id, document))
        syncManagedToolConfig()
        return clone(document)
      },
      async updateMcpServer(serverName: string, input: UpsertWorkspaceMcpServerInput) {
        const current = workspaceState.mcpDocuments[serverName]
        if (!current) {
          throw new WorkspaceApiError({
            message: 'MCP server not found',
            status: 404,
            requestId: 'req-mcp-update-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        if (input.serverName.trim() !== serverName) {
          throw new WorkspaceApiError({
            message: 'renaming MCP servers is not supported',
            status: 400,
            requestId: 'req-mcp-rename-unsupported',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        const updated: WorkspaceMcpServerDocument = {
          ...current,
          config: clone(input.config),
        }
        workspaceState.mcpDocuments = {
          ...workspaceState.mcpDocuments,
          [serverName]: updated,
        }
        const existingEntry = findToolCatalogEntry(current.sourceKey)
        replaceToolCatalogEntry(createMcpCatalogEntry(workspaceState.workspace.id, updated, existingEntry?.disabled ?? false))
        syncManagedToolConfig()
        return clone(updated)
      },
      async deleteMcpServer(serverName: string) {
        if (!workspaceState.mcpDocuments[serverName]) {
          throw new WorkspaceApiError({
            message: 'MCP server not found',
            status: 404,
            requestId: 'req-mcp-delete-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const nextDocuments = { ...workspaceState.mcpDocuments }
        delete nextDocuments[serverName]
        workspaceState.mcpDocuments = nextDocuments
        workspaceState.toolCatalog.entries = workspaceState.toolCatalog.entries.filter(entry => !(entry.kind === 'mcp' && entry.serverName === serverName))
        syncManagedToolConfig()
      },
      async listModels() {
        return clone(workspaceState.catalog.models)
      },
      async listProviderCredentials() {
        return clone(workspaceState.catalog.credentialBindings)
      },
      async listTools() {
        return clone(workspaceState.tools)
      },
      async createTool(record) {
        workspaceState.tools = [...workspaceState.tools, clone(record)]
        return clone(record)
      },
      async updateTool(toolId, record) {
        workspaceState.tools = workspaceState.tools.map(item => item.id === toolId ? clone(record) : item)
        return clone(record)
      },
      async deleteTool(toolId) {
        workspaceState.tools = workspaceState.tools.filter(item => item.id !== toolId)
      },
    },
    automations: {
      async list() {
        return clone(workspaceState.automations)
      },
      async create(record) {
        workspaceState.automations = [...workspaceState.automations, clone(record)]
        return clone(record)
      },
      async update(automationId, record) {
        workspaceState.automations = workspaceState.automations.map(item => item.id === automationId ? clone(record) : item)
        return clone(record)
      },
      async delete(automationId) {
        workspaceState.automations = workspaceState.automations.filter(item => item.id !== automationId)
      },
    },
    rbac: {
      async getUserCenterOverview() {
        return clone(workspaceState.userCenterOverview)
      },
      async listUsers() {
        return clone(workspaceState.users)
      },
      async createUser(record: CreateWorkspaceUserRequest) {
        const created: UserRecordSummary = {
          id: `user-${record.username}`,
          username: record.username,
          displayName: record.displayName,
          avatar: record.useDefaultAvatar || !record.avatar
            ? undefined
            : `data:${record.avatar.contentType};base64,${record.avatar.dataBase64}`,
          status: record.status,
          passwordState: record.useDefaultPassword || !record.password ? 'reset-required' : 'set',
          roleIds: clone(record.roleIds),
          scopeProjectIds: clone(record.scopeProjectIds),
        }
        workspaceState.users = [...workspaceState.users, clone(created)]
        workspaceState.userPasswords = {
          ...workspaceState.userPasswords,
          [created.id]: record.useDefaultPassword || !record.password ? 'changeme' : record.password,
        }
        return clone(created)
      },
      async updateUser(userId, record: UpdateWorkspaceUserRequest) {
        const currentUser = workspaceState.users.find(item => item.id === userId)
        if (!currentUser) {
          throw new WorkspaceApiError({
            message: 'User not found.',
            status: 404,
            requestId: 'req-user-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        const updated: UserRecordSummary = {
          ...currentUser,
          username: record.username,
          displayName: record.displayName,
          avatar: record.removeAvatar
            ? undefined
            : record.avatar
              ? `data:${record.avatar.contentType};base64,${record.avatar.dataBase64}`
              : currentUser.avatar,
          status: record.status,
          passwordState: record.resetPasswordToDefault || (!record.password && currentUser.passwordState === 'reset-required')
            ? 'reset-required'
            : record.password
              ? 'set'
              : currentUser.passwordState,
          roleIds: clone(record.roleIds),
          scopeProjectIds: clone(record.scopeProjectIds),
        }
        workspaceState.users = workspaceState.users.map(item => item.id === userId ? clone(updated) : item)
        if (record.resetPasswordToDefault) {
          workspaceState.userPasswords = {
            ...workspaceState.userPasswords,
            [userId]: 'changeme',
          }
        } else if (record.password) {
          workspaceState.userPasswords = {
            ...workspaceState.userPasswords,
            [userId]: record.password,
          }
        }
        if (workspaceState.userCenterOverview.currentUser.id === userId) {
          workspaceState.userCenterOverview = {
            ...workspaceState.userCenterOverview,
            currentUser: clone(updated),
          }
        }
        return clone(updated)
      },
      async deleteUser(userId) {
        workspaceState.users = workspaceState.users.filter(item => item.id !== userId)
        const nextPasswords = { ...workspaceState.userPasswords }
        delete nextPasswords[userId]
        workspaceState.userPasswords = nextPasswords
      },
      async updateCurrentUserProfile(input: UpdateCurrentUserProfileRequest) {
        const currentUserId = workspaceState.userCenterOverview.currentUser.id
        const currentUser = workspaceState.users.find(user => user.id === currentUserId)
        if (!currentUser) {
          throw new WorkspaceApiError({
            message: 'Current user not found.',
            status: 404,
            requestId: 'req-user-profile-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }

        const updated: UserRecordSummary = {
          ...currentUser,
          username: input.username.trim(),
          displayName: input.displayName.trim(),
          avatar: input.removeAvatar
            ? undefined
            : input.avatar
              ? `data:${input.avatar.contentType};base64,${input.avatar.dataBase64}`
              : currentUser.avatar,
        }
        workspaceState.users = workspaceState.users.map(user => user.id === currentUserId ? clone(updated) : user)
        workspaceState.userCenterOverview = {
          ...workspaceState.userCenterOverview,
          currentUser: clone(updated),
        }
        return clone(updated)
      },
      async changeCurrentUserPassword(input: ChangeCurrentUserPasswordRequest): Promise<ChangeCurrentUserPasswordResponse> {
        const currentUserId = workspaceState.userCenterOverview.currentUser.id
        const currentPassword = workspaceState.userPasswords[currentUserId]
        if (!currentPassword) {
          throw new WorkspaceApiError({
            message: 'Current user not found.',
            status: 404,
            requestId: 'req-user-password-not-found',
            retryable: false,
            code: 'NOT_FOUND',
          })
        }
        if (input.currentPassword !== currentPassword) {
          throw new WorkspaceApiError({
            message: 'Current password is incorrect.',
            status: 400,
            requestId: 'req-user-password-current-invalid',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }
        if (input.newPassword.length < 8) {
          throw new WorkspaceApiError({
            message: 'New password must be at least 8 characters.',
            status: 400,
            requestId: 'req-user-password-too-short',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }
        if (input.newPassword !== input.confirmPassword) {
          throw new WorkspaceApiError({
            message: 'Password confirmation does not match.',
            status: 400,
            requestId: 'req-user-password-confirm-invalid',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }
        if (input.newPassword === input.currentPassword) {
          throw new WorkspaceApiError({
            message: 'New password must be different from the current password.',
            status: 400,
            requestId: 'req-user-password-same',
            retryable: false,
            code: 'INVALID_INPUT',
          })
        }

        workspaceState.userPasswords = {
          ...workspaceState.userPasswords,
          [currentUserId]: input.newPassword,
        }
        workspaceState.users = workspaceState.users.map((user) => {
          if (user.id !== currentUserId) {
            return user
          }
          return {
            ...user,
            passwordState: 'set',
          }
        })
        workspaceState.userCenterOverview = {
          ...workspaceState.userCenterOverview,
          currentUser: {
            ...workspaceState.userCenterOverview.currentUser,
            passwordState: 'set',
          },
        }
        return {
          passwordState: 'set',
        }
      },
      async listRoles() {
        return clone(workspaceState.roles)
      },
      async createRole(record) {
        workspaceState.roles = [...workspaceState.roles, clone(record)]
        return clone(record)
      },
      async updateRole(roleId, record) {
        workspaceState.roles = workspaceState.roles.map(item => item.id === roleId ? clone(record) : item)
        return clone(record)
      },
      async deleteRole(roleId) {
        workspaceState.roles = workspaceState.roles.filter(item => item.id !== roleId)
        workspaceState.users = workspaceState.users.map(user => ({
          ...user,
          roleIds: user.roleIds.filter(id => id !== roleId),
        }))
        workspaceState.userCenterOverview = {
          ...workspaceState.userCenterOverview,
          currentUser: {
            ...workspaceState.userCenterOverview.currentUser,
            roleIds: workspaceState.userCenterOverview.currentUser.roleIds.filter(id => id !== roleId),
          },
          roleNames: workspaceState.userCenterOverview.roleNames.filter(name =>
            workspaceState.roles.some(role => role.name === name),
          ),
        }
      },
      async listPermissions() {
        return clone(workspaceState.permissions)
      },
      async createPermission(record) {
        workspaceState.permissions = [...workspaceState.permissions, clone(record)]
        return clone(record)
      },
      async updatePermission(permissionId, record) {
        workspaceState.permissions = workspaceState.permissions.map(item => item.id === permissionId ? clone(record) : item)
        return clone(record)
      },
      async deletePermission(permissionId) {
        workspaceState.permissions = workspaceState.permissions
          .filter(item => item.id !== permissionId)
          .map(permission => ({
            ...permission,
            memberPermissionIds: permission.memberPermissionIds.filter(id => id !== permissionId),
          }))
        workspaceState.roles = workspaceState.roles.map(role => ({
          ...role,
          permissionIds: role.permissionIds.filter(id => id !== permissionId),
        }))
      },
      async listMenus() {
        return clone(workspaceState.menus)
      },
      async createMenu(record) {
        workspaceState.menus = [...workspaceState.menus, clone(record)]
        return clone(record)
      },
      async updateMenu(menuId, record) {
        workspaceState.menus = workspaceState.menus.map(item => item.id === menuId ? clone(record) : item)
        return clone(record)
      },
    },
    runtime: {
      async bootstrap(): Promise<RuntimeBootstrap> {
        return {
          provider: {
            providerId: 'anthropic',
            defaultModel: 'claude-sonnet-4-5',
            defaultSurface: 'conversation',
            protocolFamily: 'anthropic_messages',
          },
          sessions: [...workspaceState.runtimeSessions.values()].map(state => clone(state.detail.summary)),
        }
      },
      async getConfig(): Promise<RuntimeEffectiveConfig> {
        return clone(workspaceState.runtimeWorkspaceConfig)
      },
      async validateConfig(_patch: RuntimeConfigPatch): Promise<RuntimeConfigValidationResult> {
        return {
          valid: true,
          errors: [],
          warnings: [],
        }
      },
      async validateConfiguredModel(input) {
        const configuredModel = workspaceState.catalog.configuredModels.find(model => model.configuredModelId === input.configuredModelId)
        return {
          valid: true,
          reachable: true,
          configuredModelId: input.configuredModelId,
          configuredModelName: configuredModel?.name,
          requestId: 'fixture-probe-request',
          consumedTokens: 8,
          errors: [],
          warnings: [],
        }
      },
      async saveConfig(patch: RuntimeConfigPatch): Promise<RuntimeEffectiveConfig> {
        const source = workspaceState.runtimeWorkspaceConfig.sources.find(item => item.scope === 'workspace')
        if (source) {
          const current = (source.document ?? {}) as Record<string, any>
          source.document = applyJsonMergePatch(current, patch.patch as Record<string, any>)
          source.exists = true
          source.loaded = true
        }

        workspaceState.runtimeWorkspaceConfig = {
          ...workspaceState.runtimeWorkspaceConfig,
          effectiveConfig: applyJsonMergePatch(
            workspaceState.runtimeWorkspaceConfig.effectiveConfig as Record<string, any>,
            patch.patch as Record<string, any>,
          ),
          effectiveConfigHash: `${workspaceState.workspace.id}-cfg-hash-${Date.now()}`,
          validation: {
            valid: true,
            errors: [],
            warnings: [],
          },
        }

        return clone(workspaceState.runtimeWorkspaceConfig)
      },
      async getProjectConfig(projectId: string): Promise<RuntimeEffectiveConfig> {
        return clone(workspaceState.runtimeProjectConfigs[projectId])
      },
      async validateProjectConfig(_projectId: string, _patch: RuntimeConfigPatch): Promise<RuntimeConfigValidationResult> {
        return {
          valid: true,
          errors: [],
          warnings: [],
        }
      },
      async saveProjectConfig(projectId: string, patch: RuntimeConfigPatch): Promise<RuntimeEffectiveConfig> {
        const config = workspaceState.runtimeProjectConfigs[projectId]
        const source = config.sources.find(item => item.scope === 'project')
        if (source) {
          const current = (source.document ?? {}) as Record<string, any>
          source.document = applyJsonMergePatch(current, patch.patch as Record<string, any>)
          source.exists = true
          source.loaded = true
        }
        workspaceState.runtimeProjectConfigs[projectId] = {
          ...config,
          effectiveConfig: applyJsonMergePatch(
            config.effectiveConfig as Record<string, any>,
            patch.patch as Record<string, any>,
          ),
          effectiveConfigHash: `${workspaceState.workspace.id}-${projectId}-project-cfg-hash-${Date.now()}`,
          validation: {
            valid: true,
            errors: [],
            warnings: [],
          },
        }
        return clone(workspaceState.runtimeProjectConfigs[projectId])
      },
      async getUserConfig(): Promise<RuntimeEffectiveConfig> {
        return clone(workspaceState.runtimeUserConfig)
      },
      async validateUserConfig(_patch: RuntimeConfigPatch): Promise<RuntimeConfigValidationResult> {
        return {
          valid: true,
          errors: [],
          warnings: [],
        }
      },
      async saveUserConfig(patch: RuntimeConfigPatch): Promise<RuntimeEffectiveConfig> {
        const source = workspaceState.runtimeUserConfig.sources.find(item => item.scope === 'user')
        if (source) {
          const current = (source.document ?? {}) as Record<string, any>
          source.document = applyJsonMergePatch(current, patch.patch as Record<string, any>)
          source.exists = true
          source.loaded = true
        }
        workspaceState.runtimeUserConfig = {
          ...workspaceState.runtimeUserConfig,
          effectiveConfig: applyJsonMergePatch(
            workspaceState.runtimeUserConfig.effectiveConfig as Record<string, any>,
            patch.patch as Record<string, any>,
          ),
          effectiveConfigHash: `${workspaceState.workspace.id}-user-owner-runtime-cfg-hash-${Date.now()}`,
          validation: {
            valid: true,
            errors: [],
            warnings: [],
          },
        }
        return clone(workspaceState.runtimeUserConfig)
      },
      async listSessions(): Promise<RuntimeSessionSummary[]> {
        return [...workspaceState.runtimeSessions.values()].map(state => clone(state.detail.summary))
      },
      async createSession(input) {
        const existing = [...workspaceState.runtimeSessions.values()].find(state => state.detail.summary.conversationId === input.conversationId)
        if (existing) {
          return clone(existing.detail)
        }

        const detail = createSessionDetail(input.conversationId, input.projectId, input.title, input.sessionKind ?? 'project')
        const state: RuntimeSessionState = {
          detail,
          events: [],
          nextSequence: 1,
        }
        workspaceState.runtimeSessions.set(detail.summary.id, state)
        return clone(detail)
      },
      async loadSession(sessionId) {
        return clone(ensureRuntimeState(sessionId).detail)
      },
      async pollEvents(sessionId, options = {}) {
        return clone(eventsAfter(ensureRuntimeState(sessionId), options.after))
      },
      async subscribeEvents(sessionId, options) {
        const state = ensureRuntimeState(sessionId)
        const timers = eventsAfter(state, options.lastEventId).map((event, index) => window.setTimeout(() => {
          options.onEvent(clone(event))
        }, index * 5))

        return {
          mode: 'sse' as const,
          close: () => {
            timers.forEach(timer => window.clearTimeout(timer))
          },
        }
      },
      async submitUserTurn(sessionId, input) {
        const state = ensureRuntimeState(sessionId)
        const permissionMode = resolveRuntimePermissionMode(input.permissionMode)
        const configuredModelId = input.configuredModelId ?? input.modelId ?? 'anthropic-primary'
        const configuredModel = workspaceState.catalog.configuredModels.find(model => model.configuredModelId === configuredModelId)
        const registryModelId = configuredModel?.modelId ?? input.modelId ?? 'claude-sonnet-4-5'
        const configuredModelName = configuredModel?.name ?? registryModelId
        const requestedActorKind = input.actorKind ?? 'agent'
        const requestedActorId = input.actorId ?? 'agent-architect'
        const actorRecord = requestedActorKind === 'team'
          ? workspaceState.teams.find(team => team.id === requestedActorId)
          : workspaceState.agents.find(agent => agent.id === requestedActorId)
        const actorLabel = actorRecord
          ? `${actorRecord.name} · ${requestedActorKind === 'team' ? 'Team' : 'Agent'}`
          : '默认智能体'
        const userMessage = createRuntimeMessage(
          state,
          'user',
          'You',
          input.content,
          registryModelId,
          configuredModelId,
          configuredModelName,
          requestedActorKind,
          requestedActorId,
        )
        state.detail.messages.push(userMessage)
        state.detail.summary.lastMessagePreview = input.content
        state.detail.summary.updatedAt = userMessage.timestamp
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(userMessage) }))

        const requiresApproval = permissionMode === 'workspace-write'
          && /run pwd|bash pwd|workspace terminal/i.test(input.content)

        if (requiresApproval) {
          const approval = createApproval(state)
          const pendingTrace = createTraceItem(state, 'Awaiting approval before running the terminal command.', 'warning', requestedActorKind, requestedActorId, actorLabel)
          state.detail.pendingApproval = approval
          state.detail.trace.push(pendingTrace)
          state.detail.run = {
            ...state.detail.run,
            status: 'waiting_approval',
            currentStep: 'runtime.run.waitingApproval',
            updatedAt: approval.createdAt,
            configuredModelId,
            configuredModelName,
            modelId: registryModelId,
            nextAction: 'runtime.run.awaitingApproval',
            requestedActorKind,
            requestedActorId,
            resolvedActorKind: requestedActorKind,
            resolvedActorId: requestedActorId,
            resolvedActorLabel: actorLabel,
          }
          state.detail.summary.status = 'waiting_approval'
          state.detail.summary.updatedAt = approval.createdAt
          state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.approval.requested', {
            approval: clone(approval),
            run: clone(state.detail.run),
          }))
          state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.trace.emitted', { trace: clone(pendingTrace) }))
          return clone(state.detail.run)
        }

        const modeLabel = permissionMode === 'read-only'
          ? 'read-only'
          : permissionMode === 'danger-full-access'
            ? 'danger-full-access'
            : 'workspace-write'
        const assistantMessage = createRuntimeMessage(
          state,
          'assistant',
          actorLabel,
          `Completed request in ${modeLabel} mode.`,
          registryModelId,
          configuredModelId,
          configuredModelName,
          requestedActorKind,
          requestedActorId,
        )
        const trace = createTraceItem(state, `Executed with ${modeLabel}.`, 'success', requestedActorKind, requestedActorId, actorLabel)

        state.detail.messages.push(assistantMessage)
        state.detail.trace.push(trace)
        state.detail.run = {
          ...state.detail.run,
          status: 'running',
          currentStep: 'runtime.run.processing',
          updatedAt: assistantMessage.timestamp,
          configuredModelId,
          configuredModelName,
          modelId: registryModelId,
          nextAction: 'runtime.run.processing',
          requestedActorKind,
          requestedActorId,
          resolvedActorKind: requestedActorKind,
          resolvedActorId: requestedActorId,
          resolvedActorLabel: actorLabel,
        }
        const immediateRun: RuntimeRunSnapshot = clone(state.detail.run)
        state.detail.summary.status = 'running'
        state.detail.summary.updatedAt = assistantMessage.timestamp
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(assistantMessage) }))
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.trace.emitted', { trace: clone(trace) }))
        state.detail.run = {
          ...state.detail.run,
          status: 'completed',
          currentStep: 'runtime.run.completed',
          nextAction: 'runtime.run.idle',
          updatedAt: assistantMessage.timestamp + 10,
        }
        state.detail.summary.status = 'completed'
        state.detail.summary.updatedAt = state.detail.run.updatedAt
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.run.updated', { run: clone(state.detail.run) }))
        return immediateRun
      },
      async resolveApproval(sessionId, approvalId, input) {
        const state = ensureRuntimeState(sessionId)
        if (state.detail.pendingApproval?.id !== approvalId) {
          return
        }

        const resolutionStatus = input.decision === 'approve' ? 'approved' : 'rejected'
        const resolvedApproval: RuntimeApprovalRequest = {
          ...state.detail.pendingApproval,
          status: resolutionStatus,
        }
        const runStatus = input.decision === 'approve' ? 'completed' : 'blocked'
        state.detail.pendingApproval = undefined
        state.detail.run = {
          ...state.detail.run,
          status: runStatus,
          currentStep: input.decision === 'approve' ? 'runtime.run.completed' : 'runtime.run.blocked',
          updatedAt: state.detail.run.updatedAt + 10,
          nextAction: input.decision === 'approve' ? 'runtime.run.idle' : 'runtime.run.awaitingInput',
        }
        state.detail.summary.status = runStatus
        state.detail.summary.updatedAt = state.detail.run.updatedAt
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.approval.resolved', {
          approval: clone(resolvedApproval),
          decision: input.decision,
          run: clone(state.detail.run),
        }))

        if (input.decision === 'approve') {
          const assistantMessage = createRuntimeMessage(
            state,
            'assistant',
            state.detail.run.resolvedActorLabel ?? 'Octopus Runtime',
            'Command approved and execution completed.',
            state.detail.run.modelId,
            state.detail.run.configuredModelId,
            state.detail.run.configuredModelName,
            state.detail.run.resolvedActorKind ?? 'agent',
            state.detail.run.resolvedActorId ?? 'agent-architect',
          )
          const trace = createTraceItem(
            state,
            'Approval granted, command executed.',
            'success',
            state.detail.run.resolvedActorKind ?? 'agent',
            state.detail.run.resolvedActorId ?? 'agent-architect',
            state.detail.run.resolvedActorLabel ?? 'Octopus Runtime',
          )
          state.detail.messages.push(assistantMessage)
          state.detail.trace.push(trace)
          state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(assistantMessage) }))
          state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.trace.emitted', { trace: clone(trace) }))
        }
      },
    },
  }
}

export function installWorkspaceApiFixture(options: FixtureOptions = {}): void {
  const hostBootstrap = createHostBootstrap()
  const workspaceStates = new Map(
    WORKSPACE_CONNECTIONS.map(connection => [connection.workspaceConnectionId, createWorkspaceFixtureState(connection, options)]),
  )

  const syncStoredSessions = () => {
    if (typeof window === 'undefined') {
      return
    }

    if (options.preloadWorkspaceSessions === false) {
      window.localStorage.removeItem('octopus-workspace-sessions')
      return
    }

    window.localStorage.setItem('octopus-workspace-sessions', JSON.stringify(
      Object.fromEntries(WORKSPACE_SESSIONS.map(session => [session.workspaceConnectionId, session])),
    ))
  }

  if (options.preloadConversationMessages) {
    const state = workspaceStates.get('conn-local')
    if (state) {
      const detail = createSessionDetail('conv-redesign', 'proj-redesign', 'Conversation Redesign')
      const runtimeState: RuntimeSessionState = {
        detail,
        events: [],
        nextSequence: 1,
      }
      const preloadedMessages = [
        createRuntimeMessage(runtimeState, 'user', 'You', '请先查看当前桌面端实现状态'),
        (() => {
          runtimeState.nextSequence += 1
          return createRuntimeMessage(runtimeState, 'assistant', 'Studio Direction Team · Team', '建议先把 schema、共享 UI 和工作台布局拆开', 'gpt-4o', 'gpt-4o', 'GPT-4o', 'team', 'team-studio')
        })(),
        (() => {
          runtimeState.nextSequence += 1
          return createRuntimeMessage(runtimeState, 'assistant', 'Architect Agent · Agent', 'Thinking...', 'gpt-4o', 'gpt-4o', 'GPT-4o', 'agent', 'agent-architect')
        })(),
      ]
      runtimeState.nextSequence += 1
      runtimeState.detail.messages = preloadedMessages
      runtimeState.detail.summary.lastMessagePreview = preloadedMessages.at(-1)?.content
      runtimeState.detail.summary.updatedAt = 90
      runtimeState.detail.run = {
        ...runtimeState.detail.run,
        status: 'completed',
        currentStep: 'runtime.run.completed',
        updatedAt: 90,
        nextAction: 'runtime.run.idle',
        modelId: 'gpt-4o',
      }
      runtimeState.detail.summary.status = 'completed'
      state.runtimeSessions.set(runtimeState.detail.summary.id, runtimeState)
    }
  }

  if (options.localRuntimeConfigTransform) {
    const localState = workspaceStates.get('conn-local')
    if (localState) {
      localState.runtimeWorkspaceConfig = options.localRuntimeConfigTransform(clone(localState.runtimeWorkspaceConfig))
    }
  }

  syncStoredSessions()

  vi.spyOn(tauriClient, 'bootstrapShellHost').mockImplementation(async () => {
    syncStoredSessions()
    return clone(hostBootstrap)
  })
  vi.spyOn(tauriClient, 'savePreferences').mockImplementation(async (preferences) => clone(preferences))
  vi.spyOn(tauriClient, 'getHostUpdateStatus').mockResolvedValue({
    currentVersion: hostBootstrap.hostState.appVersion,
    currentChannel: hostBootstrap.preferences.updateChannel,
    state: 'idle',
    latestRelease: null,
    lastCheckedAt: null,
    progress: null,
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
  })
  vi.spyOn(tauriClient, 'checkHostUpdate').mockImplementation(async (channel) => ({
    currentVersion: hostBootstrap.hostState.appVersion,
    currentChannel: channel ?? hostBootstrap.preferences.updateChannel,
    state: 'update_available',
    latestRelease: {
      version: '0.2.1',
      channel: channel ?? hostBootstrap.preferences.updateChannel,
      publishedAt: '2026-04-09T08:00:00.000Z',
      notes: '本次更新聚焦版本中心、更新流程和更清晰的产品化说明。',
      notesUrl: 'https://example.test/releases/0.2.1',
    },
    lastCheckedAt: 1_710_000_000_000,
    progress: null,
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
  }))
  vi.spyOn(tauriClient, 'downloadHostUpdate').mockResolvedValue({
    currentVersion: hostBootstrap.hostState.appVersion,
    currentChannel: hostBootstrap.preferences.updateChannel,
    state: 'downloaded',
    latestRelease: {
      version: '0.2.1',
      channel: hostBootstrap.preferences.updateChannel,
      publishedAt: '2026-04-09T08:00:00.000Z',
      notes: '本次更新聚焦版本中心、更新流程和更清晰的产品化说明。',
      notesUrl: 'https://example.test/releases/0.2.1',
    },
    lastCheckedAt: 1_710_000_000_000,
    progress: {
      downloadedBytes: 1024,
      totalBytes: 1024,
      percent: 100,
    },
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
  })
  vi.spyOn(tauriClient, 'installHostUpdate').mockResolvedValue({
    currentVersion: hostBootstrap.hostState.appVersion,
    currentChannel: hostBootstrap.preferences.updateChannel,
    state: 'installing',
    latestRelease: {
      version: '0.2.1',
      channel: hostBootstrap.preferences.updateChannel,
      publishedAt: '2026-04-09T08:00:00.000Z',
      notes: '本次更新聚焦版本中心、更新流程和更清晰的产品化说明。',
      notesUrl: 'https://example.test/releases/0.2.1',
    },
    lastCheckedAt: 1_710_000_000_000,
    progress: {
      downloadedBytes: 1024,
      totalBytes: 1024,
      percent: 100,
    },
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
  })
  vi.spyOn(tauriClient, 'healthcheck').mockResolvedValue({
    backend: { state: 'ready', transport: 'http' },
  })
  vi.spyOn(tauriClient, 'pickSkillArchive').mockResolvedValue(null)
  vi.spyOn(tauriClient, 'pickSkillFolder').mockResolvedValue(null)
  vi.spyOn(tauriClient, 'listNotifications').mockResolvedValue({
    notifications: [],
    unread: {
      total: 0,
      byScope: {
        app: 0,
        workspace: 0,
        user: 0,
      },
    },
  })
  vi.spyOn(tauriClient, 'createNotification').mockImplementation(async (input) => ({
    id: `notif-${Date.now()}`,
    scopeKind: input.scopeKind,
    scopeOwnerId: input.scopeOwnerId,
    level: input.level ?? 'info',
    title: input.title ?? 'Notification',
    body: input.body ?? '',
    source: input.source ?? 'fixture',
    createdAt: Date.now(),
    toastVisibleUntil: input.toastDurationMs ? Date.now() + input.toastDurationMs : undefined,
    routeTo: input.routeTo,
    actionLabel: input.actionLabel,
    readAt: undefined,
  }))
  vi.spyOn(tauriClient, 'markNotificationRead').mockImplementation(async (id) => ({
    id,
    scopeKind: 'app',
    level: 'info',
    title: 'Notification',
    body: '',
    source: 'fixture',
    createdAt: Date.now(),
    readAt: Date.now(),
    toastVisibleUntil: undefined,
  }))
  vi.spyOn(tauriClient, 'markAllNotificationsRead').mockResolvedValue({
    total: 0,
    byScope: {
      app: 0,
      workspace: 0,
      user: 0,
    },
  })
  vi.spyOn(tauriClient, 'dismissNotificationToast').mockImplementation(async (id) => ({
    id,
    scopeKind: 'app',
    level: 'info',
    title: 'Notification',
    body: '',
    source: 'fixture',
    createdAt: Date.now(),
    readAt: undefined,
    toastVisibleUntil: undefined,
  }))
  vi.spyOn(tauriClient, 'subscribeToNotifications').mockImplementation(() => () => {})
  vi.spyOn(tauriClient, 'restartDesktopBackend').mockResolvedValue({
    baseUrl: hostBootstrap.backend?.baseUrl ?? 'http://127.0.0.1:43127',
    authToken: hostBootstrap.backend?.authToken,
    state: 'ready',
    transport: 'http',
  })
  vi.spyOn(tauriClient, 'createWorkspaceClient').mockImplementation(({ connection }) => {
    const workspaceState = workspaceStates.get(connection.workspaceConnectionId)
    if (!workspaceState) {
      throw new Error(`Unknown workspace connection ${connection.workspaceConnectionId}`)
    }
    return createWorkspaceClientFixture(connection, workspaceState, options) as unknown as ReturnType<typeof tauriClient.createWorkspaceClient>
  })
}
