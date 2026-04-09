// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { useCatalogStore } from '@/stores/catalog'
import { useShellStore } from '@/stores/shell'
import { installWorkspaceApiFixture } from './support/workspace-fixture'

describe('useCatalogStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    window.localStorage.clear()
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
  })

  async function prepareCatalogStore() {
    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign', [])
    const catalog = useCatalogStore()
    await catalog.load()
    return { catalog, shell }
  }

  it('normalizes model rows, provider summaries, and filter options from the workspace catalog snapshot', async () => {
    const { catalog } = await prepareCatalogStore()

    expect(catalog.configuredModelRows.some(row => row.configuredModelId === 'anthropic-primary')).toBe(true)
    expect(catalog.configuredModelRows.find(row => row.configuredModelId === 'anthropic-primary')?.name).toBe('Claude Primary')
    expect(catalog.providerSummaries.some(provider => provider.providerId === 'anthropic')).toBe(true)
    expect(catalog.filterOptions.providers.some(option => option.value === 'anthropic')).toBe(true)
    expect(catalog.filterOptions.capabilities.length).toBeGreaterThan(0)
    expect(catalog.defaultSelectionRows.some(row => row.configuredModelId === 'anthropic-primary')).toBe(true)
  })

  it('keeps tool catalog and runtime-owned entries in sync when disabling a builtin tool', async () => {
    const { catalog } = await prepareCatalogStore()

    expect(catalog.toolCatalogEntries.some(entry => entry.sourceKey === 'builtin:bash')).toBe(true)

    await catalog.setToolDisabled({
      sourceKey: 'builtin:bash',
      disabled: true,
    })

    const bashEntry = catalog.toolCatalogEntries.find(entry => entry.sourceKey === 'builtin:bash')
    expect(bashEntry?.disabled).toBe(true)
  })
})
