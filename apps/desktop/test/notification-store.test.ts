// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import type {
  CreateNotificationInput,
  NotificationListResponse,
  NotificationRecord,
  NotificationUnreadSummary,
} from '@octopus/schema'

vi.mock('@/tauri/client', async () => {
  const actual = await vi.importActual<typeof import('@/tauri/client')>('@/tauri/client')
  return {
    ...actual,
    createNotification: vi.fn(),
    dismissNotificationToast: vi.fn(),
    listNotifications: vi.fn(),
    markAllNotificationsRead: vi.fn(),
    markNotificationRead: vi.fn(),
    subscribeToNotifications: vi.fn(),
  }
})

import {
  createNotification,
  dismissNotificationToast,
  listNotifications,
  markAllNotificationsRead,
  markNotificationRead,
  subscribeToNotifications,
} from '@/tauri/client'
import { useNotificationStore } from '@/stores/notifications'

const createNotificationMock = vi.mocked(createNotification)
const dismissNotificationToastMock = vi.mocked(dismissNotificationToast)
const listNotificationsMock = vi.mocked(listNotifications)
const markAllNotificationsReadMock = vi.mocked(markAllNotificationsRead)
const markNotificationReadMock = vi.mocked(markNotificationRead)
const subscribeToNotificationsMock = vi.mocked(subscribeToNotifications)

function createSummary(overrides: Partial<NotificationUnreadSummary> = {}): NotificationUnreadSummary {
  return {
    total: 0,
    byScope: {
      app: 0,
      workspace: 0,
      user: 0,
    },
    ...overrides,
  }
}

function createRecord(overrides: Partial<NotificationRecord> = {}): NotificationRecord {
  return {
    id: 'notif-1',
    scopeKind: 'app',
    level: 'info',
    title: 'Saved',
    body: 'Preferences updated.',
    source: 'settings',
    createdAt: 1,
    readAt: undefined,
    toastVisibleUntil: Date.now() + 60_000,
    scopeOwnerId: undefined,
    routeTo: undefined,
    actionLabel: undefined,
    ...overrides,
  }
}

function createListResponse(overrides: Partial<NotificationListResponse> = {}): NotificationListResponse {
  return {
    notifications: [],
    unread: createSummary(),
    ...overrides,
  }
}

describe('useNotificationStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    createNotificationMock.mockReset()
    dismissNotificationToastMock.mockReset()
    listNotificationsMock.mockReset()
    markAllNotificationsReadMock.mockReset()
    markNotificationReadMock.mockReset()
    subscribeToNotificationsMock.mockReset()
  })

  it('bootstraps notification history and subscribes to host events once', async () => {
    const unsubscribe = vi.fn()
    listNotificationsMock.mockResolvedValue(createListResponse({
      notifications: [
        createRecord({
          id: 'notif-existing',
          scopeKind: 'workspace',
        }),
      ],
      unread: createSummary({
        total: 1,
        byScope: {
          app: 0,
          workspace: 1,
          user: 0,
        },
      }),
    }))
    subscribeToNotificationsMock.mockReturnValue(unsubscribe)

    const store = useNotificationStore()
    await store.bootstrap()

    expect(listNotificationsMock).toHaveBeenCalledWith({ scope: 'all' })
    expect(subscribeToNotificationsMock).toHaveBeenCalledTimes(1)
    expect(store.notifications).toHaveLength(1)
    expect(store.unreadSummary.total).toBe(1)
    expect(store.activeToasts).toHaveLength(1)

    await store.bootstrap()
    expect(subscribeToNotificationsMock).toHaveBeenCalledTimes(1)
  })

  it('creates notifications through the adapter and updates unread state', async () => {
    const created = createRecord({
      id: 'notif-created',
      scopeKind: 'user',
      level: 'success',
      toastVisibleUntil: Date.now() + 30_000,
    })
    createNotificationMock.mockResolvedValue(created)
    subscribeToNotificationsMock.mockReturnValue(() => {})
    listNotificationsMock.mockResolvedValue(createListResponse())

    const store = useNotificationStore()
    await store.bootstrap()

    const input: CreateNotificationInput = {
      scopeKind: 'user',
      scopeOwnerId: 'user-owner',
      level: 'success',
      title: 'Profile saved',
      body: 'Profile details were updated.',
      source: 'user-center',
      toastDurationMs: 30_000,
    }

    const result = await store.notify(input)

    expect(result.id).toBe('notif-created')
    expect(createNotificationMock).toHaveBeenCalledWith(input)
    expect(store.notifications[0]?.id).toBe('notif-created')
    expect(store.unreadSummary.total).toBe(1)
    expect(store.unreadSummary.byScope.user).toBe(1)
    expect(store.activeToasts[0]?.id).toBe('notif-created')
  })

  it('marks records read, marks all read by scope, dismisses toasts, and manages panel state', async () => {
    const existing = createRecord({
      id: 'notif-a',
      scopeKind: 'app',
      toastVisibleUntil: Date.now() + 30_000,
    })
    listNotificationsMock.mockResolvedValue(createListResponse({
      notifications: [existing],
      unread: createSummary({
        total: 1,
        byScope: {
          app: 1,
          workspace: 0,
          user: 0,
        },
      }),
    }))
    subscribeToNotificationsMock.mockReturnValue(() => {})
    markNotificationReadMock.mockResolvedValue(createRecord({
      id: 'notif-a',
      scopeKind: 'app',
      readAt: 99,
      toastVisibleUntil: Date.now() + 30_000,
    }))
    markAllNotificationsReadMock.mockResolvedValue(createSummary())
    dismissNotificationToastMock.mockResolvedValue(createRecord({
      id: 'notif-a',
      scopeKind: 'app',
      readAt: 99,
      toastVisibleUntil: 0,
    }))

    const store = useNotificationStore()
    await store.bootstrap()

    store.openCenter()
    store.setFilter('app')
    expect(store.centerOpen).toBe(true)
    expect(store.filterScope).toBe('app')

    await store.markRead('notif-a')
    expect(markNotificationReadMock).toHaveBeenCalledWith('notif-a')
    expect(store.notifications[0]?.readAt).toBe(99)

    await store.dismissToast('notif-a')
    expect(dismissNotificationToastMock).toHaveBeenCalledWith('notif-a')
    expect(store.activeToasts).toHaveLength(0)

    await store.markAllRead({ scope: 'app' })
    expect(markAllNotificationsReadMock).toHaveBeenCalledWith({ scope: 'app' })
    expect(store.unreadSummary.total).toBe(0)

    store.closeCenter()
    expect(store.centerOpen).toBe(false)
  })

  it('ingests subscribed notifications through the unified event path', async () => {
    let emitNotification: ((notification: NotificationRecord) => void) | undefined
    subscribeToNotificationsMock.mockImplementation((handler) => {
      emitNotification = handler
      return () => {}
    })
    listNotificationsMock.mockResolvedValue(createListResponse())

    const store = useNotificationStore()
    await store.bootstrap()

    emitNotification?.(createRecord({
      id: 'notif-pushed',
      scopeKind: 'workspace',
      toastVisibleUntil: Date.now() + 30_000,
    }))

    expect(store.notifications[0]?.id).toBe('notif-pushed')
    expect(store.unreadSummary.total).toBe(1)
    expect(store.unreadSummary.byScope.workspace).toBe(1)
  })
})
