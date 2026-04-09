<script setup lang="ts">
import { reactive, ref, computed } from 'vue'
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
  Plus,
  Settings,
  Trash2,
  UserRound,
  Users,
  Workflow,
  Wrench,
} from 'lucide-vue-next'

import type { ProjectRecord, WorkspaceConnectionRecord } from '@octopus/schema'
import { UiButton, UiDialog, UiField, UiInput, UiPopover, UiStatusCallout, UiTextarea } from '@octopus/ui'

import ConnectWorkspaceDialog from '@/components/layout/ConnectWorkspaceDialog.vue'
import DesktopPetHost from '@/components/pet/DesktopPetHost.vue'
import { resolveWorkspaceLabel } from '@/composables/workspace-label'
import { createProjectConversationTarget, createProjectDashboardTarget, createProjectSurfaceTarget, createWorkspaceOverviewTarget, createWorkspaceProjectsTarget } from '@/i18n/navigation'
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
const connectWorkspaceDialogOpen = ref(false)
const quickCreateOpen = ref(false)
const quickCreateSubmitting = ref(false)
const deleteDialogOpen = ref(false)
const deleteSubmitting = ref(false)
const deleteTargetProjectId = ref('')

const quickCreateForm = reactive({
  name: '',
  description: '',
})
const workspaceLabel = computed(() =>
  resolveWorkspaceLabel(
    shell.activeWorkspaceConnection,
    workspaceStore.activeWorkspace?.name,
    t,
  ),
)

type NavigationItem = {
  id: string
  label: string
  routeNames: string[]
  icon: unknown
  to: object
  testId?: string
}

const iconMap: Record<MenuIconKey, unknown> = {
  dashboard: LayoutDashboard,
  conversations: MessageSquareText,
  agents: Bot,
  resources: FolderOpen,
  knowledge: LibraryBig,
  trace: Bell,
  runtime: Settings,
  projects: FolderKanban,
  models: Cpu,
  tools: Wrench,
  automations: Workflow,
  'user-center': UserRound,
  profile: UserRound,
  pet: Bot,
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
const activeProjects = computed(() => workspaceStore.projects.filter(item => item.status === 'active'))
const deleteTargetProject = computed(() =>
  activeProjects.value.find(project => project.id === deleteTargetProjectId.value) ?? null,
)

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
      id: 'workspace-projects',
      menuId: 'menu-workspace-projects',
      label: t('sidebar.navigation.projects'),
      routeNames: ['workspace-projects'],
      icon: iconMap.projects,
      to: createWorkspaceProjectsTarget(workspaceId),
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
  return runtime.sessions.find(session => session.projectId === projectId && session.sessionKind !== 'pet')?.conversationId
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
      testId: `sidebar-project-module-${projectId}-conversation`,
    },
    {
      id: `${projectId}:agents`,
      label: t('sidebar.navigation.agents'),
      routeNames: ['project-agents'],
      icon: iconMap.agents,
      to: createProjectSurfaceTarget('project-agents', workspaceId, projectId),
      testId: `sidebar-project-module-${projectId}-agents`,
    },
    {
      id: `${projectId}:resources`,
      label: t('sidebar.navigation.resources'),
      routeNames: ['project-resources'],
      icon: iconMap.resources,
      to: createProjectSurfaceTarget('project-resources', workspaceId, projectId),
      testId: `sidebar-project-module-${projectId}-resources`,
    },
    {
      id: `${projectId}:knowledge`,
      label: t('sidebar.navigation.knowledge'),
      routeNames: ['project-knowledge'],
      icon: iconMap.knowledge,
      to: createProjectSurfaceTarget('project-knowledge', workspaceId, projectId),
      testId: `sidebar-project-module-${projectId}-knowledge`,
    },
    {
      id: `${projectId}:trace`,
      label: t('sidebar.navigation.trace'),
      routeNames: ['project-trace'],
      icon: iconMap.trace,
      to: createProjectSurfaceTarget('project-trace', workspaceId, projectId),
      testId: `sidebar-project-module-${projectId}-trace`,
    },
    {
      id: `${projectId}:settings`,
      label: t('sidebar.navigation.projectSettings'),
      routeNames: ['project-settings'],
      icon: iconMap.settings,
      to: createProjectSurfaceTarget('project-settings', workspaceId, projectId),
      testId: `sidebar-project-module-${projectId}-settings`,
    },
    {
      id: `${projectId}:runtime`,
      label: t('sidebar.navigation.runtime'),
      routeNames: ['project-runtime'],
      icon: iconMap.runtime,
      to: createProjectSurfaceTarget('project-runtime', workspaceId, projectId),
      testId: `sidebar-project-module-${projectId}-runtime`,
    },
  ]
}

function isRouteActive(routeNames: string[]) {
  return routeNames.includes(String(route.name ?? ''))
}

function isProjectModuleActive(projectId: string, routeNames: string[]) {
  return currentProjectId.value === projectId && isRouteActive(routeNames)
}

function isProjectExpanded(projectId: string) {
  return currentProjectId.value === projectId
}

function closeWorkspaceMenu() {
  workspaceMenuOpen.value = false
}

function getWorkspaceConnectionLabel(connection: WorkspaceConnectionRecord) {
  return resolveWorkspaceLabel(connection, connection.label, t)
}

function getWorkspaceConnectionStatusDotClass(status: WorkspaceConnectionRecord['status']) {
  return status === 'connected' ? 'bg-status-success' : 'bg-status-error'
}

async function switchWorkspace(workspaceConnectionId: string, workspaceId: string) {
  workspaceMenuOpen.value = false
  await shell.activateWorkspaceConnection(workspaceConnectionId)
  await router.push(createWorkspaceOverviewTarget(workspaceId))
}

function openConnectWorkspaceDialog() {
  workspaceMenuOpen.value = false
  connectWorkspaceDialogOpen.value = true
}

function resetQuickCreateForm() {
  quickCreateForm.name = ''
  quickCreateForm.description = ''
}

async function submitQuickCreateProject() {
  const workspaceId = currentWorkspaceId.value
  if (!workspaceId || !quickCreateForm.name.trim() || quickCreateSubmitting.value) {
    return
  }

  quickCreateSubmitting.value = true

  try {
    const created = await workspaceStore.createProject({
      name: quickCreateForm.name,
      description: quickCreateForm.description,
    })
    if (!created) {
      return
    }

    quickCreateOpen.value = false
    resetQuickCreateForm()
    await router.push(createProjectSurfaceTarget('project-settings', workspaceId, created.id))
  } finally {
    quickCreateSubmitting.value = false
  }
}

async function openProject(projectId: string) {
  const workspaceId = currentWorkspaceId.value
  if (!workspaceId) {
    return
  }

  await router.push(createProjectDashboardTarget(workspaceId, projectId))
}

function handleProjectSummaryClick(projectId: string) {
  if (isProjectExpanded(projectId)) {
    return
  }

  void openProject(projectId)
}

function openDeleteDialog(project: ProjectRecord) {
  deleteTargetProjectId.value = project.id
  deleteDialogOpen.value = true
}

function closeDeleteDialog() {
  deleteDialogOpen.value = false
  deleteTargetProjectId.value = ''
}

async function confirmDeleteProject() {
  if (!deleteTargetProject.value || deleteSubmitting.value) {
    return
  }

  deleteSubmitting.value = true

  try {
    const updated = await workspaceStore.archiveProject(deleteTargetProject.value.id)
    if (updated) {
      closeDeleteDialog()
    }
  } finally {
    deleteSubmitting.value = false
  }
}

async function removeWorkspaceConnection(workspaceConnectionId: string, workspaceId: string) {
  workspaceMenuOpen.value = false
  const wasActive = shell.activeWorkspaceConnectionId === workspaceConnectionId
  const fallbackConnection = await shell.deleteWorkspaceConnection(workspaceConnectionId)
  if (wasActive && fallbackConnection) {
    await router.push(createWorkspaceOverviewTarget(fallbackConnection.workspaceId))
    return
  }

  if (String(route.params.workspaceId ?? '') === workspaceId && fallbackConnection) {
    await router.push(createWorkspaceOverviewTarget(fallbackConnection.workspaceId))
  }
}
</script>

<template>
  <aside
    class="flex h-full w-[280px] shrink-0 flex-col border-r border-border bg-sidebar px-3 py-3"
    :class="shell.leftSidebarCollapsed ? 'hidden' : 'flex'"
  >
    <div class="flex items-center justify-between gap-3 border-b border-border pb-3">
      <div class="flex items-center gap-3 min-w-0">
        <img src="/logo.png" class="h-8 w-8 rounded-[var(--radius-m)] object-cover" alt="Octopus logo" />
        <div class="truncate text-[14px] font-semibold text-text-primary">Octopus</div>
      </div>
      <UiButton variant="ghost" size="icon" data-testid="sidebar-collapse" class="h-8 w-8" @click="shell.toggleLeftSidebar()">
        <PanelLeftClose :size="16" />
      </UiButton>
    </div>

    <div class="mt-4 min-h-0 flex-1 overflow-y-auto">
      <div class="flex items-center justify-between gap-2 px-2 pb-2">
        <div class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
          {{ t('sidebar.projectTree.title') }}
        </div>
        <UiPopover v-model:open="quickCreateOpen" align="end" side="bottom" class="w-[300px] p-0">
          <template #trigger>
            <UiButton
              variant="ghost"
              size="icon"
              class="h-7 w-7"
              data-testid="sidebar-project-create-trigger"
              :aria-label="t('sidebar.projectTree.create')"
            >
              <Plus :size="14" />
            </UiButton>
          </template>

          <form
            data-testid="sidebar-project-create-popover"
            class="space-y-4 p-4"
            @submit.prevent="submitQuickCreateProject"
          >
            <div class="space-y-1">
              <h3 class="text-sm font-semibold text-text-primary">{{ t('sidebar.projectTree.dialogTitle') }}</h3>
              <p class="text-xs leading-5 text-text-secondary">{{ t('sidebar.projectTree.dialogDescription') }}</p>
            </div>

            <UiField :label="t('projects.fields.name')">
              <UiInput
                v-model="quickCreateForm.name"
                data-testid="sidebar-project-create-name-input"
                :placeholder="t('sidebar.projectTree.inputPlaceholder')"
              />
            </UiField>

            <UiField :label="t('projects.fields.description')">
              <UiTextarea
                v-model="quickCreateForm.description"
                data-testid="sidebar-project-create-description-input"
                :rows="4"
              />
            </UiField>

            <UiStatusCallout
              v-if="workspaceStore.error"
              tone="error"
              :description="workspaceStore.error"
            />

            <div class="flex justify-end gap-2">
              <UiButton
                type="button"
                variant="ghost"
                @click="quickCreateOpen = false"
              >
                {{ t('common.cancel') }}
              </UiButton>
              <UiButton
                type="submit"
                data-testid="sidebar-project-create-submit"
                :disabled="quickCreateSubmitting || !quickCreateForm.name.trim()"
              >
                {{ t('projects.actions.create') }}
              </UiButton>
            </div>
          </form>
        </UiPopover>
      </div>
      <div class="space-y-3">
        <div
          v-for="project in activeProjects"
          :key="project.id"
          :data-testid="`sidebar-project-${project.id}`"
          class="group rounded-[var(--radius-l)] border border-border bg-surface p-3 shadow-xs transition-colors"
        >
          <div class="flex items-center gap-2">
            <button
              type="button"
              :data-testid="`sidebar-project-summary-${project.id}`"
              class="flex min-w-0 flex-1 items-center gap-2 text-left transition-transform duration-200"
              :class="!isProjectExpanded(project.id) ? 'group-hover:-translate-x-1 cursor-pointer' : 'cursor-default'"
              @click="handleProjectSummaryClick(project.id)"
            >
              <FolderKanban :size="16" class="shrink-0 text-text-tertiary" />
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm font-semibold text-text-primary">{{ project.name }}</div>
                <div class="truncate text-xs text-text-secondary">{{ project.description }}</div>
              </div>
            </button>

            <UiButton
              v-if="!isProjectExpanded(project.id)"
              :data-testid="`sidebar-project-delete-trigger-${project.id}`"
              type="button"
              variant="ghost"
              size="icon"
              class="h-7 w-7 shrink-0 opacity-0 transition-all duration-200 pointer-events-none group-hover:opacity-100 group-hover:pointer-events-auto"
              :aria-label="t('sidebar.projectTree.remove')"
              @click.stop="openDeleteDialog(project)"
            >
              <Trash2 :size="14" />
            </UiButton>
          </div>

          <div v-if="isProjectExpanded(project.id)" class="mt-3 space-y-1">
            <RouterLink
              v-for="item in projectModules(project.id)"
              :key="item.id"
              :to="item.to"
              :data-testid="item.testId"
              class="flex items-center gap-2 rounded-[var(--radius-xs)] px-2 py-1.5 text-[12px]"
              :class="isProjectModuleActive(project.id, item.routeNames) ? 'bg-accent text-text-primary' : 'text-text-secondary hover:bg-accent'"
            >
              <component :is="item.icon" :size="14" />
              <span class="truncate">{{ item.label }}</span>
            </RouterLink>
          </div>
        </div>
      </div>
    </div>

    <div class="mt-4 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 border-t border-border pt-4">
      <UiPopover v-model:open="workspaceMenuOpen" align="start" side="top" class="min-w-0 w-[256px] p-2">
        <template #trigger>
          <button
            type="button"
            data-testid="sidebar-workspace-menu-trigger"
            class="group flex min-w-0 flex-1 items-center gap-3 rounded-[var(--radius-l)] border border-transparent p-2 text-left transition-colors hover:border-border hover:bg-accent"
            :class="{ 'border-border bg-accent shadow-xs': workspaceMenuOpen }"
          >
            <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-[var(--radius-m)] bg-accent text-primary shadow-xs">
              <LayoutDashboard :size="18" />
            </div>
            <div class="flex min-w-0 flex-1 flex-col">
              <div class="truncate text-sm font-bold text-text-primary leading-tight">
                {{ workspaceLabel }}
              </div>
              <div class="mt-0.5 truncate text-[11px] font-medium uppercase tracking-[0.08em] text-text-tertiary leading-tight">
                {{ t('sidebar.workspace.label') }}
              </div>
            </div>
            <ChevronsUpDown :size="14" class="shrink-0 text-text-tertiary transition-colors group-hover:text-text-secondary" />
          </button>
        </template>

        <div class="flex flex-col gap-2">
          <div class="mb-1 px-2 py-1.5 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
            {{ t('sidebar.workspaceMenu.title') }}
          </div>
          <div data-testid="sidebar-workspace-navigation-menu" class="flex flex-col gap-1">
            <RouterLink
              v-for="item in workspaceNavigation"
              :key="item.id"
              :data-testid="`sidebar-workspace-nav-${item.id}`"
              :to="item.to"
              class="flex items-center gap-3 rounded-[var(--radius-m)] px-3 py-2 text-[13px] transition-colors"
              :class="isRouteActive(item.routeNames) ? 'bg-accent text-text-primary font-medium' : 'text-text-secondary hover:bg-accent hover:text-text-primary'"
              @click="closeWorkspaceMenu"
            >
              <component :is="item.icon" :size="16" />
              <span class="truncate">{{ item.label }}</span>
            </RouterLink>
          </div>

          <div class="my-1 border-t border-border" />

          <div class="mb-1 px-2 py-1.5 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
            {{ t('topbar.workspaceSectionTitle') }}
          </div>
          <div data-testid="sidebar-workspace-menu-list" class="flex flex-col gap-1">
            <div
              v-for="connection in shell.workspaceConnections"
              :key="connection.workspaceConnectionId"
              class="flex items-center gap-2"
            >
              <button
                :data-testid="`sidebar-workspace-menu-item-${connection.workspaceConnectionId}`"
                type="button"
                class="flex min-w-0 flex-1 items-center gap-3 rounded-[var(--radius-m)] border px-3 py-2 text-left transition-colors"
                :class="connection.workspaceConnectionId === shell.activeWorkspaceConnectionId
                  ? 'border-border-strong bg-accent text-text-primary shadow-xs'
                  : 'border-border text-text-secondary hover:bg-accent'"
                @click="switchWorkspace(connection.workspaceConnectionId, connection.workspaceId)"
              >
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <span class="truncate text-sm font-semibold">{{ getWorkspaceConnectionLabel(connection) }}</span>
                    <span
                      :data-testid="`sidebar-workspace-status-dot-${connection.workspaceConnectionId}`"
                      class="h-2.5 w-2.5 shrink-0 rounded-full"
                      :class="getWorkspaceConnectionStatusDotClass(connection.status)"
                      :aria-label="connection.status"
                      :title="connection.status"
                    />
                  </div>
                  <div class="truncate text-[11px] text-text-tertiary">
                    {{ connection.baseUrl }}
                  </div>
                </div>
              </button>
              <UiButton
                v-if="connection.transportSecurity !== 'loopback'"
                :data-testid="`sidebar-workspace-delete-${connection.workspaceConnectionId}`"
                type="button"
                variant="ghost"
                size="icon"
                class="h-7 w-7 shrink-0"
                :aria-label="t('topbar.removeWorkspace')"
                @click="removeWorkspaceConnection(connection.workspaceConnectionId, connection.workspaceId)"
              >
                <Trash2 :size="14" />
              </UiButton>
            </div>
          </div>

          <UiButton
            data-testid="sidebar-connect-workspace-trigger"
            variant="ghost"
            class="w-full justify-start rounded-[var(--radius-m)] px-3 py-2"
            @click="openConnectWorkspaceDialog"
          >
            <Plus :size="16" class="mr-2" />
            {{ t('connectWorkspace.actions.trigger') }}
          </UiButton>
        </div>
      </UiPopover>

      <DesktopPetHost class="justify-self-end shrink-0" />
    </div>

    <ConnectWorkspaceDialog v-model:open="connectWorkspaceDialogOpen" />
    <UiDialog
      v-model:open="deleteDialogOpen"
      :title="t('sidebar.projectTree.deleteDialog.title')"
      :description="t('sidebar.projectTree.deleteDialog.description', { name: deleteTargetProject?.name ?? '' })"
      content-test-id="sidebar-project-delete-dialog"
    >
      <UiStatusCallout
        v-if="workspaceStore.error"
        tone="error"
        :description="workspaceStore.error"
      />

      <template #footer>
        <UiButton variant="ghost" @click="closeDeleteDialog">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          data-testid="sidebar-project-delete-confirm"
          :disabled="deleteSubmitting"
          @click="confirmDeleteProject"
        >
          {{ t('sidebar.projectTree.deleteDialog.confirm') }}
        </UiButton>
      </template>
    </UiDialog>
  </aside>
</template>
