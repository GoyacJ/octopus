<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import {
  Bot,
  ChevronDown,
  ChevronRight,
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
} from 'lucide-vue-next'

import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget, createProjectDashboardTarget, createProjectSurfaceTarget, createWorkspaceOverviewTarget } from '@/i18n/navigation'
import { type MenuIconKey, getMenuDefinition } from '@/navigation/menuRegistry'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()
const projectDialogOpen = ref(false)
const projectName = ref('')
const workspaceMenuOpen = ref(false)
const workspaceMenuRef = ref<HTMLElement | null>(null)

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
          return buildNavigationItem(menuId, createWorkspaceOverviewTarget(workbench.currentWorkspaceId, workbench.currentProjectId))
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

const railItems = computed(() => {
  const projectId = workbench.currentProjectId
  const primaryItems = projectId ? projectModules(projectId) : []
  const seen = new Set(primaryItems.map((item) => item.key))
  const workspaceItems = workspaceNavigation.value.filter((item) => !seen.has(item.key))

  return [...primaryItems, ...workspaceItems]
})

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

function isWorkspaceNavActive(routeNames: string[]): boolean {
  return routeNames.includes(String(route.name ?? ''))
}

const workspaceTriggerActive = computed(() =>
  workspaceNavigation.value.some((item) => isWorkspaceNavActive(item.routeNames)),
)

function closeWorkspaceMenu() {
  workspaceMenuOpen.value = false
}

function toggleWorkspaceMenu() {
  workspaceMenuOpen.value = !workspaceMenuOpen.value
}

function handleClickOutside(event: MouseEvent) {
  if (workspaceMenuRef.value?.contains(event.target as Node)) {
    return
  }

  closeWorkspaceMenu()
}

async function selectProject(projectId: string) {
  const project = workbench.projects.find((item) => item.id === projectId)
  if (!project) {
    return
  }

  workbench.selectProject(projectId)
  await router.push(createProjectDashboardTarget(project.workspaceId, project.id))
}

function openProjectDialog() {
  projectName.value = ''
  projectDialogOpen.value = true
}

function closeProjectDialog() {
  projectDialogOpen.value = false
  projectName.value = ''
}

async function confirmCreateProject() {
  const nextProjectName = projectName.value.trim()
  if (!nextProjectName) {
    return
  }

  const project = workbench.createProject(undefined, nextProjectName)
  closeProjectDialog()
  await router.push(createProjectDashboardTarget(project.workspaceId, project.id))
}

async function removeProject(projectId: string) {
  if (!window.confirm(t('sidebar.projectTree.removeConfirm'))) {
    return
  }

  const targetProjectId = workbench.removeProject(projectId)
  if (!targetProjectId) {
    return
  }

  await router.push(createProjectDashboardTarget(workbench.currentWorkspaceId, targetProjectId))
}

onMounted(() => {
  window.addEventListener('mousedown', handleClickOutside)
})

onBeforeUnmount(() => {
  window.removeEventListener('mousedown', handleClickOutside)
})
</script>

<template>
  <aside v-if="shell.leftSidebarCollapsed" class="sidebar-rail" data-testid="sidebar-rail">
    <button type="button" class="rail-toggle" :title="t('topbar.leftSidebar')" @click="shell.toggleLeftSidebar()">
      <PanelLeftClose :size="18" />
    </button>
    <RouterLink
      v-for="item in railItems"
      :key="item.key"
      :to="item.to"
      class="rail-link"
      :class="{ active: item.routeNames.includes(String(route.name ?? '')) }"
      :title="item.label"
    >
      <component :is="item.icon" :size="18" />
    </RouterLink>
  </aside>

  <aside v-else class="sidebar-shell">
    <div class="sidebar-panel">
      <div class="sidebar-toolbar">
        <div class="toolbar-copy">
          <span class="toolbar-title">{{ t('sidebar.projectTree.title') }}</span>
        </div>
        <div class="toolbar-actions">
          <button
            type="button"
            class="project-create"
            :title="t('sidebar.projectTree.create')"
            data-testid="add-project-button"
            @click="openProjectDialog"
          >
            <Plus :size="16" />
          </button>
          <button type="button" class="rail-toggle" :title="t('topbar.leftSidebar')" @click="shell.toggleLeftSidebar()">
            <PanelLeftClose :size="18" />
          </button>
        </div>
      </div>

      <div class="sidebar-main">
        <div class="project-tree-scroll scroll-y" data-testid="sidebar-project-tree-scroll">
          <div class="project-tree" data-testid="sidebar-project-tree">
            <div
              v-for="project in workbench.workspaceProjects"
              :key="project.id"
              class="project-node"
              :class="{ expanded: isProjectExpanded(project.id) }"
            >
              <div v-if="!isProjectExpanded(project.id)" class="project-delete-rail">
                <button
                  type="button"
                  class="project-delete"
                  :data-testid="`remove-project-${project.id}`"
                  :title="t('sidebar.projectTree.remove')"
                  :disabled="workbench.workspaceProjects.length <= 1"
                  @click.stop="removeProject(project.id)"
                >
                  <Trash2 :size="14" />
                </button>
              </div>

              <div class="project-node-card">
                <div
                  class="project-node-button"
                  :class="{ active: isProjectExpanded(project.id) }"
                >
                  <button
                    type="button"
                    class="project-node-trigger"
                    :data-testid="`project-node-${project.id}`"
                    @click="selectProject(project.id)"
                  >
                    <span class="project-node-leading">
                      <span class="project-node-icon">
                        <FolderKanban :size="16" />
                      </span>
                      <span class="project-node-copy">
                        <strong>{{ resolveMockField('project', project.id, 'name', project.name) }}</strong>
                        <small>{{ t('common.conversations', { count: project.conversationIds.length }) }}</small>
                      </span>
                    </span>
                  </button>
                  <ChevronRight :size="16" class="project-node-chevron" />
                </div>

                <div v-if="isProjectExpanded(project.id)" class="project-modules">
                  <RouterLink
                    v-for="item in projectModules(project.id)"
                    :key="item.key"
                    class="project-module-link"
                    :data-testid="`project-module-${project.id}-${item.key}`"
                    :class="{ active: isProjectModuleActive(project.id, item.routeNames) }"
                    :to="item.to"
                  >
                    <component :is="item.icon" :size="16" />
                    <span>{{ item.label }}</span>
                  </RouterLink>
                </div>
              </div>
            </div>
          </div>
        </div>

        <nav
          ref="workspaceMenuRef"
          class="workspace-navigation"
          data-testid="sidebar-bottom-navigation"
        >
          <button
            type="button"
            class="workspace-trigger"
            :class="{ active: workspaceTriggerActive || workspaceMenuOpen }"
            data-testid="sidebar-workspace-trigger"
            :title="t('sidebar.workspaceMenu.trigger')"
            aria-haspopup="menu"
            :aria-expanded="workspaceMenuOpen"
            @click="toggleWorkspaceMenu"
          >
            <span class="workspace-trigger-copy">
              <small>{{ t('sidebar.workspace.label') }}</small>
              <strong>{{ t('sidebar.workspaceMenu.trigger') }}</strong>
            </span>
            <ChevronDown :size="16" class="workspace-trigger-chevron" :class="{ open: workspaceMenuOpen }" />
          </button>

          <div
            v-if="workspaceMenuOpen"
            class="workspace-menu"
            data-testid="sidebar-workspace-menu"
          >
            <div class="workspace-menu-header">
              <strong>{{ t('sidebar.workspaceMenu.title') }}</strong>
            </div>
            <RouterLink
              v-for="item in workspaceNavigation"
              :key="item.key"
              class="workspace-nav-link"
              :data-testid="`sidebar-nav-${item.key}`"
              :class="{ active: isWorkspaceNavActive(item.routeNames) }"
              :to="item.to"
              @click="closeWorkspaceMenu"
            >
              <component :is="item.icon" :size="16" />
              <span>{{ item.label }}</span>
            </RouterLink>
          </div>
        </nav>
      </div>
    </div>

    <div v-if="projectDialogOpen" class="project-dialog-shell" data-testid="project-create-dialog">
      <button type="button" class="project-dialog-backdrop" @click="closeProjectDialog" />
      <section class="project-dialog">
        <div class="project-dialog-copy">
          <strong>{{ t('sidebar.projectTree.dialogTitle') }}</strong>
          <p>{{ t('sidebar.projectTree.dialogDescription') }}</p>
        </div>
        <input
          v-model="projectName"
          data-testid="project-create-input"
          :placeholder="t('sidebar.projectTree.inputPlaceholder')"
          @keydown.enter.prevent="confirmCreateProject"
        >
        <div class="project-dialog-actions">
          <button type="button" class="ghost-button" data-testid="project-create-cancel" @click="closeProjectDialog">
            {{ t('common.cancel') }}
          </button>
          <button
            type="button"
            class="primary-button"
            data-testid="project-create-confirm"
            :disabled="!projectName.trim()"
            @click="confirmCreateProject"
          >
            {{ t('common.confirm') }}
          </button>
        </div>
      </section>
    </div>
  </aside>
</template>

<style scoped>
.sidebar-shell {
  min-height: 0;
  border-right: 1px solid var(--border-subtle);
  background:
    radial-gradient(circle at top left, color-mix(in srgb, var(--brand-primary) 10%, transparent), transparent 36%),
    linear-gradient(180deg, color-mix(in srgb, var(--bg-sidebar) 96%, white), var(--bg-sidebar));
}

.sidebar-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  padding: 0.95rem 0.9rem 0.85rem;
  background: color-mix(in srgb, var(--bg-sidebar) 94%, transparent);
  overflow: hidden;
}

.sidebar-toolbar,
.toolbar-actions,
.project-node-card,
.project-node-button,
.project-node-trigger,
.project-node-leading,
.project-node-copy,
.project-module-link,
.workspace-navigation,
.workspace-trigger,
.workspace-trigger-copy,
.workspace-nav-link,
.sidebar-rail {
  display: flex;
  align-items: center;
}

.sidebar-toolbar {
  position: sticky;
  top: 0;
  z-index: 2;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  padding: 0.2rem 0.1rem 0.95rem;
  background: linear-gradient(180deg, color-mix(in srgb, var(--bg-sidebar) 98%, black), color-mix(in srgb, var(--bg-sidebar) 88%, transparent));
}

.toolbar-copy {
  display: flex;
  min-width: 0;
}

.toolbar-title {
  font-size: 0.95rem;
  font-weight: 700;
}

.project-node-copy small {
  color: var(--text-secondary);
}

.toolbar-actions {
  gap: 0.45rem;
}

.project-create,
.rail-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.15rem;
  height: 2.15rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 86%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-surface) 74%, transparent);
  color: var(--text-primary);
}

.sidebar-main {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-height: 0;
  gap: 1rem;
  overflow: hidden;
}

.project-tree-scroll {
  display: flex;
  flex: 1 1 auto;
  min-height: 0;
  max-height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  scrollbar-width: thin;
  padding-right: 0.2rem;
  padding-bottom: 0.35rem;
}

.project-tree-scroll::-webkit-scrollbar {
  width: 10px;
}

.project-tree-scroll::-webkit-scrollbar-track {
  background: var(--scrollbar-track);
  border-radius: 999px;
}

.project-tree-scroll::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border: 2px solid transparent;
  border-radius: 999px;
  background-clip: padding-box;
}

.project-tree-scroll:hover::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb-hover);
  border: 2px solid transparent;
  background-clip: padding-box;
}

.project-tree {
  display: flex;
  flex: none;
  flex-direction: column;
  gap: 0.6rem;
  width: 100%;
  min-height: max-content;
}

.project-node {
  --project-delete-width: 3.6rem;
  position: relative;
  border-radius: 1rem;
  overflow: hidden;
}

.project-delete-rail {
  position: absolute;
  inset: 0 0 0 auto;
  display: flex;
  align-items: stretch;
  justify-content: center;
  width: var(--project-delete-width);
  padding: 0.18rem 0 0.18rem 0.25rem;
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--duration-fast) var(--ease-apple);
}

.project-node-card {
  position: relative;
  z-index: 1;
  flex-direction: column;
  align-items: stretch;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 76%, transparent);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--bg-surface) 62%, transparent);
  transition:
    transform var(--duration-fast) var(--ease-apple),
    border-color var(--duration-fast) var(--ease-apple),
    background var(--duration-fast) var(--ease-apple);
}

.project-node:not(.expanded):hover .project-delete-rail,
.project-node:not(.expanded):focus-within .project-delete-rail {
  opacity: 1;
  pointer-events: auto;
}

.project-node:not(.expanded):hover .project-node-card,
.project-node:not(.expanded):focus-within .project-node-card {
  transform: translateX(calc(-1 * var(--project-delete-width)));
}

.project-node.expanded .project-node-card {
  border-color: color-mix(in srgb, var(--brand-primary) 28%, var(--border-subtle));
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 10%, transparent), transparent 46%),
    color-mix(in srgb, var(--bg-surface) 80%, transparent);
}

.project-node-button {
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.9rem 0.95rem;
}

.project-node-button.active {
  color: var(--text-primary);
}

.project-node-trigger {
  flex: 1 1 auto;
  min-width: 0;
  border: 0;
  background: transparent;
  text-align: left;
}

.project-node-leading {
  gap: 0.75rem;
  min-width: 0;
  flex: 1 1 auto;
}

.project-node-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.95rem;
  height: 1.95rem;
  border-radius: 0.75rem;
  background: color-mix(in srgb, var(--brand-primary) 16%, transparent);
  color: var(--brand-primary);
}

.project-node-copy {
  flex-direction: column;
  align-items: flex-start;
  min-width: 0;
  gap: 0.18rem;
}

.project-node-copy strong {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-node-chevron {
  flex: none;
  color: var(--text-secondary);
  transition: transform var(--duration-fast) var(--ease-apple);
}

.project-node.expanded .project-node-chevron {
  transform: rotate(90deg);
}

.project-delete {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  border: 1px solid color-mix(in srgb, var(--status-error) 24%, transparent);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--status-error) 10%, transparent);
  color: var(--text-secondary);
}

.project-delete:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.project-delete:not(:disabled):hover {
  color: var(--status-error);
  background: color-mix(in srgb, var(--status-error) 16%, transparent);
}

.project-modules {
  display: grid;
  gap: 0.3rem;
  padding: 0 0.55rem 0.55rem;
}

.project-module-link {
  gap: 0.65rem;
  min-width: 0;
  padding: 0.72rem 0.8rem;
  border-radius: 0.85rem;
  color: var(--text-secondary);
}

.project-module-link.active {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--bg-subtle) 74%, transparent);
}

.workspace-navigation {
  position: relative;
  gap: 0.32rem;
  padding-top: 0.8rem;
  border-top: 1px solid color-mix(in srgb, var(--border-subtle) 82%, transparent);
}

.workspace-navigation,
.project-node-copy,
.sidebar-rail {
  flex-direction: column;
}

.workspace-trigger {
  justify-content: space-between;
  width: 100%;
  min-width: 0;
  padding: 0.8rem 0.9rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 74%, transparent);
  border-radius: 0.95rem;
  background: color-mix(in srgb, var(--bg-surface) 54%, transparent);
  color: var(--text-primary);
  transition:
    border-color var(--duration-fast) var(--ease-apple),
    background var(--duration-fast) var(--ease-apple);
}

.workspace-trigger:hover,
.workspace-trigger.active {
  border-color: color-mix(in srgb, var(--brand-primary) 26%, var(--border-subtle));
  background: color-mix(in srgb, var(--bg-surface) 82%, transparent);
}

.workspace-trigger-copy {
  flex: 1;
  min-width: 0;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.15rem;
}

.workspace-trigger-copy small {
  color: var(--text-secondary);
}

.workspace-trigger-copy strong {
  font-size: 0.95rem;
}

.workspace-trigger-chevron {
  color: var(--text-secondary);
  transition: transform var(--duration-fast) var(--ease-apple);
}

.workspace-trigger-chevron.open {
  transform: rotate(180deg);
}

.workspace-menu {
  position: absolute;
  right: 0;
  bottom: calc(100% + 0.65rem);
  left: 0;
  z-index: 5;
  display: flex;
  flex-direction: column;
  gap: 0.22rem;
  padding: 0.55rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 76%, transparent);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--bg-sidebar) 96%, black);
  box-shadow: 0 18px 38px color-mix(in srgb, black 18%, transparent);
}

.workspace-menu-header {
  padding: 0.35rem 0.35rem 0.5rem;
}

.workspace-menu-header strong {
  font-size: 0.82rem;
  color: var(--text-secondary);
}

.workspace-nav-link {
  gap: 0.7rem;
  width: 100%;
  min-width: 0;
  padding: 0.72rem 0.85rem;
  border-radius: 0.85rem;
  color: var(--text-secondary);
}

.workspace-nav-link.active {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--bg-surface) 80%, transparent);
}

.project-dialog-shell {
  position: fixed;
  inset: 0;
  z-index: 30;
}

.project-dialog-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
}

.project-dialog {
  position: relative;
  z-index: 1;
  width: min(24rem, calc(100vw - 2rem));
  margin: 12vh auto 0;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1.1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 86%, transparent);
  border-radius: 1rem;
  background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
  box-shadow: var(--shadow-lg);
}

.project-dialog-copy {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.project-dialog-copy p {
  color: var(--text-secondary);
  line-height: 1.5;
}

.project-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

.sidebar-rail {
  gap: 0.45rem;
  min-height: 0;
  padding: 0.9rem 0.65rem;
  border-right: 1px solid var(--border-subtle);
  background:
    radial-gradient(circle at top left, color-mix(in srgb, var(--brand-primary) 10%, transparent), transparent 36%),
    linear-gradient(180deg, color-mix(in srgb, var(--bg-sidebar) 96%, white), var(--bg-sidebar));
}

.rail-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 0.9rem;
  color: var(--text-secondary);
}

.rail-link.active {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--bg-surface) 82%, transparent);
}

@media (max-width: 980px) {
  .sidebar-toolbar {
    position: static;
  }

  .workspace-navigation {
    padding-bottom: 0.35rem;
  }
}
</style>
