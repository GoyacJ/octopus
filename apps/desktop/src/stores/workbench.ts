import { defineStore } from 'pinia'
import {
  type Agent,
  ConversationIntent,
  type DecisionAction,
  type InboxItem,
  type RunDetail,
  type Team,
  type DashboardSnapshot,
  type DashboardMetric,
  type DashboardHighlight,
} from '@octopus/schema'

import { mockKey, resolveMockField, translate } from '@/i18n/copy'
import { createMockWorkbenchSeed } from '@/mock/data'

function cloneSeed<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function formatMetric(value: number): string {
  return value.toString()
}

export const useWorkbenchStore = defineStore('workbench', {
  state: () => cloneSeed(createMockWorkbenchSeed()),
  getters: {
    activeWorkspace(state) {
      return state.workspaces.find((workspace) => workspace.id === state.currentWorkspaceId)
    },
    workspaceProjects(state) {
      return state.projects.filter((project) => project.workspaceId === state.currentWorkspaceId)
    },
    activeProject(state) {
      return state.projects.find((project) => project.id === state.currentProjectId)
    },
    projectConversations(state) {
      return state.conversations.filter((conversation) => conversation.projectId === state.currentProjectId)
    },
    activeConversation(state) {
      return state.conversations.find((conversation) => conversation.id === state.currentConversationId)
    },
    conversationMessages(state) {
      return state.messages.filter((message) => message.conversationId === state.currentConversationId)
    },
    activeConversationArtifacts(state) {
      const conversation = state.conversations.find((item) => item.id === state.currentConversationId)
      if (!conversation) {
        return []
      }

      return state.artifacts.filter((artifact) => conversation.artifactIds.includes(artifact.id))
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
    projectKnowledge(state) {
      return state.knowledge.filter((item) => item.projectId === state.currentProjectId)
    },
    workspaceAgents(): Agent[] {
      return this.agents.filter((agent) => {
        if (agent.scope === 'project') {
          return this.activeProject?.agentIds.includes(agent.id)
        }

        return true
      })
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
          route: `/workspaces/${this.currentWorkspaceId}/projects/${this.currentProjectId}/conversations/${this.currentConversationId}`,
          surface: 'conversation',
        },
        {
          id: 'highlight-artifact',
          title: this.activeConversationArtifacts[0]
            ? mockKey('artifact', this.activeConversationArtifacts[0].id, 'title', this.activeConversationArtifacts[0].title)
            : 'dashboard.highlights.artifactTitle',
          description: 'dashboard.highlights.artifactDescription',
          route: `/workspaces/${this.currentWorkspaceId}/projects/${this.currentProjectId}/conversations/${this.currentConversationId}?pane=artifacts`,
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
    selectWorkspace(workspaceId: string) {
      const workspace = this.workspaces.find((item) => item.id === workspaceId)
      if (!workspace) {
        return
      }

      this.currentWorkspaceId = workspaceId
      const nextProject = this.projects.find((project) => project.workspaceId === workspaceId)
      if (!nextProject) {
        return
      }

      this.currentProjectId = nextProject.id
      this.currentConversationId = nextProject.conversationIds[0] ?? this.currentConversationId
      this.selectRunByConversation(this.currentConversationId)
    },
    selectProject(projectId: string) {
      const project = this.projects.find((item) => item.id === projectId)
      if (!project) {
        return
      }

      this.currentProjectId = projectId
      this.currentWorkspaceId = project.workspaceId
      this.currentConversationId = project.conversationIds[0] ?? this.currentConversationId
      this.selectRunByConversation(this.currentConversationId)
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
      }
    },
    sendMessage(content: string) {
      const trimmed = content.trim()
      if (!trimmed) {
        return
      }

      const conversation = this.conversations.find((item) => item.id === this.currentConversationId)
      if (!conversation) {
        return
      }

      const timestamp = Date.now()
      this.messages.push({
        id: `msg-user-${timestamp}`,
        conversationId: conversation.id,
        senderId: 'user-1',
        senderType: 'user',
        content: trimmed,
        timestamp,
      })
      this.messages.push({
        id: `msg-agent-${timestamp}`,
        conversationId: conversation.id,
        senderId: conversation.activeAgentId ?? 'agent-architect',
        senderType: 'agent',
        content: 'runtime.messages.requirementsRecorded',
        timestamp: timestamp + 1,
      })

      conversation.intent = ConversationIntent.CLARIFY
      conversation.summary = trimmed
      conversation.statusNote = 'runtime.conversation.inputIngested'
      if (conversation.recentRun) {
        conversation.recentRun.status = 'running'
        conversation.recentRun.currentStep = 'runtime.run.ingestingConstraints'
        conversation.recentRun.updatedAt = timestamp
      }

      const run = this.runs.find((item) => item.id === this.currentRunId)
      if (run) {
        run.status = 'running'
        run.currentStep = 'runtime.run.ingestingConstraints'
        run.updatedAt = timestamp
      }
    },
    requestArtifactReview(artifactId: string) {
      const artifact = this.artifacts.find((item) => item.id === artifactId)
      const conversation = this.conversations.find((item) => item.id === this.currentConversationId)
      if (!artifact || !conversation) {
        return
      }

      artifact.status = 'review'
      artifact.version += 1
      artifact.updatedAt = Date.now()
      conversation.intent = ConversationIntent.REVIEW
      conversation.statusNote = 'runtime.conversation.reviewRequested'

      const inboxId = `inbox-review-${artifactId}`
      const existing = this.inbox.find((item) => item.id === inboxId)
      if (!existing) {
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
          createdAt: Date.now(),
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
      artifact.updatedAt = Date.now()
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
      run.updatedAt = Date.now()
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
      run.updatedAt = Date.now()
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
          linkedRun.updatedAt = Date.now()
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
          linkedRun.updatedAt = Date.now()
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
    createProjectAgentCopy(agentId: string) {
      const source = this.agents.find((item) => item.id === agentId)
      if (!source) {
        return
      }

      const copyId = `${source.id}-copy-${this.currentProjectId}`
      if (this.agents.some((agent) => agent.id === copyId)) {
        return
      }

      this.agents.unshift({
        ...cloneSeed(source),
        id: copyId,
        name: `${resolveMockField('agent', source.id, 'name', source.name)} · ${translate('runtime.copy.projectSuffix')}`,
        scope: 'project',
        owner: `project:${this.currentProjectId}`,
        isProjectCopy: true,
        sourceAgentId: source.id,
      })
      const project = this.activeProject
      if (project && !project.agentIds.includes(copyId)) {
        project.agentIds.unshift(copyId)
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
  },
})
