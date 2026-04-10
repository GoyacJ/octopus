import { invoke } from '@tauri-apps/api/core'

import type {
  AvatarUploadPayload,
  CreateNotificationInput,
  CreateHostWorkspaceConnectionInput,
  ExportWorkspaceAgentBundleResult,
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
import { isTauriRuntime } from './shared'
import { normalizeShellBootstrap } from './workspace_connections'

function assertTauriHostAvailable(): void {
  if (!isTauriRuntime()) {
    throw new Error('Tauri host runtime is unavailable')
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

async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  return normalizeShellBootstrap(
    await resolveDesktopShellBootstrap(),
    defaultWorkspaceId,
    defaultProjectId,
  )
}

async function loadPreferences(
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

async function savePreferences(preferences: ShellPreferences): Promise<ShellPreferences> {
  assertTauriHostAvailable()
  return normalizeShellPreferences(
    await invoke<ShellPreferences>('save_preferences', { preferences }),
    preferences.defaultWorkspaceId,
    extractProjectIdFromShellRoute(preferences.lastVisitedRoute),
  )
}

async function getHostState(): Promise<HostState> {
  assertTauriHostAvailable()
  return await invoke<HostState>('get_host_state')
}

async function getHostUpdateStatus(): Promise<HostUpdateStatus> {
  assertTauriHostAvailable()
  return normalizeHostUpdateStatus(await invoke<HostUpdateStatus>('get_host_update_status'))
}

async function checkHostUpdate(channel?: HostUpdateChannel): Promise<HostUpdateStatus> {
  assertTauriHostAvailable()
  return normalizeHostUpdateStatus(await invoke<HostUpdateStatus>('check_host_update', { channel }))
}

async function downloadHostUpdate(): Promise<HostUpdateStatus> {
  assertTauriHostAvailable()
  return normalizeHostUpdateStatus(await invoke<HostUpdateStatus>('download_host_update'))
}

async function installHostUpdate(): Promise<HostUpdateStatus> {
  assertTauriHostAvailable()
  return normalizeHostUpdateStatus(await invoke<HostUpdateStatus>('install_host_update'))
}

async function healthcheck(): Promise<HealthcheckStatus> {
  assertTauriHostAvailable()
  return await invoke<HealthcheckStatus>('healthcheck')
}

async function restartDesktopBackend(): Promise<void> {
  assertTauriHostAvailable()
  await invoke('restart_desktop_backend')
}

async function pickAvatarImage(): Promise<AvatarUploadPayload | null> {
  assertTauriHostAvailable()
  return await invoke<AvatarUploadPayload | null>('pick_avatar_image')
}

async function pickSkillArchive(): Promise<WorkspaceFileUploadPayload[] | null> {
  assertTauriHostAvailable()
  return await invoke<WorkspaceFileUploadPayload[] | null>('pick_skill_archive')
}

async function pickSkillFolder(): Promise<WorkspaceDirectoryUploadEntry[][] | null> {
  assertTauriHostAvailable()
  return await invoke<WorkspaceDirectoryUploadEntry[][] | null>('pick_skill_folder')
}

async function pickAgentBundleFolder(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  assertTauriHostAvailable()
  return await invoke<WorkspaceDirectoryUploadEntry[] | null>('pick_agent_bundle_folder')
}

async function pickAgentBundleArchive(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  assertTauriHostAvailable()
  return await invoke<WorkspaceDirectoryUploadEntry[] | null>('pick_agent_bundle_archive')
}

async function saveAgentBundleExport(
  exportPayload: ExportWorkspaceAgentBundleResult,
  format: 'folder' | 'zip',
): Promise<void> {
  assertTauriHostAvailable()
  await invoke(format === 'folder' ? 'save_agent_bundle_folder' : 'save_agent_bundle_zip', {
    exportPayload,
  })
}

async function listWorkspaceConnections(): Promise<HostWorkspaceConnectionRecord[]> {
  assertTauriHostAvailable()
  return await invoke<HostWorkspaceConnectionRecord[]>('list_workspace_connections')
}

async function createWorkspaceConnection(
  input: CreateHostWorkspaceConnectionInput,
): Promise<HostWorkspaceConnectionRecord> {
  assertTauriHostAvailable()
  return await invoke<HostWorkspaceConnectionRecord>('create_workspace_connection', { input })
}

async function deleteWorkspaceConnection(workspaceConnectionId: string): Promise<void> {
  assertTauriHostAvailable()
  await invoke('delete_workspace_connection', { workspaceConnectionId })
}

async function listNotifications(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationListResponse> {
  assertTauriHostAvailable()
  return normalizeNotificationListResponse(
    await invoke<NotificationListResponse>('list_notifications', { filter }),
  )
}

async function createNotification(input: CreateNotificationInput): Promise<NotificationRecord> {
  assertTauriHostAvailable()
  const notification = await invoke<NotificationRecord>('create_notification', { input })
  emitNotification(notification)
  return notification
}

async function markNotificationRead(id: string): Promise<NotificationRecord> {
  assertTauriHostAvailable()
  return await invoke<NotificationRecord>('mark_notification_read', { id })
}

async function markAllNotificationsRead(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationUnreadSummary> {
  assertTauriHostAvailable()
  return await invoke<NotificationUnreadSummary>('mark_all_notifications_read', { filter })
}

async function dismissNotificationToast(id: string): Promise<NotificationRecord> {
  assertTauriHostAvailable()
  return await invoke<NotificationRecord>('dismiss_notification_toast', { id })
}

async function getNotificationUnreadSummary(): Promise<NotificationUnreadSummary> {
  assertTauriHostAvailable()
  return await invoke<NotificationUnreadSummary>('get_notification_unread_summary')
}

export const tauriShellClient = {
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
  pickAgentBundleArchive,
  pickSkillArchive,
  pickSkillFolder,
  pickAgentBundleFolder,
  saveAgentBundleExport,
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
