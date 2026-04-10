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

import { subscribeToNotifications, type NotificationSubscriber } from './notifications'
import { browserShellClient } from './shell_browser'
import { tauriShellClient } from './shell_tauri'
import { resolveHostRuntime } from './shared'

function resolveShellClient() {
  return resolveHostRuntime() === 'browser'
    ? browserShellClient
    : tauriShellClient
}

export async function bootstrapShellHost(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellBootstrap> {
  return await resolveShellClient().bootstrapShellHost(defaultWorkspaceId, defaultProjectId)
}

export async function loadPreferences(
  defaultWorkspaceId: string,
  defaultProjectId: string,
): Promise<ShellPreferences> {
  return await resolveShellClient().loadPreferences(defaultWorkspaceId, defaultProjectId)
}

export async function savePreferences(preferences: ShellPreferences): Promise<ShellPreferences> {
  return await resolveShellClient().savePreferences(preferences)
}

export async function getHostState(): Promise<HostState> {
  return await resolveShellClient().getHostState()
}

export async function getHostUpdateStatus(): Promise<HostUpdateStatus> {
  return await resolveShellClient().getHostUpdateStatus()
}

export async function checkHostUpdate(channel?: HostUpdateChannel): Promise<HostUpdateStatus> {
  return await resolveShellClient().checkHostUpdate(channel)
}

export async function downloadHostUpdate(): Promise<HostUpdateStatus> {
  return await resolveShellClient().downloadHostUpdate()
}

export async function installHostUpdate(): Promise<HostUpdateStatus> {
  return await resolveShellClient().installHostUpdate()
}

export async function healthcheck(): Promise<HealthcheckStatus> {
  return await resolveShellClient().healthcheck()
}

export async function restartDesktopBackend(): Promise<void> {
  await resolveShellClient().restartDesktopBackend()
}

export async function resolveDesktopBackendConnectionForHost(): Promise<HostBackendConnection | undefined> {
  return await resolveShellClient().resolveDesktopBackendConnection()
}

export async function pickAvatarImage(): Promise<AvatarUploadPayload | null> {
  return await resolveShellClient().pickAvatarImage()
}

export async function pickSkillArchive(): Promise<WorkspaceFileUploadPayload[] | null> {
  return await resolveShellClient().pickSkillArchive()
}

export async function pickSkillFolder(): Promise<WorkspaceDirectoryUploadEntry[][] | null> {
  return await resolveShellClient().pickSkillFolder()
}

export async function pickAgentBundleFolder(): Promise<WorkspaceDirectoryUploadEntry[] | null> {
  return await resolveShellClient().pickAgentBundleFolder()
}

export async function listWorkspaceConnections(): Promise<HostWorkspaceConnectionRecord[]> {
  return await resolveShellClient().listWorkspaceConnections()
}

export async function createWorkspaceConnection(
  input: CreateHostWorkspaceConnectionInput,
): Promise<HostWorkspaceConnectionRecord> {
  return await resolveShellClient().createWorkspaceConnection(input)
}

export async function deleteWorkspaceConnection(workspaceConnectionId: string): Promise<void> {
  return await resolveShellClient().deleteWorkspaceConnection(workspaceConnectionId)
}

export async function listNotifications(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationListResponse> {
  return await resolveShellClient().listNotifications(filter)
}

export async function createNotification(input: CreateNotificationInput): Promise<NotificationRecord> {
  return await resolveShellClient().createNotification(input)
}

export async function markNotificationRead(id: string): Promise<NotificationRecord> {
  return await resolveShellClient().markNotificationRead(id)
}

export async function markAllNotificationsRead(
  filter: NotificationFilter = { scope: 'all' },
): Promise<NotificationUnreadSummary> {
  return await resolveShellClient().markAllNotificationsRead(filter)
}

export async function dismissNotificationToast(id: string): Promise<NotificationRecord> {
  return await resolveShellClient().dismissNotificationToast(id)
}

export async function getNotificationUnreadSummary(): Promise<NotificationUnreadSummary> {
  return await resolveShellClient().getNotificationUnreadSummary()
}

export { subscribeToNotifications }
export type { NotificationSubscriber }

export const hostClient = {
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
