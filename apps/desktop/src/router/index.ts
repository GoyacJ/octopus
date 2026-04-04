import { createMemoryHistory, createRouter, createWebHashHistory } from 'vue-router'

import WorkspaceAgentsView from '@/views/WorkspaceAgentsView.vue'
import ProjectAgentsView from '@/views/ProjectAgentsView.vue'
import WorkspaceResourcesView from '@/views/WorkspaceResourcesView.vue'
import ProjectResourcesView from '@/views/ProjectResourcesView.vue'
import AutomationsView from '@/views/AutomationsView.vue'
import ConnectionsView from '@/views/ConnectionsView.vue'
import ConversationView from '@/views/ConversationView.vue'
import ProjectKnowledgeView from '@/views/ProjectKnowledgeView.vue'
import WorkspaceKnowledgeView from '@/views/WorkspaceKnowledgeView.vue'
import ModelsView from '@/views/ModelsView.vue'
import ProjectDashboardView from '@/views/ProjectDashboardView.vue'
import SettingsView from '@/views/SettingsView.vue'
import TeamsView from '@/views/TeamsView.vue'
import TraceView from '@/views/TraceView.vue'
import ToolsView from '@/views/ToolsView.vue'
import UserCenterView from '@/views/UserCenterView.vue'
import WorkspaceOverviewView from '@/views/WorkspaceOverviewView.vue'
import { createMockWorkbenchSeed } from '@/mock/data'
import { USER_CENTER_MENU_IDS, getRouteMenuId } from '@/navigation/menuRegistry'
import { useWorkbenchStore } from '@/stores/workbench'
import UserCenterMenusView from '@/views/user-center/UserCenterMenusView.vue'
import UserCenterPermissionsView from '@/views/user-center/UserCenterPermissionsView.vue'
import UserCenterProfileView from '@/views/user-center/UserCenterProfileView.vue'
import UserCenterRolesView from '@/views/user-center/UserCenterRolesView.vue'
import UserCenterUsersView from '@/views/user-center/UserCenterUsersView.vue'

function resolveUserCenterEntry(workspaceId: string) {
  const workbench = useWorkbenchStore()
  const routeName = workbench.firstAccessibleUserCenterRouteForWorkspace(workspaceId, workbench.currentUserId)
  if (routeName) {
    return {
      name: routeName,
      params: {
        workspaceId,
      },
    }
  }

  const fallbackProjectId = workbench.projects.find((project) => project.workspaceId === workspaceId)?.id ?? ''
  return {
    name: 'workspace-overview',
    params: {
      workspaceId,
    },
    query: {
      project: fallbackProjectId,
    },
  }
}

export const router = createRouter({
  history: typeof window === 'undefined' ? createMemoryHistory() : createWebHashHistory(),
  routes: [
    {
      path: '/',
      redirect: (to) => {
        const workbench = useWorkbenchStore()
        const workspaceId = workbench.currentWorkspaceId || 'default'
        const projectId = workbench.currentProjectId || ''
        return {
          path: `/workspaces/${workspaceId}/overview`,
          query: {
            project: projectId,
          },
        }
      },
    },
    {
      path: '/workspaces/:workspaceId/dashboard',
      redirect: (to) => ({
        name: 'workspace-overview',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/overview',
      name: 'workspace-overview',
      component: WorkspaceOverviewView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/dashboard',
      name: 'project-dashboard',
      component: ProjectDashboardView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/conversations',
      name: 'project-conversations',
      component: ConversationView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/conversations/:conversationId',
      name: 'conversation',
      component: ConversationView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/agents',
      name: 'project-agents',
      component: ProjectAgentsView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/resources',
      name: 'resources',
      component: ProjectResourcesView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/knowledge',
      name: 'knowledge',
      component: ProjectKnowledgeView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/trace',
      name: 'trace',
      component: TraceView,
    },
    {
      path: '/workspaces/:workspaceId/agents',
      name: 'agents',
      component: WorkspaceAgentsView,
    },
    {
      path: '/workspaces/:workspaceId/knowledge',
      name: 'workspace-knowledge',
      component: WorkspaceKnowledgeView,
    },
    {
      path: '/workspaces/:workspaceId/resources',
      name: 'workspace-resources',
      component: WorkspaceResourcesView,
    },
    {
      path: '/workspaces/:workspaceId/teams',
      name: 'teams',
      component: TeamsView,
    },
    {
      path: '/workspaces/:workspaceId/models',
      name: 'models',
      component: ModelsView,
    },
    {
      path: '/workspaces/:workspaceId/tools',
      name: 'tools',
      component: ToolsView,
    },
    {
      path: '/workspaces/:workspaceId/settings',
      name: 'settings',
      component: SettingsView,
    },
    {
      path: '/workspaces/:workspaceId/user-center',
      name: 'user-center',
      component: UserCenterView,
      redirect: (to) => {
        const workbench = useWorkbenchStore()
        return resolveUserCenterEntry(String(to.params.workspaceId ?? workbench.currentWorkspaceId))
      },
      children: [
        {
          path: 'profile',
          name: 'user-center-profile',
          component: UserCenterProfileView,
        },
        {
          path: 'users',
          name: 'user-center-users',
          component: UserCenterUsersView,
        },
        {
          path: 'roles',
          name: 'user-center-roles',
          component: UserCenterRolesView,
        },
        {
          path: 'permissions',
          name: 'user-center-permissions',
          component: UserCenterPermissionsView,
        },
        {
          path: 'menus',
          name: 'user-center-menus',
          component: UserCenterMenusView,
        },
      ],
    },
    {
      path: '/workspaces/:workspaceId/automations',
      name: 'automations',
      component: AutomationsView,
    },
    {
      path: '/workspaces/:workspaceId/connections',
      name: 'workspace-connections',
      component: ConnectionsView,
    },
    {
      path: '/connections',
      name: 'connections',
      redirect: (to) => {
        const workbench = useWorkbenchStore()
        const workspaceId = workbench.currentWorkspaceId || 'default'
        return `/workspaces/${workspaceId}/connections`
      },
    },
    {
      path: '/:pathMatch(.*)*',
      redirect: (to) => {
        const workbench = useWorkbenchStore()
        const workspaceId = workbench.currentWorkspaceId || 'default'
        const projectId = workbench.currentProjectId || ''
        const conversationId = workbench.currentConversationId || ''
        return `/workspaces/${workspaceId}/projects/${projectId}/conversations/${conversationId}`
      },
    },
  ],
})

router.beforeEach((to) => {
  const workspaceId = typeof to.params.workspaceId === 'string' ? to.params.workspaceId : undefined
  const routeMenuId = getRouteMenuId(typeof to.name === 'string' ? to.name : undefined)

  if (!workspaceId || !routeMenuId || !USER_CENTER_MENU_IDS.includes(routeMenuId)) {
    return true
  }

  const workbench = useWorkbenchStore()
  const accessibleMenuIds = workbench.effectiveMenuIdsForWorkspace(workspaceId, workbench.currentUserId)
  if (accessibleMenuIds.includes(routeMenuId)) {
    return true
  }

  return resolveUserCenterEntry(workspaceId)
})
