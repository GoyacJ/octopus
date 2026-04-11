import type {
  AccessRoleRecord,
  AgentRecord,
  ArtifactRecord,
  AutomationRecord,
  CredentialBinding,
  DataPolicyRecord,
  KnowledgeRecord,
  MenuDefinition,
  MenuPolicyRecord,
  ModelCatalogSnapshot,
  OrgUnitRecord,
  PermissionDefinition,
  PetConversationBinding,
  PetPresenceState,
  PetProfile,
  PositionRecord,
  ProjectAgentLinkRecord,
  ProjectDashboardSnapshot,
  ProjectRecord,
  ProjectTeamLinkRecord,
  RoleBindingRecord,
  RuntimeEffectiveConfig,
  SystemBootstrapStatus,
  TeamRecord,
  ToolRecord,
  UserGroupRecord,
  UserOrgAssignmentRecord,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceMcpServerDocument,
  WorkspaceOverviewSnapshot,
  WorkspaceResourceRecord,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceToolCatalogEntry,
  WorkspaceToolCatalogSnapshot,
} from '@octopus/schema'

import { buildWorkspaceMenuNodes } from '@/navigation/menuRegistry'

import { clone } from './workspace-fixture-bootstrap'
import {
  createManagementCapabilities,
  createMcpCatalogEntry,
  createSkillAsset,
  createSkillCatalogEntry,
  createSkillFileDocument,
} from './workspace-fixture-skill-helpers'
import {
  createPetPresenceState,
  createPetProfile,
  createRuntimeConfigSource,
} from './workspace-fixture-runtime'
import type { RuntimeSessionState } from './workspace-fixture-runtime'

export interface FixtureOptions {
  preloadConversationMessages?: boolean
  localRuntimeConfigTransform?: (config: RuntimeEffectiveConfig) => RuntimeEffectiveConfig
  localOwnerReady?: boolean
  localSetupRequired?: boolean
  preloadWorkspaceSessions?: boolean
  localSessionValid?: boolean
}

export interface WorkspaceFixtureState {
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
  currentUserId: string
  users: UserRecordSummary[]
  userPasswords: Record<string, string>
  orgUnits: OrgUnitRecord[]
  positions: PositionRecord[]
  userGroups: UserGroupRecord[]
  userOrgAssignments: UserOrgAssignmentRecord[]
  roles: AccessRoleRecord[]
  permissionDefinitions: PermissionDefinition[]
  roleBindings: RoleBindingRecord[]
  dataPolicies: DataPolicyRecord[]
  menus: MenuDefinition[]
  menuPolicies: MenuPolicyRecord[]
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

const RBAC_MENU_IDS = [
  'menu-workspace-overview',
  'menu-workspace-console',
  'menu-workspace-console-projects',
  'menu-workspace-console-knowledge',
  'menu-workspace-console-resources',
  'menu-workspace-console-agents',
  'menu-workspace-console-models',
  'menu-workspace-console-tools',
  'menu-workspace-automations',
  'menu-workspace-access-control',
  'menu-workspace-access-control-users',
  'menu-workspace-access-control-org',
  'menu-workspace-access-control-roles',
  'menu-workspace-access-control-policies',
  'menu-workspace-access-control-menus',
  'menu-workspace-access-control-resources',
  'menu-workspace-access-control-sessions',
] as const

const OPERATOR_MENU_IDS = [
  'menu-workspace-overview',
  'menu-workspace-console',
  'menu-workspace-console-projects',
  'menu-workspace-access-control',
  'menu-workspace-access-control-users',
] as const

function capabilityToResourceType(code: string): string {
  if (code.startsWith('tool.')) {
    return code.split('.').slice(0, 2).join('.')
  }

  if (code.startsWith('runtime.config.')) {
    return code.split('.').slice(0, 3).join('.')
  }

  if (code.startsWith('runtime.')) {
    return code.split('.').slice(0, 2).join('.')
  }

  if (code.startsWith('provider-credential.')) {
    return 'provider-credential'
  }

  const [resourceType] = code.split('.')
  return resourceType
}

function capabilityToActions(code: string): string[] {
  if (code.startsWith('tool.')) {
    return [code.split('.').slice(2).join('.')]
  }

  if (code.startsWith('runtime.config.')) {
    return [code.split('.').slice(3).join('.')]
  }

  if (code.startsWith('runtime.')) {
    return [code.split('.').slice(2).join('.')]
  }

  return [code.split('.').slice(1).join('.')]
}

function createPermissionDefinition(code: string): PermissionDefinition {
  return {
    code,
    name: code,
    description: `Capability ${code}`,
    category: 'atomic',
    resourceType: capabilityToResourceType(code),
    actions: capabilityToActions(code),
  }
}

export function createWorkspaceFixtureState(
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
          skillIds: ['skill-project-redesign-review'],
          mcpServerNames: ['redesign-ops'],
          title: 'Project agent',
          description: 'Tracks the redesign migration work.',
          status: 'active',
          updatedAt: 98,
        },
        {
          id: 'agent-template-finance',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Finance Planner Template',
          avatarPath: 'builtin-assets/avatars/finance-planner.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Deterministic finance specialist',
          tags: ['finance', 'analysis'],
          prompt: 'Create stable finance execution plans.',
          builtinToolKeys: ['bash'],
          skillIds: ['skill-builtin-financial-calculator'],
          mcpServerNames: ['finance-ops'],
          title: 'Builtin finance template',
          description: 'Readonly builtin template for finance workflows.',
          status: 'active',
          updatedAt: 97,
          integrationSource: {
            kind: 'builtin-template',
            sourceId: 'builtin-agent-finance',
          },
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
          skillIds: ['skill-project-redesign-review'],
          mcpServerNames: ['redesign-ops'],
          leaderAgentId: 'agent-redesign',
          memberAgentIds: ['agent-redesign'],
          description: 'Executes the desktop migration.',
          status: 'active',
          updatedAt: 99,
        },
        {
          id: 'team-template-finance',
          workspaceId: workspace.id,
          scope: 'workspace',
          name: 'Finance Ops Template',
          avatarPath: 'builtin-assets/avatars/finance-ops.png',
          avatar: 'data:image/png;base64,iVBORw0KGgo=',
          personality: 'Builtin finance delivery pod',
          tags: ['finance', 'ops'],
          prompt: 'Coordinate finance operations and execution.',
          builtinToolKeys: ['bash'],
          skillIds: ['skill-builtin-financial-calculator'],
          mcpServerNames: ['finance-ops'],
          leaderAgentId: 'agent-template-finance',
          memberAgentIds: ['agent-template-finance'],
          description: 'Readonly builtin team template for finance operations.',
          status: 'active',
          updatedAt: 98,
          integrationSource: {
            kind: 'builtin-template',
            sourceId: 'builtin-team-finance',
          },
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
    {
      id: 'tool-help-skill',
      workspaceId: workspace.id,
      kind: 'skill',
      name: 'Help Skill',
      description: 'Workspace-managed skill tool.',
      status: 'active',
      permissionMode: 'readonly',
      updatedAt: 100,
    },
    {
      id: 'tool-ops-mcp',
      workspaceId: workspace.id,
      kind: 'mcp',
      name: 'Ops MCP',
      description: 'Workspace-managed MCP tool.',
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

  const builtinFinancialCalculatorSkill = createSkillAsset({
    id: 'skill-builtin-financial-calculator',
    sourceKey: 'skill:builtin-assets/skills/financial-calculator/SKILL.md',
    name: 'financial-calculator',
    description: 'Builtin calculator skill bundle.',
    displayPath: 'builtin-assets/skills/financial-calculator/SKILL.md',
    workspaceOwned: false,
    sourceOrigin: 'builtin_bundle',
    files: {
      'SKILL.md': createSkillFileDocument(
        'skill-builtin-financial-calculator',
        'skill:builtin-assets/skills/financial-calculator/SKILL.md',
        'builtin-assets/skills/financial-calculator',
        'SKILL.md',
        {
          content: [
            '---',
            'name: financial-calculator',
            'description: Builtin calculator skill bundle.',
            '---',
            '',
            '# Financial Calculator',
          ].join('\n'),
          readonly: true,
        },
      ),
      'templates/formula.md': createSkillFileDocument(
        'skill-builtin-financial-calculator',
        'skill:builtin-assets/skills/financial-calculator/SKILL.md',
        'builtin-assets/skills/financial-calculator',
        'templates/formula.md',
        {
          content: '# Formula\n\n- gross_margin = revenue - cost',
          readonly: true,
        },
      ),
    },
  })

  const projectRedesignSkill = createSkillAsset({
    id: 'skill-project-redesign-review',
    sourceKey: 'skill:data/projects/proj-redesign/skills/redesign-review/SKILL.md',
    name: 'redesign-review',
    description: 'Project-scoped review checklists for the redesign work.',
    displayPath: 'data/projects/proj-redesign/skills/redesign-review/SKILL.md',
    workspaceOwned: false,
    relativePath: 'data/projects/proj-redesign/skills/redesign-review/SKILL.md',
    files: {
      'SKILL.md': createSkillFileDocument(
        'skill-project-redesign-review',
        'skill:data/projects/proj-redesign/skills/redesign-review/SKILL.md',
        'data/projects/proj-redesign/skills/redesign-review',
        'SKILL.md',
        {
          content: [
            '---',
            'name: redesign-review',
            'description: Project-scoped review checklists for the redesign work.',
            '---',
            '',
            '# Review Flow',
            '',
            'Use this skill for project-only visual and interaction review passes.',
          ].join('\n'),
          readonly: true,
        },
      ),
      'checklists/ui-audit.md': createSkillFileDocument(
        'skill-project-redesign-review',
        'skill:data/projects/proj-redesign/skills/redesign-review/SKILL.md',
        'data/projects/proj-redesign/skills/redesign-review',
        'checklists/ui-audit.md',
        {
          content: '# UI Audit\n\n- Verify tabs\n- Verify pagination\n- Verify resource ownership',
          readonly: true,
        },
      ),
    },
  })

  const skillDocuments: Record<string, WorkspaceSkillDocument> = {
    [managedHelpSkill.document.id]: managedHelpSkill.document,
    [externalClaudeSkill.document.id]: externalClaudeSkill.document,
    [externalCodexSkill.document.id]: externalCodexSkill.document,
    [builtinFinancialCalculatorSkill.document.id]: builtinFinancialCalculatorSkill.document,
    [projectRedesignSkill.document.id]: projectRedesignSkill.document,
  }

  const skillFiles: Record<string, Record<string, WorkspaceSkillFileDocument>> = {
    [managedHelpSkill.document.id]: managedHelpSkill.files,
    [externalClaudeSkill.document.id]: externalClaudeSkill.files,
    [externalCodexSkill.document.id]: externalCodexSkill.files,
    [builtinFinancialCalculatorSkill.document.id]: builtinFinancialCalculatorSkill.files,
    [projectRedesignSkill.document.id]: projectRedesignSkill.files,
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
    'redesign-ops': {
      serverName: 'redesign-ops',
      sourceKey: 'mcp:redesign-ops',
      displayPath: 'config/runtime/projects/proj-redesign.json',
      scope: 'project',
      config: {
        type: 'stdio',
        command: 'octopus-redesign-mcp',
      },
    },
    'finance-ops': {
      serverName: 'finance-ops',
      sourceKey: 'mcp:finance-ops',
      displayPath: 'builtin-assets/mcps/finance-ops.json',
      scope: 'builtin',
      config: {
        type: 'http',
        url: 'https://finance.example.test/mcp',
      },
    },
  }

  const workspaceConsumers = {
    architect: {
      kind: 'agent',
      id: 'agent-architect',
      name: 'Architect Agent',
      scope: 'workspace',
    } as const,
    coder: {
      kind: 'agent',
      id: 'agent-coder',
      name: 'Coder Agent',
      scope: 'workspace',
    } as const,
    studioTeam: {
      kind: 'team',
      id: 'team-studio',
      name: 'Studio Direction Team',
      scope: 'workspace',
    } as const,
  }

  const projectConsumers = {
    redesignAgent: {
      kind: 'agent',
      id: 'agent-redesign',
      name: 'Redesign Copilot',
      scope: 'project',
      ownerId: 'proj-redesign',
      ownerLabel: 'Desktop Redesign',
    } as const,
    redesignTeam: {
      kind: 'team',
      id: 'team-redesign',
      name: 'Redesign Tiger Team',
      scope: 'project',
      ownerId: 'proj-redesign',
      ownerLabel: 'Desktop Redesign',
    } as const,
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
        ownerScope: 'builtin',
        ownerLabel: 'Octopus Builtin',
        consumers: [workspaceConsumers.architect, projectConsumers.redesignAgent, workspaceConsumers.studioTeam],
      },
      {
        id: 'builtin-read-file',
        workspaceId: workspace.id,
        kind: 'builtin',
        name: 'read_file',
        description: 'Read files from the active workspace.',
        availability: 'healthy',
        requiredPermission: 'readonly',
        sourceKey: 'builtin:read_file',
        displayPath: 'runtime builtin registry',
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        builtinKey: 'read_file',
        ownerScope: 'builtin',
        ownerLabel: 'Octopus Builtin',
      },
      {
        id: 'builtin-write-file',
        workspaceId: workspace.id,
        kind: 'builtin',
        name: 'write_file',
        description: 'Write files into the active workspace.',
        availability: 'healthy',
        requiredPermission: 'workspace-write',
        sourceKey: 'builtin:write_file',
        displayPath: 'runtime builtin registry',
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        builtinKey: 'write_file',
        ownerScope: 'builtin',
        ownerLabel: 'Octopus Builtin',
      },
      {
        id: 'builtin-rg',
        workspaceId: workspace.id,
        kind: 'builtin',
        name: 'rg',
        description: 'Search exact text in the workspace.',
        availability: 'healthy',
        requiredPermission: 'readonly',
        sourceKey: 'builtin:rg',
        displayPath: 'runtime builtin registry',
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        builtinKey: 'rg',
        ownerScope: 'builtin',
        ownerLabel: 'Octopus Builtin',
      },
      {
        id: 'builtin-apply-patch',
        workspaceId: workspace.id,
        kind: 'builtin',
        name: 'apply_patch',
        description: 'Apply structured patches to workspace files.',
        availability: 'healthy',
        requiredPermission: 'workspace-write',
        sourceKey: 'builtin:apply_patch',
        displayPath: 'runtime builtin registry',
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        builtinKey: 'apply_patch',
        ownerScope: 'builtin',
        ownerLabel: 'Octopus Builtin',
      },
      {
        id: 'builtin-web-search',
        workspaceId: workspace.id,
        kind: 'builtin',
        name: 'web_search',
        description: 'Query the web when external verification is required.',
        availability: 'healthy',
        requiredPermission: 'readonly',
        sourceKey: 'builtin:web_search',
        displayPath: 'runtime builtin registry',
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        builtinKey: 'web_search',
        ownerScope: 'builtin',
        ownerLabel: 'Octopus Builtin',
      },
      {
        id: 'builtin-image-query',
        workspaceId: workspace.id,
        kind: 'builtin',
        name: 'image_query',
        description: 'Search image sources for visual references.',
        availability: 'healthy',
        requiredPermission: 'readonly',
        sourceKey: 'builtin:image_query',
        displayPath: 'runtime builtin registry',
        disabled: false,
        management: createManagementCapabilities(true, false, false),
        builtinKey: 'image_query',
        ownerScope: 'builtin',
        ownerLabel: 'Octopus Builtin',
      },
      createSkillCatalogEntry(workspace.id, managedHelpSkill.document, false, {
        ownerScope: 'workspace',
        ownerId: workspace.id,
        ownerLabel: workspace.name,
        consumers: [workspaceConsumers.architect, workspaceConsumers.studioTeam],
      }),
      createSkillCatalogEntry(workspace.id, externalClaudeSkill.document, false, {
        ownerScope: 'builtin',
        ownerLabel: 'External Skill Source',
        consumers: [workspaceConsumers.coder],
      }),
      createSkillCatalogEntry(workspace.id, externalCodexSkill.document, false, {
        ownerScope: 'builtin',
        ownerLabel: 'External Skill Source',
        consumers: [workspaceConsumers.coder],
      }),
      createSkillCatalogEntry(workspace.id, builtinFinancialCalculatorSkill.document, false, {
        ownerScope: 'builtin',
        ownerLabel: 'Builtin',
        consumers: [workspaceConsumers.architect],
      }),
      createSkillCatalogEntry(workspace.id, projectRedesignSkill.document, false, {
        management: createManagementCapabilities(false, false, false),
        ownerScope: 'project',
        ownerId: 'proj-redesign',
        ownerLabel: 'Desktop Redesign',
        consumers: [projectConsumers.redesignAgent, projectConsumers.redesignTeam],
      }),
      {
        ...createMcpCatalogEntry(workspace.id, mcpDocuments.ops, false, {
          ownerScope: 'workspace',
          ownerId: workspace.id,
          ownerLabel: workspace.name,
          consumers: [workspaceConsumers.architect, workspaceConsumers.studioTeam],
          toolNames: ['mcp__ops__tail_logs'],
          statusDetail: 'MCP handshake timed out',
        }),
        availability: 'attention',
      },
      createMcpCatalogEntry(workspace.id, mcpDocuments['redesign-ops'], false, {
        management: createManagementCapabilities(false, false, false),
        ownerScope: 'project',
        ownerId: 'proj-redesign',
        ownerLabel: 'Desktop Redesign',
        consumers: [projectConsumers.redesignAgent, projectConsumers.redesignTeam],
        toolNames: ['mcp__redesign_ops__capture_snapshot'],
        description: 'Project-scoped MCP server.',
      }),
      createMcpCatalogEntry(workspace.id, mcpDocuments['finance-ops'], false, {
        management: createManagementCapabilities(true, false, false),
        ownerScope: 'builtin',
        ownerLabel: 'Builtin',
        consumers: [
          workspaceConsumers.architect,
          {
            kind: 'agent',
            id: 'agent-template-finance',
            name: 'Finance Planner Template',
            scope: 'workspace',
          },
          {
            kind: 'team',
            id: 'team-template-finance',
            name: 'Finance Ops Template',
            scope: 'workspace',
          },
        ],
        toolNames: ['mcp__finance_ops__run_report'],
        description: 'Builtin finance MCP server.',
      }),
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
        }]
      : []),
    {
      id: 'user-operator',
      username: 'operator',
      displayName: 'Lin Zhou',
      avatar: 'data:image/png;base64,iVBORw0KGgo=',
      status: 'active',
      passwordState: 'set',
    },
  ]

  const orgUnits: OrgUnitRecord[] = [{
    id: 'org-root',
    parentId: undefined,
    code: 'workspace-root',
    name: workspace.name,
    status: 'active',
  }]

  const positions: PositionRecord[] = []
  const userGroups: UserGroupRecord[] = []

  const userOrgAssignments: UserOrgAssignmentRecord[] = users.map(user => ({
    userId: user.id,
    orgUnitId: 'org-root',
    isPrimary: true,
    positionIds: [],
    userGroupIds: [],
  }))

  const menus: MenuDefinition[] = buildWorkspaceMenuNodes(workspace.id)
    .filter(menu => RBAC_MENU_IDS.includes(menu.id as (typeof RBAC_MENU_IDS)[number]))
    .map(menu => ({
      id: menu.id,
      parentId: menu.parentId,
      label: menu.label,
      routeName: menu.routeName,
      source: menu.source,
      status: menu.status,
      order: menu.order,
      featureCode: 'feature:' + (menu.routeName ?? menu.id),
    }))

  const ownerPermissionCodes = [
    'workspace.overview.read',
    'project.view',
    'project.manage',
    'team.view',
    'team.manage',
    'team.import',
    'access.users.read',
    'access.users.manage',
    'access.org.read',
    'access.org.manage',
    'access.roles.read',
    'access.roles.manage',
    'access.policies.read',
    'access.policies.manage',
    'access.menus.read',
    'access.menus.manage',
    'access.sessions.read',
    'access.sessions.manage',
    'runtime.session.read',
    'runtime.config.workspace.read',
    'runtime.config.workspace.manage',
    'runtime.config.project.read',
    'runtime.config.project.manage',
    'runtime.config.user.read',
    'runtime.config.user.manage',
    'runtime.approval.resolve',
    'agent.view',
    'agent.edit',
    'agent.import',
    'agent.export',
    'agent.delete',
    'resource.view',
    'resource.upload',
    'resource.update',
    'resource.delete',
    'knowledge.view',
    'knowledge.retrieve',
    'tool.catalog.view',
    'tool.catalog.manage',
    'provider-credential.view',
    'tool.builtin.view',
    'tool.builtin.invoke',
    'tool.builtin.configure',
    'tool.skill.view',
    'tool.skill.invoke',
    'tool.skill.configure',
    'tool.mcp.view',
    'tool.mcp.invoke',
    'tool.mcp.configure',
    'tool.mcp.bind-credential',
    'automation.view',
    'automation.manage',
    'pet.view',
    'pet.manage',
    'artifact.view',
    'inbox.view',
  ] as const

  const operatorPermissionCodes = [
    'project.view',
    'team.view',
    'access.users.read',
    'runtime.session.read',
    'agent.view',
    'resource.view',
    'knowledge.view',
    'tool.catalog.view',
    'provider-credential.view',
    'tool.builtin.view',
    'tool.builtin.invoke',
    'tool.skill.view',
    'tool.skill.invoke',
    'tool.mcp.view',
    'tool.mcp.invoke',
    'automation.view',
    'pet.view',
    'artifact.view',
    'inbox.view',
  ] as const

  const roles: AccessRoleRecord[] = [
    {
      id: 'role-owner',
      name: 'Owner',
      code: 'owner',
      description: 'Full workspace access.',
      status: 'active',
      permissionCodes: [...ownerPermissionCodes],
    },
    {
      id: 'role-operator',
      name: 'Operator',
      code: 'operator',
      description: 'Daily operations access.',
      status: 'active',
      permissionCodes: [...operatorPermissionCodes],
    },
  ]

  const permissionDefinitions: PermissionDefinition[] = Array.from(new Set([
    ...ownerPermissionCodes,
    ...operatorPermissionCodes,
  ])).map(createPermissionDefinition)

  const roleBindings: RoleBindingRecord[] = [
    ...(ownerReady
      ? [{
          id: 'binding-user-owner-role-owner',
          roleId: 'role-owner',
          subjectType: 'user',
          subjectId: 'user-owner',
          effect: 'allow',
        }]
      : []),
    {
      id: 'binding-user-operator-role-operator',
      roleId: 'role-operator',
      subjectType: 'user',
      subjectId: 'user-operator',
      effect: 'allow',
    },
  ]

  const dataPolicies: DataPolicyRecord[] = [{
    id: 'policy-user-operator-projects',
    name: 'Operator project access',
    subjectType: 'user',
    subjectId: 'user-operator',
    resourceType: 'project',
    scopeType: 'selected-projects',
    projectIds: projects.map(project => project.id),
    tags: [],
    classifications: [],
    effect: 'allow',
  }]

  const menuPolicies: MenuPolicyRecord[] = []

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
    currentUserId: users[0]?.id ?? '',
    users,
    userPasswords: ownerReady ? { 'user-owner': 'owner-owner', 'user-operator': 'operator-operator' } : { 'user-operator': 'operator-operator' },
    orgUnits,
    positions,
    userGroups,
    userOrgAssignments,
    roles,
    permissionDefinitions,
    roleBindings,
    dataPolicies,
    menus,
    menuPolicies,
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
