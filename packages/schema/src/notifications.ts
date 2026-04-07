export type NotificationScopeKind = 'app' | 'workspace' | 'user'
export type NotificationLevel = 'info' | 'success' | 'warning' | 'error'
export type NotificationFilterScope = 'all' | NotificationScopeKind

export interface NotificationRecord {
  id: string
  scopeKind: NotificationScopeKind
  scopeOwnerId?: string
  level: NotificationLevel
  title: string
  body: string
  source: string
  createdAt: number
  readAt?: number
  toastVisibleUntil?: number
  routeTo?: string
  actionLabel?: string
}

export interface CreateNotificationInput {
  scopeKind: NotificationScopeKind
  scopeOwnerId?: string
  level?: NotificationLevel
  title?: string
  body?: string
  source?: string
  toastDurationMs?: number
  routeTo?: string
  actionLabel?: string
}

export interface NotificationFilter {
  scope?: NotificationFilterScope
}

export interface NotificationUnreadSummary {
  total: number
  byScope: Record<NotificationScopeKind, number>
}

export interface NotificationListResponse {
  notifications: NotificationRecord[]
  unread: NotificationUnreadSummary
}

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
