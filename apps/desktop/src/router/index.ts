import { createMemoryHistory, createRouter, createWebHashHistory } from 'vue-router'

import AgentsView from '@/views/AgentsView.vue'
import AutomationsView from '@/views/AutomationsView.vue'
import ConnectionsView from '@/views/ConnectionsView.vue'
import ConversationView from '@/views/ConversationView.vue'
import DashboardView from '@/views/DashboardView.vue'
import KnowledgeView from '@/views/KnowledgeView.vue'
import SettingsView from '@/views/SettingsView.vue'
import TeamsView from '@/views/TeamsView.vue'
import TraceView from '@/views/TraceView.vue'
import { createMockWorkbenchSeed } from '@/mock/data'

const seed = createMockWorkbenchSeed()

export const router = createRouter({
  history: typeof window === 'undefined' ? createMemoryHistory() : createWebHashHistory(),
  routes: [
    {
      path: '/',
      redirect: {
        path: `/workspaces/${seed.currentWorkspaceId}/dashboard`,
        query: {
          project: seed.currentProjectId,
        },
      },
    },
    {
      path: '/workspaces/:workspaceId/dashboard',
      name: 'dashboard',
      component: DashboardView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/conversations/:conversationId',
      name: 'conversation',
      component: ConversationView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/knowledge',
      name: 'knowledge',
      component: KnowledgeView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/trace',
      name: 'trace',
      component: TraceView,
    },
    {
      path: '/workspaces/:workspaceId/agents',
      name: 'agents',
      component: AgentsView,
    },
    {
      path: '/workspaces/:workspaceId/teams',
      name: 'teams',
      component: TeamsView,
    },
    {
      path: '/workspaces/:workspaceId/settings',
      name: 'settings',
      component: SettingsView,
    },
    {
      path: '/workspaces/:workspaceId/automations',
      name: 'automations',
      component: AutomationsView,
    },
    {
      path: '/connections',
      name: 'connections',
      component: ConnectionsView,
    },
    {
      path: '/:pathMatch(.*)*',
      redirect: `/workspaces/${seed.currentWorkspaceId}/projects/${seed.currentProjectId}/conversations/${seed.currentConversationId}`,
    },
  ],
})
