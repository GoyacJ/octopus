import { useI18n } from 'vue-i18n'

import { useNotificationStore } from '@/stores/notifications'
import { useWorkspaceStore } from '@/stores/workspace'

export function useWorkspaceModelNotifications() {
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
      source: 'workspace-models',
      toastDurationMs: 4000,
    })
  }

  async function notifyValidationSuccess(name: string, tokens: number) {
    await notify(
      'success',
      t('models.notifications.validation.successTitle'),
      t('models.notifications.validation.successBody', { name, tokens }),
    )
  }

  async function notifyValidationFailed(name: string, message: string) {
    await notify(
      'error',
      t('models.notifications.validation.errorTitle'),
      t('models.notifications.validation.errorBody', { name, message }),
    )
  }

  async function notifySaveSuccess(name: string) {
    await notify(
      'success',
      t('models.notifications.save.successTitle'),
      t('models.notifications.save.successBody', { name }),
    )
  }

  async function notifySaveFailed(name: string, message: string) {
    await notify(
      'error',
      t('models.notifications.save.errorTitle'),
      t('models.notifications.save.errorBody', { name, message }),
    )
  }

  async function notifyCreateSuccess(name: string) {
    await notify(
      'success',
      t('models.notifications.create.successTitle'),
      t('models.notifications.create.successBody', { name }),
    )
  }

  async function notifyCreateFailed(name: string, message: string) {
    await notify(
      'error',
      t('models.notifications.create.errorTitle'),
      t('models.notifications.create.errorBody', { name, message }),
    )
  }

  async function notifyDeleteSuccess(name: string) {
    await notify(
      'success',
      t('models.notifications.delete.successTitle'),
      t('models.notifications.delete.successBody', { name }),
    )
  }

  async function notifyDeleteWarning(name: string, message: string) {
    await notify(
      'warning',
      t('models.notifications.delete.warningTitle'),
      t('models.notifications.delete.warningBody', { name, message }),
    )
  }

  async function notifyDeleteFailed(name: string, message: string) {
    await notify(
      'error',
      t('models.notifications.delete.errorTitle'),
      t('models.notifications.delete.errorBody', { name, message }),
    )
  }

  return {
    notifyCreateFailed,
    notifyCreateSuccess,
    notifyDeleteFailed,
    notifyDeleteSuccess,
    notifyDeleteWarning,
    notifySaveFailed,
    notifySaveSuccess,
    notifyValidationFailed,
    notifyValidationSuccess,
  }
}
