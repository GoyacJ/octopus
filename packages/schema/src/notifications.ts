import type {
  CreateNotificationInput as OpenApiCreateNotificationInput,
  NotificationFilter as OpenApiNotificationFilter,
  NotificationFilterScope as OpenApiNotificationFilterScope,
  NotificationLevel as OpenApiNotificationLevel,
  NotificationListResponse as OpenApiNotificationListResponse,
  NotificationRecord as OpenApiNotificationRecord,
  NotificationScopeKind as OpenApiNotificationScopeKind,
  NotificationUnreadSummary as OpenApiNotificationUnreadSummary,
} from './generated'

export type NotificationScopeKind = OpenApiNotificationScopeKind
export type NotificationLevel = OpenApiNotificationLevel
export type NotificationFilterScope = OpenApiNotificationFilterScope
export type NotificationRecord = OpenApiNotificationRecord
export type CreateNotificationInput = OpenApiCreateNotificationInput
export type NotificationFilter = OpenApiNotificationFilter
export type NotificationUnreadSummary = OpenApiNotificationUnreadSummary
export type NotificationListResponse = OpenApiNotificationListResponse

const NOTIFICATION_SCOPES: NotificationScopeKind[] = ['app', 'workspace', 'user']

export function createDefaultNotificationUnreadSummary(): NotificationUnreadSummary {
  return {
    total: 0,
    byScope: {
      app: 0,
      workspace: 0,
      user: 0,
    },
  }
}

export function normalizeNotificationRecord(
  notification: NotificationRecord,
): NotificationRecord {
  return {
    ...notification,
    level: notification.level ?? 'info',
    title: notification.title || 'Notification',
    body: notification.body || '',
    source: notification.source || 'system',
  }
}

export function normalizeNotificationListResponse(
  response?: Partial<NotificationListResponse> | null,
): NotificationListResponse {
  const notifications = (response?.notifications ?? []).map(normalizeNotificationRecord)
  const unread = createDefaultNotificationUnreadSummary()

  for (const notification of notifications) {
    if (notification.readAt) {
      continue
    }

    unread.total += 1
    unread.byScope[notification.scopeKind] += 1
  }

  const incomingUnread = response?.unread
  if (!incomingUnread) {
    return {
      notifications,
      unread,
    }
  }

  return {
    notifications,
    unread: {
      total: typeof incomingUnread.total === 'number' ? incomingUnread.total : unread.total,
      byScope: NOTIFICATION_SCOPES.reduce<Record<NotificationScopeKind, number>>((acc, scope) => {
        acc[scope] = incomingUnread.byScope?.[scope] ?? unread.byScope[scope]
        return acc
      }, {
        app: 0,
        workspace: 0,
        user: 0,
      }),
    },
  }
}
