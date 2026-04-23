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

  it('keeps generation-only model rows runtime-truthful while filtering them out of conversation-facing catalog options', async () => {
    const { catalog } = await prepareCatalogStore({
      stateTransform(state, connection) {
        if (connection.workspaceConnectionId !== 'conn-local') {
          return
        }

        state.catalog.models.push({
          modelId: 'gpt-4o-generate',
          label: 'GPT-4o Generate',
          providerId: 'openai',
          description: 'Prompt-only generation model.',
          family: 'gpt-4o',
          track: 'generation',
          enabled: true,
          recommendedFor: 'Single shot generation',
          availability: 'configured',
          defaultPermission: 'auto',
          surfaceBindings: [
            {
              surface: 'conversation',
              protocolFamily: 'openai_responses',
              enabled: true,
              executionProfile: {
                executionClass: 'single_shot_generation',
                toolLoop: false,
                upstreamStreaming: true,
              },
            },
          ],
          capabilities: [],
          metadata: {},
        })

        state.catalog.configuredModels.push({
          configuredModelId: 'openai-generate',
          name: 'OpenAI Generate',
          providerId: 'openai',
          modelId: 'gpt-4o-generate',
          credentialRef: 'env:OPENAI_API_KEY',
          budgetPolicy: {
            totalBudgetTokens: 4096,
            reservationStrategy: 'fixed',
          },
          tokenUsage: {
            usedTokens: 128,
            exhausted: false,
          },
          enabled: true,
          source: 'workspace',
          status: 'configured',
          configured: true,
        })
      },
    })

    expect(catalog.configuredModelRows.find(row => row.configuredModelId === 'openai-generate')).toMatchObject({
      surfaces: ['conversation'],
      conversationSurfaces: [],
      executionClass: 'single_shot_generation',
      supportsConversationExecution: false,
    })
    expect(catalog.configuredModelOptions.some(option => option.value === 'openai-generate')).toBe(false)
  })

  it('keeps missing-credential models visible in management lists while excluding them from runtime-ready options', async () => {
    const { catalog } = await prepareCatalogStore({
      stateTransform(state, connection) {
        if (connection.workspaceConnectionId !== 'conn-local') {
          return
        }

        state.catalog.configuredModels = state.catalog.configuredModels.map(model => (
          model.configuredModelId === 'anthropic-alt'
            ? {
                ...model,
                credentialRef: undefined,
                status: 'missing_credentials',
                configured: false,
              } as any
            : model
        ))
      },
    })

    expect(catalog.configuredModelOptions.some(option => option.value === 'anthropic-alt')).toBe(true)
    expect(catalog.workspaceConfiguredModelOptions.some(option => option.value === 'anthropic-alt')).toBe(true)
    expect(catalog.runnableConfiguredModelOptions.some(option => option.value === 'anthropic-alt')).toBe(false)
    expect(catalog.workspaceRunnableConfiguredModelOptions.some(option => option.value === 'anthropic-alt')).toBe(false)
    expect(catalog.defaultConversationRunnableConfiguredModelOption?.value).toBe('anthropic-primary')
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
    expect(catalog.managementProjection.assets.length).toBeGreaterThan(0)

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

  it('surfaces capability-aware MCP entries while keeping the server asset grouped', async () => {
    const { catalog } = await prepareCatalogStore({
      toolCatalogTransform(entries) {
        const workspaceMcp = entries.find(entry => entry.sourceKey === 'mcp:ops')
        expect(workspaceMcp).toBeTruthy()
        return [
          ...entries.filter(entry => entry.sourceKey !== 'mcp:ops'),
          {
            ...workspaceMcp!,
            id: 'mcp-ops-tool-tail-logs',
            assetId: 'mcp-asset-ops',
            capabilityId: 'mcp_tool__ops__tail_logs',
            sourceKind: 'mcp_tool',
            executionKind: 'tool',
            name: 'tail_logs',
            description: 'Tail operational logs from the ops server.',
            toolNames: ['tail_logs'],
          } as any,
          {
            ...workspaceMcp!,
            id: 'mcp-ops-prompt-deploy-review',
            assetId: 'mcp-asset-ops',
            capabilityId: 'mcp_prompt__ops__deploy_review',
            sourceKind: 'mcp_prompt',
            executionKind: 'prompt_skill',
            name: 'deploy_review',
            description: 'Prepare a deploy review checklist.',
            toolNames: [],
          } as any,
          {
            ...workspaceMcp!,
            id: 'mcp-ops-resource-guide',
            assetId: 'mcp-asset-ops',
            capabilityId: 'mcp_resource__ops__guide_txt',
            sourceKind: 'mcp_resource',
            executionKind: 'resource',
            name: 'Ops Guide',
            description: 'Operational guide resource.',
            toolNames: [],
            resourceUri: 'file://ops-guide.txt',
          } as any,
        ]
      },
    })

    const mcpEntries = catalog.managementProjection.entries.filter(entry => entry.sourceKey === 'mcp:ops')
    expect(mcpEntries).toHaveLength(3)
    expect(mcpEntries).toEqual(expect.arrayContaining([
      expect.objectContaining({ sourceKind: 'mcp_tool', executionKind: 'tool' }),
      expect.objectContaining({ sourceKind: 'mcp_prompt', executionKind: 'prompt_skill' }),
      expect.objectContaining({ sourceKind: 'mcp_resource', executionKind: 'resource' }),
    ]))

    const mcpAssets = catalog.managementProjection.assets.filter(entry => entry.sourceKey === 'mcp:ops')
    expect(mcpAssets).toHaveLength(1)
    expect(mcpAssets[0]).toMatchObject({
      assetId: 'mcp-asset-ops',
      sourceKinds: ['mcp_prompt', 'mcp_resource', 'mcp_tool'],
      executionKinds: ['prompt_skill', 'resource', 'tool'],
    })

    const workspaceMcp = catalog.managementProjection.mcpServerPackages.find(entry => entry.sourceKey === 'mcp:ops')
    expect(workspaceMcp).toMatchObject({
      promptNames: ['deploy_review'],
      resourceUris: ['file://ops-guide.txt'],
      toolNames: ['tail_logs'],
      sourceKinds: ['mcp_prompt', 'mcp_resource', 'mcp_tool'],
      executionKinds: ['prompt_skill', 'resource', 'tool'],
    })
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
