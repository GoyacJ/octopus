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

  async function prepareCatalogStore(
    fixtureOptions: Parameters<typeof installWorkspaceApiFixture>[0] = {},
  ) {
    vi.restoreAllMocks()
    installWorkspaceApiFixture(fixtureOptions)
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

  it('keeps management entries in sync when disabling a builtin tool', async () => {
    const { catalog } = await prepareCatalogStore()

    expect(catalog.managementEntries.some(entry => entry.sourceKey === 'builtin:bash')).toBe(true)

    await catalog.setAssetDisabled({
      sourceKey: 'builtin:bash',
      disabled: true,
    })

    const bashEntry = catalog.managementEntries.find(entry => entry.sourceKey === 'builtin:bash')
    expect(bashEntry?.disabled).toBe(true)
  })

  it('exposes a capability management projection with asset and package manifests', async () => {
    const { catalog } = await prepareCatalogStore()

    expect(catalog.managementProjection.entries.length).toBeGreaterThan(0)
    expect(catalog.managementProjection.assets).toHaveLength(catalog.managementProjection.entries.length)

    const builtinEntry = catalog.managementProjection.entries.find(entry => entry.sourceKey === 'builtin:bash')
    expect(builtinEntry?.enabled).toBe(true)
    expect(builtinEntry?.state).toBe('builtin')

    const managedSkill = catalog.managementProjection.skillPackages
      .find(entry => entry.sourceKey === 'skill:data/skills/help/SKILL.md')
    expect(managedSkill?.packageKind).toBe('workspace')
    expect(managedSkill?.importStatus).toBe('managed')
    expect(managedSkill?.exportStatus).toBe('exportable')

    const externalSkill = catalog.managementProjection.skillPackages
      .find(entry => entry.sourceKey === 'skill:.claude/skills/external-help/SKILL.md')
    expect(externalSkill?.packageKind).toBe('external')
    expect(externalSkill?.importStatus).toBe('copy-required')
    expect(externalSkill?.exportStatus).toBe('readonly')

    const builtinBundleSkill = catalog.managementProjection.skillPackages
      .find(entry => entry.sourceKey === 'skill:builtin-assets/skills/financial-calculator/SKILL.md')
    expect(builtinBundleSkill?.packageKind).toBe('builtin')
    expect(builtinBundleSkill?.importStatus).toBe('copy-required')

    const workspaceMcp = catalog.managementProjection.mcpServerPackages.find(entry => entry.sourceKey === 'mcp:ops')
    expect(workspaceMcp?.packageKind).toBe('workspace')
    expect(workspaceMcp?.health).toBe('attention')
    expect(workspaceMcp?.state).toBe('workspace')
  })

  it('keeps management projection state in sync when disabling a builtin tool', async () => {
    const { catalog } = await prepareCatalogStore()

    await catalog.setAssetDisabled({
      sourceKey: 'builtin:bash',
      disabled: true,
    })

    const bashEntry = catalog.managementProjection.entries.find(entry => entry.sourceKey === 'builtin:bash')
    expect(bashEntry?.enabled).toBe(false)
    expect(bashEntry?.state).toBe('disabled')
  })

  it('uses the host capability management projection as the only management surface', async () => {
    const { catalog } = await prepareCatalogStore({
      managementProjectionTransform(projection) {
        return {
          ...projection,
          entries: projection.entries.filter(entry => entry.sourceKey !== 'builtin:bash'),
          assets: projection.assets.filter(asset => asset.sourceKey !== 'builtin:bash'),
          skillPackages: projection.skillPackages,
          mcpServerPackages: projection.mcpServerPackages,
        }
      },
    })

    expect(catalog.managementProjection.entries.some(entry => entry.sourceKey === 'builtin:bash')).toBe(false)
    expect(catalog.managementEntries.some(entry => entry.sourceKey === 'builtin:bash')).toBe(false)
  })
})
