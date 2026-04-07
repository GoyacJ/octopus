export {
  bootstrapShellHost,
  getHostState,
  healthcheck,
  hostClient,
  loadPreferences,
  pickAvatarImage,
  pickSkillArchive,
  pickSkillFolder,
  restartDesktopBackend,
  savePreferences,
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
} from './shell'

export {
  createIdempotencyKey,
  createWorkspaceClient,
} from './workspace-client'
