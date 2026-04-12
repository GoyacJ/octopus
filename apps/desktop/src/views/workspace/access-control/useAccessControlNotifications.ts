import { useNotificationStore } from '@/stores/notifications'
import { useWorkspaceStore } from '@/stores/workspace'

export function useAccessControlNotifications(source: string) {
  const notificationStore = useNotificationStore()
  const workspaceStore = useWorkspaceStore()

  async function notify(level: 'success' | 'warning' | 'error', title: string, body?: string) {
    await notificationStore.notify({
      scopeKind: 'workspace',
      scopeOwnerId: workspaceStore.currentWorkspaceId || undefined,
      level,
      title,
      body,
      source,
      toastDurationMs: 4000,
    })
  }

  async function notifySuccess(title: string, body?: string) {
    await notify('success', title, body)
  }

  async function notifyWarning(title: string, body?: string) {
    await notify('warning', title, body)
  }

  async function notifyError(title: string, body?: string) {
    await notify('error', title, body)
  }

  return {
    notifyError,
    notifySuccess,
    notifyWarning,
  }
}
