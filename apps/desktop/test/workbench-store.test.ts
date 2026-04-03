import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { ConversationIntent } from '@octopus/schema'
import { useWorkbenchStore } from '@/stores/workbench'

describe('useWorkbenchStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('switches workspace and keeps project-scoped surfaces in sync', () => {
    const store = useWorkbenchStore()

    store.selectWorkspace('ws-enterprise')

    expect(store.activeWorkspace?.id).toBe('ws-enterprise')
    expect(store.activeProject?.workspaceId).toBe('ws-enterprise')
    expect(store.workspaceProjects.every((project) => project.workspaceId === 'ws-enterprise')).toBe(true)
    expect(store.workspaceInbox.every((item) => item.workspaceId === 'ws-enterprise')).toBe(true)
  })

  it('builds workspace overview and project dashboard snapshots from the seeded mock data', () => {
    const store = useWorkbenchStore()

    expect(store.workspaceOverview.workspaceId).toBe('ws-local')
    expect(store.workspaceOverview.userMetrics).toHaveLength(3)
    expect(store.workspaceOverview.userActivity).toHaveLength(6)
    expect(store.workspaceOverview.workspaceMetrics).toHaveLength(8)
    expect(store.workspaceOverview.projectSummary.projectId).toBe('proj-redesign')
    expect(store.workspaceOverview.projectSummary.conversationTokenTop.slice(0, 1)).toEqual([
      expect.objectContaining({
        id: 'conv-redesign',
      }),
    ])

    expect(store.projectDashboard.project.id).toBe('proj-redesign')
    expect(store.projectDashboard.resourceMetrics).toHaveLength(5)
    expect(store.projectDashboard.progress.progress).toBe(64)
    expect(store.projectDashboard.dataMetrics).toHaveLength(9)
    expect(store.projectDashboard.activity).toHaveLength(2)
  })

  it('updates project details from the project dashboard editor', () => {
    const store = useWorkbenchStore()

    store.updateProjectDetails('proj-redesign', {
      name: 'Desktop Shell Console',
      goal: '统一概览和控制台的统计口径。',
      phase: 'Execution',
      summary: '当前正在拆分工作区概览与项目控制台。',
    })

    expect(store.activeProject).toMatchObject({
      name: 'Desktop Shell Console',
      goal: '统一概览和控制台的统计口径。',
      phase: 'Execution',
      summary: '当前正在拆分工作区概览与项目控制台。',
    })
  })

  it('requests artifact review by creating an approval item and updating conversation intent', () => {
    const store = useWorkbenchStore()

    store.requestArtifactReview('art-roadmap')

    expect(store.activeConversation?.intent).toBe(ConversationIntent.REVIEW)
    expect(store.workspaceInbox.some((item) => item.relatedId === 'art-roadmap' && item.type === 'knowledge_promotion_approval')).toBe(true)
  })

  it('approves a pending inbox item and resumes the linked run', () => {
    const store = useWorkbenchStore()
    const pendingItem = store.workspaceInbox.find((item) => item.status === 'pending')

    expect(pendingItem).toBeDefined()

    store.resolveInboxItem(pendingItem!.id, 'approve')

    expect(store.workspaceInbox.find((item) => item.id === pendingItem!.id)?.status).toBe('resolved')
    expect(store.activeConversation?.intent).toBe(ConversationIntent.EXECUTE)
    expect(store.activeRun?.status).toBe('running')
  })

  it('separates workspace-level agents from project-level agents while keeping project references visible', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')
    store.createProjectAgentCopy('agent-architect')

    expect(store.workspaceLevelAgents.some((agent) => agent.id === 'agent-architect')).toBe(true)
    expect(store.workspaceLevelAgents.some((agent) => agent.id === 'agent-architect-copy-proj-redesign')).toBe(false)
    expect(store.projectAgents.some((agent) => agent.id === 'agent-architect')).toBe(true)
    expect(store.projectAgents.some((agent) => agent.id === 'agent-architect-copy-proj-redesign')).toBe(true)
  })

  it('creates workspace and project scoped agent assets with predictable ownership', () => {
    const store = useWorkbenchStore()

    const workspaceAgent = store.createAgent('workspace')
    store.selectProject('proj-redesign')
    const projectAgent = store.createAgent('project')

    expect(workspaceAgent.owner).toBe('workspace:ws-local')
    expect(workspaceAgent.scope).toBe('workspace')
    expect(workspaceAgent.title).toBe('工作区能力专员')
    expect(workspaceAgent.metrics?.activeTasks).toBe(0)
    expect(projectAgent.owner).toBe('project:proj-redesign')
    expect(projectAgent.scope).toBe('project')
    expect(projectAgent.title).toBe('项目执行专员')
    expect(store.activeProject?.agentIds).toContain(projectAgent.id)
  })

  it('creates workspace and project scoped team assets with predictable ownership', () => {
    const store = useWorkbenchStore()

    const workspaceTeam = store.createTeam('workspace')
    store.selectProject('proj-redesign')
    const projectTeam = store.createTeam('project')

    expect(workspaceTeam.workspaceId).toBe('ws-local')
    expect(workspaceTeam.useScope).toBe('workspace')
    expect(workspaceTeam.title).toBe('工作区协作编组')
    expect(workspaceTeam.workflow).toEqual(['方向同步', '能力分工', '协作交付'])
    expect(projectTeam.projectId).toBe('proj-redesign')
    expect(projectTeam.useScope).toBe('project')
    expect(projectTeam.title).toBe('项目协同小队')
    expect(store.activeProject?.teamIds).toContain(projectTeam.id)
    expect(projectTeam.structureMode).toBe('flow')
    expect(projectTeam.structureNodes[0]).toMatchObject({
      position: expect.any(Object),
    })
  })

  it('removes project references without deleting workspace-owned assets', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')
    store.removeProjectAgentReference('agent-architect')
    store.removeProjectTeamReference('team-studio')

    expect(store.projectReferencedAgents.some((agent) => agent.id === 'agent-architect')).toBe(false)
    expect(store.workspaceLevelAgents.some((agent) => agent.id === 'agent-architect')).toBe(true)
    expect(store.projectReferencedTeams.some((team) => team.id === 'team-studio')).toBe(false)
    expect(store.workspaceLevelTeams.some((team) => team.id === 'team-studio')).toBe(true)
  })

  it('builds stats, recommendations, and filter facets for the agent center', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')

    expect(store.agentCenterStats).toMatchObject({
      agentCount: 2,
      teamCount: 2,
      onlineCount: 2,
    })
    expect(store.agentCenterStats.activeTaskCount).toBeGreaterThan(0)
    expect(store.agentCenterRecommendations.agents[0]?.id).toBe('agent-coder')
    expect(store.agentCenterRecommendations.teams[0]?.id).toBe('team-redesign-copy')
    expect(store.agentFilterFacets).toContain('前端开发')
    expect(store.teamFilterFacets).toContain('设计')
  })

  it('deletes project-owned assets and removes their project linkage', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')
    const agent = store.createAgent('project')
    const team = store.createTeam('project')

    store.deleteAgent(agent.id)
    store.deleteTeam(team.id)

    expect(store.agents.some((item) => item.id === agent.id)).toBe(false)
    expect(store.teams.some((item) => item.id === team.id)).toBe(false)
    expect(store.activeProject?.agentIds).not.toContain(agent.id)
    expect(store.activeProject?.teamIds).not.toContain(team.id)
  })

  it('creates a mock workspace with a starter project and conversation and selects it', () => {
    const store = useWorkbenchStore()

    store.createWorkspace()

    expect(store.workspaces).toHaveLength(3)
    expect(store.currentWorkspaceId).toBe('ws-mock-3')
    expect(store.activeWorkspace?.projectIds).toEqual(['proj-mock-3'])
    expect(store.currentProjectId).toBe('proj-mock-3')
    expect(store.currentConversationId).toBe('conv-mock-3')
    expect(store.workspaceProjects.some((project) => project.id === 'proj-mock-3')).toBe(true)
    expect(store.projectConversations.some((conversation) => conversation.id === 'conv-mock-3')).toBe(true)
  })

  it('creates a mock conversation in the current project and selects it', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')
    const conversation = store.createConversation()

    expect(conversation.projectId).toBe('proj-redesign')
    expect(store.currentConversationId).toBe(conversation.id)
    expect(store.projectConversations.some((item) => item.id === conversation.id)).toBe(true)
    expect(store.activeRun?.conversationId).toBe(conversation.id)
  })

  it('sends a message from the composer payload and preserves mock metadata', () => {
    const store = useWorkbenchStore()

    store.selectConversation('conv-redesign')
    store.completeActiveRun('completed')
    const beforeCount = store.conversationMessages.length

    store.sendMessage({
      content: '请把右侧详情面板改为只在会话页显示。',
      modelId: 'gpt-4o',
      permissionMode: 'readonly',
      actorKind: 'team',
      actorId: 'team-redesign-copy',
      resourceIds: ['res-prd-folder', 'art-roadmap'],
      attachments: [
        {
          id: 'file-brief',
          name: '需求说明.md',
          kind: 'file',
        },
      ],
    })

    expect(store.conversationMessages).toHaveLength(beforeCount + 2)
    const sentMessage = store.conversationMessages.find((message) => message.content === '请把右侧详情面板改为只在会话页显示。')

    expect(sentMessage).toMatchObject({
      senderType: 'user',
      content: '请把右侧详情面板改为只在会话页显示。',
      modelId: 'gpt-4o',
      permissionMode: 'readonly',
      actorKind: 'team',
      actorId: 'team-redesign-copy',
      resourceIds: ['res-prd-folder', 'art-roadmap'],
    })
    expect(sentMessage?.attachments).toEqual([
      {
        id: 'file-brief',
        name: '需求说明.md',
        kind: 'file',
      },
    ])
    expect(store.activeConversation?.activeTeamId).toBe('team-redesign-copy')
  })

  it('queues follow-up messages while the conversation is still busy', () => {
    const store = useWorkbenchStore()

    store.selectConversation('conv-redesign')
    const beforeCount = store.conversationMessages.length

    store.sendMessage({
      content: '继续补齐会话页右侧面板的数据分组。',
      modelId: 'gpt-4o',
      permissionMode: 'auto',
      actorKind: undefined,
      actorId: undefined,
      resourceIds: [],
      attachments: [],
    })

    expect(store.conversationMessages).toHaveLength(beforeCount)
    expect(store.activeConversationQueue).toHaveLength(1)
    expect(store.activeConversationQueue[0]).toMatchObject({
      content: '继续补齐会话页右侧面板的数据分组。',
      requestedActorKind: undefined,
      requestedActorId: undefined,
      resolvedActorKind: 'team',
      resolvedActorId: 'team-redesign-copy',
    })
    expect(store.activeConversation?.statusNote).toBe('runtime.conversation.messageQueued')
  })

  it('removes queued messages and drains the queue after the active run completes', () => {
    const store = useWorkbenchStore()

    store.selectConversation('conv-redesign')

    store.sendMessage({
      content: '先进入队列，等待当前运行结束。',
      modelId: 'gpt-4o',
      permissionMode: 'auto',
      actorKind: undefined,
      actorId: undefined,
      resourceIds: [],
      attachments: [],
    })

    const queuedItemId = store.activeConversationQueue[0]?.id
    expect(queuedItemId).toBeDefined()

    store.removeQueuedMessage(queuedItemId!)
    expect(store.activeConversationQueue).toHaveLength(0)

    const beforeMessageCount = store.conversationMessages.length

    store.sendMessage({
      content: '这条消息会在运行完成后自动出队执行。',
      modelId: 'gpt-4o',
      permissionMode: 'readonly',
      actorKind: undefined,
      actorId: undefined,
      resourceIds: [],
      attachments: [],
    })

    expect(store.activeConversationQueue).toHaveLength(1)

    store.completeActiveRun('completed')

    expect(store.activeConversationQueue).toHaveLength(0)
    expect(store.conversationMessages).toHaveLength(beforeMessageCount + 2)
    expect(store.conversationMessages.at(-2)).toMatchObject({
      senderType: 'user',
      content: '这条消息会在运行完成后自动出队执行。',
      usedDefaultActor: true,
      actorKind: 'team',
      actorId: 'team-redesign-copy',
      permissionMode: 'readonly',
    })
  })

  it('rolls back to a selected message and removes later derived entities', () => {
    const store = useWorkbenchStore()

    store.selectConversation('conv-redesign')
    store.completeActiveRun('completed')

    store.sendMessage({
      content: '生成一轮新的消息、知识和资源，再验证回滚。',
      modelId: 'gpt-4o',
      permissionMode: 'auto',
      actorKind: 'agent',
      actorId: 'agent-coder',
      resourceIds: [],
      attachments: [],
    })

    expect(store.artifacts.some((artifact) => artifact.id.startsWith('art-generated-'))).toBe(true)
    expect(store.resources.some((resource) => resource.id.startsWith('res-generated-'))).toBe(true)
    expect(store.resources.find((resource) => resource.id.startsWith('res-generated-'))?.origin).toBe('generated')
    expect(store.knowledge.some((entry) => entry.id.startsWith('knowledge-generated-'))).toBe(true)
    expect(store.conversationMemories.some((memory) => memory.id.startsWith('memory-conversation-'))).toBe(true)
    expect(store.traces.some((trace) => trace.id.startsWith('trace-generated-'))).toBe(true)

    store.rollbackConversationToMessage('msg-redesign-2')

    expect(store.conversationMessages.map((message) => message.id)).toEqual(['msg-redesign-1', 'msg-redesign-2'])
    expect(store.activeConversationQueue).toHaveLength(0)
    expect(store.artifacts.some((artifact) => artifact.id.startsWith('art-generated-'))).toBe(false)
    expect(store.resources.some((resource) => resource.id.startsWith('res-generated-'))).toBe(false)
    expect(store.knowledge.some((entry) => entry.id.startsWith('knowledge-generated-'))).toBe(false)
    expect(store.conversationMemories.some((memory) => memory.id.startsWith('memory-conversation-'))).toBe(false)
    expect(store.traces.some((trace) => trace.id.startsWith('trace-generated-'))).toBe(false)
    expect(store.activeConversation?.statusNote).toBe('runtime.conversation.rolledBack')
    expect(store.activeRun?.currentStep).toBe('runtime.run.rolledBackToCheckpoint')
  })

  it('toggles pet chat, sends pet replies, and keeps per-user pet assignment stable', () => {
    const store = useWorkbenchStore()

    const originalPetId = store.currentUserPet?.id
    expect(originalPetId).toBeDefined()
    expect(store.currentUserPetMessages.at(0)?.sender).toBe('pet')

    expect(store.currentUserPetPresence?.chatOpen).toBe(false)
    store.togglePetChat(true)
    expect(store.currentUserPetPresence?.chatOpen).toBe(true)
    expect(store.currentUserPetPresence?.motionState).toBe('chat')

    const beforeCount = store.currentUserPetMessages.length
    const reply = store.sendPetMessage('你好，今天的工作安排是什么？')
    expect(reply?.sender).toBe('pet')
    expect(store.currentUserPetMessages).toHaveLength(beforeCount + 2)
    expect(store.currentUserPetMessages.at(-2)).toMatchObject({
      sender: 'user',
      content: '你好，今天的工作安排是什么？',
    })
    expect(store.currentUserPetMessages.at(-1)?.content).toContain(store.currentUserPet?.displayName ?? '')

    store.switchCurrentUser('user-operator')
    const operatorPetId = store.currentUserPet?.id
    expect(operatorPetId).toBeDefined()
    expect(operatorPetId).not.toBe(originalPetId)

    store.switchCurrentUser('user-admin')
    expect(store.currentUserPet?.id).toBe(originalPetId)
  })

  it('moves the pet within bounded screen offsets', () => {
    const store = useWorkbenchStore()
    const before = { ...store.currentUserPetPresence!.position }

    store.nudgePetPosition(-100, -100)
    expect(store.currentUserPetPresence?.position.x).toBeGreaterThanOrEqual(16)
    expect(store.currentUserPetPresence?.position.y).toBeGreaterThanOrEqual(16)

    store.nudgePetPosition(20, 12)
    expect(store.currentUserPetPresence?.position.x).toBeGreaterThanOrEqual(before.x)
    expect(store.currentUserPetPresence?.position.y).toBeGreaterThanOrEqual(before.y)
  })

  it('creates project resources for files and folders and exposes them in the current project scope', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')

    const fileResource = store.createProjectResource('file')
    const folderResource = store.createProjectResource('folder')

    expect(store.projectResources.some((item) => item.id === fileResource.id && item.kind === 'file' && item.origin === 'source')).toBe(true)
    expect(store.projectResources.some((item) => item.id === folderResource.id && item.kind === 'folder' && item.origin === 'source')).toBe(true)
    expect(store.activeProject?.resourceIds).toContain(fileResource.id)
    expect(store.activeProject?.resourceIds).toContain(folderResource.id)
  })

  it('creates and updates url resources in the current project scope', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')

    const urlResource = store.createProjectResource('url', {
      name: 'Design spec',
      location: 'https://example.com/spec',
    })

    expect(store.projectResources.some((item) => item.id === urlResource.id && item.kind === 'url' && item.origin === 'source')).toBe(true)
    expect(store.activeProject?.resourceIds).toContain(urlResource.id)

    store.updateProjectResource(urlResource.id, {
      name: 'Updated design spec',
      location: 'https://example.com/spec-v2',
    })

    expect(store.resources.find((item) => item.id === urlResource.id)).toMatchObject({
      name: 'Updated design spec',
      location: 'https://example.com/spec-v2',
    })
  })

  it('removes artifact resources and clears project, conversation, and message references', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')
    store.selectConversation('conv-redesign')

    store.removeProjectResource('art-roadmap')

    expect(store.artifacts.some((item) => item.id === 'art-roadmap')).toBe(false)
    expect(store.activeProject?.artifactIds).not.toContain('art-roadmap')
    expect(store.activeConversation?.artifactIds).not.toContain('art-roadmap')
    expect(store.messages.some((message) =>
      (message.artifacts ?? []).includes('art-roadmap')
      || (message.resourceIds ?? []).includes('art-roadmap')
      || (message.attachments ?? []).some((attachment) => attachment.kind === 'artifact' && attachment.id === 'art-roadmap'),
    )).toBe(false)
  })

  it('exports a unified agent asset payload for both agent and team entities', () => {
    const store = useWorkbenchStore()

    const agentExport = JSON.parse(store.exportAgentAsset('agent', 'agent-architect'))
    const teamExport = JSON.parse(store.exportAgentAsset('team', 'team-redesign-copy'))

    expect(agentExport.kind).toBe('agent')
    expect(agentExport.entity.id).toBe('agent-architect')
    expect(teamExport.kind).toBe('team')
    expect(teamExport.entity.id).toBe('team-redesign-copy')
    expect(teamExport.entity.structureNodes.length).toBeGreaterThan(0)
  })

  it('creates a mock project in the current workspace with a starter conversation and selects it', () => {
    const store = useWorkbenchStore()

    store.selectWorkspace('ws-local')
    const project = store.createProject()

    expect(project.workspaceId).toBe('ws-local')
    expect(store.currentProjectId).toBe(project.id)
    expect(store.activeWorkspace?.projectIds.includes(project.id)).toBe(true)
    expect(store.workspaceProjects.some((item) => item.id === project.id)).toBe(true)
    expect(store.projectConversations).toHaveLength(1)
    expect(store.currentConversationId).toBe(store.projectConversations[0]?.id)
    expect(store.activeRun?.conversationId).toBe(store.currentConversationId)
  })

  it('removes an active project and switches to the next remaining project in the workspace', () => {
    const store = useWorkbenchStore()

    store.selectWorkspace('ws-local')
    const nextProjectId = store.removeProject('proj-redesign')

    expect(nextProjectId).toBe('proj-governance')
    expect(store.workspaceProjects.some((item) => item.id === 'proj-redesign')).toBe(false)
    expect(store.conversations.some((item) => item.projectId === 'proj-redesign')).toBe(false)
    expect(store.currentProjectId).toBe('proj-governance')
    expect(store.currentConversationId).toBe('conv-governance')
  })

  it('blocks removal of the last remaining project in a workspace', () => {
    const store = useWorkbenchStore()

    store.selectWorkspace('ws-enterprise')
    const blocked = store.removeProject('proj-launch')

    expect(blocked).toBeNull()
    expect(store.workspaceProjects).toHaveLength(1)
    expect(store.currentProjectId).toBe('proj-launch')
  })

  it('removes a conversation and selects the next available conversation in the same project', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')
    const addedConversation = store.createConversation()
    store.selectConversation('conv-redesign')

    const targetConversationId = store.removeConversation('conv-redesign')

    expect(targetConversationId).toBe(addedConversation.id)
    expect(store.projectConversations.some((item) => item.id === 'conv-redesign')).toBe(false)
    expect(store.currentConversationId).toBe(addedConversation.id)
    expect(store.activeRun?.conversationId).toBe(addedConversation.id)
  })

  it('clears the active conversation when the final conversation in a project is removed', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-governance')
    const targetConversationId = store.removeConversation('conv-governance')

    expect(targetConversationId).toBeNull()
    expect(store.projectConversations).toHaveLength(0)
    expect(store.currentProjectId).toBe('proj-governance')
    expect(store.currentConversationId).toBe('')
    expect(store.activeRun).toBeUndefined()
  })

  it('removes a non-active workspace and its scoped mock data', () => {
    const store = useWorkbenchStore()

    store.createWorkspace()
    store.selectWorkspace('ws-local')
    store.removeWorkspace('ws-mock-3')

    expect(store.workspaces.some((workspace) => workspace.id === 'ws-mock-3')).toBe(false)
    expect(store.projects.some((project) => project.workspaceId === 'ws-mock-3')).toBe(false)
    expect(store.conversations.some((conversation) => conversation.projectId === 'proj-mock-3')).toBe(false)
    expect(store.connections.some((connection) => connection.workspaceId === 'ws-mock-3')).toBe(false)
    expect(store.currentWorkspaceId).toBe('ws-local')
  })

  it('removes the active workspace and switches to the next remaining workspace', () => {
    const store = useWorkbenchStore()

    store.createWorkspace()
    store.selectWorkspace('ws-local')
    const nextWorkspaceId = store.removeWorkspace('ws-local')

    expect(nextWorkspaceId).toBe('ws-enterprise')
    expect(store.currentWorkspaceId).toBe('ws-enterprise')
    expect(store.currentProjectId).toBe('proj-launch')
  })

  it('blocks removal of the last remaining workspace', () => {
    const store = useWorkbenchStore()

    store.removeWorkspace('ws-enterprise')
    const blocked = store.removeWorkspace('ws-local')

    expect(blocked).toBeNull()
    expect(store.workspaces).toHaveLength(1)
    expect(store.currentWorkspaceId).toBe('ws-local')
  })

  it('expands bundle permissions through workspace role bindings', () => {
    const store = useWorkbenchStore()

    store.switchCurrentUser('user-operator')

    expect(store.currentMembership?.roleIds).toContain('role-ws-local-operator')
    expect(store.effectivePermissionIdsByUser('user-operator')).toContain('perm-ws-local-project-redesign')
    expect(store.effectivePermissionIdsByUser('user-operator')).toContain('perm-ws-local-agent-coder')
    expect(store.effectivePermissionIdsByUser('user-operator')).toContain('perm-ws-local-skill-vue')
    expect(store.effectivePermissionIdsByUser('user-operator')).not.toContain('perm-ws-local-bundle-builder')
  })

  it('syncs effective menus to the switched session user', () => {
    const store = useWorkbenchStore()

    expect(store.effectiveMenuIdsByUser('user-admin')).toContain('menu-user-center-roles')

    store.switchCurrentUser('user-operator')

    expect(store.currentUser?.id).toBe('user-operator')
    expect(store.effectiveMenuIdsByUser('user-operator')).toContain('menu-user-center')
    expect(store.effectiveMenuIdsByUser('user-operator')).toContain('menu-user-center-users')
    expect(store.effectiveMenuIdsByUser('user-operator')).not.toContain('menu-user-center-roles')
  })

  it('blocks deleting a role that is still bound to a workspace member', () => {
    const store = useWorkbenchStore()

    expect(store.deleteRole('role-ws-local-admin')).toBe(false)
    expect(store.workspaceRoles.some((role) => role.id === 'role-ws-local-admin')).toBe(true)
  })

  it('removes disabled menus from the effective navigation tree', () => {
    const store = useWorkbenchStore()

    expect(store.effectiveMenuIdsByUser('user-admin')).toContain('menu-tools')

    store.updateMenu('menu-tools', {
      status: 'disabled',
    })

    expect(store.effectiveMenuIdsByUser('user-admin')).not.toContain('menu-tools')
  })

  it('manages workspace tool definitions for builtin, skill, and mcp entries', () => {
    const store = useWorkbenchStore()

    expect(store.workspaceToolDefinitions.some((tool) => tool.kind === 'builtin')).toBe(true)
    expect(store.workspaceSkillTools.find((tool) => tool.id === 'skill-vue')?.content).toContain('Composition API')
    expect(store.workspaceMcpTools.find((tool) => tool.id === 'mcp-figma')?.serverName).toBe('figma-mcp')

    store.updateBuiltinTool('builtin-read', {
      permissionMode: 'readonly',
      status: 'disabled',
    })
    expect(store.workspaceBuiltinTools.find((tool) => tool.id === 'builtin-read')).toMatchObject({
      permissionMode: 'readonly',
      status: 'disabled',
    })

    const createdSkill = store.createSkillTool({
      name: 'Release Notes Writer',
      description: 'Summarize release deltas for desktop handoff.',
      permissionMode: 'ask',
      content: 'Generate concise release notes grouped by workspace impact.',
    })
    expect(store.workspaceSkillTools.some((tool) => tool.id === createdSkill.id)).toBe(true)

    store.updateSkillTool(createdSkill.id, {
      name: 'Release Notes Assistant',
      content: 'Generate concise release notes with rollout cautions.',
      status: 'disabled',
    })
    expect(store.workspaceSkillTools.find((tool) => tool.id === createdSkill.id)).toMatchObject({
      name: 'Release Notes Assistant',
      status: 'disabled',
    })

    store.toggleSkillToolStatus(createdSkill.id)
    expect(store.workspaceSkillTools.find((tool) => tool.id === createdSkill.id)?.status).toBe('active')
    expect(store.deleteSkillTool(createdSkill.id)).toBe(true)
    expect(store.workspaceSkillTools.some((tool) => tool.id === createdSkill.id)).toBe(false)

    const createdMcp = store.createMcpTool({
      name: 'Release MCP',
      description: 'Read release policies and checklists.',
      permissionMode: 'ask',
      serverName: 'release-mcp',
      endpoint: 'https://example.test/mcp/release',
      toolNames: ['get_release', 'list_checks'],
      notes: 'Used for release coordination.',
    })
    expect(store.workspaceMcpTools.some((tool) => tool.id === createdMcp.id)).toBe(true)

    store.updateMcpTool(createdMcp.id, {
      toolNames: ['get_release', 'list_checks', 'approve_release'],
      notes: 'Updated release flow.',
      status: 'disabled',
    })
    expect(store.workspaceMcpTools.find((tool) => tool.id === createdMcp.id)).toMatchObject({
      notes: 'Updated release flow.',
      status: 'disabled',
    })

    store.toggleMcpToolStatus(createdMcp.id)
    expect(store.workspaceMcpTools.find((tool) => tool.id === createdMcp.id)?.status).toBe('active')
    expect(store.deleteMcpTool(createdMcp.id)).toBe(true)
    expect(store.workspaceMcpTools.some((tool) => tool.id === createdMcp.id)).toBe(false)
  })

  it('builds user-center overview and governance summaries from seeded mock data', () => {
    const store = useWorkbenchStore()

    expect(store.userCenterOverview.metrics).toHaveLength(4)
    expect(store.userCenterOverview.alerts.length).toBeGreaterThan(0)
    expect(store.userCenterOverview.quickLinks.length).toBeGreaterThan(0)
    expect(store.workspaceUserListItems.some((user) => user.id === 'user-intern' && user.roleNames.length === 0)).toBe(true)
    expect(store.workspaceRoleListItems.some((role) => role.id === 'role-ws-local-observer' && role.riskFlags.length > 0)).toBe(true)
    expect(store.workspacePermissionListItems.some((permission) => permission.id === 'perm-ws-local-bundle-audit' && permission.riskFlags.length > 0)).toBe(true)
    expect(store.workspaceMenuTreeItems.some((menu) => menu.id === 'menu-automations' && menu.roleUsageCount > 0)).toBe(true)
  })
})
