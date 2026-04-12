import { useI18n } from 'vue-i18n'

import { enumLabel } from '@/i18n/copy'
import { useNotificationStore } from '@/stores/notifications'
import { useWorkspaceStore } from '@/stores/workspace'

type UploadKind = 'file' | 'folder'
type ResourceScope = 'personal' | 'project' | 'workspace'

export function useProjectResourceNotifications() {
  const { t } = useI18n()
  const notificationStore = useNotificationStore()
  const workspaceStore = useWorkspaceStore()

  async function notify(level: 'success' | 'warning' | 'error', title: string, body?: string) {
    await notificationStore.notify({
      scopeKind: 'workspace',
      scopeOwnerId: workspaceStore.currentWorkspaceId || undefined,
      level,
      title,
      body,
      source: 'project-resources',
      toastDurationMs: 4000,
    })
  }

  async function notifyStatusChanged(name: string, nextEnabled: boolean) {
    await notify(
      nextEnabled ? 'success' : 'warning',
      t(`resources.notifications.status.${nextEnabled ? 'enabled' : 'disabled'}.title`),
      t(`resources.notifications.status.${nextEnabled ? 'enabled' : 'disabled'}.body`, { name }),
    )
  }

  async function notifyStatusChangeFailed(name: string, message: string) {
    await notify(
      'error',
      t('resources.notifications.status.errorTitle'),
      t('resources.notifications.status.errorBody', { name, message }),
    )
  }

  async function notifyVisibilityChanged(name: string, visibility: 'private' | 'public') {
    await notify(
      'success',
      t('resources.notifications.visibility.successTitle'),
      t('resources.notifications.visibility.successBody', {
        name,
        visibility: enumLabel('resourceVisibility', visibility),
      }),
    )
  }

  async function notifyVisibilityChangeFailed(name: string, message: string) {
    await notify(
      'error',
      t('resources.notifications.visibility.errorTitle'),
      t('resources.notifications.visibility.errorBody', { name, message }),
    )
  }

  async function notifyDeleteSuccess(name: string) {
    await notify(
      'success',
      t('resources.notifications.delete.successTitle'),
      t('resources.notifications.delete.successBody', { name }),
    )
  }

  async function notifyDeleteFailed(name: string, message: string) {
    await notify(
      'error',
      t('resources.notifications.delete.errorTitle'),
      t('resources.notifications.delete.errorBody', { name, message }),
    )
  }

  async function notifyUploadSuccess(kind: UploadKind, name: string, destination: string) {
    await notify(
      'success',
      t(`resources.notifications.upload.${kind}Title`),
      t(`resources.notifications.upload.${kind}Body`, { name, destination }),
    )
  }

  async function notifyUploadFailed(kind: UploadKind, message: string) {
    await notify(
      'error',
      t(`resources.notifications.upload.${kind}ErrorTitle`),
      t(`resources.notifications.upload.${kind}ErrorBody`, { message }),
    )
  }

  async function notifyPromoteSuccess(name: string, scope: ResourceScope) {
    await notify(
      'success',
      t('resources.notifications.promote.successTitle'),
      t('resources.notifications.promote.successBody', {
        name,
        scope: enumLabel('resourceScope', scope),
      }),
    )
  }

  async function notifyPromoteSubmitted(name: string) {
    await notify(
      'success',
      t('resources.notifications.promote.submittedTitle'),
      t('resources.notifications.promote.submittedBody', { name }),
    )
  }

  async function notifyPromoteFailed(name: string, message: string) {
    await notify(
      'error',
      t('resources.notifications.promote.errorTitle'),
      t('resources.notifications.promote.errorBody', { name, message }),
    )
  }

  return {
    notifyDeleteFailed,
    notifyDeleteSuccess,
    notifyPromoteFailed,
    notifyPromoteSubmitted,
    notifyPromoteSuccess,
    notifyStatusChanged,
    notifyStatusChangeFailed,
    notifyUploadFailed,
    notifyUploadSuccess,
    notifyVisibilityChanged,
    notifyVisibilityChangeFailed,
  }
}
