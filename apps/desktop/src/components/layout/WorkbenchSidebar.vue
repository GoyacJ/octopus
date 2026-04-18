<script setup lang="ts">
import { reactive, ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import {
  Bell,
  Bot,
  ChevronsUpDown,
  Cpu,
  FileText,
  FolderKanban,
  FolderOpen,
  LayoutDashboard,
  LibraryBig,
  ListTodo,
  MessageSquareText,
  Network,
  PanelLeftClose,
  Plus,
  Settings,
  ShieldCheck,
  Trash2,
  UserRound,
  Users,
  Wrench,
} from 'lucide-vue-next'

import type { ProjectRecord, WorkspaceConnectionRecord } from '@octopus/schema'
import { UiButton, UiDialog, UiField, UiInput, UiPopover, UiSelect, UiStatusCallout, UiTextarea } from '@octopus/ui'

import ConnectWorkspaceDialog from '@/components/layout/ConnectWorkspaceDialog.vue'
import DesktopPetHost from '@/components/pet/DesktopPetHost.vue'
import ProjectResourceDirectoryField from '@/components/projects/ProjectResourceDirectoryField.vue'
import {
  isProjectMember,
  isProjectModuleAllowed,
  isProjectOwner,
  isProjectOwnerOnlyRoute,
  projectModuleForRouteName,
  resolveProjectActorUserId,
} from '@/composables/project-governance'
import { resolveWorkspaceLabel } from '@/composables/workspace-label'
import {
  createProjectConversationTarget,
  createProjectDashboardTarget,
  createProjectSurfaceTarget,
  createWorkspaceConsoleTarget,
  createWorkspaceOverviewTarget,
} from '@/i18n/navigation'
import { type MenuIconKey } from '@/navigation/menuRegistry'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import {
  buildInheritedProjectAssignments,
  buildProjectSetupPresetSeed,
  type ProjectSetupPreset,
} from '@/stores/project_setup'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useTeamStore } from '@/stores/team'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const agentStore = useAgentStore()
const catalogStore = useCatalogStore()
const teamStore = useTeamStore()
const workspaceStore = useWorkspaceStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()
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
  resourceDirectory: '',
  preset: 'general' as ProjectSetupPreset,
  leaderAgentId: '',
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
  deliverables: FileText,
  tasks: ListTodo,
  agents: Bot,
  resources: FolderOpen,
  knowledge: LibraryBig,
  trace: Bell,
  projects: FolderKanban,
  models: Cpu,
  tools: Wrench,
  console: LayoutDashboard,
  'access-control': ShieldCheck,
  profile: UserRound,
  pet: Bot,
  users: UserRound,
  roles: UserRound,
  permissions: UserRound,
  menus: UserRound,
  organization: Network,
  policy: ShieldCheck,
  'resource-policy': FolderOpen,
  sessions: Bell,
  settings: Settings,
  connections: Settings, // Fallback if still needed
  teams: Users,
  bell: Bell,
}

const currentWorkspaceId = computed(() => workspaceStore.currentWorkspaceId)
const currentProjectId = computed(() => workspaceStore.currentProjectId)
const currentProjectActorUserId = computed(() =>
  resolveProjectActorUserId(
    workspaceAccessControlStore.currentUser?.id,
    workspaceAccessControlStore.loading ? undefined : shell.activeWorkspaceSession?.session.userId,
  ),
)
const activeProjects = computed(() =>
  workspaceStore.projects.filter((item) => {
    if (item.status !== 'active') {
      return false
    }
    if (!currentProjectActorUserId.value) {
      return false
    }
    return isProjectMember(item, currentProjectActorUserId.value)
  }),
)
const deleteTargetProject = computed(() =>
  activeProjects.value.find(project => project.id === deleteTargetProjectId.value) ?? null,
)
const hasAccessControlAuthorization = computed(() => Boolean(workspaceAccessControlStore.authorization))

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
      id: 'workspace-console',
      menuId: 'menu-workspace-console',
      label: t('sidebar.navigation.console'),
      routeNames: [
        'workspace-console',
        'workspace-console-projects',
        'workspace-console-knowledge',
        'workspace-console-resources',
        'workspace-console-agents',
        'workspace-console-models',
        'workspace-console-tools',
      ],
      icon: iconMap.console,
      to: createWorkspaceConsoleTarget(workspaceId),
    },
    {
      id: 'workspace-access-control',
      menuId: 'menu-workspace-access-control',
      label: t('sidebar.navigation.accessControl'),
      routeNames: [
        'workspace-access-control',
        'workspace-access-control-members',
        'workspace-access-control-access',
        'workspace-access-control-governance',
      ],
      icon: iconMap['access-control'],
      to: {
        name: 'workspace-access-control',
        params: { workspaceId },
      },
    },
  ]

  return items.filter((item) => {
    if (item.id === 'workspace-access-control') {
      return !hasAccessControlAuthorization.value || workspaceAccessControlStore.canShowAccessControlNavigation
    }

    if (!hasAccessControlAuthorization.value) {
      return true
    }

    return !item.menuId || workspaceAccessControlStore.currentEffectiveMenuIds.includes(item.menuId)
  })
})

function projectConversationId(projectId: string) {
  const recentConversations = [
    ...(workspaceStore.activeOverview?.recentConversations ?? []),
    ...(workspaceStore.getProjectDashboard(projectId)?.recentConversations ?? []),
  ]
    .filter(conversation => conversation.projectId === projectId)
    .sort((left, right) => right.updatedAt - left.updatedAt)

  const latestRecentConversation = recentConversations[0]
  if (latestRecentConversation) {
    return latestRecentConversation.id
  }

  return runtime.sessions
    .filter(session => session.projectId === projectId && session.sessionKind !== 'pet')
    .sort((left, right) => right.updatedAt - left.updatedAt)[0]
    ?.conversationId
}

function projectModules(projectId: string): NavigationItem[] {
  const workspaceId = currentWorkspaceId.value
  const project = activeProjects.value.find(item => item.id === projectId)
  const actorUserId = currentProjectActorUserId.value
  if (!workspaceId || !project || !actorUserId) {
    return []
  }

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
      id: `${projectId}:deliverables`,
      label: t('sidebar.navigation.deliverables'),
      routeNames: ['project-deliverables'],
      icon: iconMap.deliverables,
      to: createProjectSurfaceTarget('project-deliverables', workspaceId, projectId),
      testId: `sidebar-project-module-${projectId}-deliverables`,
    },
    {
      id: `${projectId}:tasks`,
      label: t('sidebar.navigation.tasks'),
      routeNames: ['project-tasks'],
      icon: iconMap.tasks,
      to: createProjectSurfaceTarget('project-tasks', workspaceId, projectId),
      testId: `sidebar-project-module-${projectId}-tasks`,
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
  ].filter((item) => {
    const routeName = item.routeNames[0]
    if (isProjectOwnerOnlyRoute(routeName)) {
      return isProjectOwner(project, actorUserId)
    }
    const module = projectModuleForRouteName(routeName)
    return !module || isProjectModuleAllowed(workspaceStore.activeWorkspace, project, module)
  })
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

function projectGroupClasses(projectId: string) {
  return [
    'group rounded-[var(--radius-l)] border p-2 transition-colors',
    isProjectExpanded(projectId)
      ? 'border-border bg-surface/80'
      : 'border-transparent bg-transparent',
  ].join(' ')
}

function projectSummaryClasses(projectId: string) {
  return [
    'ui-focus-ring flex min-w-0 flex-1 items-center gap-2 rounded-[var(--radius-m)] px-2 py-2 text-left transition-colors',
    isProjectExpanded(projectId)
      ? 'cursor-default'
      : 'cursor-pointer text-text-secondary hover:bg-subtle hover:text-text-primary',
  ].join(' ')
}

function workspaceTriggerClasses() {
  return [
    'ui-focus-ring group flex min-w-0 flex-1 items-center gap-3 rounded-[var(--radius-l)] border p-2 text-left transition-colors',
    workspaceMenuOpen.value
      ? 'border-border-strong bg-accent'
      : 'border-transparent hover:border-border hover:bg-subtle',
  ].join(' ')
}

function workspaceTriggerIconClasses() {
  return [
    'flex h-9 w-9 shrink-0 items-center justify-center rounded-[var(--radius-m)] bg-primary/10 text-primary transition-colors',
    workspaceMenuOpen.value ? 'bg-surface' : '',
  ].join(' ').trim()
}

function workspaceTriggerCaretClasses() {
  return workspaceMenuOpen.value
    ? 'text-text-secondary'
    : 'text-text-tertiary transition-colors group-hover:text-text-secondary'
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
  quickCreateForm.resourceDirectory = ''
  quickCreateForm.preset = 'general'
  quickCreateForm.leaderAgentId = quickCreateLeaderOptions.value[0]?.value ?? ''
}

const quickCreatePresetOptions = computed(() => ([
  { value: 'general', label: String(t('projects.presets.options.general.label')) },
  { value: 'engineering', label: String(t('projects.presets.options.engineering.label')) },
  { value: 'documentation', label: String(t('projects.presets.options.documentation.label')) },
  { value: 'advanced', label: String(t('projects.presets.options.advanced.label')) },
]))

const quickCreateLeaderOptions = computed(() =>
  agentStore.workspaceOwnedAgents
    .filter(agent => agent.status === 'active')
    .map(agent => ({
      value: agent.id,
      label: agent.name,
    })),
)

watch(
  () => workspaceStore.activeConnectionId,
  async (connectionId) => {
    if (!connectionId) {
      return
    }

    await Promise.all([
      catalogStore.ensureLoaded(connectionId),
      agentStore.ensureLoaded(connectionId),
      teamStore.ensureLoaded(connectionId),
    ])
  },
  { immediate: true },
)

async function submitQuickCreateProject() {
  const workspaceId = currentWorkspaceId.value
  if (
    !workspaceId
    || !quickCreateForm.name.trim()
    || !quickCreateForm.resourceDirectory.trim()
    || !quickCreateForm.leaderAgentId
    || quickCreateSubmitting.value
  ) {
    return
  }

  quickCreateSubmitting.value = true

  try {
    const connectionId = workspaceStore.activeConnectionId
    if (connectionId) {
      await Promise.all([
        catalogStore.load(connectionId),
        agentStore.load(connectionId),
        teamStore.load(connectionId),
      ])
    }

    const presetSeed = buildProjectSetupPresetSeed(quickCreateForm.preset, {
      models: catalogStore.configuredModelOptions,
      tools: catalogStore.managementProjection.assets.filter(entry => entry.enabled),
      agents: agentStore.workspaceAgents,
      teams: teamStore.workspaceTeams,
    })

    const created = await workspaceStore.createProject({
      name: quickCreateForm.name,
      description: quickCreateForm.description,
      resourceDirectory: quickCreateForm.resourceDirectory,
      leaderAgentId: quickCreateForm.leaderAgentId || undefined,
      assignments: buildInheritedProjectAssignments(presetSeed.assignments?.models),
    })
    if (!created) {
      return
    }

    await (
      presetSeed.modelSettings
        ? workspaceStore.saveProjectModelSettings(created.id, presetSeed.modelSettings)
        : Promise.resolve(null)
    )

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
        <UiPopover
          v-model:open="quickCreateOpen"
          align="end"
          side="bottom"
          class="w-[320px] overflow-hidden p-0"
        >
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
            class="flex flex-col"
            @submit.prevent="submitQuickCreateProject"
          >
            <div
              data-testid="sidebar-project-create-intro"
              class="space-y-1 border-b border-border bg-subtle px-4 py-3"
            >
              <h3 class="text-sm font-semibold text-text-primary">{{ t('sidebar.projectTree.dialogTitle') }}</h3>
              <p class="text-xs leading-5 text-text-secondary">{{ t('sidebar.projectTree.dialogDescription') }}</p>
            </div>

            <div class="space-y-4 px-4 py-4">
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

              <UiField
                :label="t('projects.fields.preset')"
                :hint="t('projects.presets.hint')"
              >
                <UiSelect
                  v-model="quickCreateForm.preset"
                  data-testid="sidebar-project-create-preset-select"
                  :options="quickCreatePresetOptions"
                />
              </UiField>

              <UiField
                :label="t('projects.fields.leader')"
                :hint="t('projects.fields.leaderHint')"
              >
                <UiSelect
                  v-model="quickCreateForm.leaderAgentId"
                  data-testid="sidebar-project-create-leader-select"
                  :options="quickCreateLeaderOptions"
                />
              </UiField>

              <div class="rounded-[var(--radius-l)] border border-border bg-surface-muted px-3 py-2 text-xs leading-5 text-text-secondary">
                {{ t(`projects.presets.options.${quickCreateForm.preset}.description`) }}
              </div>

              <div
                data-testid="sidebar-project-create-inheritance-hint"
                class="rounded-[var(--radius-l)] border border-dashed border-border bg-surface px-3 py-2 text-xs leading-5 text-text-secondary"
              >
                {{ t('projects.inheritanceHint') }}
              </div>

              <ProjectResourceDirectoryField
                v-model="quickCreateForm.resourceDirectory"
                path-test-id="sidebar-project-create-resource-directory-path"
                pick-test-id="sidebar-project-create-resource-directory-pick"
              />

              <UiStatusCallout
                v-if="workspaceStore.error"
                tone="error"
                :description="workspaceStore.error"
              />
            </div>

            <div
              data-testid="sidebar-project-create-actions"
              class="flex justify-end gap-2 border-t border-border bg-subtle px-4 py-3"
            >
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
                :disabled="quickCreateSubmitting || !quickCreateForm.name.trim() || !quickCreateForm.resourceDirectory.trim() || !quickCreateForm.leaderAgentId"
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
          :class="projectGroupClasses(project.id)"
        >
          <div class="flex items-center gap-2">
            <button
              type="button"
              :data-testid="`sidebar-project-summary-${project.id}`"
              :class="projectSummaryClasses(project.id)"
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
              class="flex items-center gap-2 rounded-[var(--radius-xs)] border border-transparent px-2 py-1.5 text-[12px] transition-colors"
              :class="isProjectModuleActive(project.id, item.routeNames) ? 'border-border-strong bg-accent text-text-primary' : 'text-text-secondary hover:border-border hover:bg-subtle hover:text-text-primary'"
            >
              <component :is="item.icon" :size="14" />
              <span class="truncate">{{ item.label }}</span>
            </RouterLink>
          </div>
        </div>
      </div>
    </div>

    <div class="mt-4 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 border-t border-border pt-4">
      <UiPopover
        v-model:open="workspaceMenuOpen"
        align="start"
        side="top"
        class="min-w-0 w-[272px] overflow-hidden p-0"
      >
        <template #trigger>
          <button
            type="button"
            data-testid="sidebar-workspace-menu-trigger"
            :class="workspaceTriggerClasses()"
          >
            <div
              data-testid="sidebar-workspace-menu-trigger-icon"
              :class="workspaceTriggerIconClasses()"
            >
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
            <ChevronsUpDown :size="14" class="shrink-0" :class="workspaceTriggerCaretClasses()" />
          </button>
        </template>

        <div class="flex flex-col">
          <div
            data-testid="sidebar-workspace-menu-intro"
            class="border-b border-border bg-subtle px-3 py-3"
          >
            <div class="truncate text-sm font-semibold text-text-primary">
              {{ workspaceLabel }}
            </div>
            <div class="mt-1 text-[11px] font-medium uppercase tracking-[0.08em] text-text-tertiary">
              {{ t('sidebar.workspaceMenu.title') }}
            </div>
          </div>
          <div class="flex flex-col gap-3 px-2 py-2">
            <div>
              <div data-testid="sidebar-workspace-navigation-menu" class="flex flex-col gap-1">
                <RouterLink
                  v-for="item in workspaceNavigation"
                  :key="item.id"
                  :data-testid="`sidebar-workspace-nav-${item.id}`"
                  :to="item.to"
                  class="flex items-center gap-3 rounded-[var(--radius-m)] border border-transparent px-3 py-2 text-[13px] transition-colors"
                  :class="isRouteActive(item.routeNames) ? 'border-border-strong bg-accent text-text-primary font-medium' : 'text-text-secondary hover:border-border hover:bg-subtle hover:text-text-primary'"
                  @click="closeWorkspaceMenu"
                >
                  <component :is="item.icon" :size="16" />
                  <span class="truncate">{{ item.label }}</span>
                </RouterLink>
              </div>
            </div>

            <div class="border-t border-border pt-3">
              <div class="mb-1 px-1 py-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
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
                      ? 'border-border-strong bg-accent text-text-primary'
                      : 'border-border text-text-secondary hover:border-border-strong hover:bg-subtle'"
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
            </div>
          </div>

          <div
            data-testid="sidebar-workspace-menu-actions"
            class="border-t border-border bg-subtle px-2 py-2"
          >
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
