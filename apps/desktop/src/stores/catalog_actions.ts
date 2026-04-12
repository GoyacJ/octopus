import type { Ref } from 'vue'

import type {
  CapabilityAssetDisablePatch,
  CapabilityManagementProjection,
  CopyWorkspaceSkillToManagedInput,
  CreateWorkspaceSkillInput,
  ModelCatalogSnapshot,
  ToolRecord,
  UpdateWorkspaceSkillFileInput,
  UpdateWorkspaceSkillInput,
  UpsertWorkspaceMcpServerInput,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
  ImportWorkspaceSkillArchiveInput,
  ImportWorkspaceSkillFolderInput,
} from '@octopus/schema'

import {
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { normalizeSnapshot } from './catalog_normalizers'

interface CatalogActionContext {
  snapshots: Ref<Record<string, ModelCatalogSnapshot>>
  managementProjectionsByConnection: Ref<Record<string, CapabilityManagementProjection>>
  skillDocumentsByConnection: Ref<Record<string, Record<string, WorkspaceSkillDocument>>>
  skillTreesByConnection: Ref<Record<string, Record<string, WorkspaceSkillTreeDocument>>>
  skillFilesByConnection: Ref<Record<string, Record<string, WorkspaceSkillFileDocument>>>
  mcpDocumentsByConnection: Ref<Record<string, Record<string, WorkspaceMcpServerDocument>>>
  toolsByConnection: Ref<Record<string, ToolRecord[]>>
  requestTokens: Ref<Record<string, number>>
  errors: Ref<Record<string, string>>
}

export function createCatalogActions(context: CatalogActionContext) {
  function replaceManagementProjection(
    connectionId: string,
    nextManagementProjection: CapabilityManagementProjection,
  ) {
    context.managementProjectionsByConnection.value = {
      ...context.managementProjectionsByConnection.value,
      [connectionId]: nextManagementProjection,
    }
  }

  async function load(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(context.requestTokens.value[connectionId] ?? 0)
    context.requestTokens.value[connectionId] = token
    try {
      const [nextSnapshot, nextManagementProjection, nextTools] = await Promise.all([
        client.catalog.getSnapshot(),
        client.catalog.getManagementProjection(),
        client.catalog.listTools(),
      ])
      if (context.requestTokens.value[connectionId] !== token) {
        return
      }
      context.snapshots.value = {
        ...context.snapshots.value,
        [connectionId]: normalizeSnapshot(nextSnapshot),
      }
      context.managementProjectionsByConnection.value = {
        ...context.managementProjectionsByConnection.value,
        [connectionId]: nextManagementProjection,
      }
      context.toolsByConnection.value = {
        ...context.toolsByConnection.value,
        [connectionId]: nextTools,
      }
      context.errors.value = {
        ...context.errors.value,
        [connectionId]: '',
      }
    } catch (cause) {
      if (context.requestTokens.value[connectionId] === token) {
        context.errors.value = {
          ...context.errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load workspace catalog',
        }
      }
    }
  }

  async function refreshManagementProjection(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return {
        entries: [],
        assets: [],
        skillPackages: [],
        mcpServerPackages: [],
      } satisfies CapabilityManagementProjection
    }
    const projection = await resolvedClient.client.catalog.getManagementProjection()
    replaceManagementProjection(resolvedClient.connectionId, projection)
    return projection
  }

  async function setAssetDisabled(patch: CapabilityAssetDisablePatch) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const projection = await client.catalog.setAssetDisabled(patch)
    replaceManagementProjection(connectionId, projection)
    return projection
  }

  async function getSkillDocument(skillId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.getSkill(skillId)
    context.skillDocumentsByConnection.value = {
      ...context.skillDocumentsByConnection.value,
      [connectionId]: {
        ...(context.skillDocumentsByConnection.value[connectionId] ?? {}),
        [skillId]: document,
      },
    }
    return document
  }

  async function getSkillTreeDocument(skillId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.getSkillTree(skillId)
    context.skillTreesByConnection.value = {
      ...context.skillTreesByConnection.value,
      [connectionId]: {
        ...(context.skillTreesByConnection.value[connectionId] ?? {}),
        [skillId]: document,
      },
    }
    return document
  }

  function skillFileCacheKey(skillId: string, relativePath: string) {
    return `${skillId}:${relativePath}`
  }

  async function getSkillFileDocument(skillId: string, relativePath: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.getSkillFile(skillId, relativePath)
    context.skillFilesByConnection.value = {
      ...context.skillFilesByConnection.value,
      [connectionId]: {
        ...(context.skillFilesByConnection.value[connectionId] ?? {}),
        [skillFileCacheKey(skillId, relativePath)]: document,
      },
    }
    return document
  }

  async function createSkill(input: CreateWorkspaceSkillInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.createSkill(input)
    context.skillDocumentsByConnection.value = {
      ...context.skillDocumentsByConnection.value,
      [connectionId]: {
        ...(context.skillDocumentsByConnection.value[connectionId] ?? {}),
        [document.id]: document,
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function updateSkill(skillId: string, input: UpdateWorkspaceSkillInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.updateSkill(skillId, input)
    context.skillDocumentsByConnection.value = {
      ...context.skillDocumentsByConnection.value,
      [connectionId]: {
        ...(context.skillDocumentsByConnection.value[connectionId] ?? {}),
        [skillId]: document,
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function updateSkillFile(
    skillId: string,
    relativePath: string,
    input: UpdateWorkspaceSkillFileInput,
  ) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.updateSkillFile(skillId, relativePath, input)
    context.skillFilesByConnection.value = {
      ...context.skillFilesByConnection.value,
      [connectionId]: {
        ...(context.skillFilesByConnection.value[connectionId] ?? {}),
        [skillFileCacheKey(skillId, relativePath)]: document,
      },
    }
    context.skillDocumentsByConnection.value = {
      ...context.skillDocumentsByConnection.value,
      [connectionId]: {
        ...(context.skillDocumentsByConnection.value[connectionId] ?? {}),
        [skillId]: await client.catalog.getSkill(skillId),
      },
    }
    context.skillTreesByConnection.value = {
      ...context.skillTreesByConnection.value,
      [connectionId]: {
        ...(context.skillTreesByConnection.value[connectionId] ?? {}),
        [skillId]: await client.catalog.getSkillTree(skillId),
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function importSkillArchive(input: ImportWorkspaceSkillArchiveInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.importSkillArchive(input)
    context.skillDocumentsByConnection.value = {
      ...context.skillDocumentsByConnection.value,
      [connectionId]: {
        ...(context.skillDocumentsByConnection.value[connectionId] ?? {}),
        [document.id]: document,
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function importSkillFolder(input: ImportWorkspaceSkillFolderInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.importSkillFolder(input)
    context.skillDocumentsByConnection.value = {
      ...context.skillDocumentsByConnection.value,
      [connectionId]: {
        ...(context.skillDocumentsByConnection.value[connectionId] ?? {}),
        [document.id]: document,
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function copySkillToManaged(skillId: string, input: CopyWorkspaceSkillToManagedInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.copySkillToManaged(skillId, input)
    context.skillDocumentsByConnection.value = {
      ...context.skillDocumentsByConnection.value,
      [connectionId]: {
        ...(context.skillDocumentsByConnection.value[connectionId] ?? {}),
        [document.id]: document,
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function deleteSkill(skillId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.catalog.deleteSkill(skillId)
    const nextDocuments = { ...(context.skillDocumentsByConnection.value[connectionId] ?? {}) }
    delete nextDocuments[skillId]
    const nextTrees = { ...(context.skillTreesByConnection.value[connectionId] ?? {}) }
    delete nextTrees[skillId]
    const nextFiles = Object.fromEntries(
      Object.entries(context.skillFilesByConnection.value[connectionId] ?? {})
        .filter(([key]) => !key.startsWith(`${skillId}:`)),
    )
    context.skillDocumentsByConnection.value = {
      ...context.skillDocumentsByConnection.value,
      [connectionId]: nextDocuments,
    }
    context.skillTreesByConnection.value = {
      ...context.skillTreesByConnection.value,
      [connectionId]: nextTrees,
    }
    context.skillFilesByConnection.value = {
      ...context.skillFilesByConnection.value,
      [connectionId]: nextFiles,
    }
    await refreshManagementProjection(connectionId)
  }

  async function getMcpServerDocument(serverName: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.getMcpServer(serverName)
    context.mcpDocumentsByConnection.value = {
      ...context.mcpDocumentsByConnection.value,
      [connectionId]: {
        ...(context.mcpDocumentsByConnection.value[connectionId] ?? {}),
        [serverName]: document,
      },
    }
    return document
  }

  async function createMcpServer(input: UpsertWorkspaceMcpServerInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.createMcpServer(input)
    context.mcpDocumentsByConnection.value = {
      ...context.mcpDocumentsByConnection.value,
      [connectionId]: {
        ...(context.mcpDocumentsByConnection.value[connectionId] ?? {}),
        [document.serverName]: document,
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function copyMcpServerToManaged(serverName: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.copyMcpServerToManaged(serverName)
    context.mcpDocumentsByConnection.value = {
      ...context.mcpDocumentsByConnection.value,
      [connectionId]: {
        ...(context.mcpDocumentsByConnection.value[connectionId] ?? {}),
        [document.serverName]: document,
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function updateMcpServer(serverName: string, input: UpsertWorkspaceMcpServerInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const document = await client.catalog.updateMcpServer(serverName, input)
    context.mcpDocumentsByConnection.value = {
      ...context.mcpDocumentsByConnection.value,
      [connectionId]: {
        ...(context.mcpDocumentsByConnection.value[connectionId] ?? {}),
        [document.serverName]: document,
      },
    }
    await refreshManagementProjection(connectionId)
    return document
  }

  async function deleteMcpServer(serverName: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.catalog.deleteMcpServer(serverName)
    const nextDocuments = { ...(context.mcpDocumentsByConnection.value[connectionId] ?? {}) }
    delete nextDocuments[serverName]
    context.mcpDocumentsByConnection.value = {
      ...context.mcpDocumentsByConnection.value,
      [connectionId]: nextDocuments,
    }
    await refreshManagementProjection(connectionId)
  }

  async function createTool(record: ToolRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.catalog.createTool(record)
    context.toolsByConnection.value = {
      ...context.toolsByConnection.value,
      [connectionId]: [...(context.toolsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function updateTool(toolId: string, record: ToolRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.catalog.updateTool(toolId, record)
    context.toolsByConnection.value = {
      ...context.toolsByConnection.value,
      [connectionId]: (context.toolsByConnection.value[connectionId] ?? []).map(item => item.id === toolId ? updated : item),
    }
    return updated
  }

  async function removeTool(toolId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.catalog.deleteTool(toolId)
    context.toolsByConnection.value = {
      ...context.toolsByConnection.value,
      [connectionId]: (context.toolsByConnection.value[connectionId] ?? []).filter(item => item.id !== toolId),
    }
  }

  return {
    load,
    refreshManagementProjection,
    setAssetDisabled,
    getSkillDocument,
    getSkillTreeDocument,
    getSkillFileDocument,
    createSkill,
    updateSkill,
    updateSkillFile,
    importSkillArchive,
    importSkillFolder,
    copySkillToManaged,
    deleteSkill,
    getMcpServerDocument,
    createMcpServer,
    copyMcpServerToManaged,
    updateMcpServer,
    deleteMcpServer,
    createTool,
    updateTool,
    removeTool,
  }
}
