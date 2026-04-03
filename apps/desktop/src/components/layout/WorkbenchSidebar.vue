<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import {
  Bot,
  ChevronDown,
  Cpu,
  FolderOpen,
  FolderKanban,
  LayoutDashboard,
  LibraryBig,
  MessageSquareText,
  PanelLeftClose,
  PlaySquare,
  Plus,
  Trash2,
  UserRound,
  Workflow,
  Wrench,
  Search,
  Settings,
  Bell,
  MoreHorizontal
} from 'lucide-vue-next'

import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget, createProjectDashboardTarget, createProjectSurfaceTarget, createWorkspaceOverviewTarget, createWorkspaceSwitchTarget } from '@/i18n/navigation'
import { type MenuIconKey, getMenuDefinition } from '@/navigation/menuRegistry'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'
import { UiButton, UiDialog, UiInput, UiPopover } from '@octopus/ui'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const projectDialogOpen = ref(false)
const projectDeleteDialogOpen = ref(false)
const projectName = ref('')
const pendingDeleteProjectId = ref<string | null>(null)
const workspaceMenuOpen = ref(false)

const workspaceLabel = computed(() =>
  workbench.activeWorkspace
    ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name)
    : 'Octopus',
)

interface NavigationItem {
  key: string
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
  trace: PlaySquare,
  models: Cpu,
  tools: Wrench,
  automations: Workflow,
  'user-center': UserRound,
  profile: UserRound,
  users: UserRound,
  roles: UserRound,
  permissions: UserRound,
  menus: UserRound,
}

const workspaceNavigationMenuIds = [
  'menu-workspace-overview',
  'menu-knowledge',
  'menu-agents',
  'menu-models',
  'menu-tools',
  'menu-automations',
  'menu-user-center',
] as const

const projectNavigationMenuIds = [
  'menu-project-dashboard',
  'menu-conversations',
  'menu-agents',
  'menu-resources',
  'menu-knowledge',
  'menu-trace',
] as const

function menuLabel(menuId: string) {
  return workbench.workspaceMenus.find((menu) => menu.id === menuId)?.label ?? getMenuDefinition(menuId)?.defaultLabel ?? menuId
}

function buildNavigationItem(menuId: string, to: object, key = menuId.replace('menu-', '')): NavigationItem | undefined {
  const definition = getMenuDefinition(menuId)
  if (!definition || !workbench.currentEffectiveMenuIds.includes(menuId)) {
    return undefined
  }

  return {
    key,
    label: menuLabel(menuId),
    routeNames: definition.routeNames,
    icon: iconMap[definition.icon],
    to,
  }
}

const workspaceNavigation = computed<NavigationItem[]>(() =>
  workspaceNavigationMenuIds
    .map((menuId) => {
      switch (menuId) {
        case 'menu-workspace-overview':
          return buildNavigationItem(menuId, createWorkspaceOverviewTarget(workbench.currentWorkspaceId, workbench.currentProjectId), 'workspace-overview')
        case 'menu-knowledge':
          return buildNavigationItem(menuId, createProjectSurfaceTarget('knowledge', workbench.currentWorkspaceId, workbench.currentProjectId))
        case 'menu-agents':
          return buildNavigationItem(menuId, {
            name: 'agents',
            params: { workspaceId: workbench.currentWorkspaceId },
          })
        case 'menu-models':
          return buildNavigationItem(menuId, {
            name: 'models',
            params: { workspaceId: workbench.currentWorkspaceId },
          })
        case 'menu-tools':
          return buildNavigationItem(menuId, {
            name: 'tools',
            params: { workspaceId: workbench.currentWorkspaceId },
          })
        case 'menu-automations':
          return buildNavigationItem(menuId, {
            name: 'automations',
            params: { workspaceId: workbench.currentWorkspaceId },
          })
        case 'menu-user-center':
          return buildNavigationItem(menuId, {
            name: 'user-center',
            params: { workspaceId: workbench.currentWorkspaceId },
          })
        default:
          return undefined
      }
    })
    .filter((item): item is NavigationItem => Boolean(item)),
)

function isProjectExpanded(projectId: string): boolean {
  return workbench.currentProjectId === projectId
}

function projectModules(projectId: string): NavigationItem[] {
  const workspaceId = workbench.currentWorkspaceId
  const firstConversationId = workbench.firstConversationIdForProject(projectId)

  return projectNavigationMenuIds
    .map((menuId) => {
      switch (menuId) {
        case 'menu-project-dashboard':
          return buildNavigationItem(menuId, createProjectDashboardTarget(workspaceId, projectId), 'dashboard')
        case 'menu-conversations':
          return buildNavigationItem(menuId, createProjectConversationTarget(workspaceId, projectId, firstConversationId))
        case 'menu-agents':
          return buildNavigationItem(menuId, createProjectSurfaceTarget('project-agents', workspaceId, projectId))
        case 'menu-resources':
          return buildNavigationItem(menuId, createProjectSurfaceTarget('resources', workspaceId, projectId))
        case 'menu-knowledge':
          return buildNavigationItem(menuId, createProjectSurfaceTarget('knowledge', workspaceId, projectId))
        case 'menu-trace':
          return buildNavigationItem(menuId, createProjectSurfaceTarget('trace', workspaceId, projectId))
        default:
          return undefined
      }
    })
    .filter((item): item is NavigationItem => Boolean(item))
}

function isProjectModuleActive(projectId: string, routeNames: string[]): boolean {
  return workbench.currentProjectId === projectId && routeNames.includes(String(route.name ?? ''))
}

const workspaceItems = computed(() => workbench.workspaces.map((workspace) => ({
  id: workspace.id,
  label: resolveMockField('workspace', workspace.id, 'name', workspace.name),
  helper: workspace.isLocal ? t('topbar.localWorkspace') : t('topbar.sharedWorkspace'),
  active: workspace.id === workbench.currentWorkspaceId,
})))

async function selectProject(projectId: string) {
  const project = workbench.projects.find((item) => item.id === projectId)
  if (!project) {
    return
  }
  workbench.selectProject(projectId)
  await router.push(createProjectDashboardTarget(project.workspaceId, project.id))
}

async function switchWorkspace(workspaceId: string) {
  if (!workspaceId || workspaceId === workbench.currentWorkspaceId) {
    workspaceMenuOpen.value = false
    return
  }
  workbench.selectWorkspace(workspaceId)
  workspaceMenuOpen.value = false
  await router.push(createWorkspaceSwitchTarget(workbench.workspaces, workspaceId))
}

function openProjectDialog() {
  projectName.value = ''
  projectDialogOpen.value = true
}

function closeProjectDialog() {
  projectDialogOpen.value = false
}

function openProjectDeleteDialog(projectId: string) {
  pendingDeleteProjectId.value = projectId
  projectDeleteDialogOpen.value = true
}

async function confirmCreateProject() {
  const nextProjectName = projectName.value.trim()
  if (!nextProjectName) return
  const project = workbench.createProject(undefined, nextProjectName)
  closeProjectDialog()
  await router.push(createProjectDashboardTarget(project.workspaceId, project.id))
}

async function confirmRemoveProject() {
  if (!pendingDeleteProjectId.value) return
  const targetProjectId = workbench.removeProject(pendingDeleteProjectId.value)
  projectDeleteDialogOpen.value = false
  if (targetProjectId) {
    await router.push(createProjectDashboardTarget(workbench.currentWorkspaceId, targetProjectId))
  }
}
</script>

<template>
  <aside
    class="flex h-full min-h-0 flex-col bg-sidebar transition-all duration-300 ease-in-out border-r border-border-subtle"
    :class="shell.leftSidebarCollapsed ? 'w-0 opacity-0' : 'w-[240px] opacity-100'"
  >
    <!-- Workspace Switcher (Notion Style) -->
    <header class="p-2">
      <UiPopover v-model:open="workspaceMenuOpen" align="start" class="w-64 p-1">
        <template #trigger>
          <button class="flex w-full items-center gap-2 rounded-md p-2 text-left hover:bg-accent transition-colors group">
            <div class="flex h-5 w-5 shrink-0 items-center justify-center rounded-sm bg-primary/10 text-[10px] font-bold text-primary">
              {{ workspaceLabel.slice(0, 1).toUpperCase() }}
            </div>
            <span class="truncate text-sm font-semibold text-text-primary flex-1">{{ workspaceLabel }}</span>
            <ChevronDown :size="14" class="text-text-tertiary opacity-0 group-hover:opacity-100 transition-opacity" />
          </button>
        </template>
        
        <div class="flex flex-col gap-0.5">
          <div class="px-2 py-1.5 text-[10px] font-bold uppercase tracking-wider text-text-tertiary">{{ t('topbar.workspaceSectionTitle') }}</div>
          <button
            v-for="ws in workspaceItems"
            :key="ws.id"
            class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-accent"
            :class="ws.active ? 'bg-accent' : ''"
            @click="switchWorkspace(ws.id)"
          >
            <div class="flex h-5 w-5 items-center justify-center rounded-sm bg-primary/10 text-[10px] font-bold">
              {{ ws.label.slice(0, 1).toUpperCase() }}
            </div>
            <span class="truncate">{{ ws.label }}</span>
          </button>
        </div>
      </UiPopover>
    </header>

    <div class="flex-1 overflow-y-auto overflow-x-hidden p-2 space-y-4">
      <!-- Fast Actions -->
      <nav class="space-y-0.5">
        <button @click="shell.openSearch" class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm text-text-secondary hover:bg-accent group">
          <Search :size="16" />
          <span>{{ t('topbar.searchPlaceholder') }}</span>
          <kbd class="ml-auto text-[10px] opacity-40 group-hover:opacity-100">⌘K</kbd>
        </button>
        <RouterLink :to="{ name: 'settings', params: { workspaceId: workbench.currentWorkspaceId } }" class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm text-text-secondary hover:bg-accent">
          <Settings :size="16" />
          <span>{{ t('topbar.settings') }}</span>
        </RouterLink>
        <button class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm text-text-secondary hover:bg-accent">
          <Bell :size="16" />
          <span>{{ t('topbar.inbox') }}</span>
        </button>
      </nav>

      <!-- Workspace Level Navigation -->
      <section>
        <div class="px-2 py-1.5 text-[10px] font-bold uppercase tracking-wider text-text-tertiary flex items-center justify-between">
          <span>WORKSPACE</span>
        </div>
        <div class="space-y-0.5">
          <RouterLink
            v-for="item in workspaceNavigation"
            :key="item.key"
            :to="item.to"
            class="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors"
            :class="route.name === item.key || (item.routeNames && item.routeNames.includes(String(route.name))) ? 'bg-accent text-text-primary font-medium' : 'text-text-secondary hover:bg-accent hover:text-text-primary'"
          >
            <component :is="item.icon" :size="16" class="shrink-0 opacity-70" />
            <span class="truncate">{{ item.label }}</span>
          </RouterLink>
        </div>
      </section>

      <!-- Projects Tree -->
      <section>
        <div class="px-2 py-1.5 text-[10px] font-bold uppercase tracking-wider text-text-tertiary flex items-center justify-between group">
          <span>{{ t('sidebar.projectTree.title') }}</span>
          <button @click="openProjectDialog" class="opacity-0 group-hover:opacity-100 hover:bg-accent p-0.5 rounded transition-all">
            <Plus :size="12" />
          </button>
        </div>
        <div class="space-y-0.5">
          <div v-for="project in workbench.workspaceProjects" :key="project.id" class="space-y-0.5">
            <button
              @click="selectProject(project.id)"
              class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors group"
              :class="workbench.currentProjectId === project.id ? 'bg-accent text-text-primary' : 'text-text-secondary hover:bg-accent'"
            >
              <FolderKanban :size="16" class="shrink-0 opacity-70" :class="workbench.currentProjectId === project.id ? 'text-primary' : ''" />
              <span class="truncate flex-1" :class="workbench.currentProjectId === project.id ? 'font-medium' : ''">{{ project.name }}</span>
              <button @click.stop="openProjectDeleteDialog(project.id)" class="opacity-0 group-hover:opacity-100 hover:bg-primary/10 p-0.5 rounded transition-all">
                <Trash2 :size="12" class="text-text-tertiary hover:text-destructive" />
              </button>
            </button>
            
            <!-- Project Modules Sub-tree -->
            <div v-if="workbench.currentProjectId === project.id" class="pl-4 space-y-0.5">
              <RouterLink
                v-for="module in projectModules(project.id)"
                :key="module.key"
                :to="module.to"
                class="flex items-center gap-2 rounded-md px-2 py-1 text-[13px] transition-colors"
                :class="isProjectModuleActive(project.id, module.routeNames) ? 'text-text-primary font-medium bg-primary/5' : 'text-text-tertiary hover:bg-accent hover:text-text-primary'"
              >
                <component :is="module.icon" :size="14" class="shrink-0 opacity-60" />
                <span class="truncate">{{ module.label }}</span>
              </RouterLink>
            </div>
          </div>
        </div>
      </section>
    </div>

    <!-- User Profile Bottom (Notion Style) -->
    <footer class="p-2 border-t border-border-subtle">
      <button class="flex w-full items-center gap-2 rounded-md p-2 text-left hover:bg-accent transition-colors">
        <div class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] font-bold text-white uppercase">
          {{ workbench.currentUser?.nickname?.slice(0, 1) || 'U' }}
        </div>
        <span class="truncate text-xs font-medium text-text-secondary flex-1">{{ workbench.currentUser?.nickname }}</span>
        <MoreHorizontal :size="14" class="text-text-tertiary opacity-40" />
      </button>
    </footer>
  </aside>

  <!-- Rail/Collapsed State -->
  <aside
    v-if="shell.leftSidebarCollapsed"
    class="flex h-full w-[48px] flex-col items-center py-2 border-r border-border-subtle bg-sidebar gap-4"
  >
    <button @click="shell.toggleLeftSidebar()" class="p-2 hover:bg-accent rounded-md text-text-tertiary">
      <PanelLeftClose :size="18" class="rotate-180" />
    </button>
    <div class="flex flex-col gap-2">
      <button v-for="ws in workspaceItems.filter(i => i.active)" :key="ws.id" class="w-8 h-8 rounded-md bg-primary/10 flex items-center justify-center text-xs font-bold text-primary">
        {{ ws.label.slice(0, 1).toUpperCase() }}
      </button>
    </div>
  </aside>

  <UiDialog
    :open="projectDialogOpen"
    :title="t('sidebar.projectTree.dialogTitle')"
    @update:open="projectDialogOpen = $event"
  >
    <UiInput v-model="projectName" :placeholder="t('sidebar.projectTree.inputPlaceholder')" @keydown.enter="confirmCreateProject" />
    <template #footer>
      <UiButton variant="ghost" @click="projectDialogOpen = false">{{ t('common.cancel') }}</UiButton>
      <UiButton @click="confirmCreateProject">{{ t('common.confirm') }}</UiButton>
    </template>
  </UiDialog>

  <UiDialog
    :open="projectDeleteDialogOpen"
    :title="t('sidebar.projectTree.remove')"
    @update:open="projectDeleteDialogOpen = $event"
  >
    <p class="text-sm text-text-secondary">{{ t('sidebar.projectTree.removeConfirm') }}</p>
    <template #footer>
      <UiButton variant="ghost" @click="projectDeleteDialogOpen = false">{{ t('common.cancel') }}</UiButton>
      <UiButton variant="destructive" @click="confirmRemoveProject">{{ t('sidebar.projectTree.remove') }}</UiButton>
    </template>
  </UiDialog>
</template>
