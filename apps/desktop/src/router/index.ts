import { getActivePinia } from 'pinia'
import { createMemoryHistory, createRouter, createWebHashHistory } from 'vue-router'

import ConnectionsView from '@/views/app/ConnectionsView.vue'
import SettingsView from '@/views/app/SettingsView.vue'
import ConversationView from '@/views/project/ConversationView.vue'
import ProjectAgentsView from '@/views/project/ProjectAgentsView.vue'
import ProjectDashboardView from '@/views/project/ProjectDashboardView.vue'
import ProjectKnowledgeView from '@/views/project/ProjectKnowledgeView.vue'
import ProjectResourcesView from '@/views/project/ProjectResourcesView.vue'
import ProjectRuntimeConfigView from '@/views/project/ProjectRuntimeConfigView.vue'
import ProjectSettingsView from '@/views/project/ProjectSettingsView.vue'
import TraceView from '@/views/project/TraceView.vue'
import AutomationsView from '@/views/workspace/AutomationsView.vue'
import ModelsView from '@/views/workspace/ModelsView.vue'
import PersonalCenterView from '@/views/workspace/PersonalCenterView.vue'
import PermissionCenterView from '@/views/workspace/PermissionCenterView.vue'
import ProjectsView from '@/views/workspace/ProjectsView.vue'
import ToolsView from '@/views/workspace/ToolsView.vue'
import WorkspaceAgentsView from '@/views/workspace/WorkspaceAgentsView.vue'
import WorkspaceConsoleView from '@/views/workspace/WorkspaceConsoleView.vue'
import WorkspaceKnowledgeView from '@/views/workspace/WorkspaceKnowledgeView.vue'
import WorkspaceOverviewView from '@/views/workspace/WorkspaceOverviewView.vue'
import WorkspaceResourcesView from '@/views/workspace/WorkspaceResourcesView.vue'
import PersonalCenterPetView from '@/views/workspace/personal-center/PersonalCenterPetView.vue'
import PersonalCenterProfileView from '@/views/workspace/personal-center/PersonalCenterProfileView.vue'
import PermissionCenterMenusView from '@/views/workspace/permission-center/PermissionCenterMenusView.vue'
import PermissionCenterPermissionsView from '@/views/workspace/permission-center/PermissionCenterPermissionsView.vue'
import PermissionCenterRolesView from '@/views/workspace/permission-center/PermissionCenterRolesView.vue'
import PermissionCenterUsersView from '@/views/workspace/permission-center/PermissionCenterUsersView.vue'
import { CONSOLE_MENU_IDS, PERMISSION_CENTER_MENU_IDS, getRouteMenuId } from '@/navigation/menuRegistry'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { useWorkspaceStore } from '@/stores/workspace'

function resolveShellStore() {
  const pinia = getActivePinia()
  return pinia ? useShellStore(pinia) : null
}

function resolveWorkspaceStore() {
  const pinia = getActivePinia()
  return pinia ? useWorkspaceStore(pinia) : null
}

function resolveWorkspaceAccessStore() {
  const pinia = getActivePinia()
  return pinia ? useWorkspaceAccessStore(pinia) : null
}

function resolveWorkspaceId(): string {
  const shell = resolveShellStore()
  const workspaceStore = resolveWorkspaceStore()
  return shell?.activeWorkspaceConnection?.workspaceId
    || workspaceStore?.currentWorkspaceId
    || shell?.preferences.defaultWorkspaceId
    || 'ws-local'
}

function resolveProjectId(): string {
  const workspaceStore = resolveWorkspaceStore()
  return workspaceStore?.currentProjectId
    || workspaceStore?.projects[0]?.id
    || ''
}

function resolveWorkspaceOverviewTarget(workspaceId: string) {
  const workspaceStore = resolveWorkspaceStore()
  return {
    name: 'workspace-overview',
    params: { workspaceId },
    query: workspaceStore?.currentProjectId ? { project: workspaceStore.currentProjectId } : undefined,
  } as const
}

function resolvePermissionCenterEntry(workspaceId: string) {
  const workspaceAccessStore = resolveWorkspaceAccessStore()
  const routeName = workspaceAccessStore?.firstAccessiblePermissionCenterRouteName
  if (routeName) {
    return {
      name: routeName,
      params: { workspaceId },
    } as const
  }

  return resolveWorkspaceOverviewTarget(workspaceId)
}

function resolveConsoleFallback(workspaceId: string) {
  const workspaceAccessStore = resolveWorkspaceAccessStore()
  if (workspaceAccessStore?.firstAccessibleConsoleRouteName) {
    return {
      name: 'workspace-console',
      params: { workspaceId },
    } as const
  }

  return resolveWorkspaceOverviewTarget(workspaceId)
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
      path: '/workspaces/:workspaceId/console',
      name: 'workspace-console',
      component: WorkspaceConsoleView,
    },
    {
      path: '/workspaces/:workspaceId/console/projects',
      name: 'workspace-console-projects',
      component: ProjectsView,
    },
    {
      path: '/workspaces/:workspaceId/console/knowledge',
      name: 'workspace-console-knowledge',
      component: WorkspaceKnowledgeView,
    },
    {
      path: '/workspaces/:workspaceId/console/resources',
      name: 'workspace-console-resources',
      component: WorkspaceResourcesView,
    },
    {
      path: '/workspaces/:workspaceId/console/agents',
      name: 'workspace-console-agents',
      component: WorkspaceAgentsView,
    },
    {
      path: '/workspaces/:workspaceId/console/models',
      name: 'workspace-console-models',
      component: ModelsView,
    },
    {
      path: '/workspaces/:workspaceId/console/tools',
      name: 'workspace-console-tools',
      component: ToolsView,
    },
    {
      path: '/workspaces/:workspaceId/projects',
      redirect: (to) => ({
        name: 'workspace-console-projects',
        params: { workspaceId: to.params.workspaceId },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/knowledge',
      redirect: (to) => ({
        name: 'workspace-console-knowledge',
        params: { workspaceId: to.params.workspaceId },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/resources',
      redirect: (to) => ({
        name: 'workspace-console-resources',
        params: { workspaceId: to.params.workspaceId },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/agents',
      redirect: (to) => ({
        name: 'workspace-console-agents',
        params: { workspaceId: to.params.workspaceId },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/models',
      redirect: (to) => ({
        name: 'workspace-console-models',
        params: { workspaceId: to.params.workspaceId },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/tools',
      redirect: (to) => ({
        name: 'workspace-console-tools',
        params: { workspaceId: to.params.workspaceId },
        query: to.query,
      }),
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
      path: '/workspaces/:workspaceId/teams',
      name: 'workspace-teams',
      redirect: (to) => ({
        name: 'workspace-console-agents',
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
      path: '/settings',
      name: 'app-settings',
      component: SettingsView,
    },
    {
      path: '/workspaces/:workspaceId/permission-center',
      name: 'workspace-permission-center',
      component: PermissionCenterView,
      redirect: (to) => resolvePermissionCenterEntry(
        typeof to.params.workspaceId === 'string' && to.params.workspaceId
          ? to.params.workspaceId
          : resolveWorkspaceId(),
      ),
      children: [
        {
          path: 'users',
          name: 'workspace-permission-center-users',
          component: PermissionCenterUsersView,
        },
        {
          path: 'roles',
          name: 'workspace-permission-center-roles',
          component: PermissionCenterRolesView,
        },
        {
          path: 'permissions',
          name: 'workspace-permission-center-permissions',
          component: PermissionCenterPermissionsView,
        },
        {
          path: 'menus',
          name: 'workspace-permission-center-menus',
          component: PermissionCenterMenusView,
        },
      ],
    },
    {
      path: '/workspaces/:workspaceId/personal-center',
      name: 'workspace-personal-center',
      component: PersonalCenterView,
      redirect: (to) => ({
        name: 'workspace-personal-center-profile',
        params: {
          workspaceId: to.params.workspaceId,
        },
      }),
      children: [
        {
          path: 'profile',
          name: 'workspace-personal-center-profile',
          component: PersonalCenterProfileView,
        },
        {
          path: 'pet',
          name: 'workspace-personal-center-pet',
          component: PersonalCenterPetView,
        },
      ],
    },
    {
      path: '/workspaces/:workspaceId/user-center',
      redirect: (to) => ({
        name: 'workspace-permission-center',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/user-center/profile',
      redirect: (to) => ({
        name: 'workspace-personal-center-profile',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/user-center/pet',
      redirect: (to) => ({
        name: 'workspace-personal-center-pet',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/user-center/users',
      redirect: (to) => ({
        name: 'workspace-permission-center-users',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/user-center/roles',
      redirect: (to) => ({
        name: 'workspace-permission-center-roles',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/user-center/permissions',
      redirect: (to) => ({
        name: 'workspace-permission-center-permissions',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: to.query,
      }),
    },
    {
      path: '/workspaces/:workspaceId/user-center/menus',
      redirect: (to) => ({
        name: 'workspace-permission-center-menus',
        params: {
          workspaceId: to.params.workspaceId,
        },
        query: to.query,
      }),
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
          name: 'workspace-overview',
          params: { workspaceId },
          query: projectId ? { project: projectId } : undefined,
        }
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

  const workspaceAccessStore = resolveWorkspaceAccessStore()
  if (!workspaceAccessStore) {
    return true
  }

  if (!workspaceAccessStore.menus.length && !workspaceAccessStore.roles.length && !workspaceAccessStore.currentUser) {
    return true
  }

  if (PERMISSION_CENTER_MENU_IDS.includes(routeMenuId)) {
    if (workspaceAccessStore.currentEffectiveMenuIds.includes(routeMenuId)) {
      return true
    }

    return resolvePermissionCenterEntry(workspaceId)
  }

  if (CONSOLE_MENU_IDS.includes(routeMenuId)) {
    if (workspaceAccessStore.currentEffectiveMenuIds.includes(routeMenuId)) {
      return true
    }

    return resolveConsoleFallback(workspaceId)
  }

  return true
})
