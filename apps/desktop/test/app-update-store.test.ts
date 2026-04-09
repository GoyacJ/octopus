// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { createDefaultShellPreferences } from '@octopus/schema'

const getHostUpdateStatus = vi.fn()
const checkHostUpdate = vi.fn()
const downloadHostUpdate = vi.fn()
const installHostUpdate = vi.fn()

async function loadStores() {
  vi.resetModules()
  vi.doMock('@/tauri/client', () => ({
    getHostUpdateStatus,
    checkHostUpdate,
    downloadHostUpdate,
    installHostUpdate,
  }))

  const shellModule = await import('@/stores/shell')
  const updateModule = await import('@/stores/app-update')

  return {
    useShellStore: shellModule.useShellStore,
    useAppUpdateStore: updateModule.useAppUpdateStore,
  }
}

function createStatus(overrides: Record<string, unknown> = {}) {
  return {
    currentVersion: '0.2.0',
    currentChannel: 'formal',
    state: 'idle',
    latestRelease: null,
    lastCheckedAt: null,
    progress: null,
    capabilities: {
      canCheck: true,
      canDownload: true,
      canInstall: true,
      supportsChannels: true,
    },
    errorCode: null,
    errorMessage: null,
    ...overrides,
  }
}

describe('app update store', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    setActivePinia(createPinia())
  })

  it('initializes once and performs a single startup update check', async () => {
    getHostUpdateStatus.mockResolvedValue(createStatus())
    checkHostUpdate.mockResolvedValue(createStatus({
      state: 'up_to_date',
      lastCheckedAt: 1_710_000_000_000,
    }))

    const { useShellStore, useAppUpdateStore } = await loadStores()
    const shell = useShellStore()
    shell.applyShellPreferences({
      ...createDefaultShellPreferences('ws-local', 'proj-redesign'),
      updateChannel: 'formal',
    })

    const store = useAppUpdateStore()
    await store.initialize()
    await store.initialize()

    expect(getHostUpdateStatus).toHaveBeenCalledTimes(1)
    expect(checkHostUpdate).toHaveBeenCalledTimes(1)
    expect(store.status?.state).toBe('up_to_date')
  })

  it('persists channel changes through shell preferences and re-checks updates', async () => {
    getHostUpdateStatus.mockResolvedValue(createStatus())
    checkHostUpdate.mockResolvedValue(createStatus({
      currentChannel: 'preview',
      state: 'update_available',
      latestRelease: {
        version: '0.2.1-preview.7',
        channel: 'preview',
        notes: 'Preview release notes',
        publishedAt: '2026-04-09T08:00:00.000Z',
        notesUrl: 'https://example.test/release-notes',
      },
    }))

    const { useShellStore, useAppUpdateStore } = await loadStores()
    const shell = useShellStore()
    shell.applyShellPreferences({
      ...createDefaultShellPreferences('ws-local', 'proj-redesign'),
      updateChannel: 'formal',
    })
    shell.updatePreferences = vi.fn(async (patch) => {
      shell.applyShellPreferences({
        ...shell.preferences,
        ...patch,
      })
    })

    const store = useAppUpdateStore()
    await store.initialize()
    await store.setUpdateChannel('preview')

    expect(shell.updatePreferences).toHaveBeenCalledWith({ updateChannel: 'preview' })
    expect(checkHostUpdate).toHaveBeenLastCalledWith('preview')
    expect(store.status?.currentChannel).toBe('preview')
    expect(store.status?.state).toBe('update_available')
  })

  it('tracks download and install transitions through host update actions', async () => {
    getHostUpdateStatus.mockResolvedValue(createStatus({
      state: 'update_available',
      latestRelease: {
        version: '0.2.1',
        channel: 'formal',
        notes: 'Formal release notes',
        publishedAt: '2026-04-09T08:00:00.000Z',
        notesUrl: 'https://example.test/release-notes',
      },
    }))
    checkHostUpdate.mockResolvedValue(createStatus({
      state: 'update_available',
      latestRelease: {
        version: '0.2.1',
        channel: 'formal',
        notes: 'Formal release notes',
        publishedAt: '2026-04-09T08:00:00.000Z',
        notesUrl: 'https://example.test/release-notes',
      },
    }))
    downloadHostUpdate.mockResolvedValue(createStatus({
      state: 'downloaded',
      progress: {
        downloadedBytes: 1024,
        totalBytes: 1024,
        percent: 100,
      },
    }))
    installHostUpdate.mockResolvedValue(createStatus({
      state: 'installing',
    }))

    const { useShellStore, useAppUpdateStore } = await loadStores()
    const shell = useShellStore()
    shell.applyShellPreferences({
      ...createDefaultShellPreferences('ws-local', 'proj-redesign'),
      updateChannel: 'formal',
    })

    const store = useAppUpdateStore()
    await store.initialize()
    await store.downloadUpdate()
    expect(store.status?.state).toBe('downloaded')

    await store.installUpdate()
    expect(store.status?.state).toBe('installing')
  })
})
