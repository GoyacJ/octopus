import type { RouteLocationRaw } from 'vue-router'

import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import {
  createProjectSurfaceTarget,
  createWorkspaceConsoleSurfaceTarget,
} from '@/i18n/navigation'
import { useNotificationStore } from '@/stores/notifications'
import { useWorkspaceStore } from '@/stores/workspace'

type NotificationLevel = 'success' | 'warning' | 'error'

export function useWorkspaceProjectNotifications() {
  const { t } = useI18n()
  const route = useRoute()
  const router = useRouter()
  const notificationStore = useNotificationStore()
  const workspaceStore = useWorkspaceStore()

  function resolveRoute(target?: RouteLocationRaw) {
    if (!target) {
      return undefined
    }

    const href = router.resolve(target).href
    return href.startsWith('#') ? href.slice(1) : href
  }

  async function notify(
    level: NotificationLevel,
    title: string,
    body: string,
    routeTo?: RouteLocationRaw,
    actionLabel?: string,
  ) {
    await notificationStore.notify({
      scopeKind: 'workspace',
      scopeOwnerId: workspaceStore.currentWorkspaceId || undefined,
      level,
      title,
      body,
      source: 'workspace-project-governance',
      toastDurationMs: 4000,
      routeTo: resolveRoute(routeTo),
      actionLabel,
    })
  }

  function projectSettingsTarget(projectId: string, query?: Record<string, string | undefined>) {
    const workspaceId = resolveWorkspaceId()
    if (!workspaceId) {
      return undefined
    }

    return createProjectSurfaceTarget('project-settings', workspaceId, projectId, query)
  }

  function workspaceSettingsTarget() {
    const workspaceId = resolveWorkspaceId()
    if (!workspaceId) {
      return undefined
    }

    return createWorkspaceConsoleSurfaceTarget('workspace-console-settings', workspaceId)
  }

  function workspaceProjectsTarget() {
    const workspaceId = resolveWorkspaceId()
    if (!workspaceId) {
      return undefined
    }

    return createWorkspaceConsoleSurfaceTarget('workspace-console-projects', workspaceId)
  }

  function resolveWorkspaceId() {
    const routeWorkspaceId = route.params.workspaceId
    if (typeof routeWorkspaceId === 'string' && routeWorkspaceId.length > 0) {
      return routeWorkspaceId
    }
    return workspaceStore.currentWorkspaceId || undefined
  }

  async function notifyWorkspaceSettingsSaved(workspaceName: string) {
    await notify(
      'success',
      t('workspaceSettings.notifications.saved.title'),
      t('workspaceSettings.notifications.saved.body', { workspaceName }),
      workspaceSettingsTarget(),
      t('workspaceSettings.notifications.saved.actionLabel'),
    )
  }

  async function notifyProjectCreated(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projects.notifications.created.title'),
      t('projects.notifications.created.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projects.notifications.created.actionLabel'),
    )
  }

  async function notifyProjectBasicsSaved(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projectSettings.notifications.basicsSaved.title'),
      t('projectSettings.notifications.basicsSaved.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projectSettings.notifications.basicsSaved.actionLabel'),
    )
  }

  async function notifyProjectLeaderSaved(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projectSettings.notifications.leaderSaved.title'),
      t('projectSettings.notifications.leaderSaved.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projectSettings.notifications.leaderSaved.actionLabel'),
    )
  }

  async function notifyProjectGrantScopeSaved(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projectSettings.notifications.grantScopeSaved.title'),
      t('projectSettings.notifications.grantScopeSaved.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projectSettings.notifications.grantScopeSaved.actionLabel'),
    )
  }

  async function notifyProjectRuntimeSaved(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projectSettings.notifications.runtimeSaved.title'),
      t('projectSettings.notifications.runtimeSaved.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projectSettings.notifications.runtimeSaved.actionLabel'),
    )
  }

  async function notifyProjectMembersSaved(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projectSettings.notifications.membersSaved.title'),
      t('projectSettings.notifications.membersSaved.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projectSettings.notifications.membersSaved.actionLabel'),
    )
  }

  async function notifyProjectArchived(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projectSettings.notifications.archived.title'),
      t('projectSettings.notifications.archived.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projectSettings.notifications.archived.actionLabel'),
    )
  }

  async function notifyProjectRestored(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projectSettings.notifications.restored.title'),
      t('projectSettings.notifications.restored.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projectSettings.notifications.restored.actionLabel'),
    )
  }

  async function notifyProjectDeletionRequested(projectName: string, projectId: string) {
    await notify(
      'warning',
      t('projectSettings.notifications.deletionRequested.title'),
      t('projectSettings.notifications.deletionRequested.body', { projectName }),
      projectSettingsTarget(projectId, { review: 'deletion-request' }),
      t('projectSettings.notifications.deletionRequested.actionLabel'),
    )
  }

  async function notifyProjectDeletionApproved(projectName: string, projectId: string) {
    await notify(
      'success',
      t('projectSettings.notifications.deletionApproved.title'),
      t('projectSettings.notifications.deletionApproved.body', { projectName }),
      projectSettingsTarget(projectId, { review: 'deletion-request' }),
      t('projectSettings.notifications.deletionApproved.actionLabel'),
    )
  }

  async function notifyProjectDeletionRejected(projectName: string, projectId: string) {
    await notify(
      'warning',
      t('projectSettings.notifications.deletionRejected.title'),
      t('projectSettings.notifications.deletionRejected.body', { projectName }),
      projectSettingsTarget(projectId),
      t('projectSettings.notifications.deletionRejected.actionLabel'),
    )
  }

  async function notifyProjectDeleted(projectName: string) {
    await notify(
      'success',
      t('projectSettings.notifications.deleted.title'),
      t('projectSettings.notifications.deleted.body', { projectName }),
      workspaceProjectsTarget(),
      t('projectSettings.notifications.deleted.actionLabel'),
    )
  }

  return {
    notifyProjectArchived,
    notifyProjectBasicsSaved,
    notifyProjectCreated,
    notifyProjectDeleted,
    notifyProjectDeletionApproved,
    notifyProjectDeletionRejected,
    notifyProjectDeletionRequested,
    notifyProjectGrantScopeSaved,
    notifyProjectLeaderSaved,
    notifyProjectMembersSaved,
    notifyProjectRestored,
    notifyProjectRuntimeSaved,
    notifyWorkspaceSettingsSaved,
  }
}
