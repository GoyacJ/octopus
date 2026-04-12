import type {
  CapabilityAssetExportStatus,
  CapabilityAssetImportStatus,
  CapabilityAssetManifest,
  CapabilityAssetState,
  CapabilityManagementEntry,
  CapabilityManagementProjection,
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

function toAssetManifest<TEntry extends WorkspaceToolCatalogEntry>(
  entry: TEntry,
): CapabilityAssetManifest & { kind: TEntry['kind'] } {
  return {
    assetId: entry.id,
    workspaceId: entry.workspaceId,
    sourceKey: entry.sourceKey,
    kind: entry.kind,
    name: entry.name,
    description: entry.description,
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
  const manifest = toAssetManifest(entry)
  if (entry.kind === 'builtin') {
    return {
      ...entry,
      ...manifest,
    } as Extract<CapabilityManagementEntry, { kind: 'builtin' }>
  }
  if (entry.kind === 'skill') {
    return {
      ...entry,
      ...manifest,
    } as Extract<CapabilityManagementEntry, { kind: 'skill' }>
  }
  return {
    ...entry,
    ...manifest,
  } as Extract<CapabilityManagementEntry, { kind: 'mcp' }>
}

function toSkillPackageManifest(entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>): SkillPackageManifest {
  return {
    ...toAssetManifest(entry),
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

function toMcpServerPackageManifest(entry: Extract<WorkspaceToolCatalogEntry, { kind: 'mcp' }>): McpServerPackageManifest {
  return {
    ...toAssetManifest(entry),
    kind: 'mcp',
    packageKind: entry.scope,
    serverName: entry.serverName,
    endpoint: entry.endpoint,
    toolNames: [...entry.toolNames],
    scope: entry.scope,
    statusDetail: entry.statusDetail,
  }
}

export function deriveCapabilityManagementProjection(entries: WorkspaceToolCatalogEntry[]): CapabilityManagementProjection {
  const managementEntries = entries.map(toManagementEntry)
  return {
    entries: managementEntries,
    assets: managementEntries.map((entry) => {
      const {
        assetId,
        workspaceId,
        sourceKey,
        kind,
        name,
        description,
        displayPath,
        ownerScope,
        ownerId,
        ownerLabel,
        requiredPermission,
        management,
        installed,
        enabled,
        health,
        state,
        importStatus,
        exportStatus,
      } = entry
      return {
        assetId,
        workspaceId,
        sourceKey,
        kind,
        name,
        description,
        displayPath,
        ownerScope,
        ownerId,
        ownerLabel,
        requiredPermission,
        management,
        installed,
        enabled,
        health,
        state,
        importStatus,
        exportStatus,
      }
    }),
    skillPackages: entries
      .filter((entry): entry is Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }> => entry.kind === 'skill')
      .map(toSkillPackageManifest),
    mcpServerPackages: entries
      .filter((entry): entry is Extract<WorkspaceToolCatalogEntry, { kind: 'mcp' }> => entry.kind === 'mcp')
      .map(toMcpServerPackageManifest),
  }
}
