// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import type { InboxItemRecord } from '@octopus/schema'

const { resolveWorkspaceClientForConnectionMock } = vi.hoisted(() => ({
  resolveWorkspaceClientForConnectionMock: vi.fn(),
}))

vi.mock('@/stores/workspace-scope', () => ({
  activeWorkspaceConnectionId: vi.fn(() => 'conn-local'),
  createWorkspaceRequestToken: vi.fn((nextValue = 0) => nextValue + 1),
  resolveWorkspaceClientForConnection: resolveWorkspaceClientForConnectionMock,
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
    actionable: true,
    routeTo: '/workspaces/ws-local/projects/proj-redesign/runtime',
    actionLabel: 'Review approval',
    createdAt: 1,
    ...overrides,
  }
}

describe('useInboxStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resolveWorkspaceClientForConnectionMock.mockReset()
  })

  it('bootstraps inbox items and derives actionable counts', async () => {
    const list = vi.fn().mockResolvedValue([
      createInboxItem(),
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
    expect(store.actionableCount).toBe(1)
    expect(store.bootstrapped).toBe(true)
    expect(store.error).toBe('')
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
