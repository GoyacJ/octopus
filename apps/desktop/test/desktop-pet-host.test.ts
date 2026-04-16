// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { createApp, nextTick } from 'vue'

import type { NotificationRecord } from '@octopus/schema'

import App from '@/App.vue'
import i18n from '@/plugins/i18n'
import { router } from '@/router'
import { useNotificationStore } from '@/stores/notifications'
import { useRuntimeStore } from '@/stores/runtime'
import * as tauriClient from '@/tauri/client'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
})

function mountApp() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const container = document.createElement('div')
  document.body.appendChild(container)

  const app = createApp(App)
  app.use(pinia)
  app.use(i18n)
  app.use(router)
  app.mount(container)

  return {
    container,
    destroy() {
      app.unmount()
      container.remove()
    },
  }
}

async function waitFor(predicate: () => boolean, timeoutMs = 2000) {
  const startedAt = Date.now()
  while (!predicate()) {
    if (Date.now() - startedAt > timeoutMs) {
      throw new Error('Timed out waiting for desktop pet reminder state')
    }
    await nextTick()
    await new Promise(resolve => window.setTimeout(resolve, 20))
  }
}

function createNotification(overrides: Partial<NotificationRecord> = {}): NotificationRecord {
  return {
    id: 'notif-pet-bubble',
    scopeKind: 'workspace',
    scopeOwnerId: 'ws-local',
    level: 'info',
    title: 'Workspace follow-up',
    body: 'Check the latest project changes.',
    source: 'runtime',
    createdAt: Date.now(),
    readAt: undefined,
    toastVisibleUntil: Date.now() + 60_000,
    routeTo: '/workspaces/ws-local/console/projects',
    actionLabel: 'Open',
    ...overrides,
  }
}

describe('Desktop pet host reminders', () => {
  beforeEach(async () => {
    vi.restoreAllMocks()
    vi.useRealTimers()
    installWorkspaceApiFixture()
    document.body.innerHTML = ''
    await router.push('/workspaces/ws-local/overview')
    await router.isReady()
  })

  it('renders a reminder bubble next to the sidebar pet host, expires by TTL, and keeps the notification record', async () => {
    const mounted = mountApp()

    try {
      await waitFor(() => mounted.container.querySelector('[data-testid="desktop-pet-host"]') !== null)

      vi.useFakeTimers()
      vi.setSystemTime(new Date('2026-04-16T04:00:00.000Z'))

      const notifications = useNotificationStore()
      notifications.ingest(createNotification({
        id: 'notif-reminder-ttl',
        toastVisibleUntil: Date.now() + 2_000,
      }))

      await nextTick()

      const host = mounted.container.querySelector('[data-testid="desktop-pet-host"]')
      const bubble = mounted.container.querySelector('[data-testid="desktop-pet-bubble"]')

      expect(host).not.toBeNull()
      expect(bubble).not.toBeNull()
      expect(host?.contains(bubble)).toBe(true)
      expect(mounted.container.querySelector('[data-testid="ui-toast-item-notif-reminder-ttl"]')).toBeNull()

      await vi.advanceTimersByTimeAsync(2_001)
      await nextTick()

      expect(mounted.container.querySelector('[data-testid="desktop-pet-bubble"]')).toBeNull()
      expect(notifications.notifications.find(notification => notification.id === 'notif-reminder-ttl')).toBeTruthy()
    } finally {
      mounted.destroy()
      vi.useRealTimers()
    }
  })

  it('routes through the notification deep link and dismisses only the bubble presentation without runtime side effects', async () => {
    const mounted = mountApp()

    try {
      await waitFor(() => mounted.container.querySelector('[data-testid="desktop-pet-host"]') !== null)

      const notifications = useNotificationStore()
      const runtime = useRuntimeStore()
      const ensureSessionSpy = vi.spyOn(runtime, 'ensureSession')
      const submitTurnSpy = vi.spyOn(runtime, 'submitTurn')
      const dismissToastMock = vi.mocked(tauriClient.dismissNotificationToast)

      dismissToastMock.mockResolvedValue(createNotification({
        id: 'notif-reminder-route',
        toastVisibleUntil: undefined,
      }))

      notifications.ingest(createNotification({
        id: 'notif-reminder-route',
        routeTo: '/workspaces/ws-local/console/projects',
      }))

      await waitFor(() => mounted.container.querySelector('[data-testid="desktop-pet-bubble-action"]') !== null)

      expect(mounted.container.querySelector('[data-testid="ui-toast-item-notif-reminder-route"]')).toBeNull()

      mounted.container
        .querySelector<HTMLElement>('[data-testid="desktop-pet-bubble-action"]')
        ?.click()

      await waitFor(() => router.currentRoute.value.fullPath === '/workspaces/ws-local/console/projects')

      expect(dismissToastMock).toHaveBeenCalledWith('notif-reminder-route')
      expect(ensureSessionSpy).not.toHaveBeenCalled()
      expect(submitTurnSpy).not.toHaveBeenCalled()
      expect(notifications.notifications.find(notification => notification.id === 'notif-reminder-route')).toBeTruthy()
      expect(notifications.notifications.find(notification => notification.id === 'notif-reminder-route')?.readAt).toBeUndefined()
    } finally {
      mounted.destroy()
    }
  })
})
