import { createMemoryHistory, createRouter, createWebHashHistory } from 'vue-router'

import ConnectionsView from '@/views/app/ConnectionsView.vue'
import SettingsView from '@/views/app/SettingsView.vue'
import ConversationView from '@/views/project/ConversationView.vue'
import ProjectAgentsView from '@/views/project/ProjectAgentsView.vue'
import ProjectDashboardView from '@/views/project/ProjectDashboardView.vue'
import ProjectKnowledgeView from '@/views/project/ProjectKnowledgeView.vue'
import ProjectResourcesView from '@/views/project/ProjectResourcesView.vue'
import TraceView from '@/views/project/TraceView.vue'
import AutomationsView from '@/views/workspace/AutomationsView.vue'
import ModelsView from '@/views/workspace/ModelsView.vue'
import TeamsView from '@/views/workspace/TeamsView.vue'
import ToolsView from '@/views/workspace/ToolsView.vue'
import UserCenterView from '@/views/workspace/UserCenterView.vue'
import WorkspaceAgentsView from '@/views/workspace/WorkspaceAgentsView.vue'
import WorkspaceKnowledgeView from '@/views/workspace/WorkspaceKnowledgeView.vue'
import WorkspaceOverviewView from '@/views/workspace/WorkspaceOverviewView.vue'
import WorkspaceResourcesView from '@/views/workspace/WorkspaceResourcesView.vue'
import UserCenterMenusView from '@/views/workspace/user/UserCenterMenusView.vue'
import UserCenterPermissionsView from '@/views/workspace/user/UserCenterPermissionsView.vue'
import UserCenterProfileView from '@/views/workspace/user/UserCenterProfileView.vue'
import UserCenterRolesView from '@/views/workspace/user/UserCenterRolesView.vue'
import UserCenterUsersView from '@/views/workspace/user/UserCenterUsersView.vue'
import { createMockWorkbenchSeed } from '@/mock/data'
import { USER_CENTER_MENU_IDS, getRouteMenuId } from '@/navigation/menuRegistry'
import { useWorkbenchStore } from '@/stores/workbench'

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
      name: 'project-conversation',
      component: ConversationView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/agents',
      name: 'project-agents',
      component: ProjectAgentsView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/resources',
      name: 'project-resources',
      component: ProjectResourcesView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/knowledge',
      name: 'project-knowledge',
      component: ProjectKnowledgeView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/trace',
      name: 'project-trace',
      component: TraceView,
    },
    {
      path: '/workspaces/:workspaceId/agents',
      name: 'workspace-agents',
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
      name: 'workspace-teams',
      component: TeamsView,
    },
    {
      path: '/workspaces/:workspaceId/models',
      name: 'workspace-models',
      component: ModelsView,
    },
    {
      path: '/workspaces/:workspaceId/tools',
      name: 'workspace-tools',
      component: ToolsView,
    },
    {
      path: '/settings',
      name: 'app-settings',
      component: SettingsView,
    },
    {
      path: '/workspaces/:workspaceId/user-center',
      name: 'workspace-user-center',
      component: UserCenterView,
      redirect: (to) => {
        const workbench = useWorkbenchStore()
        return resolveUserCenterEntry(String(to.params.workspaceId ?? workbench.currentWorkspaceId))
      },
      children: [
        {
          path: 'profile',
          name: 'workspace-user-center-profile',
          component: UserCenterProfileView,
        },
        {
          path: 'users',
          name: 'workspace-user-center-users',
          component: UserCenterUsersView,
        },
        {
          path: 'roles',
          name: 'workspace-user-center-roles',
          component: UserCenterRolesView,
        },
        {
          path: 'permissions',
          name: 'workspace-user-center-permissions',
          component: UserCenterPermissionsView,
        },
        {
          path: 'menus',
          name: 'workspace-user-center-menus',
          component: UserCenterMenusView,
        },
      ],
    },
    {
      path: '/workspaces/:workspaceId/automations',
      name: 'workspace-automations',
      component: AutomationsView,
    },
    {
      path: '/connections',
      name: 'app-connections',
      component: ConnectionsView,
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

  if (!workspaceId || !routeMenuId) {
    return true
  }

  if (!USER_CENTER_MENU_IDS.includes(routeMenuId)) {
    return true
  }

  const workbench = useWorkbenchStore()
  const accessibleMenuIds = workbench.effectiveMenuIdsForWorkspace(workspaceId, workbench.currentUserId)
  if (accessibleMenuIds.includes(routeMenuId)) {
    return true
  }

  return resolveUserCenterEntry(workspaceId)
})
