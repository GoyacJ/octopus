import { getActivePinia } from 'pinia'
import { createMemoryHistory, createRouter, createWebHashHistory } from 'vue-router'
import type { Router, RouteRecordRaw } from 'vue-router'

import AuthLoginView from '@/views/auth/AuthLoginView.vue'
import ConnectionsView from '@/views/app/ConnectionsView.vue'
import SettingsView from '@/views/app/SettingsView.vue'
import ConversationView from '@/views/project/ConversationView.vue'
import ProjectAgentsView from '@/views/project/ProjectAgentsView.vue'
import ProjectDashboardView from '@/views/project/ProjectDashboardView.vue'
import ProjectDeliverablesView from '@/views/project/ProjectDeliverablesView.vue'
import ProjectKnowledgeView from '@/views/project/ProjectKnowledgeView.vue'
import ProjectResourcesView from '@/views/project/ProjectResourcesView.vue'
import ProjectSettingsView from '@/views/project/ProjectSettingsView.vue'
import TraceView from '@/views/project/TraceView.vue'
import AutomationsView from '@/views/workspace/AutomationsView.vue'
import AccessControlView from '@/views/workspace/AccessControlView.vue'
import ModelsView from '@/views/workspace/ModelsView.vue'
import PersonalCenterView from '@/views/workspace/PersonalCenterView.vue'
import ProjectsView from '@/views/workspace/ProjectsView.vue'
import ToolsView from '@/views/workspace/ToolsView.vue'
import WorkspaceAgentsView from '@/views/workspace/WorkspaceAgentsView.vue'
import WorkspaceConsoleView from '@/views/workspace/WorkspaceConsoleView.vue'
import WorkspaceKnowledgeView from '@/views/workspace/WorkspaceKnowledgeView.vue'
import WorkspaceOverviewView from '@/views/workspace/WorkspaceOverviewView.vue'
import WorkspaceResourcesView from '@/views/workspace/WorkspaceResourcesView.vue'
import AccessGovernanceView from '@/views/workspace/access-control/AccessGovernanceView.vue'
import AccessMembersView from '@/views/workspace/access-control/AccessMembersView.vue'
import AccessPermissionsView from '@/views/workspace/access-control/AccessPermissionsView.vue'
import PersonalCenterPetView from '@/views/workspace/personal-center/PersonalCenterPetView.vue'
import PersonalCenterProfileView from '@/views/workspace/personal-center/PersonalCenterProfileView.vue'
import {
  isProjectMember,
  isProjectModuleAllowed,
  isProjectOwner,
  isProjectOwnerOnlyRoute,
  projectModuleForRouteName,
  resolveProjectActorUserId,
} from '@/composables/project-governance'
import { CONSOLE_MENU_IDS, getRouteMenuId } from '@/navigation/menuRegistry'
import { useAuthStore } from '@/stores/auth'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

const isBrowserHostRuntime = import.meta.env.VITE_HOST_RUNTIME === 'browser'

function resolveShellStore() {
  const pinia = getActivePinia()
  return pinia ? useShellStore(pinia) : null
}

function resolveWorkspaceStore() {
  const pinia = getActivePinia()
  return pinia ? useWorkspaceStore(pinia) : null
}

function resolveWorkspaceAccessControlStore() {
  const pinia = getActivePinia()
  return pinia ? useWorkspaceAccessControlStore(pinia) : null
}

function resolveAuthStore() {
  const pinia = getActivePinia()
  return pinia ? useAuthStore(pinia) : null
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
  const shell = resolveShellStore()
  const workspaceStore = resolveWorkspaceStore()
  return workspaceStore?.currentProjectId
    || workspaceStore?.projects[0]?.id
    || shell?.defaultProjectId
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

function resolveProjectDashboardTarget(workspaceId: string, projectId: string) {
  return {
    name: 'project-dashboard',
    params: { workspaceId, projectId },
  } as const
}

function resolveAccessControlEntry(workspaceId: string) {
  const workspaceAccessControlStore = resolveWorkspaceAccessControlStore()
  const routeName = workspaceAccessControlStore?.firstAccessibleAccessControlRouteName
  if (routeName) {
    return {
      name: routeName,
      params: { workspaceId },
    } as const
  }

  return resolveWorkspaceOverviewTarget(workspaceId)
}

function resolveConsoleFallback(workspaceId: string) {
  const workspaceAccessControlStore = resolveWorkspaceAccessControlStore()
  const routeName = workspaceAccessControlStore?.firstAccessibleConsoleRouteName
  if (routeName) {
    return {
      name: routeName,
      params: { workspaceId },
    } as const
  }

  const accessLoaded = Boolean(
    workspaceAccessControlStore
    && (
      workspaceAccessControlStore.menuDefinitions.length
      || workspaceAccessControlStore.currentUser
    ),
  )

  if (accessLoaded) {
    return resolveWorkspaceOverviewTarget(workspaceId)
  }

  return {
    name: 'workspace-console-projects',
    params: { workspaceId },
  } as const
}

const ACCESS_CONTROL_SECTION_BY_ROUTE_NAME = {
  'workspace-access-control-members': 'members',
  'workspace-access-control-access': 'access',
  'workspace-access-control-governance': 'governance',
} as const

async function ensureWorkspaceGuardContext(workspaceId: string) {
  const shell = resolveShellStore()
  const workspaceStore = resolveWorkspaceStore()
  const workspaceAccessControlStore = resolveWorkspaceAccessControlStore()

  if (!shell || !workspaceStore || !workspaceAccessControlStore) {
    return null
  }

  if (!shell.workspaceConnections.length || !shell.workspaceConnections.some(item => item.workspaceId === workspaceId)) {
    await shell.bootstrap(workspaceId, resolveProjectId() || 'proj-redesign')
  } else {
    await shell.activateWorkspaceByWorkspaceId(workspaceId)
  }

  if (!shell.activeWorkspaceConnectionId) {
    return null
  }

  await workspaceStore.ensureWorkspaceBootstrap(shell.activeWorkspaceConnectionId)

  return {
    shell,
    workspaceStore,
    workspaceAccessControlStore,
  }
}

async function ensureProjectGuardContext(workspaceId: string, projectId: string) {
  const context = await ensureWorkspaceGuardContext(workspaceId)
  if (!context || !context.shell.activeWorkspaceConnectionId) {
    return null
  }

  const startedAt = performance.now()
  await context.workspaceAccessControlStore.ensureAuthorizationContext(context.shell.activeWorkspaceConnectionId)
  if (import.meta.env.DEV) {
    console.debug(
      `[router] ensureProjectGuardContext ${context.shell.activeWorkspaceConnectionId} ${Math.round(performance.now() - startedAt)}ms`,
    )
  }

  return context
}

function resolveBrowserLoginTarget(redirect?: string | null) {
  if (redirect && redirect !== '/login') {
    return redirect
  }

  const workspaceId = resolveWorkspaceId()
  return resolveConsoleFallback(workspaceId)
}

function createRoutes(): RouteRecordRaw[] {
  return [
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
      path: '/login',
      name: 'auth-login',
      component: AuthLoginView,
      meta: {
        layout: 'auth',
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
      redirect: (to) => resolveConsoleFallback(
        typeof to.params.workspaceId === 'string' && to.params.workspaceId
          ? to.params.workspaceId
          : resolveWorkspaceId(),
      ),
      children: [
        {
          path: 'projects',
          name: 'workspace-console-projects',
          component: ProjectsView,
          props: { embedded: true },
        },
        {
          path: 'knowledge',
          name: 'workspace-console-knowledge',
          component: WorkspaceKnowledgeView,
          props: { embedded: true },
        },
        {
          path: 'resources',
          name: 'workspace-console-resources',
          component: WorkspaceResourcesView,
          props: { embedded: true },
        },
        {
          path: 'agents',
          name: 'workspace-console-agents',
          component: WorkspaceAgentsView,
          props: { embedded: true },
        },
        {
          path: 'models',
          name: 'workspace-console-models',
          component: ModelsView,
          props: { embedded: true },
        },
        {
          path: 'tools',
          name: 'workspace-console-tools',
          component: ToolsView,
          props: { embedded: true },
        },
      ],
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
      path: '/workspaces/:workspaceId/projects/:projectId/deliverables',
      name: 'project-deliverables',
      component: ProjectDeliverablesView,
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
      path: '/workspaces/:workspaceId/access-control',
      name: 'workspace-access-control',
      component: AccessControlView,
      children: [
        {
          path: 'members',
          name: 'workspace-access-control-members',
          component: AccessMembersView,
        },
        {
          path: 'access',
          name: 'workspace-access-control-access',
          component: AccessPermissionsView,
        },
        {
          path: 'governance',
          name: 'workspace-access-control-governance',
          component: AccessGovernanceView,
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
  ]
}

function installRouterGuards(router: Router) {
  router.beforeEach(async (to) => {
    const authStore = resolveAuthStore()
    if (isBrowserHostRuntime && authStore?.isReady) {
      if (!authStore.isAuthenticated && to.name !== 'auth-login') {
        return {
          name: 'auth-login',
          query: {
            redirect: to.fullPath,
          },
        }
      }

      if (authStore.isAuthenticated && to.name === 'auth-login') {
        return resolveBrowserLoginTarget(
          typeof to.query.redirect === 'string' ? to.query.redirect : null,
        )
      }
    }

    const workspaceId = typeof to.params.workspaceId === 'string' ? to.params.workspaceId : undefined
    const projectId = typeof to.params.projectId === 'string' ? to.params.projectId : undefined
    const routeName = typeof to.name === 'string' ? to.name : undefined
    const routeMenuId = getRouteMenuId(routeName)

    if (workspaceId && projectId) {
      const context = await ensureProjectGuardContext(workspaceId, projectId)
      if (context) {
        const { shell, workspaceStore, workspaceAccessControlStore } = context
        const project = workspaceStore.projects.find(item => item.id === projectId) ?? null
        const actorUserId = resolveProjectActorUserId(
          workspaceAccessControlStore.currentUser?.id,
          shell.activeWorkspaceSession?.session.userId,
        )

        if (!project || !isProjectMember(project, actorUserId)) {
          return resolveWorkspaceOverviewTarget(workspaceId)
        }

        if (isProjectOwnerOnlyRoute(routeName ?? null) && !isProjectOwner(project, actorUserId)) {
          return resolveProjectDashboardTarget(workspaceId, projectId)
        }

        const module = projectModuleForRouteName(routeName)
        if (module && !isProjectModuleAllowed(workspaceStore.activeWorkspace, project, module)) {
          return resolveProjectDashboardTarget(workspaceId, projectId)
        }
      }
    }

    const accessSection = routeName
      ? ACCESS_CONTROL_SECTION_BY_ROUTE_NAME[routeName as keyof typeof ACCESS_CONTROL_SECTION_BY_ROUTE_NAME]
      : undefined

    if (workspaceId && (routeName === 'workspace-access-control' || accessSection)) {
      const context = await ensureWorkspaceGuardContext(workspaceId)
      if (!context || !context.shell.activeWorkspaceConnectionId) {
        return true
      }

      if (routeName === 'workspace-access-control-governance') {
        await context.workspaceAccessControlStore.loadGovernanceData(context.shell.activeWorkspaceConnectionId)
      } else if (accessSection) {
        await context.workspaceAccessControlStore.loadMembersData(context.shell.activeWorkspaceConnectionId)
      } else {
        await context.workspaceAccessControlStore.loadExperience(context.shell.activeWorkspaceConnectionId)
      }

      if (routeName === 'workspace-access-control') {
        return resolveAccessControlEntry(workspaceId)
      }

      if (accessSection && !context.workspaceAccessControlStore.accessSectionGrants[accessSection]) {
        return resolveAccessControlEntry(workspaceId)
      }

      return true
    }

    if (!workspaceId || !routeMenuId) {
      return true
    }

    const workspaceAccessControlStore = resolveWorkspaceAccessControlStore()
    if (!workspaceAccessControlStore) {
      return true
    }

    if (!workspaceAccessControlStore.menuDefinitions.length && !workspaceAccessControlStore.currentUser) {
      return true
    }

    if (CONSOLE_MENU_IDS.includes(routeMenuId)) {
      if (workspaceAccessControlStore.currentEffectiveMenuIds.includes(routeMenuId)) {
        return true
      }

      return resolveConsoleFallback(workspaceId)
    }

    return true
  })
}

export function createAppRouter() {
  const router = createRouter({
    history: typeof window === 'undefined' ? createMemoryHistory() : createWebHashHistory(),
    routes: createRoutes(),
  })

  installRouterGuards(router)

  return router
}

export const router = createAppRouter()
