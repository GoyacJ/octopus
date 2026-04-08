import { describe, expect, it } from 'vitest'

import {
  createDefaultNotificationUnreadSummary,
  normalizeNotificationListResponse,
  type NotificationRecord,
} from '@octopus/schema'

function createNotification(overrides: Partial<NotificationRecord> = {}): NotificationRecord {
  return {
    id: 'notif-1',
    scopeKind: 'app',
    level: 'info',
    title: 'Notification title',
    body: 'Notification body',
    source: 'test-suite',
    createdAt: 1,
    readAt: undefined,
    toastVisibleUntil: undefined,
    scopeOwnerId: undefined,
    routeTo: undefined,
    actionLabel: undefined,
    ...overrides,
  }
}

describe('notification schema', () => {
  it('creates an empty unread summary for every supported scope', () => {
    expect(createDefaultNotificationUnreadSummary()).toEqual({
      total: 0,
      byScope: {
        app: 0,
        workspace: 0,
        user: 0,
      },
    })
  })

  it('normalizes list payloads and derives unread counts from unread records', () => {
    const response = normalizeNotificationListResponse({
      notifications: [
        createNotification({
          id: 'notif-app',
          scopeKind: 'app',
        }),
        createNotification({
          id: 'notif-workspace',
          scopeKind: 'workspace',
          readAt: 20,
        }),
        createNotification({
          id: 'notif-user',
          scopeKind: 'user',
        }),
      ],
    })

    expect(response.notifications).toHaveLength(3)
    expect(response.unread.total).toBe(2)
    expect(response.unread.byScope).toEqual({
      app: 1,
      workspace: 0,
      user: 1,
    })
  })
})
