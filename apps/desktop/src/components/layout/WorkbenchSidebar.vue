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
import { UiButton, UiDialog, UiInput } from '@octopus/ui'

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

function openProjectDeleteDialog(projectId: string) {
  pendingDeleteProjectId.value = projectId
  projectDeleteDialogOpen.value = true
}

function closeProjectDeleteDialog() {
  projectDeleteDialogOpen.value = false
  pendingDeleteProjectId.value = null
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

async function confirmRemoveProject() {
  if (!pendingDeleteProjectId.value) {
    return
  }

  const targetProjectId = workbench.removeProject(pendingDeleteProjectId.value)
  closeProjectDeleteDialog()
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
              <div class="project-node-card">
                <div class="project-node-button">
                  <button
                    type="button"
                    class="project-node-trigger"
                    :data-testid="`project-node-${project.id}`"
                    @click="selectProject(project.id)"
                  >
                    <span class="project-node-icon">
                      <FolderKanban :size="16" />
                    </span>
                    <span class="project-node-copy">
                      <strong>{{ resolveMockField('project', project.id, 'name', project.name) }}</strong>
                      <small>{{ t('common.conversations', { count: project.conversationIds.length }) }}</small>
                    </span>
                    <ChevronRight :size="16" class="project-node-chevron" />
                  </button>
                  <button
                    v-if="!isProjectExpanded(project.id)"
                    type="button"
                    class="project-remove"
                    :title="t('sidebar.projectTree.remove')"
                    :data-testid="`remove-project-${project.id}`"
                    @click.stop="openProjectDeleteDialog(project.id)"
                  >
                    <Trash2 :size="14" />
                  </button>
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

    <div
      v-if="projectDialogOpen"
      class="project-dialog-overlay"
      data-testid="project-create-dialog"
    >
      <button type="button" class="project-dialog-backdrop" @click="closeProjectDialog" />
      <section class="project-dialog-card">
        <header class="project-dialog-header">
          <div>
            <h2>{{ t('sidebar.projectTree.dialogTitle') }}</h2>
            <p>{{ t('sidebar.projectTree.dialogDescription') }}</p>
          </div>
          <button type="button" class="project-dialog-close" @click="closeProjectDialog">×</button>
        </header>
        <div class="project-dialog-body">
          <UiInput
            v-model="projectName"
            data-testid="project-create-input"
            :placeholder="t('sidebar.projectTree.inputPlaceholder')"
            @keydown.enter.prevent="confirmCreateProject"
          />
        </div>
        <footer class="project-dialog-footer">
          <UiButton
            variant="ghost"
            data-testid="project-create-cancel"
            @click="closeProjectDialog"
          >
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton
            data-testid="project-create-confirm"
            :disabled="!projectName.trim()"
            @click="confirmCreateProject"
          >
            {{ t('common.confirm') }}
          </UiButton>
        </footer>
      </section>
    </div>

    <UiDialog
      :open="projectDeleteDialogOpen"
      :title="t('sidebar.projectTree.remove')"
      :description="t('sidebar.projectTree.removeConfirm')"
      :close-label="t('common.cancel')"
      @update:open="(open) => { if (!open) closeProjectDeleteDialog() }"
    >
      <template #footer>
        <UiButton
          variant="ghost"
          data-testid="project-delete-cancel"
          @click="closeProjectDeleteDialog"
        >
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          data-testid="project-delete-confirm"
          @click="confirmRemoveProject"
        >
          {{ t('sidebar.projectTree.remove') }}
        </UiButton>
      </template>
    </UiDialog>
  </aside>
</template>

<style scoped>
.sidebar-shell {
  min-height: 0;
  border-right: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: var(--bg-sidebar);
}

.sidebar-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  padding: 0.95rem 0.85rem;
  background: transparent;
  overflow: hidden;
}

.sidebar-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.15rem 0.1rem 0.95rem;
  background: var(--bg-sidebar);
  position: sticky;
  top: 0;
  z-index: 2;
}

.toolbar-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text-primary);
}

.project-create,
.project-remove,
.rail-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.1rem;
  height: 2.1rem;
  border: 1px solid var(--border-subtle);
  border-radius: 999px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  box-shadow: var(--shadow-xs);
  transition: all var(--duration-fast) var(--ease-apple);
}

.project-create:hover,
.rail-toggle:hover {
  border-color: color-mix(in srgb, var(--brand-primary) 22%, var(--border-subtle));
  color: var(--text-primary);
  background: color-mix(in srgb, var(--bg-subtle) 70%, var(--bg-surface));
}

.sidebar-main {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-height: 0;
  gap: 1.25rem;
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
  padding-right: 0.2rem;
  padding-bottom: 0.35rem;
}

.project-tree {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: 100%;
}

.project-node {
  position: relative;
  border-radius: var(--radius-l);
  overflow: hidden;
}

.project-node-card {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-l) + 1px);
  background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
  box-shadow: var(--shadow-xs);
  transition: all var(--duration-fast) var(--ease-apple);
}

.project-node.expanded .project-node-card {
  border-color: color-mix(in srgb, var(--brand-primary) 18%, var(--border-subtle));
  background: var(--bg-surface);
  box-shadow: var(--shadow-sm);
}

.project-node-button {
  position: relative;
  overflow: hidden;
}

.project-remove {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 3.25rem;
  height: auto;
  border: 0;
  border-left: 1px solid color-mix(in srgb, var(--status-error) 24%, var(--border-subtle));
  border-radius: 0;
  background: color-mix(in srgb, var(--status-error) 10%, var(--bg-surface));
  color: var(--status-error);
  box-shadow: none;
  opacity: 0;
  pointer-events: none;
  transform: translateX(100%);
}

.project-node:not(.expanded):hover .project-remove,
.project-node:not(.expanded):focus-within .project-remove {
  opacity: 1;
  pointer-events: auto;
  transform: translateX(0);
}

.project-remove:hover {
  background: color-mix(in srgb, var(--status-error) 18%, var(--bg-surface));
  color: var(--status-error);
}

.project-node-trigger {
  width: 100%;
  min-width: 0;
  border: 0;
  background: transparent;
  text-align: left;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.85rem 0.95rem;
  transition: transform var(--duration-fast) var(--ease-apple);
}

.project-node:not(.expanded):hover .project-node-trigger,
.project-node:not(.expanded):focus-within .project-node-trigger {
  transform: translateX(-2.9rem);
}

.project-node-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  border-radius: var(--radius-m);
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
  color: var(--brand-primary);
}

.project-node-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.project-node-copy strong {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-node-copy small {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.project-node-chevron {
  margin-left: auto;
  color: var(--text-tertiary);
  transition: transform var(--duration-fast) var(--ease-apple);
}

.project-node.expanded .project-node-chevron {
  transform: rotate(90deg);
  color: var(--text-primary);
}

.project-modules {
  display: grid;
  gap: 0.2rem;
  padding: 0 0.5rem 0.5rem;
}

.project-module-link {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.65rem 0.8rem;
  border-radius: var(--radius-m);
  color: var(--text-secondary);
  font-size: 0.85rem;
  font-weight: 500;
  transition: all var(--duration-fast) var(--ease-apple);
}

.project-module-link:hover {
  background: color-mix(in srgb, var(--bg-subtle) 82%, transparent);
  color: var(--text-primary);
}

.project-module-link.active {
  background: color-mix(in srgb, var(--brand-primary) 9%, var(--bg-subtle));
  color: var(--brand-primary);
  font-weight: 600;
}

.workspace-navigation {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding-top: 0.8rem;
  border-top: 1px solid var(--border-subtle);
}

.workspace-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 0.8rem 0.95rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-l) + 1px);
  background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
  box-shadow: var(--shadow-xs);
  transition: all var(--duration-fast) var(--ease-apple);
}

.workspace-trigger:hover,
.workspace-trigger.active {
  border-color: color-mix(in srgb, var(--brand-primary) 20%, var(--border-subtle));
  background: var(--bg-surface);
  box-shadow: var(--shadow-sm);
}

.workspace-trigger-copy {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  min-width: 0;
}

.workspace-trigger-copy small {
  font-size: 0.7rem;
  color: var(--text-tertiary);
  text-transform: uppercase;
  font-weight: 700;
  letter-spacing: 0.05em;
}

.workspace-trigger-copy strong {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
}

.workspace-trigger-chevron {
  color: var(--text-tertiary);
  transition: transform var(--duration-fast) var(--ease-apple);
}

.workspace-trigger-chevron.open {
  transform: rotate(180deg);
}

.workspace-menu {
  position: absolute;
  bottom: calc(100% + 0.5rem);
  left: 0;
  right: 0;
  z-index: 10;
  display: flex;
  flex-direction: column;
  padding: 0.5rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-l) + 2px);
  background: var(--bg-popover);
  box-shadow: var(--shadow-lg);
  backdrop-filter: blur(18px);
}

.workspace-menu-header {
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--text-tertiary);
  text-transform: uppercase;
}

.workspace-nav-link {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.65rem 0.75rem;
  border-radius: var(--radius-m);
  color: var(--text-secondary);
  font-size: 0.85rem;
  transition: all var(--duration-fast) var(--ease-apple);
}

.workspace-nav-link:hover {
  background: var(--bg-subtle);
  color: var(--text-primary);
}

.workspace-nav-link.active {
  background: color-mix(in srgb, var(--brand-primary) 10%, var(--bg-subtle));
  color: var(--brand-primary);
  font-weight: 600;
}

.project-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
}

.project-dialog-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(10, 15, 30, 0.42);
  backdrop-filter: blur(14px);
}

.project-dialog-card {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: min(100%, 32rem);
  padding: 1.2rem;
  border-radius: calc(var(--radius-xl) + 2px);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: var(--bg-popover);
  box-shadow: var(--shadow-lg);
}

.project-dialog-header,
.project-dialog-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.project-dialog-header {
  align-items: flex-start;
}

.project-dialog-header h2 {
  font-size: 1.05rem;
  font-weight: 700;
}

.project-dialog-header p {
  margin-top: 0.25rem;
  color: var(--text-secondary);
  font-size: 0.88rem;
  line-height: 1.6;
}

.project-dialog-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  color: var(--text-secondary);
}

.project-dialog-close:hover {
  background: color-mix(in srgb, var(--bg-subtle) 82%, transparent);
  color: var(--text-primary);
}

.project-dialog-footer {
  justify-content: flex-end;
}

.sidebar-rail {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.9rem 0.65rem;
  border-right: 1px solid var(--border-subtle);
  background: var(--bg-sidebar);
}

.rail-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: var(--radius-m);
  color: var(--text-secondary);
  transition: all var(--duration-fast) var(--ease-apple);
}

.rail-link:hover {
  background: var(--bg-subtle);
  color: var(--text-primary);
}

.rail-link.active {
  background: color-mix(in srgb, var(--brand-primary) 15%, var(--bg-subtle));
  color: var(--brand-primary);
}
</style>
