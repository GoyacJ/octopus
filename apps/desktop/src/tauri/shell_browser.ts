import type {
  AvatarUploadPayload,
  CreateNotificationInput,
  CreateHostWorkspaceConnectionInput,
  HealthcheckStatus,
  HostBackendConnection,
  HostState,
  HostUpdateChannel,
  HostUpdateStatus,
  HostWorkspaceConnectionRecord,
  NotificationFilter,
  NotificationListResponse,
  NotificationRecord,
  NotificationUnreadSummary,
  ShellBootstrap,
  ShellPreferences,
  WorkspaceDirectoryUploadEntry,
  WorkspaceFileUploadPayload,
} from '@octopus/schema'
import {
  extractProjectIdFromShellRoute,
  normalizeHostUpdateStatus,
  normalizeNotificationListResponse,
  normalizeShellPreferences,
} from '@octopus/schema'

import { emitNotification } from './notifications'
import {
  fetchHostOpenApi,
  resolveBrowserHostApiBaseUrl,
  resolveBrowserHostAuthToken,
} from './shared'
import { normalizeShellBootstrap } from './workspace_connections'

function resolveBrowserHostConfig(): { baseUrl: string, authToken: string } {
  return {
    baseUrl: resolveBrowserHostApiBaseUrl(),
    authToken: resolveBrowserHostAuthToken(),
  }
}

async function readBrowserFile(file: File): Promise<WorkspaceFileUploadPayload> {
  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onerror = () => reject(reader.error ?? new Error('Failed to read avatar file'))
    reader.onload = () => resolve(String(reader.result ?? ''))
    reader.readAsDataURL(file)
  })

  return {
    fileName: file.name,
    contentType: file.type || 'application/octet-stream',
    byteSize: file.size,
    dataBase64: dataUrl.split(',', 2)[1] ?? '',
  }
}

async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  const payload = await fetchHostOpenApi(
    baseUrl,
    authToken,
    '/api/v1/host/bootstrap',
    'get',
  )

  return normalizeShellBootstrap(payload, defaultWorkspaceId, defaultProjectId)
}

async function loadPreferences(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeShellPreferences(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/preferences', 'get'),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function savePreferences(preferences: ShellPreferences): Promise<ShellPreferences> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeShellPreferences(
    await fetchHostOpenApi(
      baseUrl,
      authToken,
      '/api/v1/host/preferences',
      'put',
      { body: JSON.stringify(preferences) },
    ),
    preferences.defaultWorkspaceId,
    extractProjectIdFromShellRoute(preferences.lastVisitedRoute),
  )
}

async function getHostState(
  defaultWorkspaceId = 'ws-local',
  defaultProjectId = 'proj-redesign',
): Promise<HostState> {
  return (await bootstrapShellHost(defaultWorkspaceId, defaultProjectId)).hostState
}

async function healthcheck(): Promise<HealthcheckStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/health', 'get')
}

async function getHostUpdateStatus(): Promise<HostUpdateStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeHostUpdateStatus(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/update-status', 'get'),
  )
}

async function checkHostUpdate(channel?: HostUpdateChannel): Promise<HostUpdateStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeHostUpdateStatus(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/update-check', 'post', {
      body: JSON.stringify(channel ? { channel } : {}),
    }),
  )
}

async function downloadHostUpdate(): Promise<HostUpdateStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeHostUpdateStatus(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/update-download', 'post'),
  )
}

async function installHostUpdate(): Promise<HostUpdateStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeHostUpdateStatus(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/update-install', 'post'),
  )
}

async function restartDesktopBackend(): Promise<void> {
  throw new Error('Browser host runtime does not support desktop backend restart')
}

async function resolveDesktopBackendConnection(
  defaultWorkspaceId = 'ws-local',
  defaultProjectId = 'proj-redesign',
): Promise<HostBackendConnection | undefined> {
  return (await bootstrapShellHost(defaultWorkspaceId, defaultProjectId)).backend
}

async function pickAvatarImage(): Promise<AvatarUploadPayload | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = 'image/png,image/jpeg,image/webp'

  return await new Promise<AvatarUploadPayload | null>((resolve) => {
    input.addEventListener('change', async () => {
      const file = input.files?.[0]
      if (!file) {
        resolve(null)
        return
      }

      resolve(await readBrowserFile(file))
    }, { once: true })
    input.click()
  })
}

async function pickSkillArchive(): Promise<WorkspaceFileUploadPayload[] | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.zip,application/zip'
  input.multiple = true

  return await new Promise<WorkspaceFileUploadPayload[] | null>((resolve) => {
    input.addEventListener('change', async () => {
      const files = Array.from(input.files ?? [])
      if (!files.length) {
        resolve(null)
        return
      }

      resolve(await Promise.all(files.map(readBrowserFile)))
    }, { once: true })
    input.click()
  })
}

async function pickSkillFolder(): Promise<WorkspaceDirectoryUploadEntry[][] | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.setAttribute('webkitdirectory', 'true')
  input.multiple = true

  return await new Promise<WorkspaceDirectoryUploadEntry[][] | null>((resolve) => {
    input.addEventListener('change', async () => {
      const files = Array.from(input.files ?? [])
      if (!files.length) {
        resolve(null)
        return
      }

      const payloads = await Promise.all(files.map(async (file) => ({
        ...(await readBrowserFile(file)),
        relativePath: (file.webkitRelativePath || file.name).replace(/\\/g, '/'),
      })))
      const grouped = new Map<string, WorkspaceDirectoryUploadEntry[]>()
      for (const payload of payloads) {
        const groupKey = payload.relativePath.split('/')[0] ?? payload.fileName
        grouped.set(groupKey, [...(grouped.get(groupKey) ?? []), payload])
      }
      resolve(Array.from(grouped.values()))
    }, { once: true })
    input.click()
  })
}

async function pickAgentBundleFolder(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  const input = document.createElement('input')
  input.type = 'file'
  input.setAttribute('webkitdirectory', 'true')
  input.multiple = true

  return await new Promise<WorkspaceDirectoryUploadEntry[] | null>((resolve) => {
    input.addEventListener('change', async () => {
      const files = Array.from(input.files ?? [])
      if (!files.length) {
        resolve(null)
        return
      }

      const payloads = await Promise.all(files.map(async (file) => ({
        ...(await readBrowserFile(file)),
        relativePath: (file.webkitRelativePath || file.name).replace(/\\/g, '/'),
      })))
      const topLevelNames = new Set(
        payloads
          .map(file => file.relativePath.split('/')[0])
          .filter((value): value is string => Boolean(value)),
      )
      const rootPrefix = topLevelNames.size === 1 ? `${[...topLevelNames][0]}/` : ''

      resolve(payloads.map(file => ({
        ...file,
        relativePath: rootPrefix && file.relativePath.startsWith(rootPrefix)
          ? file.relativePath.slice(rootPrefix.length)
          : file.relativePath,
      })))
    }, { once: true })
    input.click()
  })
}

async function listWorkspaceConnections(): Promise<HostWorkspaceConnectionRecord[]> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/workspace-connections', 'get')
}

async function createWorkspaceConnection(
  input: CreateHostWorkspaceConnectionInput,
): Promise<HostWorkspaceConnectionRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/workspace-connections', 'post', {
    body: JSON.stringify(input),
  })
}

async function deleteWorkspaceConnection(workspaceConnectionId: string): Promise<void> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  await fetchHostOpenApi(
    baseUrl,
    authToken,
    '/api/v1/host/workspace-connections/{connectionId}',
    'delete',
    {
      pathParams: {
        connectionId: workspaceConnectionId,
      },
    },
  )
}

async function listNotifications(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationListResponse> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeNotificationListResponse(
    await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/notifications', 'get', {
      queryParams: {
        scope: filter.scope,
      },
    }),
  )
}

async function createNotification(input: CreateNotificationInput): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  const notification = await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/notifications', 'post', {
    body: JSON.stringify(input),
  })
  emitNotification(notification)
  return notification
}

async function markNotificationRead(id: string): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(
    baseUrl,
    authToken,
    '/api/v1/host/notifications/{notificationId}/read',
    'post',
    {
      pathParams: {
        notificationId: id,
      },
    },
  )
}

async function markAllNotificationsRead(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationUnreadSummary> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/notifications/read-all', 'post', {
    body: JSON.stringify(filter),
  })
}

async function dismissNotificationToast(id: string): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(
    baseUrl,
    authToken,
    '/api/v1/host/notifications/{notificationId}/dismiss-toast',
    'post',
    {
      pathParams: {
        notificationId: id,
      },
    },
  )
}

async function getNotificationUnreadSummary(): Promise<NotificationUnreadSummary> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostOpenApi(baseUrl, authToken, '/api/v1/host/notifications/unread-summary', 'get')
}

export const browserShellClient = {
  bootstrapShellHost,
  loadPreferences,
  savePreferences,
  getHostState,
  getHostUpdateStatus,
  checkHostUpdate,
  downloadHostUpdate,
  installHostUpdate,
  healthcheck,
  restartDesktopBackend,
  resolveDesktopBackendConnection,
  pickAvatarImage,
  pickSkillArchive,
  pickSkillFolder,
  pickAgentBundleFolder,
  listWorkspaceConnections,
  createWorkspaceConnection,
  deleteWorkspaceConnection,
  listNotifications,
  createNotification,
  markNotificationRead,
  markAllNotificationsRead,
  dismissNotificationToast,
  getNotificationUnreadSummary,
}
