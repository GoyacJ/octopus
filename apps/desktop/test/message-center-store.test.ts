// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { useInboxStore } from '@/stores/inbox'
import { useMessageCenterStore } from '@/stores/message-center'
import { useNotificationStore } from '@/stores/notifications'

describe('useMessageCenterStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('combines unread notification and actionable inbox counts and remembers the active tab', () => {
    const notifications = useNotificationStore()
    const inbox = useInboxStore()
    const messageCenter = useMessageCenterStore()

    notifications.unreadSummaryState.total = 3
    inbox.itemsState = [
      {
        id: 'inbox-1',
        workspaceId: 'ws-local',
        projectId: 'proj-redesign',
        itemType: 'approval',
        title: 'Need approval',
        description: 'Runtime needs approval.',
        status: 'pending',
        priority: 'high',
        actionable: true,
        routeTo: '/workspaces/ws-local/projects/proj-redesign/settings',
        actionLabel: 'Review approval',
        createdAt: 1,
      },
    ]

    expect(messageCenter.combinedCount).toBe(4)
    expect(messageCenter.activeTab).toBe('notifications')

    messageCenter.setActiveTab('inbox')
    messageCenter.openCenter()
    messageCenter.closeCenter()

    expect(messageCenter.activeTab).toBe('inbox')
    expect(messageCenter.open).toBe(false)
  })
})
