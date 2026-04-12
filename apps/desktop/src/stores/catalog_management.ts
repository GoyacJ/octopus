import type {
  CapabilityExecutionKind,
  CapabilityAssetExportStatus,
  CapabilityAssetImportStatus,
  CapabilityAssetManifest,
  CapabilityAssetState,
  CapabilityManagementEntry,
  CapabilityManagementProjection,
  CapabilitySourceKind,
  McpServerPackageManifest,
  SkillPackageManifest,
  WorkspaceToolCatalogEntry,
} from '@octopus/schema'

function resolveImportStatus(entry: WorkspaceToolCatalogEntry): CapabilityAssetImportStatus {
  if (entry.kind === 'builtin') {
    return 'not-importable'
  }
  if (entry.kind === 'skill') {
    return entry.workspaceOwned || entry.ownerScope === 'project' ? 'managed' : 'copy-required'
  }
  return entry.scope === 'builtin' ? 'copy-required' : 'managed'
}

function resolveExportStatus(entry: WorkspaceToolCatalogEntry): CapabilityAssetExportStatus {
  if (entry.kind === 'builtin') {
    return 'not-exportable'
  }
  return entry.management.canEdit || entry.management.canDelete ? 'exportable' : 'readonly'
}

function resolveAssetState(entry: WorkspaceToolCatalogEntry): CapabilityAssetState {
  if (entry.disabled) {
    return 'disabled'
  }
  if (entry.kind === 'builtin') {
    return 'builtin'
  }
  if (entry.kind === 'skill') {
    if (entry.shadowedBy) {
      return 'shadowed'
    }
    if (entry.ownerScope === 'project') {
      return 'project'
    }
    if (entry.sourceOrigin === 'builtin_bundle') {
      return 'builtin'
    }
    if (entry.workspaceOwned) {
      return entry.ownerScope === 'workspace' ? 'workspace' : 'managed'
    }
    return 'external'
  }
  return entry.scope
}

function capabilityAssetId(entry: WorkspaceToolCatalogEntry) {
  return entry.assetId ?? entry.id
}

function capabilityId(entry: WorkspaceToolCatalogEntry) {
  return entry.capabilityId ?? entry.id
}

function capabilitySourceKind(entry: WorkspaceToolCatalogEntry): CapabilitySourceKind {
  if (entry.sourceKind) {
    return entry.sourceKind
  }
  if (entry.kind === 'builtin') {
    return 'builtin'
  }
  if (entry.kind === 'skill') {
    return entry.sourceOrigin === 'builtin_bundle' ? 'bundled_skill' : 'local_skill'
  }
  return 'mcp_tool'
}

function capabilityExecutionKind(entry: WorkspaceToolCatalogEntry): CapabilityExecutionKind {
  if (entry.executionKind) {
    return entry.executionKind
  }
  if (entry.kind === 'skill') {
    return 'prompt_skill'
  }
  if (entry.kind === 'mcp' && entry.resourceUri) {
    return 'resource'
  }
  return 'tool'
}

function uniqueSorted<T extends string>(values: readonly T[]): T[] {
  return [...new Set(values.filter(Boolean))].sort()
}

function assetName(entry: WorkspaceToolCatalogEntry) {
  return entry.kind === 'mcp' ? (entry.serverName ?? entry.name) : entry.name
}

function assetDescription(entry: WorkspaceToolCatalogEntry) {
  if (entry.kind !== 'mcp') {
    return entry.description
  }
  return entry.scope === 'builtin' ? 'Builtin MCP server template.' : 'Configured MCP server.'
}

function toAssetManifest<TEntry extends WorkspaceToolCatalogEntry>(
  entries: TEntry[],
): CapabilityAssetManifest & { kind: TEntry['kind'] } {
  const entry = entries[0]
  return {
    assetId: capabilityAssetId(entry),
    workspaceId: entry.workspaceId,
    sourceKey: entry.sourceKey,
    kind: entry.kind,
    sourceKinds: uniqueSorted(entries.map(item => capabilitySourceKind(item))),
    executionKinds: uniqueSorted(entries.map(item => capabilityExecutionKind(item))),
    name: assetName(entry),
    description: assetDescription(entry),
    displayPath: entry.displayPath,
    ownerScope: entry.ownerScope,
    ownerId: entry.ownerId,
    ownerLabel: entry.ownerLabel,
    requiredPermission: entry.requiredPermission,
    management: entry.management,
    installed: true,
    enabled: !entry.disabled,
    health: entry.availability,
    state: resolveAssetState(entry),
    importStatus: resolveImportStatus(entry),
    exportStatus: resolveExportStatus(entry),
  }
}

function toManagementEntry(entry: WorkspaceToolCatalogEntry): CapabilityManagementEntry {
  const manifest = toAssetManifest([entry])
  const { sourceKinds: _sourceKinds, executionKinds: _executionKinds, ...assetFields } = manifest
  if (entry.kind === 'builtin') {
    return {
      ...entry,
      ...assetFields,
      assetId: capabilityAssetId(entry),
      capabilityId: capabilityId(entry),
      sourceKind: capabilitySourceKind(entry),
      executionKind: capabilityExecutionKind(entry),
    } as Extract<CapabilityManagementEntry, { kind: 'builtin' }>
  }
  if (entry.kind === 'skill') {
    return {
      ...entry,
      ...assetFields,
      assetId: capabilityAssetId(entry),
      capabilityId: capabilityId(entry),
      sourceKind: capabilitySourceKind(entry),
      executionKind: capabilityExecutionKind(entry),
    } as Extract<CapabilityManagementEntry, { kind: 'skill' }>
  }
  return {
    ...entry,
    ...assetFields,
    assetId: capabilityAssetId(entry),
    capabilityId: capabilityId(entry),
    sourceKind: capabilitySourceKind(entry),
    executionKind: capabilityExecutionKind(entry),
    resourceUri: entry.resourceUri,
  } as Extract<CapabilityManagementEntry, { kind: 'mcp' }>
}

function toSkillPackageManifest(entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>): SkillPackageManifest {
  return {
    ...toAssetManifest([entry]),
    kind: 'skill',
    packageKind: entry.ownerScope === 'project'
      ? 'project'
      : entry.sourceOrigin === 'builtin_bundle'
        ? 'builtin'
        : entry.workspaceOwned
          ? 'workspace'
          : 'external',
    active: entry.active,
    shadowedBy: entry.shadowedBy,
    sourceOrigin: entry.sourceOrigin,
    workspaceOwned: entry.workspaceOwned,
    relativePath: entry.relativePath,
  }
}

function toMcpServerPackageManifest(entries: Extract<WorkspaceToolCatalogEntry, { kind: 'mcp' }>[]): McpServerPackageManifest {
  const entry = entries[0]
  return {
    ...toAssetManifest(entries),
    kind: 'mcp',
    packageKind: entry.scope,
    serverName: entry.serverName,
    endpoint: entry.endpoint,
    toolNames: uniqueSorted(entries.flatMap(item => item.toolNames ?? [])),
    promptNames: uniqueSorted(entries
      .filter(item => capabilitySourceKind(item) === 'mcp_prompt')
      .map(item => item.name)),
    resourceUris: uniqueSorted(entries
      .map(item => item.resourceUri)
      .filter((value): value is string => Boolean(value))),
    scope: entry.scope,
    statusDetail: entry.statusDetail,
  }
}

export function deriveCapabilityManagementProjection(entries: WorkspaceToolCatalogEntry[]): CapabilityManagementProjection {
  const managementEntries = entries.map(toManagementEntry)
  const groupedAssets = new Map<string, WorkspaceToolCatalogEntry[]>()
  for (const entry of entries) {
    const key = `${capabilityAssetId(entry)}::${entry.sourceKey}`
    groupedAssets.set(key, [...(groupedAssets.get(key) ?? []), entry])
  }
  return {
    entries: managementEntries,
    assets: [...groupedAssets.values()].map(group => toAssetManifest(group)),
    skillPackages: entries
      .filter((entry): entry is Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }> => entry.kind === 'skill')
      .map(toSkillPackageManifest),
    mcpServerPackages: [...groupedAssets.values()]
      .filter((group): group is Extract<WorkspaceToolCatalogEntry, { kind: 'mcp' }>[] => group[0]?.kind === 'mcp')
      .map(toMcpServerPackageManifest),
  }
}
