import { vi } from 'vitest'

import type {
  AgentRecord,
  AutomationRecord,
  KnowledgeRecord,
  LoginResponse,
  MenuRecord,
  ModelCatalogSnapshot,
  PermissionRecord,
  ProjectDashboardSnapshot,
  ProjectRecord,
  ProviderCredentialRecord,
  RegisterWorkspaceOwnerRequest,
  RegisterWorkspaceOwnerResponse,
  RoleRecord,
  RuntimeApprovalRequest,
  RuntimeBootstrap,
  RuntimeConfigPatch,
  RuntimeConfigValidationResult,
  RuntimeEventEnvelope,
  RuntimeEffectiveConfig,
  RuntimeMessage,
  RuntimeRunSnapshot,
  RuntimeSessionDetail,
  RuntimeSessionSummary,
  RuntimeTraceItem,
  ShellBootstrap,
  SystemBootstrapStatus,
  TeamRecord,
  ToolRecord,
  UserCenterOverviewSnapshot,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceOverviewSnapshot,
  WorkspaceResourceRecord,
  WorkspaceToolCatalogSnapshot,
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
  workspaceKnowledge: KnowledgeRecord[]
  projectKnowledge: Record<string, KnowledgeRecord[]>
  agents: AgentRecord[]
  teams: TeamRecord[]
  catalog: ModelCatalogSnapshot
  toolCatalog: WorkspaceToolCatalogSnapshot
  tools: ToolRecord[]
  automations: AutomationRecord[]
  userCenterOverview: UserCenterOverviewSnapshot
  users: UserRecordSummary[]
  roles: RoleRecord[]
  permissions: PermissionRecord[]
  menus: MenuRecord[]
  runtimeSessions: Map<string, RuntimeSessionState>
  runtimeWorkspaceConfig: RuntimeEffectiveConfig
  runtimeProjectConfigs: Record<string, RuntimeEffectiveConfig>
  runtimeUserConfig: RuntimeEffectiveConfig
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
        status: 'configured',
        updatedAt: 102,
        tags: ['api'],
      },
    ],
  ]))

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
          title: 'Compliance lead',
          description: 'Reviews launch and compliance readiness.',
          status: 'active',
          updatedAt: 120,
        },
      ]

  const teams: TeamRecord[] = local
    ? [
        {
          id: 'team-studio',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Studio Direction Team',
          description: 'Owns shared UX and shell direction.',
          status: 'active',
          memberIds: ['agent-architect', 'agent-coder'],
          updatedAt: 100,
        },
        {
          id: 'team-redesign',
          workspaceId: workspace.id,
          projectId: 'proj-redesign',
          scope: 'project',
          name: 'Redesign Tiger Team',
          description: 'Executes the desktop migration.',
          status: 'active',
          memberIds: ['agent-redesign'],
          updatedAt: 99,
        },
      ]
    : [
        {
          id: 'team-launch',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Launch Readiness Team',
          description: 'Coordinates enterprise rollout.',
          status: 'active',
          memberIds: ['agent-gov'],
          updatedAt: 120,
        },
      ]

  const providerCredentials: ProviderCredentialRecord[] = [
    {
      id: `${workspace.id}-credential-openai`,
      workspaceId: workspace.id,
      provider: 'openai',
      name: 'OpenAI Primary',
      baseUrl: 'https://api.openai.com/v1',
      status: 'healthy',
    },
  ]

  const catalog: ModelCatalogSnapshot = {
    models: [
      {
        id: 'gpt-4o',
        workspaceId: workspace.id,
        label: 'GPT-4o',
        provider: 'openai',
        description: 'Balanced model for interactive work.',
        recommendedFor: 'General desktop orchestration',
        availability: 'healthy',
        defaultPermission: 'auto',
      },
      {
        id: 'claude-sonnet-4-5',
        workspaceId: workspace.id,
        label: 'Claude Sonnet 4.5',
        provider: 'anthropic',
        description: 'Runtime-heavy work and reasoning.',
        recommendedFor: 'Runtime sessions',
        availability: 'configured',
        defaultPermission: 'readonly',
      },
    ],
    providerCredentials,
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
        builtinKey: 'bash',
      },
      {
        id: 'skill-help',
        workspaceId: workspace.id,
        kind: 'skill',
        name: 'help',
        description: 'Helpful local skill.',
        availability: 'healthy',
        requiredPermission: 'readonly',
        sourceKey: 'skill:help',
        displayPath: '~/.codex/skills/help/SKILL.md',
        active: true,
        sourceOrigin: 'skills_dir',
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
        serverName: 'ops',
        endpoint: 'https://ops.example.test/mcp',
        toolNames: ['mcp__ops__tail_logs'],
        statusDetail: 'MCP handshake timed out',
        scope: 'workspace',
      },
    ],
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
          displayName: local ? 'Lobster Owner' : 'Enterprise Owner',
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
        'menu-workspace-user-center-profile',
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
        'menu-workspace-user-center-profile',
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
        ...clone(runtimeWorkspaceConfig.effectiveConfig),
        approvals: {
          defaultMode: 'manual',
        },
      },
      effectiveConfigHash: `${workspace.id}-${project.id}-project-cfg-hash-1`,
      sources: [
        createRuntimeConfigSource('workspace', workspace.id),
        createRuntimeConfigSource('project', workspace.id, project.id),
      ],
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
      ...clone(runtimeWorkspaceConfig.effectiveConfig),
      provider: {
        defaultModel: 'claude-sonnet-4-5',
      },
    },
    effectiveConfigHash: `${workspace.id}-user-owner-runtime-cfg-hash-1`,
    sources: [
      createRuntimeConfigSource('workspace', workspace.id),
      createRuntimeConfigSource('user', workspace.id, 'user-owner'),
    ],
    validation: {
      valid: true,
      errors: [],
      warnings: [],
    },
    secretReferences: [],
  }

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
    workspaceKnowledge,
    projectKnowledge,
    agents,
    teams,
    catalog,
    toolCatalog,
    tools,
    automations,
    userCenterOverview,
    users,
    roles,
    permissions,
    menus,
    runtimeSessions: new Map(),
    runtimeWorkspaceConfig,
    runtimeProjectConfigs,
    runtimeUserConfig,
  }
}

function createSessionDetail(conversationId: string, projectId: string, title: string): RuntimeSessionDetail {
  const sessionId = `rt-${conversationId}`
  return {
    summary: {
      id: sessionId,
      conversationId,
      projectId,
      title,
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
    modelId,
    status: state.detail.run.status,
  }
}

function createTraceItem(
  state: RuntimeSessionState,
  detail: string,
  tone: RuntimeTraceItem['tone'] = 'info',
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
    actor: 'Octopus Runtime',
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
    knowledge: {
      async listWorkspace() {
        return clone(workspaceState.workspaceKnowledge)
      },
      async listProject(projectId) {
        return clone(workspaceState.projectKnowledge[projectId] ?? [])
      },
    },
    agents: {
      async list() {
        return clone(workspaceState.agents)
      },
      async create(record) {
        workspaceState.agents = [...workspaceState.agents, clone(record)]
        return clone(record)
      },
      async update(agentId, record) {
        workspaceState.agents = workspaceState.agents.map(item => item.id === agentId ? clone(record) : item)
        return clone(record)
      },
      async delete(agentId) {
        workspaceState.agents = workspaceState.agents.filter(item => item.id !== agentId)
      },
    },
    teams: {
      async list() {
        return clone(workspaceState.teams)
      },
      async create(record) {
        workspaceState.teams = [...workspaceState.teams, clone(record)]
        return clone(record)
      },
      async update(teamId, record) {
        workspaceState.teams = workspaceState.teams.map(item => item.id === teamId ? clone(record) : item)
        return clone(record)
      },
      async delete(teamId) {
        workspaceState.teams = workspaceState.teams.filter(item => item.id !== teamId)
      },
    },
    catalog: {
      async getSnapshot() {
        return clone(workspaceState.catalog)
      },
      async getToolCatalog() {
        return clone(workspaceState.toolCatalog)
      },
      async listModels() {
        return clone(workspaceState.catalog.models)
      },
      async listProviderCredentials() {
        return clone(workspaceState.catalog.providerCredentials)
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
      async createUser(record) {
        workspaceState.users = [...workspaceState.users, clone(record)]
        return clone(record)
      },
      async updateUser(userId, record) {
        workspaceState.users = workspaceState.users.map(item => item.id === userId ? clone(record) : item)
        if (workspaceState.userCenterOverview.currentUser.id === userId) {
          workspaceState.userCenterOverview = {
            ...workspaceState.userCenterOverview,
            currentUser: clone(record),
          }
        }
        return clone(record)
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
            provider: 'anthropic',
            defaultModel: 'claude-sonnet-4-5',
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

        const detail = createSessionDetail(input.conversationId, input.projectId, input.title)
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
        const userMessage = createRuntimeMessage(state, 'user', 'You', input.content, input.modelId)
        state.detail.messages.push(userMessage)
        state.detail.summary.lastMessagePreview = input.content
        state.detail.summary.updatedAt = userMessage.timestamp
        state.events.push(createEvent(state, workspaceState.workspace.id, 'runtime.message.created', { message: clone(userMessage) }))

        const requiresApproval = permissionMode === 'workspace-write'
          && /run pwd|bash pwd|workspace terminal/i.test(input.content)

        if (requiresApproval) {
          const approval = createApproval(state)
          const pendingTrace = createTraceItem(state, 'Awaiting approval before running the terminal command.', 'warning')
          state.detail.pendingApproval = approval
          state.detail.trace.push(pendingTrace)
          state.detail.run = {
            ...state.detail.run,
            status: 'waiting_approval',
            currentStep: 'runtime.run.waitingApproval',
            updatedAt: approval.createdAt,
            modelId: input.modelId,
            nextAction: 'runtime.run.awaitingApproval',
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
          'Octopus Runtime',
          `Completed request in ${modeLabel} mode.`,
          input.modelId,
        )
        const trace = createTraceItem(state, `Executed with ${modeLabel}.`, 'success')

        state.detail.messages.push(assistantMessage)
        state.detail.trace.push(trace)
        state.detail.run = {
          ...state.detail.run,
          status: 'running',
          currentStep: 'runtime.run.processing',
          updatedAt: assistantMessage.timestamp,
          modelId: input.modelId,
          nextAction: 'runtime.run.processing',
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
            'Octopus Runtime',
            'Command approved and execution completed.',
            state.detail.run.modelId,
          )
          const trace = createTraceItem(state, 'Approval granted, command executed.', 'success')
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
          return createRuntimeMessage(runtimeState, 'assistant', 'Octopus Runtime', '建议先把 schema、共享 UI 和工作台布局拆开')
        })(),
        (() => {
          runtimeState.nextSequence += 1
          return createRuntimeMessage(runtimeState, 'assistant', 'Octopus Runtime', 'Thinking...')
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
  vi.spyOn(tauriClient, 'healthcheck').mockResolvedValue({
    backend: { state: 'ready', transport: 'http' },
  })
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
