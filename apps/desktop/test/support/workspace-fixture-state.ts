import type {
  AccessRoleRecord,
  AgentRecord,
  CapabilityManagementProjection,
  CredentialBinding,
  DataPolicyRecord,
  DeliverableSummary,
  InboxItemRecord,
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
  ProtectedResourceDescriptor,
  ProjectAgentLinkRecord,
  ProjectDashboardSnapshot,
  ProjectPromotionRequest,
  ProjectRecord,
  ProjectTeamLinkRecord,
  TaskDetail,
  TaskInterventionRecord,
  TaskRunSummary,
  TaskSummary,
  DeliverableVersionContent,
  DeliverableVersionSummary,
  RoleBindingRecord,
  ResourcePolicyRecord,
  RuntimeEffectiveConfig,
  SystemBootstrapStatus,
  TeamRecord,
  ToolRecord,
  UserGroupRecord,
  UserOrgAssignmentRecord,
  UserRecordSummary,
  WorkspaceConnectionRecord,
  WorkspaceDirectoryBrowserResponse,
  WorkspaceMcpServerDocument,
  WorkspaceOverviewSnapshot,
  WorkspaceResourceChildrenRecord,
  WorkspaceResourceContentDocument,
  WorkspaceResourceRecord,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceToolCatalogEntry,
} from '@octopus/schema'

import { buildWorkspaceMenuNodes } from '@/navigation/menuRegistry'
import { deriveCapabilityManagementProjection } from '@/stores/catalog_management'

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
  extraAccessUsersCount?: number
  includeAccessOrgHierarchy?: boolean
  locale?: string
  stateTransform?: (
    state: WorkspaceFixtureState,
    connection: WorkspaceConnectionRecord,
  ) => void
  toolCatalogTransform?: (entries: WorkspaceToolCatalogEntry[]) => WorkspaceToolCatalogEntry[]
  managementProjectionTransform?: (
    projection: CapabilityManagementProjection,
  ) => CapabilityManagementProjection
}

export interface WorkspaceFixtureState {
  systemBootstrap: SystemBootstrapStatus
  workspace: WorkspaceOverviewSnapshot['workspace']
  overview: WorkspaceOverviewSnapshot
  projects: ProjectRecord[]
  projectPromotionRequests: ProjectPromotionRequest[]
  dashboards: Record<string, ProjectDashboardSnapshot>
  taskIdSequence: number
  taskRunIdSequence: number
  taskInterventionIdSequence: number
  taskDetailsByKey: Map<string, TaskDetail>
  taskRunsByKey: Map<string, TaskRunSummary[]>
  taskInterventionsByKey: Map<string, TaskInterventionRecord[]>
  workspaceResources: WorkspaceResourceRecord[]
  projectResources: Record<string, WorkspaceResourceRecord[]>
  resourceContents: Record<string, WorkspaceResourceContentDocument>
  resourceChildren: Record<string, WorkspaceResourceChildrenRecord[]>
  remoteDirectories: Record<string, WorkspaceDirectoryBrowserResponse>
  deliverables: DeliverableSummary[]
  deliverableVersionSummaries: Map<string, DeliverableVersionSummary[]>
  deliverableVersionContents: Map<string, DeliverableVersionContent>
  inboxItems: InboxItemRecord[]
  workspaceKnowledge: KnowledgeRecord[]
  projectKnowledge: Record<string, KnowledgeRecord[]>
  agents: AgentRecord[]
  projectAgentLinks: Record<string, ProjectAgentLinkRecord[]>
  teams: TeamRecord[]
  projectTeamLinks: Record<string, ProjectTeamLinkRecord[]>
  catalog: ModelCatalogSnapshot
  toolCatalog: { entries: WorkspaceToolCatalogEntry[] }
  managementProjection: CapabilityManagementProjection
  skillDocuments: Record<string, WorkspaceSkillDocument>
  skillFiles: Record<string, Record<string, WorkspaceSkillFileDocument>>
  mcpDocuments: Record<string, WorkspaceMcpServerDocument>
  tools: ToolRecord[]
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
  resourcePolicies: ResourcePolicyRecord[]
  protectedResourceMetadata: ProtectedResourceDescriptor[]
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
  'menu-workspace-access-control',
] as const

const OPERATOR_MENU_IDS = [
  'menu-workspace-overview',
  'menu-workspace-console',
  'menu-workspace-console-projects',
  'menu-workspace-access-control',
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
    projectDefaultPermissions: {
      agents: 'allow',
      resources: 'allow',
      tools: local ? 'allow' : 'deny',
      knowledge: 'allow',
      tasks: 'allow',
    },
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
          resourceDirectory: 'data/projects/proj-redesign/resources',
          leaderAgentId: 'agent-architect',
          ownerUserId: 'user-owner',
          memberUserIds: ['user-owner', 'user-operator'],
          permissionOverrides: {
            agents: 'inherit',
            resources: 'inherit',
            tools: 'inherit',
            knowledge: 'inherit',
            tasks: 'inherit',
          },
          linkedWorkspaceAssets: {
            agentIds: ['agent-architect'],
            resourceIds: [`${workspace.id}-res-workspace-1`],
            toolSourceKeys: ['builtin:bash', 'mcp:ops'],
            knowledgeIds: ['knowledge-workspace-1'],
          },
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
          resourceDirectory: 'data/projects/proj-governance/resources',
          leaderAgentId: 'agent-coder',
          ownerUserId: 'user-owner',
          memberUserIds: ['user-owner'],
          permissionOverrides: {
            agents: 'inherit',
            resources: 'inherit',
            tools: 'inherit',
            knowledge: 'inherit',
            tasks: 'inherit',
          },
          linkedWorkspaceAssets: {
            agentIds: [],
            resourceIds: [],
            toolSourceKeys: [],
            knowledgeIds: [],
          },
        },
      ]
    : [
        {
          id: 'proj-launch',
          workspaceId: workspace.id,
          name: 'Launch Readiness',
          status: 'active',
          description: 'Enterprise launch planning and cutover execution.',
          resourceDirectory: '/remote/projects/launch-readiness/resources',
          ownerUserId: 'user-owner',
          memberUserIds: ['user-owner'],
          permissionOverrides: {
            agents: 'inherit',
            resources: 'inherit',
            tools: 'inherit',
            knowledge: 'inherit',
            tasks: 'inherit',
          },
          linkedWorkspaceAssets: {
            agentIds: [],
            resourceIds: [],
            toolSourceKeys: [],
            knowledgeIds: [],
          },
        },
      ]

  const projectPromotionRequests: ProjectPromotionRequest[] = local
    ? [
        {
          id: 'promotion-proj-redesign-res-4',
          workspaceId: workspace.id,
          projectId: 'proj-redesign',
          assetType: 'resource',
          assetId: 'proj-redesign-res-4',
          requestedByUserId: 'user-owner',
          submittedByOwnerUserId: 'user-owner',
          requiredWorkspaceCapability: 'resource.publish',
          status: 'pending',
          createdAt: 105,
          updatedAt: 105,
        },
      ]
    : []

  const resourcePolicies: ResourcePolicyRecord[] = []
  const protectedResourceMetadata: ProtectedResourceDescriptor[] = []

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
          id: 'conv-redesign-ops',
          workspaceId: workspace.id,
          projectId: 'proj-redesign',
          sessionId: 'rt-conv-redesign-ops',
          title: 'Operator Handoff',
          status: 'running',
          updatedAt: 99,
          lastMessagePreview: 'Queued operator follow-up and review items.',
        },
        {
          id: 'conv-redesign-review',
          workspaceId: workspace.id,
          projectId: 'proj-redesign',
          sessionId: 'rt-conv-redesign-review',
          title: 'QA Review Sweep',
          status: 'completed',
          updatedAt: 98,
          lastMessagePreview: 'Closed dashboard accessibility fixes.',
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
        {
          id: 'activity-sync',
          title: 'Workspace synced',
          description: 'Bootstrap and projections loaded.',
          timestamp: 100,
          actorId: 'user-owner',
          actorType: 'user',
          resource: 'workspace.dashboard',
          outcome: 'success',
        },
        {
          id: 'activity-runtime',
          title: 'Runtime event replay',
          description: 'Recovered session stream after reconnect.',
          timestamp: 96,
          actorId: 'user-owner',
          actorType: 'user',
          resource: 'runtime.session',
          outcome: 'success',
        },
        {
          id: 'activity-approval',
          title: 'Approval queue updated',
          description: 'A runtime approval item was routed to the owner queue.',
          timestamp: 95,
          actorId: 'user-operator',
          actorType: 'user',
          resource: 'runtime.approval',
          outcome: 'pending',
        },
        {
          id: 'activity-tooling',
          title: 'Tooling observation refreshed',
          description: 'Tool usage and token ledgers were re-aggregated.',
          timestamp: 94,
          actorId: 'user-operator',
          actorType: 'user',
          resource: 'tool.usage',
          outcome: 'success',
        },
      ]
    : [
        {
          id: 'activity-launch',
          title: 'Launch dashboard refreshed',
          description: 'Enterprise projection rebuilt.',
          timestamp: 120,
          actorId: 'user-owner',
          actorType: 'user',
          resource: 'workspace.dashboard',
          outcome: 'success',
        },
      ]

  const recentTasksByProjectId: Record<string, TaskSummary[]> = local
    ? {
        'proj-redesign': [
          {
            id: 'task-redesign-release-brief',
            projectId: 'proj-redesign',
            title: 'Release Brief Refresh',
            goal: 'Refresh the release brief from the latest design and runtime changes.',
            defaultActorRef: 'agent-architect',
            status: 'running',
            latestResultSummary: 'Drafting the updated brief and collecting deliverable links.',
            latestTransition: {
              kind: 'progressed',
              summary: 'Compiled the latest deliverables into the release brief draft.',
              at: 100,
              runId: 'task-run-redesign-release-brief',
            },
            viewStatus: 'attention',
            attentionReasons: ['updated'],
            attentionUpdatedAt: 100,
            activeTaskRunId: 'task-run-redesign-release-brief',
            analyticsSummary: {
              runCount: 4,
              manualRunCount: 2,
              scheduledRunCount: 2,
              completionCount: 3,
              failureCount: 0,
              takeoverCount: 1,
              approvalRequiredCount: 1,
              averageRunDurationMs: 420000,
              lastSuccessfulRunAt: 96,
            },
            updatedAt: 100,
          },
          {
            id: 'task-redesign-regression-sweep',
            projectId: 'proj-redesign',
            title: 'Regression Sweep',
            goal: 'Run the desktop regression checklist and summarize failures.',
            defaultActorRef: 'agent-architect',
            status: 'attention',
            scheduleSpec: 'FREQ=DAILY;BYHOUR=9;BYMINUTE=0',
            nextRunAt: 108,
            lastRunAt: 94,
            latestResultSummary: 'Hit a runtime error while validating the workspace overview flow.',
            latestFailureCategory: 'runtime_error',
            latestTransition: {
              kind: 'failed',
              summary: 'The last regression sweep failed during overview validation.',
              at: 94,
              runId: 'task-run-redesign-regression-sweep',
            },
            viewStatus: 'attention',
            attentionReasons: ['failed', 'takeover_recommended'],
            attentionUpdatedAt: 94,
            analyticsSummary: {
              runCount: 6,
              manualRunCount: 1,
              scheduledRunCount: 5,
              completionCount: 5,
              failureCount: 1,
              takeoverCount: 1,
              approvalRequiredCount: 0,
              averageRunDurationMs: 510000,
              lastSuccessfulRunAt: 92,
            },
            updatedAt: 94,
          },
          {
            id: 'task-redesign-approval-gate',
            projectId: 'proj-redesign',
            title: 'Approval Gate Review',
            goal: 'Pause before publishing the release brief update and wait for operator approval.',
            defaultActorRef: 'agent-architect',
            status: 'attention',
            lastRunAt: 89,
            latestResultSummary: 'Waiting for approval before publishing the release brief update.',
            latestTransition: {
              kind: 'waiting_approval',
              summary: 'The active run is paused until approval is recorded for the publish step.',
              at: 89,
              runId: 'task-run-redesign-approval-gate',
            },
            viewStatus: 'attention',
            attentionReasons: ['needs_approval'],
            attentionUpdatedAt: 89,
            activeTaskRunId: 'task-run-redesign-approval-gate',
            analyticsSummary: {
              runCount: 2,
              manualRunCount: 2,
              scheduledRunCount: 0,
              completionCount: 1,
              failureCount: 0,
              takeoverCount: 0,
              approvalRequiredCount: 1,
              averageRunDurationMs: 300000,
              lastSuccessfulRunAt: 82,
            },
            updatedAt: 89,
          },
        ],
        'proj-governance': [
          {
            id: 'task-governance-menu-audit',
            projectId: 'proj-governance',
            title: 'Menu Policy Audit',
            goal: 'Validate the menu policy matrix for operator and owner roles.',
            defaultActorRef: 'agent-architect',
            status: 'ready',
            latestTransition: {
              kind: 'created',
              summary: 'Task is ready for the next manual launch.',
              at: 90,
              runId: null,
            },
            viewStatus: 'configured',
            attentionReasons: [],
            analyticsSummary: {
              runCount: 1,
              manualRunCount: 1,
              scheduledRunCount: 0,
              completionCount: 1,
              failureCount: 0,
              takeoverCount: 0,
              approvalRequiredCount: 0,
              averageRunDurationMs: 180000,
              lastSuccessfulRunAt: 90,
            },
            updatedAt: 90,
          },
        ],
      }
    : {
        'proj-launch': [
          {
            id: 'task-launch-cutover-checklist',
            projectId: 'proj-launch',
            title: 'Cutover Checklist',
            goal: 'Prepare the enterprise cutover checklist and note blockers.',
            defaultActorRef: 'agent-launch',
            status: 'running',
            latestResultSummary: 'Collecting final blocker notes from launch resources.',
            latestTransition: {
              kind: 'launched',
              summary: 'Scheduled launch run started for the current cutover window.',
              at: 120,
              runId: 'task-run-launch-cutover-checklist',
            },
            viewStatus: 'healthy',
            attentionReasons: [],
            activeTaskRunId: 'task-run-launch-cutover-checklist',
            analyticsSummary: {
              runCount: 3,
              manualRunCount: 0,
              scheduledRunCount: 3,
              completionCount: 2,
              failureCount: 0,
              takeoverCount: 0,
              approvalRequiredCount: 1,
              averageRunDurationMs: 360000,
              lastSuccessfulRunAt: 110,
            },
            updatedAt: 120,
          },
        ],
      }

  const projectTokenUsage = projects
    .map(project => ({
      projectId: project.id,
      projectName: project.name,
      usedTokens: project.id === 'proj-redesign' ? 125000 : 24000,
    }))
    .sort((left, right) => right.usedTokens - left.usedTokens)

  const overview: WorkspaceOverviewSnapshot = {
    workspace,
    metrics: [
      { id: 'projects', label: 'Projects', value: String(projects.length), tone: 'accent' },
      { id: 'conversations', label: 'Conversations', value: String(recentConversations.length), tone: 'info' },
      { id: 'alerts', label: 'Alerts', value: local ? '0' : '1', tone: local ? 'default' : 'warning' },
    ],
    projects,
    projectTokenUsage,
    recentConversations,
    recentActivity,
  }

  const dashboards: Record<string, ProjectDashboardSnapshot> = Object.fromEntries(projects.map(project => [
    project.id,
    {
      project,
      usedTokens: project.id === 'proj-redesign' ? 125000 : 24000,
      metrics: [
        { id: 'sessions', label: 'Sessions', value: String(recentConversations.filter(item => item.projectId === project.id).length), tone: 'accent' },
        { id: 'resources', label: 'Resources', value: local ? '2' : '1', tone: 'info' },
      ],
      overview: project.id === 'proj-redesign'
        ? {
            memberCount: 2,
            activeUserCount: 2,
            agentCount: 4,
            teamCount: 2,
            conversationCount: 3,
            messageCount: 98,
            toolCallCount: 43,
            approvalCount: 4,
            resourceCount: 8,
            knowledgeCount: 5,
            toolCount: 3,
            tokenRecordCount: 7,
            totalTokens: 125000,
            activityCount: 24,
            taskCount: recentTasksByProjectId[project.id]?.length ?? 0,
            activeTaskCount: recentTasksByProjectId[project.id]?.filter(task => task.status === 'running').length ?? 0,
            attentionTaskCount: recentTasksByProjectId[project.id]?.filter(task => task.viewStatus === 'attention').length ?? 0,
            scheduledTaskCount: recentTasksByProjectId[project.id]?.filter(task => typeof task.scheduleSpec === 'string').length ?? 0,
          }
        : {
            memberCount: 1,
            activeUserCount: 1,
            agentCount: 1,
            teamCount: 1,
            conversationCount: recentConversations.filter(item => item.projectId === project.id).length,
            messageCount: 12,
            toolCallCount: 5,
            approvalCount: 0,
            resourceCount: 4,
            knowledgeCount: 2,
            toolCount: 2,
            tokenRecordCount: 3,
            totalTokens: 24000,
            activityCount: 6,
            taskCount: recentTasksByProjectId[project.id]?.length ?? 0,
            activeTaskCount: recentTasksByProjectId[project.id]?.filter(task => task.status === 'running').length ?? 0,
            attentionTaskCount: recentTasksByProjectId[project.id]?.filter(task => task.viewStatus === 'attention').length ?? 0,
            scheduledTaskCount: recentTasksByProjectId[project.id]?.filter(task => typeof task.scheduleSpec === 'string').length ?? 0,
          },
      trend: project.id === 'proj-redesign'
        ? [
            { id: 'bucket-0', label: '1', timestamp: 1712620800000, conversationCount: 0, messageCount: 8, toolCallCount: 3, approvalCount: 0, tokenCount: 12000 },
            { id: 'bucket-1', label: '2', timestamp: 1712707200000, conversationCount: 1, messageCount: 10, toolCallCount: 4, approvalCount: 1, tokenCount: 14000 },
            { id: 'bucket-2', label: '3', timestamp: 1712793600000, conversationCount: 0, messageCount: 13, toolCallCount: 5, approvalCount: 0, tokenCount: 16000 },
            { id: 'bucket-3', label: '4', timestamp: 1712880000000, conversationCount: 1, messageCount: 15, toolCallCount: 6, approvalCount: 1, tokenCount: 18000 },
            { id: 'bucket-4', label: '5', timestamp: 1712966400000, conversationCount: 0, messageCount: 14, toolCallCount: 7, approvalCount: 0, tokenCount: 17000 },
            { id: 'bucket-5', label: '6', timestamp: 1713052800000, conversationCount: 1, messageCount: 18, toolCallCount: 8, approvalCount: 1, tokenCount: 22000 },
            { id: 'bucket-6', label: '7', timestamp: 1713139200000, conversationCount: 1, messageCount: 20, toolCallCount: 10, approvalCount: 1, tokenCount: 26000 },
          ]
        : [
            { id: `${project.id}-bucket-0`, label: '1', timestamp: 1712620800000, conversationCount: 0, messageCount: 4, toolCallCount: 1, approvalCount: 0, tokenCount: 6000 },
            { id: `${project.id}-bucket-1`, label: '2', timestamp: 1712707200000, conversationCount: 1, messageCount: 8, toolCallCount: 4, approvalCount: 0, tokenCount: 18000 },
          ],
      userStats: project.id === 'proj-redesign'
        ? [
            {
              userId: 'user-owner',
              displayName: 'Octopus Owner',
              activityCount: 14,
              conversationCount: 2,
              messageCount: 58,
              toolCallCount: 25,
              approvalCount: 3,
              tokenCount: 76000,
              activityTrend: [2, 2, 2, 2, 2, 2, 2],
              tokenTrend: [7000, 8000, 9000, 11000, 10000, 14000, 17000],
            },
            {
              userId: 'user-operator',
              displayName: 'Workspace Operator',
              activityCount: 10,
              conversationCount: 2,
              messageCount: 40,
              toolCallCount: 18,
              approvalCount: 1,
              tokenCount: 49000,
              activityTrend: [1, 1, 1, 2, 1, 2, 2],
              tokenTrend: [5000, 6000, 7000, 7000, 7000, 8000, 9000],
            },
          ]
        : [
            {
              userId: 'user-owner',
              displayName: local ? 'Octopus Owner' : 'Enterprise Owner',
              activityCount: 6,
              conversationCount: 1,
              messageCount: 12,
              toolCallCount: 5,
              approvalCount: 0,
              tokenCount: 24000,
              activityTrend: [2, 4],
              tokenTrend: [6000, 18000],
            },
          ],
      conversationInsights: project.id === 'proj-redesign'
        ? [
            {
              id: 'rt-conv-redesign',
              conversationId: 'conv-redesign',
              title: 'Conversation Redesign',
              status: 'completed',
              updatedAt: 100,
              lastMessagePreview: 'Runtime-only conversation state is active.',
              messageCount: 28,
              toolCallCount: 12,
              approvalCount: 1,
              tokenCount: 42000,
            },
            {
              id: 'rt-conv-redesign-ops',
              conversationId: 'conv-redesign-ops',
              title: 'Operator Handoff',
              status: 'running',
              updatedAt: 99,
              lastMessagePreview: 'Queued operator follow-up and review items.',
              messageCount: 24,
              toolCallCount: 16,
              approvalCount: 2,
              tokenCount: 47000,
            },
            {
              id: 'rt-conv-redesign-review',
              conversationId: 'conv-redesign-review',
              title: 'QA Review Sweep',
              status: 'completed',
              updatedAt: 98,
              lastMessagePreview: 'Closed dashboard accessibility fixes.',
              messageCount: 18,
              toolCallCount: 8,
              approvalCount: 1,
              tokenCount: 36000,
            },
          ]
        : recentConversations
            .filter(item => item.projectId === project.id)
            .map(item => ({
              id: item.sessionId,
              conversationId: item.id,
              title: item.title,
              status: item.status,
              updatedAt: item.updatedAt,
              lastMessagePreview: item.lastMessagePreview,
              messageCount: 12,
              toolCallCount: 5,
              approvalCount: 0,
              tokenCount: 24000,
            })),
      toolRanking: project.id === 'proj-redesign'
        ? [
            { id: 'read', label: 'Read', value: 18, helper: 'Documentation and resource lookups' },
            { id: 'terminal', label: 'Terminal', value: 14, helper: 'Command execution and checks' },
            { id: 'ops-mcp', label: 'Ops MCP', value: 9, helper: 'Workspace-side integrations' },
          ]
        : [
            { id: 'terminal', label: 'Terminal', value: 5, helper: 'Operational validation' },
          ],
      resourceBreakdown: project.id === 'proj-redesign'
        ? [
            { id: 'resources', label: 'Resources', value: 8 },
            { id: 'knowledge', label: 'Knowledge', value: 5 },
            { id: 'agents', label: 'Agents', value: 4 },
            { id: 'teams', label: 'Teams', value: 2 },
            { id: 'tools', label: 'Tools', value: 3 },
            { id: 'sessions', label: 'Sessions', value: 3 },
          ]
        : [
            { id: 'resources', label: 'Resources', value: 4 },
            { id: 'knowledge', label: 'Knowledge', value: 2 },
            { id: 'agents', label: 'Agents', value: 1 },
            { id: 'teams', label: 'Teams', value: 1 },
            { id: 'tools', label: 'Tools', value: 2 },
          ],
      modelBreakdown: project.id === 'proj-redesign'
        ? [
            { id: 'anthropic-primary', label: 'Claude Primary', value: 82000 },
            { id: 'openai-primary', label: 'GPT-4o', value: 43000 },
          ]
        : [
            { id: 'anthropic-primary', label: 'Claude Primary', value: 24000 },
          ],
      recentConversations: recentConversations.filter(item => item.projectId === project.id),
      recentActivity: recentActivity.filter(item => project.id === 'proj-redesign' || item.id === 'activity-launch'),
      recentTasks: recentTasksByProjectId[project.id] ?? [],
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
      scope: 'workspace',
      visibility: 'public',
      ownerUserId: 'user-owner',
      storagePath: local ? 'data/resources/workspace/shared-specs' : '/remote/shared/runbooks',
      previewKind: 'folder',
      status: 'healthy',
      updatedAt: 100,
      tags: ['docs', 'shared'],
    },
    {
      id: `${workspace.id}-res-workspace-2`,
      workspaceId: workspace.id,
      kind: 'file',
      name: local ? 'Personal Scratchpad' : 'My Checklist',
      location: local ? '/workspace/personal/scratchpad.md' : '/remote/users/user-owner/checklist.md',
      origin: 'source',
      scope: 'personal',
      visibility: 'private',
      ownerUserId: 'user-owner',
      storagePath: local ? 'data/resources/workspace/personal/scratchpad.md' : '/remote/users/user-owner/checklist.md',
      contentType: 'text/markdown',
      byteSize: 96,
      previewKind: 'markdown',
      status: 'healthy',
      updatedAt: 99,
      tags: ['personal', 'notes'],
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
        scope: 'project',
        visibility: 'public',
        ownerUserId: 'user-owner',
        storagePath: `${project.resourceDirectory}/brief.md`,
        contentType: 'text/markdown',
        byteSize: 148,
        previewKind: 'markdown',
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
        scope: 'project',
        visibility: 'public',
        ownerUserId: 'user-owner',
        previewKind: 'url',
        sourceArtifactId: project.id === 'proj-redesign' ? 'artifact-run-conv-redesign' : undefined,
        status: 'healthy',
        updatedAt: 102,
        tags: ['api'],
      },
      {
        id: `${project.id}-res-3`,
        workspaceId: workspace.id,
        projectId: project.id,
        kind: 'folder',
        name: `${project.name} Personal Notes`,
        location: `${project.resourceDirectory}/personal-notes`,
        origin: 'source',
        scope: 'personal',
        visibility: 'private',
        ownerUserId: 'user-owner',
        storagePath: `${project.resourceDirectory}/personal-notes`,
        previewKind: 'folder',
        status: 'healthy',
        updatedAt: 103,
        tags: ['notes', 'personal'],
      },
      {
        id: `${project.id}-res-4`,
        workspaceId: workspace.id,
        projectId: project.id,
        kind: 'folder',
        name: `${project.name} Shared Assets`,
        location: `${project.resourceDirectory}/shared-assets`,
        origin: 'source',
        scope: 'workspace',
        visibility: 'public',
        ownerUserId: 'user-owner',
        storagePath: `${project.resourceDirectory}/shared-assets`,
        previewKind: 'folder',
        status: 'healthy',
        updatedAt: 104,
        tags: ['shared', 'assets'],
      },
    ],
  ]))

  const resourceContents: Record<string, WorkspaceResourceContentDocument> = {
    [`${workspace.id}-res-workspace-2`]: {
      resourceId: `${workspace.id}-res-workspace-2`,
      previewKind: 'markdown',
      fileName: 'scratchpad.md',
      contentType: 'text/markdown',
      byteSize: 96,
      textContent: '# Scratchpad\n\n- Review resource preview\n- Verify remote directory selection',
    },
    'proj-redesign-res-1': {
      resourceId: 'proj-redesign-res-1',
      previewKind: 'markdown',
      fileName: 'brief.md',
      contentType: 'text/markdown',
      byteSize: 148,
      textContent: '# Desktop Redesign Brief\n\n- Rebuild the project resources workbench.\n- Align resource import with the project directory.\n',
    },
    'proj-redesign-res-2': {
      resourceId: 'proj-redesign-res-2',
      previewKind: 'url',
      externalUrl: 'https://example.test/proj-redesign/api',
    },
    'proj-governance-res-1': {
      resourceId: 'proj-governance-res-1',
      previewKind: 'markdown',
      fileName: 'brief.md',
      contentType: 'text/markdown',
      byteSize: 132,
      textContent: '# Workspace Governance Brief\n\n- Audit roles, policies, and resources.\n',
    },
    'proj-launch-res-1': {
      resourceId: 'proj-launch-res-1',
      previewKind: 'markdown',
      fileName: 'brief.md',
      contentType: 'text/markdown',
      byteSize: 128,
      textContent: '# Launch Readiness Brief\n\n- Validate the release runway and checkpoints.\n',
    },
  }

  const resourceChildren: Record<string, WorkspaceResourceChildrenRecord[]> = {
    [`${workspace.id}-res-workspace-1`]: [
      {
        name: 'design-system',
        relativePath: 'design-system',
        kind: 'folder',
        previewKind: 'folder',
        updatedAt: 100,
      },
      {
        name: 'workspace-resources.pdf',
        relativePath: 'workspace-resources.pdf',
        kind: 'file',
        previewKind: 'pdf',
        contentType: 'application/pdf',
        byteSize: 4096,
        updatedAt: 100,
      },
    ],
    'proj-redesign-res-3': [
      {
        name: 'ideas.md',
        relativePath: 'ideas.md',
        kind: 'file',
        previewKind: 'markdown',
        contentType: 'text/markdown',
        byteSize: 84,
        updatedAt: 103,
      },
    ],
    'proj-redesign-res-4': [
      {
        name: 'hero.png',
        relativePath: 'hero.png',
        kind: 'file',
        previewKind: 'image',
        contentType: 'image/png',
        byteSize: 2048,
        updatedAt: 104,
      },
      {
        name: 'handoff.pdf',
        relativePath: 'handoff.pdf',
        kind: 'file',
        previewKind: 'pdf',
        contentType: 'application/pdf',
        byteSize: 8192,
        updatedAt: 104,
      },
    ],
    'proj-governance-res-3': [
      {
        name: 'policy-drafts',
        relativePath: 'policy-drafts',
        kind: 'folder',
        previewKind: 'folder',
        updatedAt: 103,
      },
    ],
    'proj-governance-res-4': [
      {
        name: 'rbac-matrix.csv',
        relativePath: 'rbac-matrix.csv',
        kind: 'file',
        previewKind: 'text',
        contentType: 'text/csv',
        byteSize: 512,
        updatedAt: 104,
      },
    ],
    'proj-launch-res-3': [
      {
        name: 'checklist.md',
        relativePath: 'checklist.md',
        kind: 'file',
        previewKind: 'markdown',
        contentType: 'text/markdown',
        byteSize: 120,
        updatedAt: 103,
      },
    ],
    'proj-launch-res-4': [
      {
        name: 'cutover.mp4',
        relativePath: 'cutover.mp4',
        kind: 'file',
        previewKind: 'video',
        contentType: 'video/mp4',
        byteSize: 12_000,
        updatedAt: 104,
      },
    ],
  }

  const remoteDirectories: Record<string, WorkspaceDirectoryBrowserResponse> = {
    '': {
      currentPath: '/remote',
      entries: [
        { name: 'projects', path: '/remote/projects' },
        { name: 'shared', path: '/remote/shared' },
      ],
    },
    '/remote': {
      currentPath: '/remote',
      entries: [
        { name: 'projects', path: '/remote/projects' },
        { name: 'shared', path: '/remote/shared' },
      ],
    },
    '/remote/projects': {
      currentPath: '/remote/projects',
      parentPath: '/remote',
      entries: [
        { name: 'launch-readiness', path: '/remote/projects/launch-readiness' },
        { name: 'agent-studio', path: '/remote/projects/agent-studio' },
      ],
    },
    '/remote/projects/launch-readiness': {
      currentPath: '/remote/projects/launch-readiness',
      parentPath: '/remote/projects',
      entries: [
        { name: 'resources', path: '/remote/projects/launch-readiness/resources' },
      ],
    },
    '/remote/projects/launch-readiness/resources': {
      currentPath: '/remote/projects/launch-readiness/resources',
      parentPath: '/remote/projects/launch-readiness',
      entries: [
        { name: 'design-assets', path: '/remote/projects/launch-readiness/resources/design-assets' },
      ],
    },
  }

  const deliverables: DeliverableSummary[] = [
    {
      id: 'artifact-run-conv-redesign',
      workspaceId: workspace.id,
      projectId: 'proj-redesign',
      conversationId: 'conv-redesign',
      title: 'Runtime Delivery Summary',
      status: 'review',
      latestVersion: 3,
      latestVersionRef: {
        artifactId: 'artifact-run-conv-redesign',
        title: 'Runtime Delivery Summary',
        version: 3,
        previewKind: 'markdown',
        contentType: 'text/markdown',
        updatedAt: 103,
      },
      previewKind: 'markdown',
      promotionState: 'candidate',
      updatedAt: 103,
      contentType: 'text/markdown',
    },
    {
      id: 'artifact-run-conv-approval',
      workspaceId: workspace.id,
      projectId: 'proj-redesign',
      conversationId: 'conv-approval',
      title: 'Approval Command Output',
      status: 'draft',
      latestVersion: 1,
      latestVersionRef: {
        artifactId: 'artifact-run-conv-approval',
        title: 'Approval Command Output',
        version: 1,
        previewKind: 'text',
        contentType: 'text/plain',
        updatedAt: 104,
      },
      previewKind: 'text',
      promotionState: 'not-promoted',
      updatedAt: 104,
      contentType: 'text/plain',
    },
    {
      id: 'artifact-1',
      workspaceId: workspace.id,
      projectId: 'proj-redesign',
      conversationId: 'conv-redesign',
      title: 'Workspace Protocol Baseline',
      status: 'approved',
      latestVersion: 5,
      latestVersionRef: {
        artifactId: 'artifact-1',
        title: 'Workspace Protocol Baseline',
        version: 5,
        previewKind: 'markdown',
        contentType: 'text/markdown',
        updatedAt: 100,
      },
      previewKind: 'markdown',
      promotionState: 'promoted',
      updatedAt: 100,
      contentType: 'text/markdown',
    },
  ]

  const inboxItems: InboxItemRecord[] = local
    ? [
        {
          id: 'inbox-approval-runtime',
          workspaceId: workspace.id,
          projectId: 'proj-redesign',
          itemType: 'approval',
          title: 'Runtime approval pending',
          description: 'A runtime command is waiting for approval before execution can continue.',
          status: 'pending',
          priority: 'high',
          actionable: true,
          routeTo: `/workspaces/${workspace.id}/projects/proj-redesign/settings`,
          actionLabel: 'Review approval',
          createdAt: 105,
        },
        {
          id: 'inbox-note-governance',
          workspaceId: workspace.id,
          projectId: 'proj-governance',
          itemType: 'follow_up',
          title: 'Governance review completed',
          description: 'The latest governance review has been recorded for audit reference.',
          status: 'completed',
          priority: 'medium',
          actionable: false,
          createdAt: 99,
        },
      ]
    : [
        {
          id: 'inbox-enterprise-review',
          workspaceId: workspace.id,
          projectId: 'proj-launch',
          itemType: 'approval',
          title: 'Launch review pending',
          description: 'The launch checklist needs operator approval.',
          status: 'pending',
          priority: 'high',
          actionable: true,
          routeTo: `/workspaces/${workspace.id}/projects/proj-launch/dashboard`,
          actionLabel: 'Open checklist',
          createdAt: 105,
        },
      ]

  const workspaceKnowledge: KnowledgeRecord[] = [
    {
      id: `${workspace.id}-knowledge-1`,
      workspaceId: workspace.id,
      title: local ? 'Workspace Protocol Baseline' : 'Enterprise Release Policy',
      summary: 'Projection snapshot used by the desktop shell.',
      kind: 'shared',
      scope: 'workspace',
      status: 'shared',
      visibility: 'public',
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
        scope: 'project',
        status: 'reviewed',
        visibility: 'public',
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
          leaderRef: 'agent:agent-architect',
          memberRefs: ['agent:agent-architect', 'agent:agent-coder'],
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
          leaderRef: 'agent:agent-redesign',
          memberRefs: ['agent:agent-redesign'],
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
          leaderRef: 'agent:agent-template-finance',
          memberRefs: ['agent:agent-template-finance'],
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
          leaderRef: 'agent:agent-gov',
          memberRefs: ['agent:agent-gov'],
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
            runtimeSupport: {
              prompt: true,
              conversation: true,
              toolLoop: true,
              streaming: false,
            },
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
            runtimeSupport: {
              prompt: true,
              conversation: true,
              toolLoop: true,
              streaming: false,
            },
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
            runtimeSupport: {
              prompt: true,
              conversation: true,
              toolLoop: true,
              streaming: false,
            },
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
            runtimeSupport: {
              prompt: true,
              conversation: true,
              toolLoop: true,
              streaming: false,
            },
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
  } satisfies { entries: WorkspaceToolCatalogEntry[] }
  const transformedToolCatalog = options.toolCatalogTransform
    ? { entries: options.toolCatalogTransform(clone(toolCatalog.entries)) }
    : toolCatalog
  const managementProjectionBase = deriveCapabilityManagementProjection(transformedToolCatalog.entries)
  const managementProjection = options.managementProjectionTransform
    ? options.managementProjectionTransform(clone(managementProjectionBase))
    : managementProjectionBase

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
      displayName: 'Workspace Operator',
      avatar: 'data:image/png;base64,iVBORw0KGgo=',
      status: 'active',
      passwordState: 'set',
    },
    ...Array.from({ length: options.extraAccessUsersCount ?? 0 }, (_, index) => {
      const suffix = String(index + 1).padStart(2, '0')
      return {
        id: `user-extra-${suffix}`,
        username: `extra-${suffix}`,
        displayName: `Access User ${suffix}`,
        avatar: 'data:image/png;base64,iVBORw0KGgo=',
        status: 'active' as const,
        passwordState: 'set' as const,
      }
    }),
  ]

  const orgUnits: OrgUnitRecord[] = [{
    id: 'org-root',
    parentId: undefined,
    code: 'workspace-root',
    name: workspace.name,
    status: 'active',
  }]

  if (options.includeAccessOrgHierarchy) {
    orgUnits.push(
      {
        id: 'org-engineering',
        parentId: 'org-root',
        code: 'engineering',
        name: 'Engineering',
        status: 'active',
      },
      {
        id: 'org-platform',
        parentId: 'org-engineering',
        code: 'platform',
        name: 'Platform',
        status: 'active',
      },
      {
        id: 'org-design',
        parentId: 'org-root',
        code: 'design',
        name: 'Design',
        status: 'disabled',
      },
    )
  }

  const positions: PositionRecord[] = []
  const userGroups: UserGroupRecord[] = []

  const userOrgAssignments: UserOrgAssignmentRecord[] = users.map((user, index) => ({
    userId: user.id,
    orgUnitId: options.includeAccessOrgHierarchy
      ? (index % 3 === 0 ? 'org-platform' : index % 3 === 1 ? 'org-engineering' : 'org-design')
      : 'org-root',
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
    'runtime.submit_turn',
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
    'runtime.submit_turn',
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
    'pet.view',
    'artifact.view',
    'inbox.view',
  ] as const

  const roles: AccessRoleRecord[] = [
    {
      id: 'role-owner',
      name: 'Owner',
      code: 'system.owner',
      description: 'Full workspace access.',
      source: 'system',
      editable: false,
      status: 'active',
      permissionCodes: [...ownerPermissionCodes],
    },
    {
      id: 'role-operator',
      name: 'Admin',
      code: 'system.admin',
      description: 'Daily operations access.',
      source: 'system',
      editable: false,
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

  const state: WorkspaceFixtureState = {
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
    projectPromotionRequests,
    dashboards,
    taskIdSequence: 0,
    taskRunIdSequence: 0,
    taskInterventionIdSequence: 0,
    taskDetailsByKey: new Map(),
    taskRunsByKey: new Map(),
    taskInterventionsByKey: new Map(),
    workspaceResources,
    projectResources,
    resourceContents,
    resourceChildren,
    remoteDirectories,
    deliverables,
    deliverableVersionSummaries: new Map(),
    deliverableVersionContents: new Map(),
    inboxItems,
    workspaceKnowledge,
    projectKnowledge,
    agents,
    projectAgentLinks,
    teams,
    projectTeamLinks,
    catalog,
    toolCatalog: transformedToolCatalog,
    managementProjection,
    skillDocuments,
    skillFiles,
    mcpDocuments,
    tools,
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
    resourcePolicies,
    protectedResourceMetadata,
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

  if (local) {
    const approvalRun: TaskRunSummary = {
      id: 'task-run-redesign-approval-gate',
      taskId: 'task-redesign-approval-gate',
      triggerType: 'manual',
      status: 'waiting_approval',
      sessionId: 'task-session-redesign-approval-gate',
      conversationId: 'task-conversation-approval',
      runtimeRunId: 'runtime-run-redesign-approval-gate',
      actorRef: 'agent-architect',
      startedAt: 88,
      completedAt: null,
      resultSummary: 'Waiting for approval before publishing the release brief update.',
      pendingApprovalId: 'approval-task-run-redesign-approval-gate',
      failureCategory: null,
      failureSummary: null,
      viewStatus: 'attention',
      attentionReasons: ['needs_approval'],
      attentionUpdatedAt: 89,
      deliverableRefs: [
        {
          artifactId: 'artifact-run-conv-approval',
          title: 'Approval Command Output',
          version: 1,
          previewKind: 'text',
          contentType: 'text/plain',
          updatedAt: 89,
        },
      ],
      artifactRefs: [],
      latestTransition: {
        kind: 'waiting_approval',
        summary: 'The run is paused for approval before the publish step.',
        at: 89,
        runId: 'runtime-run-redesign-approval-gate',
      },
    }

    const approvalDetail: TaskDetail = {
      id: 'task-redesign-approval-gate',
      projectId: 'proj-redesign',
      title: 'Approval Gate Review',
      goal: 'Pause before publishing the release brief update and wait for operator approval.',
      brief: 'Only publish the refreshed release brief after operator approval is recorded for the final wording.',
      defaultActorRef: 'agent-architect',
      status: 'attention',
      scheduleSpec: null,
      nextRunAt: null,
      lastRunAt: 89,
      latestResultSummary: 'Waiting for approval before publishing the release brief update.',
      latestFailureCategory: null,
      latestTransition: {
        kind: 'waiting_approval',
        summary: 'The active run is paused until approval is recorded for the publish step.',
        at: 89,
        runId: 'task-run-redesign-approval-gate',
      },
      viewStatus: 'attention',
      attentionReasons: ['needs_approval'],
      attentionUpdatedAt: 89,
      activeTaskRunId: 'task-run-redesign-approval-gate',
      analyticsSummary: {
        runCount: 2,
        manualRunCount: 2,
        scheduledRunCount: 0,
        completionCount: 1,
        failureCount: 0,
        takeoverCount: 0,
        approvalRequiredCount: 1,
        averageRunDurationMs: 300000,
        lastSuccessfulRunAt: 82,
      },
      contextBundle: {
        refs: [
          {
            kind: 'deliverable',
            refId: 'artifact-run-conv-redesign',
            title: 'Runtime Delivery Summary',
            subtitle: 'Latest release brief draft',
            versionRef: 'v3',
            pinMode: 'snapshot',
          },
        ],
        pinnedInstructions: 'Hold the publish step until an operator approves the final release wording.',
        resolutionMode: 'explicit_plus_project_defaults',
        lastResolvedAt: 89,
      },
      latestDeliverableRefs: [
        {
          artifactId: 'artifact-run-conv-approval',
          title: 'Approval Command Output',
          version: 1,
          previewKind: 'text',
          contentType: 'text/plain',
          updatedAt: 89,
        },
      ],
      latestArtifactRefs: [],
      runHistory: [approvalRun],
      interventionHistory: [],
      activeRun: approvalRun,
      createdBy: 'user-owner',
      updatedBy: 'user-operator',
      createdAt: 84,
      updatedAt: 89,
    }

    const approvalKey = 'proj-redesign:task-redesign-approval-gate'
    state.taskDetailsByKey.set(approvalKey, clone(approvalDetail))
    state.taskRunsByKey.set(approvalKey, clone(approvalDetail.runHistory))
    state.taskInterventionsByKey.set(approvalKey, [])
  }

  return state
}
