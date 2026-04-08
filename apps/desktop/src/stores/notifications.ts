import { defineStore } from 'pinia'

import type {
  CreateNotificationInput,
  NotificationFilter,
  NotificationFilterScope,
  NotificationListResponse,
  NotificationRecord,
  NotificationUnreadSummary,
} from '@octopus/schema'
import {
  createDefaultNotificationUnreadSummary,
  normalizeNotificationListResponse,
} from '@octopus/schema'

import * as tauriClient from '@/tauri/client'

function sortNotifications(notifications: NotificationRecord[]): NotificationRecord[] {
  return [...notifications].sort((left, right) => right.createdAt - left.createdAt)
}

function mergeNotification(
  notifications: NotificationRecord[],
  notification: NotificationRecord,
): NotificationRecord[] {
  const index = notifications.findIndex(item => item.id === notification.id)
  if (index < 0) {
    return sortNotifications([notification, ...notifications])
  }

  const next = [...notifications]
  next.splice(index, 1, notification)
  return sortNotifications(next)
}

function deriveUnreadSummary(notifications: NotificationRecord[]): NotificationUnreadSummary {
  const unread = createDefaultNotificationUnreadSummary()

  for (const notification of notifications) {
    if (notification.readAt) {
      continue
    }

    unread.total += 1
    unread.byScope[notification.scopeKind] += 1
  }

  return unread
}

function normalizeResponse(response: NotificationListResponse): NotificationListResponse {
  const normalized = normalizeNotificationListResponse(response)
  return {
    notifications: sortNotifications(normalized.notifications),
    unread: normalized.unread,
  }
}

export const useNotificationStore = defineStore('notifications', {
  state: () => ({
    notificationsState: [] as NotificationRecord[],
    unreadSummaryState: createDefaultNotificationUnreadSummary(),
    filterScope: 'all' as NotificationFilterScope,
    centerOpen: false,
    bootstrapped: false,
    unsubscribe: null as null | (() => void),
  }),
  getters: {
    notifications(state): NotificationRecord[] {
      return state.notificationsState
    },
    unreadSummary(state): NotificationUnreadSummary {
      return state.unreadSummaryState
    },
    filteredNotifications(state): NotificationRecord[] {
      if (state.filterScope === 'all') {
        return state.notificationsState
      }

      return state.notificationsState.filter(
        notification => notification.scopeKind === state.filterScope,
      )
    },
    activeToasts(state): NotificationRecord[] {
      const now = Date.now()
      return state.notificationsState.filter(notification =>
        typeof notification.toastVisibleUntil === 'number' && notification.toastVisibleUntil > now,
      )
    },
  },
  actions: {
    applyResponse(response: NotificationListResponse) {
      const normalized = normalizeResponse(response)
      this.notificationsState = normalized.notifications
      this.unreadSummaryState = normalized.unread
    },
    ingest(notification: NotificationRecord) {
      this.notificationsState = mergeNotification(this.notificationsState, notification)
      this.unreadSummaryState = deriveUnreadSummary(this.notificationsState)
    },
    async bootstrap() {
      if (this.bootstrapped) {
        return
      }

      this.applyResponse(await tauriClient.listNotifications({ scope: 'all' }))
      this.unsubscribe = tauriClient.subscribeToNotifications((notification) => {
        this.ingest(notification)
      })
      this.bootstrapped = true
    },
    async notify(input: CreateNotificationInput): Promise<NotificationRecord> {
      const notification = await tauriClient.createNotification(input)
      this.ingest(notification)
      return notification
    },
    async markRead(id: string): Promise<NotificationRecord> {
      const notification = await tauriClient.markNotificationRead(id)
      this.ingest(notification)
      return notification
    },
    async markAllRead(filter?: NotificationFilter) {
      const nextFilter = filter ?? { scope: this.filterScope }
      const nextSummary = await tauriClient.markAllNotificationsRead(nextFilter)
      this.notificationsState = this.notificationsState.map((notification) => {
        if (notification.readAt) {
          return notification
        }

        if (nextFilter.scope && nextFilter.scope !== 'all' && notification.scopeKind !== nextFilter.scope) {
          return notification
        }

        return {
          ...notification,
          readAt: Date.now(),
        }
      })
      this.unreadSummaryState = nextSummary
    },
    async dismissToast(id: string): Promise<NotificationRecord> {
      const notification = await tauriClient.dismissNotificationToast(id)
      this.ingest(notification)
      return notification
    },
    openCenter() {
      this.centerOpen = true
    },
    closeCenter() {
      this.centerOpen = false
    },
    setFilter(scope: NotificationFilterScope) {
      this.filterScope = scope
    },
  },
})
