import { createMemoryHistory, createRouter, createWebHashHistory } from 'vue-router'

import SettingsView from '@/views/app/SettingsView.vue'
import ConnectionsView from '@/views/app/ConnectionsView.vue'
import ConversationView from '@/views/project/ConversationView.vue'
import ProjectAgentsView from '@/views/project/ProjectAgentsView.vue'
import ProjectDashboardView from '@/views/project/ProjectDashboardView.vue'
import ProjectKnowledgeView from '@/views/project/ProjectKnowledgeView.vue'
import ProjectResourcesView from '@/views/project/ProjectResourcesView.vue'
import ProjectSettingsView from '@/views/project/ProjectSettingsView.vue'
import ProjectRuntimeConfigView from '@/views/project/ProjectRuntimeConfigView.vue'
import TraceView from '@/views/project/TraceView.vue'
import AutomationsView from '@/views/workspace/AutomationsView.vue'
import ModelsView from '@/views/workspace/ModelsView.vue'
import ProjectsView from '@/views/workspace/ProjectsView.vue'
import ToolsView from '@/views/workspace/ToolsView.vue'
import UserCenterView from '@/views/workspace/UserCenterView.vue'
import WorkspaceAgentsView from '@/views/workspace/WorkspaceAgentsView.vue'
import WorkspaceKnowledgeView from '@/views/workspace/WorkspaceKnowledgeView.vue'
import WorkspaceOverviewView from '@/views/workspace/WorkspaceOverviewView.vue'
import WorkspaceResourcesView from '@/views/workspace/WorkspaceResourcesView.vue'
import UserCenterMenusView from '@/views/workspace/user/UserCenterMenusView.vue'
import UserCenterPermissionsView from '@/views/workspace/user/UserCenterPermissionsView.vue'
import UserCenterPetView from '@/views/workspace/user/UserCenterPetView.vue'
import UserCenterProfileView from '@/views/workspace/user/UserCenterProfileView.vue'
import UserCenterRolesView from '@/views/workspace/user/UserCenterRolesView.vue'
import UserCenterUsersView from '@/views/workspace/user/UserCenterUsersView.vue'
import UserRecentConversationsView from '@/views/workspace/user/UserRecentConversationsView.vue'
import UserTodoListView from '@/views/workspace/user/UserTodoListView.vue'
import { USER_CENTER_MENU_IDS, getRouteMenuId } from '@/navigation/menuRegistry'
import { useShellStore } from '@/stores/shell'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

function resolveWorkspaceId(): string {
  const shell = useShellStore()
  const workspaceStore = useWorkspaceStore()
  return shell.activeWorkspaceConnection?.workspaceId
    || workspaceStore.currentWorkspaceId
    || shell.preferences.defaultWorkspaceId
    || 'ws-local'
}

function resolveProjectId(): string {
  const workspaceStore = useWorkspaceStore()
  return workspaceStore.currentProjectId
    || workspaceStore.projects[0]?.id
    || ''
}

function resolveUserCenterEntry(workspaceId: string) {
  const userCenterStore = useUserCenterStore()
  const workspaceStore = useWorkspaceStore()
  const routeName = userCenterStore.firstAccessibleUserCenterRouteName
  if (routeName) {
    return {
      name: routeName,
      params: { workspaceId },
    }
  }

  return {
    name: 'workspace-overview',
    params: { workspaceId },
    query: workspaceStore.currentProjectId ? { project: workspaceStore.currentProjectId } : undefined,
  }
}

export const router = createRouter({
  history: typeof window === 'undefined' ? createMemoryHistory() : createWebHashHistory(),
  routes: [
    {
      path: '/',
      redirect: () => {
        const workspaceId = resolveWorkspaceId()
        const projectId = resolveProjectId()
        return {
          name: 'workspace-overview',
          params: { workspaceId },
          query: projectId ? { project: projectId } : undefined,
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
      path: '/workspaces/:workspaceId/projects',
      name: 'workspace-projects',
      component: ProjectsView,
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
      path: '/workspaces/:workspaceId/projects/:projectId/settings',
      name: 'project-settings',
      component: ProjectSettingsView,
    },
    {
      path: '/workspaces/:workspaceId/projects/:projectId/runtime',
      name: 'project-runtime',
      component: ProjectRuntimeConfigView,
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
      redirect: (to) => ({
        name: 'workspace-agents',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: {
          ...to.query,
          tab: 'team',
        },
      }),
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
      redirect: (to) => resolveUserCenterEntry(
        typeof to.params.workspaceId === 'string' && to.params.workspaceId
          ? to.params.workspaceId
          : resolveWorkspaceId(),
      ),
      children: [
        {
          path: 'profile',
          name: 'workspace-user-center-profile',
          component: UserCenterProfileView,
        },
        {
          path: 'pet',
          name: 'workspace-user-center-pet',
          component: UserCenterPetView,
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
        {
          path: 'recent-conversations',
          name: 'workspace-user-center-recent-conversations',
          component: UserRecentConversationsView,
        },
        {
          path: 'todos',
          name: 'workspace-user-center-todos',
          component: UserTodoListView,
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
      redirect: () => {
        const workspaceId = resolveWorkspaceId()
        const projectId = resolveProjectId()
        return {
          name: projectId ? 'project-conversations' : 'workspace-overview',
          params: projectId ? { workspaceId, projectId } : { workspaceId },
        }
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

  const userCenterStore = useUserCenterStore()
  if (!userCenterStore.menus.length && !userCenterStore.roles.length && !userCenterStore.currentUser) {
    return true
  }

  if (userCenterStore.currentEffectiveMenuIds.includes(routeMenuId)) {
    return true
  }

  return resolveUserCenterEntry(workspaceId)
})
