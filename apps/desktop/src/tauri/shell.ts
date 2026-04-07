import { invoke } from '@tauri-apps/api/core'

import type {
  AvatarUploadPayload,
  ConnectionProfile,
  CreateNotificationInput,
  CreateHostWorkspaceConnectionInput,
  HealthcheckStatus,
  HostWorkspaceConnectionRecord,
  HostBackendConnection,
  HostState,
  NotificationFilter,
  NotificationListResponse,
  NotificationRecord,
  NotificationUnreadSummary,
  ShellBootstrap,
  ShellPreferences,
  TransportSecurityLevel,
  WorkspaceDirectoryUploadEntry,
  WorkspaceFileUploadPayload,
  WorkspaceConnectionRecord,
} from '@octopus/schema'
import {
  extractProjectIdFromShellRoute,
  normalizeNotificationListResponse,
  normalizeShellPreferences,
} from '@octopus/schema'

import {
  fetchHostApi,
  isTauriRuntime,
  resolveBrowserHostApiBaseUrl,
  resolveBrowserHostAuthToken,
  resolveHostRuntime,
} from './shared'

function assertTauriHostAvailable(): void {
  if (!isTauriRuntime()) {
    throw new Error('Tauri host runtime is unavailable')
  }
}

type NotificationSubscriber = (notification: NotificationRecord) => void

const notificationSubscribers = new Set<NotificationSubscriber>()

function emitNotification(notification: NotificationRecord): void {
  for (const subscriber of notificationSubscribers) {
    subscriber(notification)
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

function resolveTransportSecurity(mode: ConnectionProfile['mode']): TransportSecurityLevel {
  switch (mode) {
    case 'local':
      return 'loopback'
    case 'shared':
      return 'trusted'
    case 'remote':
      return 'public'
    default:
      return 'trusted'
  }
}

function resolveWorkspaceConnectionStatus(
  connection: ConnectionProfile,
  backend?: HostBackendConnection,
): WorkspaceConnectionRecord['status'] {
  if (connection.mode === 'local') {
    if (backend?.state === 'ready') {
      return 'connected'
    }

    return backend?.state === 'unavailable' ? 'unreachable' : 'disconnected'
  }

  switch (connection.state) {
    case 'connected':
    case 'local-ready':
      return 'connected'
    case 'disconnected':
      return 'disconnected'
    default:
      return 'disconnected'
  }
}

function resolveWorkspaceConnectionBaseUrl(
  connection: ConnectionProfile,
  backend?: HostBackendConnection,
): string {
  if (connection.baseUrl) {
    return connection.baseUrl
  }

  if (connection.mode === 'local' && backend?.baseUrl) {
    return backend.baseUrl
  }

  return 'http://127.0.0.1'
}

function toWorkspaceConnectionRecord(
  connection: ConnectionProfile,
  backend?: HostBackendConnection,
): WorkspaceConnectionRecord {
  return {
    workspaceConnectionId: connection.id,
    workspaceId: connection.workspaceId,
    label: connection.label,
    baseUrl: resolveWorkspaceConnectionBaseUrl(connection, backend),
    transportSecurity: resolveTransportSecurity(connection.mode),
    authMode: 'session-token',
    lastUsedAt: connection.lastSyncAt,
    status: resolveWorkspaceConnectionStatus(connection, backend),
  }
}

function resolveWorkspaceConnections(
  connections: ConnectionProfile[],
  backend?: HostBackendConnection,
): WorkspaceConnectionRecord[] {
  return connections.map((connection) => toWorkspaceConnectionRecord(connection, backend))
}

function normalizeShellBootstrap(
  payload: ShellBootstrap,
  defaultWorkspaceId: string,
  defaultProjectId: string,
): ShellBootstrap {
  const preferences = normalizeShellPreferences(
    payload.preferences,
    defaultWorkspaceId,
    defaultProjectId,
  )

  return {
    ...payload,
    preferences,
    workspaceConnections: resolveWorkspaceConnections(
      payload.connections ?? [],
      payload.backend,
    ),
  }
}

async function resolveDesktopShellBootstrap(): Promise<ShellBootstrap> {
  assertTauriHostAvailable()
  return await invoke<ShellBootstrap>('bootstrap_shell')
}

async function resolveDesktopBackendConnection(): Promise<HostBackendConnection | undefined> {
  assertTauriHostAvailable()
  return await invoke<HostBackendConnection>('get_backend_connection')
}

async function bootstrapShellHostTauri(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  return normalizeShellBootstrap(
    await resolveDesktopShellBootstrap(),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function loadPreferencesTauri(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  assertTauriHostAvailable()
  return normalizeShellPreferences(
    await invoke<ShellPreferences>('load_preferences'),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function savePreferencesTauri(
  preferences: ShellPreferences,
): Promise<ShellPreferences> {
  assertTauriHostAvailable()
  return normalizeShellPreferences(
    await invoke<ShellPreferences>('save_preferences', { preferences }),
    preferences.defaultWorkspaceId,
    extractProjectIdFromShellRoute(preferences.lastVisitedRoute),
  )
}

async function getHostStateTauri(): Promise<HostState> {
  assertTauriHostAvailable()
  return await invoke<HostState>('get_host_state')
}

async function healthcheckTauri(): Promise<HealthcheckStatus> {
  assertTauriHostAvailable()
  return await invoke<HealthcheckStatus>('healthcheck')
}

async function restartDesktopBackendTauri(): Promise<void> {
  assertTauriHostAvailable()
  await invoke('restart_desktop_backend')
}

async function pickAvatarImageTauri(): Promise<AvatarUploadPayload | null> {
  assertTauriHostAvailable()
  return await invoke<AvatarUploadPayload | null>('pick_avatar_image')
}

async function pickSkillArchiveTauri(): Promise<WorkspaceFileUploadPayload[] | null> {
  assertTauriHostAvailable()
  return await invoke<WorkspaceFileUploadPayload[] | null>('pick_skill_archive')
}

async function pickSkillFolderTauri(): Promise<WorkspaceDirectoryUploadEntry[][] | null> {
  assertTauriHostAvailable()
  return await invoke<WorkspaceDirectoryUploadEntry[][] | null>('pick_skill_folder')
}

async function pickAgentBundleFolderTauri(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  assertTauriHostAvailable()
  return await invoke<WorkspaceDirectoryUploadEntry[] | null>('pick_agent_bundle_folder')
}

async function listWorkspaceConnectionsTauri(): Promise<HostWorkspaceConnectionRecord[]> {
  assertTauriHostAvailable()
  return await invoke<HostWorkspaceConnectionRecord[]>('list_workspace_connections')
}

async function createWorkspaceConnectionTauri(
  input: CreateHostWorkspaceConnectionInput,
): Promise<HostWorkspaceConnectionRecord> {
  assertTauriHostAvailable()
  return await invoke<HostWorkspaceConnectionRecord>('create_workspace_connection', { input })
}

async function deleteWorkspaceConnectionTauri(
  workspaceConnectionId: string,
): Promise<void> {
  assertTauriHostAvailable()
  await invoke('delete_workspace_connection', { workspaceConnectionId })
}

async function listNotificationsTauri(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationListResponse> {
  assertTauriHostAvailable()
  return normalizeNotificationListResponse(
    await invoke<NotificationListResponse>('list_notifications', { filter }),
  )
}

async function createNotificationTauri(
  input: CreateNotificationInput,
): Promise<NotificationRecord> {
  assertTauriHostAvailable()
  const notification = await invoke<NotificationRecord>('create_notification', { input })
  emitNotification(notification)
  return notification
}

async function markNotificationReadTauri(id: string): Promise<NotificationRecord> {
  assertTauriHostAvailable()
  return await invoke<NotificationRecord>('mark_notification_read', { id })
}

async function markAllNotificationsReadTauri(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationUnreadSummary> {
  assertTauriHostAvailable()
  return await invoke<NotificationUnreadSummary>('mark_all_notifications_read', { filter })
}

async function dismissNotificationToastTauri(id: string): Promise<NotificationRecord> {
  assertTauriHostAvailable()
  return await invoke<NotificationRecord>('dismiss_notification_toast', { id })
}

async function getNotificationUnreadSummaryTauri(): Promise<NotificationUnreadSummary> {
  assertTauriHostAvailable()
  return await invoke<NotificationUnreadSummary>('get_notification_unread_summary')
}

function resolveBrowserHostConfig(): { baseUrl: string, authToken: string } {
  return {
    baseUrl: resolveBrowserHostApiBaseUrl(),
    authToken: resolveBrowserHostAuthToken(),
  }
}

async function bootstrapShellHostBrowser(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  const payload = await fetchHostApi<ShellBootstrap>(
    baseUrl,
    authToken,
    '/api/v1/host/bootstrap',
    { method: 'GET' },
  )

  return normalizeShellBootstrap(payload, defaultWorkspaceId, defaultProjectId)
}

async function loadPreferencesBrowser(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeShellPreferences(
    await fetchHostApi<ShellPreferences>(
      baseUrl,
      authToken,
      '/api/v1/host/preferences',
      { method: 'GET' },
    ),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function savePreferencesBrowser(
  preferences: ShellPreferences,
): Promise<ShellPreferences> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return normalizeShellPreferences(
    await fetchHostApi<ShellPreferences>(
      baseUrl,
      authToken,
      '/api/v1/host/preferences',
      {
        method: 'PUT',
        body: JSON.stringify(preferences),
      },
    ),
    preferences.defaultWorkspaceId,
    extractProjectIdFromShellRoute(preferences.lastVisitedRoute),
  )
}

async function getHostStateBrowser(
  defaultWorkspaceId = 'ws-local',
  defaultProjectId = 'proj-redesign',
): Promise<HostState> {
  return (await bootstrapShellHostBrowser(defaultWorkspaceId, defaultProjectId)).hostState
}

async function healthcheckBrowser(): Promise<HealthcheckStatus> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostApi<HealthcheckStatus>(
    baseUrl,
    authToken,
    '/api/v1/host/health',
    { method: 'GET' },
  )
}

async function restartDesktopBackendBrowser(): Promise<void> {
  throw new Error('Browser host runtime does not support desktop backend restart')
}

async function resolveDesktopBackendConnectionBrowser(
  defaultWorkspaceId = 'ws-local',
  defaultProjectId = 'proj-redesign',
): Promise<HostBackendConnection | undefined> {
  return (await bootstrapShellHostBrowser(defaultWorkspaceId, defaultProjectId)).backend
}

async function pickAvatarImageBrowser(): Promise<AvatarUploadPayload | null> {
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

async function pickSkillArchiveBrowser(): Promise<WorkspaceFileUploadPayload[] | null> {
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

async function pickSkillFolderBrowser(): Promise<WorkspaceDirectoryUploadEntry[][] | null> {
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

async function pickAgentBundleFolderBrowser(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
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
      const rootPrefix = topLevelNames.size === 1
        ? `${[...topLevelNames][0]}/`
        : ''

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

async function listWorkspaceConnectionsBrowser(): Promise<HostWorkspaceConnectionRecord[]> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostApi<HostWorkspaceConnectionRecord[]>(
    baseUrl,
    authToken,
    '/api/v1/host/workspace-connections',
    { method: 'GET' },
  )
}

async function createWorkspaceConnectionBrowser(
  input: CreateHostWorkspaceConnectionInput,
): Promise<HostWorkspaceConnectionRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostApi<HostWorkspaceConnectionRecord>(
    baseUrl,
    authToken,
    '/api/v1/host/workspace-connections',
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
  )
}

async function deleteWorkspaceConnectionBrowser(
  workspaceConnectionId: string,
): Promise<void> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  await fetchHostApi<null>(
    baseUrl,
    authToken,
    `/api/v1/host/workspace-connections/${workspaceConnectionId}`,
    { method: 'DELETE' },
  )
}

async function listNotificationsBrowser(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationListResponse> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  const search = new URLSearchParams()
  if (filter.scope) {
    search.set('scope', filter.scope)
  }

  return normalizeNotificationListResponse(
    await fetchHostApi<NotificationListResponse>(
      baseUrl,
      authToken,
      `/api/v1/host/notifications${search.size ? `?${search.toString()}` : ''}`,
      { method: 'GET' },
    ),
  )
}

async function createNotificationBrowser(
  input: CreateNotificationInput,
): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  const notification = await fetchHostApi<NotificationRecord>(
    baseUrl,
    authToken,
    '/api/v1/host/notifications',
    {
      method: 'POST',
      body: JSON.stringify(input),
    },
  )
  emitNotification(notification)
  return notification
}

async function markNotificationReadBrowser(id: string): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostApi<NotificationRecord>(
    baseUrl,
    authToken,
    `/api/v1/host/notifications/${id}/read`,
    { method: 'POST' },
  )
}

async function markAllNotificationsReadBrowser(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationUnreadSummary> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostApi<NotificationUnreadSummary>(
    baseUrl,
    authToken,
    '/api/v1/host/notifications/read-all',
    {
      method: 'POST',
      body: JSON.stringify(filter),
    },
  )
}

async function dismissNotificationToastBrowser(id: string): Promise<NotificationRecord> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostApi<NotificationRecord>(
    baseUrl,
    authToken,
    `/api/v1/host/notifications/${id}/dismiss-toast`,
    { method: 'POST' },
  )
}

async function getNotificationUnreadSummaryBrowser(): Promise<NotificationUnreadSummary> {
  const { baseUrl, authToken } = resolveBrowserHostConfig()
  return await fetchHostApi<NotificationUnreadSummary>(
    baseUrl,
    authToken,
    '/api/v1/host/notifications/unread-summary',
    { method: 'GET' },
  )
}

export async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  return resolveHostRuntime() === 'browser'
    ? await bootstrapShellHostBrowser(defaultWorkspaceId, defaultProjectId)
    : await bootstrapShellHostTauri(defaultWorkspaceId, defaultProjectId)
}

export async function loadPreferences(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  return resolveHostRuntime() === 'browser'
    ? await loadPreferencesBrowser(defaultWorkspaceId, defaultProjectId)
    : await loadPreferencesTauri(defaultWorkspaceId, defaultProjectId)
}

export async function savePreferences(
  preferences: ShellPreferences,
): Promise<ShellPreferences> {
  return resolveHostRuntime() === 'browser'
    ? await savePreferencesBrowser(preferences)
    : await savePreferencesTauri(preferences)
}

export async function getHostState(): Promise<HostState> {
  return resolveHostRuntime() === 'browser'
    ? await getHostStateBrowser()
    : await getHostStateTauri()
}

export async function healthcheck(): Promise<HealthcheckStatus> {
  return resolveHostRuntime() === 'browser'
    ? await healthcheckBrowser()
    : await healthcheckTauri()
}

export async function restartDesktopBackend(): Promise<void> {
  if (resolveHostRuntime() === 'browser') {
    await restartDesktopBackendBrowser()
    return
  }

  await restartDesktopBackendTauri()
}

export async function resolveDesktopBackendConnectionForHost(): Promise<HostBackendConnection | undefined> {
  return resolveHostRuntime() === 'browser'
    ? await resolveDesktopBackendConnectionBrowser()
    : await resolveDesktopBackendConnection()
}

export async function pickAvatarImage(): Promise<AvatarUploadPayload | null> {
  return resolveHostRuntime() === 'browser'
    ? await pickAvatarImageBrowser()
    : await pickAvatarImageTauri()
}

export async function pickSkillArchive(): Promise<WorkspaceFileUploadPayload[] | null> {
  return resolveHostRuntime() === 'browser'
    ? await pickSkillArchiveBrowser()
    : await pickSkillArchiveTauri()
}

export async function pickSkillFolder(): Promise<WorkspaceDirectoryUploadEntry[][] | null> {
  return resolveHostRuntime() === 'browser'
    ? await pickSkillFolderBrowser()
    : await pickSkillFolderTauri()
}

export async function pickAgentBundleFolder(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  return resolveHostRuntime() === 'browser'
    ? await pickAgentBundleFolderBrowser()
    : await pickAgentBundleFolderTauri()
}

export async function listWorkspaceConnections(): Promise<HostWorkspaceConnectionRecord[]> {
  return resolveHostRuntime() === 'browser'
    ? await listWorkspaceConnectionsBrowser()
    : await listWorkspaceConnectionsTauri()
}

export async function createWorkspaceConnection(
  input: CreateHostWorkspaceConnectionInput,
): Promise<HostWorkspaceConnectionRecord> {
  return resolveHostRuntime() === 'browser'
    ? await createWorkspaceConnectionBrowser(input)
    : await createWorkspaceConnectionTauri(input)
}

export async function deleteWorkspaceConnection(
  workspaceConnectionId: string,
): Promise<void> {
  return resolveHostRuntime() === 'browser'
    ? await deleteWorkspaceConnectionBrowser(workspaceConnectionId)
    : await deleteWorkspaceConnectionTauri(workspaceConnectionId)
}

export async function listNotifications(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationListResponse> {
  return resolveHostRuntime() === 'browser'
    ? await listNotificationsBrowser(filter)
    : await listNotificationsTauri(filter)
}

export async function createNotification(
  input: CreateNotificationInput,
): Promise<NotificationRecord> {
  return resolveHostRuntime() === 'browser'
    ? await createNotificationBrowser(input)
    : await createNotificationTauri(input)
}

export async function markNotificationRead(id: string): Promise<NotificationRecord> {
  return resolveHostRuntime() === 'browser'
    ? await markNotificationReadBrowser(id)
    : await markNotificationReadTauri(id)
}

export async function markAllNotificationsRead(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationUnreadSummary> {
  return resolveHostRuntime() === 'browser'
    ? await markAllNotificationsReadBrowser(filter)
    : await markAllNotificationsReadTauri(filter)
}

export async function dismissNotificationToast(id: string): Promise<NotificationRecord> {
  return resolveHostRuntime() === 'browser'
    ? await dismissNotificationToastBrowser(id)
    : await dismissNotificationToastTauri(id)
}

export async function getNotificationUnreadSummary(): Promise<NotificationUnreadSummary> {
  return resolveHostRuntime() === 'browser'
    ? await getNotificationUnreadSummaryBrowser()
    : await getNotificationUnreadSummaryTauri()
}

export function subscribeToNotifications(
  subscriber: NotificationSubscriber,
): () => void {
  notificationSubscribers.add(subscriber)
  return () => {
    notificationSubscribers.delete(subscriber)
  }
}

export const hostClient = {
  bootstrapShellHost,
  loadPreferences,
  savePreferences,
  getHostState,
  healthcheck,
  restartDesktopBackend,
  resolveDesktopBackendConnection: resolveDesktopBackendConnectionForHost,
  pickAvatarImage,
  pickAgentBundleFolder,
  pickSkillArchive,
  pickSkillFolder,
  listWorkspaceConnections,
  createWorkspaceConnection,
  deleteWorkspaceConnection,
  listNotifications,
  createNotification,
  markNotificationRead,
  markAllNotificationsRead,
  dismissNotificationToast,
  getNotificationUnreadSummary,
  subscribeToNotifications,
}
