// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import type { InboxItemRecord } from '@octopus/schema'

const { resolveWorkspaceClientForConnectionMock, useShellStoreMock } = vi.hoisted(() => ({
  resolveWorkspaceClientForConnectionMock: vi.fn(),
  useShellStoreMock: vi.fn(),
}))

vi.mock('@/stores/workspace-scope', () => ({
  activeWorkspaceConnectionId: vi.fn(() => 'conn-local'),
  createWorkspaceRequestToken: vi.fn((nextValue = 0) => nextValue + 1),
  resolveWorkspaceClientForConnection: resolveWorkspaceClientForConnectionMock,
}))

vi.mock('@/stores/shell', () => ({
  useShellStore: useShellStoreMock,
}))

import { useInboxStore } from '@/stores/inbox'

function createInboxItem(overrides: Partial<InboxItemRecord> = {}): InboxItemRecord {
  return {
    id: 'inbox-1',
    workspaceId: 'ws-local',
    projectId: 'proj-redesign',
    itemType: 'approval',
    title: 'Need approval',
    description: 'Runtime needs approval.',
    status: 'pending',
    priority: 'high',
    targetUserId: 'user-owner',
    actionable: true,
    routeTo: '/workspaces/ws-local/projects/proj-redesign/settings',
    actionLabel: 'Review approval',
    createdAt: 1,
    ...overrides,
  }
}

describe('useInboxStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resolveWorkspaceClientForConnectionMock.mockReset()
    useShellStoreMock.mockReset()
    useShellStoreMock.mockReturnValue({
      activeWorkspaceConnectionId: 'conn-local',
      workspaceSessionsState: {
        'conn-local': {
          session: {
            userId: 'user-owner',
          },
        },
      },
    })
  })

  it('bootstraps inbox items and derives actionable counts', async () => {
    const list = vi.fn().mockResolvedValue([
      createInboxItem(),
      createInboxItem({
        id: 'inbox-hidden',
        targetUserId: 'user-operator',
      }),
      createInboxItem({
        id: 'inbox-2',
        actionable: false,
        status: 'completed',
      }),
    ])
    resolveWorkspaceClientForConnectionMock.mockReturnValue({
      connectionId: 'conn-local',
      client: {
        inbox: {
          list,
        },
      },
    })

    const store = useInboxStore()
    await store.bootstrap()

    expect(list).toHaveBeenCalledTimes(1)
    expect(store.items).toHaveLength(2)
    expect(store.items.every(item => item.targetUserId === 'user-owner')).toBe(true)
    expect(store.actionableCount).toBe(1)
    expect(store.bootstrapped).toBe(true)
    expect(store.error).toBe('')
  })

  it('reloads inbox items when the active session user changes on the same connection', async () => {
    const list = vi.fn()
      .mockResolvedValueOnce([
        createInboxItem({
          id: 'inbox-owner',
          targetUserId: 'user-owner',
        }),
      ])
      .mockResolvedValueOnce([
        createInboxItem({
          id: 'inbox-operator',
          targetUserId: 'user-operator',
        }),
      ])

    resolveWorkspaceClientForConnectionMock.mockReturnValue({
      connectionId: 'conn-local',
      client: {
        inbox: {
          list,
        },
      },
    })

    const shell = {
      activeWorkspaceConnectionId: 'conn-local',
      workspaceSessionsState: {
        'conn-local': {
          session: {
            userId: 'user-owner',
          },
        },
      },
    }
    useShellStoreMock.mockReturnValue(shell)

    const store = useInboxStore()
    await store.bootstrap()

    shell.workspaceSessionsState['conn-local'].session.userId = 'user-operator'
    await store.bootstrap()

    expect(list).toHaveBeenCalledTimes(2)
    expect(store.items).toHaveLength(1)
    expect(store.items[0]?.targetUserId).toBe('user-operator')
  })

  it('keeps inbox failures non-blocking and reports zero actionable items on error', async () => {
    resolveWorkspaceClientForConnectionMock.mockReturnValue({
      connectionId: 'conn-local',
      client: {
        inbox: {
          list: vi.fn().mockRejectedValue(new Error('Inbox unavailable')),
        },
      },
    })

    const store = useInboxStore()
    await store.bootstrap()

    expect(store.items).toEqual([])
    expect(store.actionableCount).toBe(0)
    expect(store.error).toContain('Inbox unavailable')
    expect(store.loading).toBe(false)
  })
})
