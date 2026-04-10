import type { NotificationRecord } from '@octopus/schema'

export type NotificationSubscriber = (notification: NotificationRecord) => void

const notificationSubscribers = new Set<NotificationSubscriber>()

export function emitNotification(notification: NotificationRecord): void {
  for (const subscriber of notificationSubscribers) {
    subscriber(notification)
  }
}

export function subscribeToNotifications(subscriber: NotificationSubscriber): () => void {
  notificationSubscribers.add(subscriber)
  return () => {
    notificationSubscribers.delete(subscriber)
  }
}
