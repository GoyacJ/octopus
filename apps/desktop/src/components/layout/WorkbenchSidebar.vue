<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import {
  Bell,
  Bot,
  ChevronsUpDown,
  Cpu,
  FolderKanban,
  FolderOpen,
  LayoutDashboard,
  LibraryBig,
  MessageSquareText,
  PanelLeftClose,
  Settings,
  UserRound,
  Users,
  Workflow,
  Wrench,
} from 'lucide-vue-next'

import { UiButton, UiPopover } from '@octopus/ui'

import { createProjectConversationTarget, createProjectDashboardTarget, createProjectSurfaceTarget, createWorkspaceOverviewTarget } from '@/i18n/navigation'
import { type MenuIconKey } from '@/navigation/menuRegistry'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const userCenterStore = useUserCenterStore()
const runtime = useRuntimeStore()

const workspaceMenuOpen = ref(false)
const workspaceLabel = computed(() => {
  const connection = shell.activeWorkspaceConnection
  if (connection?.transportSecurity === 'loopback') {
    return t('topbar.localWorkspace')
  }
  return connection?.label ?? workspaceStore.activeWorkspace?.name ?? t('common.workspace')
})

type NavigationItem = {
  id: string
  label: string
  routeNames: string[]
  icon: unknown
  to: object
}

const iconMap: Record<MenuIconKey, unknown> = {
  dashboard: LayoutDashboard,
  conversations: MessageSquareText,
  agents: Bot,
  resources: FolderOpen,
  knowledge: LibraryBig,
  trace: Bell,
  runtime: Settings,
  models: Cpu,
  tools: Wrench,
  automations: Workflow,
  'user-center': UserRound,
  profile: UserRound,
  users: UserRound,
  roles: UserRound,
  permissions: UserRound,
  menus: UserRound,
  settings: Settings,
  connections: Settings, // Fallback if still needed
  teams: Users,
  bell: Bell,
}

const currentWorkspaceId = computed(() => workspaceStore.currentWorkspaceId)
const currentProjectId = computed(() => workspaceStore.currentProjectId)

const workspaceNavigation = computed<NavigationItem[]>(() => {
  const workspaceId = currentWorkspaceId.value
  if (!workspaceId) {
    return []
  }

  const items: Array<NavigationItem & { menuId?: string }> = [
    {
      id: 'workspace-overview',
      menuId: 'menu-workspace-overview',
      label: t('sidebar.navigation.overview'),
      routeNames: ['workspace-overview'],
      icon: iconMap.dashboard,
      to: createWorkspaceOverviewTarget(workspaceId, currentProjectId.value || undefined),
    },
    {
      id: 'workspace-knowledge',
      menuId: 'menu-workspace-knowledge',
      label: t('sidebar.navigation.knowledge'),
      routeNames: ['workspace-knowledge'],
      icon: iconMap.knowledge,
      to: { name: 'workspace-knowledge', params: { workspaceId } },
    },
    {
      id: 'workspace-resources',
      menuId: 'menu-workspace-resources',
      label: t('sidebar.navigation.resources'),
      routeNames: ['workspace-resources'],
      icon: iconMap.resources,
      to: { name: 'workspace-resources', params: { workspaceId } },
    },
    {
      id: 'workspace-agents',
      menuId: 'menu-workspace-agents',
      label: t('sidebar.navigation.agents'),
      routeNames: ['workspace-agents'],
      icon: iconMap.agents,
      to: { name: 'workspace-agents', params: { workspaceId } },
    },
    {
      id: 'workspace-teams',
      menuId: 'menu-workspace-teams',
      label: t('sidebar.navigation.teams'),
      routeNames: ['workspace-teams'],
      icon: iconMap.teams,
      to: { name: 'workspace-teams', params: { workspaceId } },
    },
    {
      id: 'workspace-models',
      menuId: 'menu-workspace-models',
      label: t('sidebar.navigation.models'),
      routeNames: ['workspace-models'],
      icon: iconMap.models,
      to: { name: 'workspace-models', params: { workspaceId } },
    },
    {
      id: 'workspace-tools',
      menuId: 'menu-workspace-tools',
      label: t('sidebar.navigation.tools'),
      routeNames: ['workspace-tools'],
      icon: iconMap.tools,
      to: { name: 'workspace-tools', params: { workspaceId } },
    },
    {
      id: 'workspace-automations',
      menuId: 'menu-workspace-automations',
      label: t('sidebar.navigation.automations'),
      routeNames: ['workspace-automations'],
      icon: iconMap.automations,
      to: { name: 'workspace-automations', params: { workspaceId } },
    },
    {
      id: 'workspace-user-center',
      menuId: 'menu-workspace-user-center',
      label: t('sidebar.navigation.userCenter'),
      routeNames: [
        'workspace-user-center',
        'workspace-user-center-profile',
        'workspace-user-center-users',
        'workspace-user-center-roles',
        'workspace-user-center-permissions',
        'workspace-user-center-menus',
      ],
      icon: iconMap['user-center'],
      to: {
        name: userCenterStore.firstAccessibleUserCenterRouteName ?? 'workspace-user-center',
        params: { workspaceId },
      },
    },
  ]

  if (!userCenterStore.currentEffectiveMenuIds.length) {
    return items
  }

  return items.filter(item => !item.menuId || userCenterStore.currentEffectiveMenuIds.includes(item.menuId))
})

function projectConversationId(projectId: string) {
  return runtime.sessions.find(session => session.projectId === projectId)?.conversationId
}

function projectModules(projectId: string): NavigationItem[] {
  const workspaceId = currentWorkspaceId.value
  return [
    {
      id: `${projectId}:dashboard`,
      label: t('sidebar.navigation.dashboard'),
      routeNames: ['project-dashboard'],
      icon: iconMap.dashboard,
      to: createProjectDashboardTarget(workspaceId, projectId),
    },
    {
      id: `${projectId}:conversation`,
      label: t('sidebar.projectModules.conversations'),
      routeNames: ['project-conversations', 'project-conversation'],
      icon: iconMap.conversations,
      to: createProjectConversationTarget(workspaceId, projectId, projectConversationId(projectId)),
    },
    {
      id: `${projectId}:agents`,
      label: t('sidebar.navigation.agents'),
      routeNames: ['project-agents'],
      icon: iconMap.agents,
      to: createProjectSurfaceTarget('project-agents', workspaceId, projectId),
    },
    {
      id: `${projectId}:resources`,
      label: t('sidebar.navigation.resources'),
      routeNames: ['project-resources'],
      icon: iconMap.resources,
      to: createProjectSurfaceTarget('project-resources', workspaceId, projectId),
    },
    {
      id: `${projectId}:knowledge`,
      label: t('sidebar.navigation.knowledge'),
      routeNames: ['project-knowledge'],
      icon: iconMap.knowledge,
      to: createProjectSurfaceTarget('project-knowledge', workspaceId, projectId),
    },
    {
      id: `${projectId}:trace`,
      label: t('sidebar.navigation.trace'),
      routeNames: ['project-trace'],
      icon: iconMap.trace,
      to: createProjectSurfaceTarget('project-trace', workspaceId, projectId),
    },
    {
      id: `${projectId}:runtime`,
      label: t('sidebar.navigation.runtime'),
      routeNames: ['project-runtime'],
      icon: iconMap.runtime,
      to: createProjectSurfaceTarget('project-runtime', workspaceId, projectId),
    },
  ]
}

function isRouteActive(routeNames: string[]) {
  return routeNames.includes(String(route.name ?? ''))
}

function isProjectModuleActive(projectId: string, routeNames: string[]) {
  return currentProjectId.value === projectId && isRouteActive(routeNames)
}

async function switchWorkspace(workspaceConnectionId: string, workspaceId: string) {
  workspaceMenuOpen.value = false
  await shell.activateWorkspaceConnection(workspaceConnectionId)
  await router.push(createWorkspaceOverviewTarget(workspaceId))
}
</script>

<template>
  <aside
    class="flex h-full w-[280px] shrink-0 flex-col border-r border-border-subtle bg-sidebar px-4 py-4 dark:border-white/[0.05]"
    :class="shell.leftSidebarCollapsed ? 'hidden' : 'flex'"
  >
    <div class="flex items-center justify-between gap-3 border-b border-border-subtle pb-4 dark:border-white/[0.05]">
      <div class="flex items-center gap-3 min-w-0">
        <img src="/logo.jpg" class="h-8 w-8 rounded-lg object-cover" alt="Logo" />
        <div class="truncate text-base font-bold text-text-primary">网易Lobster</div>
      </div>
      <UiButton variant="ghost" size="icon" data-testid="sidebar-collapse" class="h-8 w-8" @click="shell.toggleLeftSidebar()">
        <PanelLeftClose :size="16" />
      </UiButton>
    </div>

    <div class="mt-6 min-h-0 flex-1 overflow-y-auto">
      <div class="px-2 pb-2 text-[11px] font-bold uppercase tracking-widest text-text-tertiary">
        {{ t('topbar.workspaceSectionTitle') }}
      </div>
      <div class="mb-4 space-y-1">
        <button
          v-for="connection in shell.workspaceConnections"
          :key="connection.workspaceConnectionId"
          type="button"
          class="flex w-full items-center justify-between rounded-xl border px-3 py-2 text-left transition-colors"
          :class="connection.workspaceConnectionId === shell.activeWorkspaceConnectionId
            ? 'border-primary/30 bg-primary/[0.08] text-text-primary'
            : 'border-border-subtle text-text-secondary hover:bg-accent dark:border-white/[0.05]'"
          @click="switchWorkspace(connection.workspaceConnectionId, connection.workspaceId)"
        >
          <span class="truncate text-sm font-medium">{{ connection.label }}</span>
          <span class="truncate pl-3 text-[11px] uppercase tracking-[0.18em] text-text-tertiary">
            {{ connection.transportSecurity === 'loopback' ? t('topbar.localWorkspace') : t('topbar.sharedWorkspace') }}
          </span>
        </button>
      </div>

      <div class="px-2 pb-2 text-[11px] font-bold uppercase tracking-widest text-text-tertiary">
        {{ t('sidebar.projectTree.title') }}
      </div>
      <div class="space-y-3">
        <div
          v-for="project in workspaceStore.projects"
          :key="project.id"
          class="rounded-xl border border-border-subtle p-3 dark:border-white/[0.05]"
        >
          <div class="flex items-center gap-2">
            <FolderKanban :size="16" class="text-text-tertiary" />
            <div class="min-w-0 flex-1">
              <div class="truncate text-sm font-semibold text-text-primary">{{ project.name }}</div>
              <div class="truncate text-xs text-text-secondary">{{ project.description }}</div>
            </div>
          </div>

          <div class="mt-3 space-y-1">
            <RouterLink
              v-for="item in projectModules(project.id)"
              :key="item.id"
              :to="item.to"
              class="flex items-center gap-2 rounded-md px-2 py-1.5 text-xs"
              :class="isProjectModuleActive(project.id, item.routeNames) ? 'bg-primary/[0.08] text-text-primary' : 'text-text-secondary hover:bg-accent'"
            >
              <component :is="item.icon" :size="14" />
              <span class="truncate">{{ item.label }}</span>
            </RouterLink>
          </div>
        </div>
      </div>
    </div>

    <div class="mt-4 border-t border-border-subtle pt-4 dark:border-white/[0.05]">
      <UiPopover v-model:open="workspaceMenuOpen" align="start" side="top" class="w-[256px] p-2">
        <template #trigger>
          <button
            type="button"
            class="group flex w-full items-center gap-3 rounded-xl border border-transparent p-2 text-left transition-all hover:border-border-subtle hover:bg-accent/50 active:scale-[0.98]"
            :class="{ 'border-border-subtle bg-accent/50 shadow-sm': workspaceMenuOpen }"
          >
            <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-gradient-to-br from-primary/20 to-primary/5 text-primary shadow-inner">
              <LayoutDashboard :size="18" />
            </div>
            <div class="flex min-w-0 flex-1 flex-col">
              <div class="truncate text-sm font-bold text-text-primary leading-tight">
                {{ workspaceLabel }}
              </div>
              <div class="truncate text-[11px] font-medium text-text-tertiary leading-tight mt-0.5 uppercase tracking-wider">
                {{ t('sidebar.workspace.label') }}
              </div>
            </div>
            <ChevronsUpDown :size="14" class="shrink-0 text-text-tertiary transition-colors group-hover:text-text-secondary" />
          </button>
        </template>

        <div class="flex flex-col gap-0.5">
          <div class="px-2 py-1.5 mb-1 text-[11px] font-bold uppercase tracking-widest text-text-tertiary/70">
            {{ t('sidebar.workspaceMenu.title') }}
          </div>
          <RouterLink
            v-for="item in workspaceNavigation"
            :key="item.id"
            :to="item.to"
            class="flex items-center gap-3 rounded-md px-3 py-2 text-[13px] transition-colors"
            :class="isRouteActive(item.routeNames) ? 'bg-primary/10 text-primary font-medium' : 'text-text-secondary hover:bg-accent hover:text-text-primary'"
            @click="workspaceMenuOpen = false"
          >
            <component :is="item.icon" :size="16" />
            <span class="truncate">{{ item.label }}</span>
          </RouterLink>

          <div class="my-1.5 border-t border-border-subtle dark:border-white/[0.05]" />

          <RouterLink
            :to="{ name: 'app-settings' }"
            class="flex items-center gap-3 rounded-md px-3 py-2 text-[13px] transition-colors"
            :class="isRouteActive(['app-settings']) ? 'bg-primary/10 text-primary font-medium' : 'text-text-secondary hover:bg-accent hover:text-text-primary'"
            @click="workspaceMenuOpen = false"
          >
            <Settings :size="16" />
            <span>{{ t('topbar.settings') }}</span>
          </RouterLink>
        </div>
      </UiPopover>
    </div>
  </aside>
</template>
