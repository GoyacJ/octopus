import { createMemoryHistory, createRouter, createWebHashHistory } from 'vue-router'

import AgentsView from '@/views/AgentsView.vue'
import AutomationsView from '@/views/AutomationsView.vue'
import ConnectionsView from '@/views/ConnectionsView.vue'
import ConversationView from '@/views/ConversationView.vue'
import KnowledgeView from '@/views/KnowledgeView.vue'
import ModelsView from '@/views/ModelsView.vue'
import ProjectDashboardView from '@/views/ProjectDashboardView.vue'
import ResourcesView from '@/views/ResourcesView.vue'
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

const seed = createMockWorkbenchSeed()

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

  const fallbackProjectId = workbench.projects.find((project) => project.workspaceId === workspaceId)?.id ?? seed.currentProjectId
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
      redirect: {
        path: `/workspaces/${seed.currentWorkspaceId}/overview`,
        query: {
          project: seed.currentProjectId,
        },
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
      component: AgentsView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/resources',
      name: 'resources',
      component: ResourcesView,
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
      redirect: (to) => resolveUserCenterEntry(String(to.params.workspaceId ?? seed.currentWorkspaceId)),
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
      redirect: `/workspaces/${seed.currentWorkspaceId}/connections`,
    },
    {
      path: '/:pathMatch(.*)*',
      redirect: `/workspaces/${seed.currentWorkspaceId}/projects/${seed.currentProjectId}/conversations/${seed.currentConversationId}`,
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
