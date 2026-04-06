import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ModelCatalogSnapshot,
  ProviderCredentialRecord,
  ToolRecord,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

export const useCatalogStore = defineStore('catalog', () => {
  const snapshots = ref<Record<string, ModelCatalogSnapshot>>({})
  const toolsByConnection = ref<Record<string, ToolRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const snapshot = computed(() => snapshots.value[activeConnectionId.value] ?? { models: [], providerCredentials: [] })
  const models = computed(() => snapshot.value.models)
  const providerCredentials = computed<ProviderCredentialRecord[]>(() => snapshot.value.providerCredentials)
  const tools = computed(() => toolsByConnection.value[activeConnectionId.value] ?? [])
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

  async function load(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    try {
      const [nextSnapshot, nextTools] = await Promise.all([
        client.catalog.getSnapshot(),
        client.catalog.listTools(),
      ])
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      snapshots.value = {
        ...snapshots.value,
        [connectionId]: nextSnapshot,
      }
      toolsByConnection.value = {
        ...toolsByConnection.value,
        [connectionId]: nextTools,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load model catalog',
        }
      }
    }
  }

  async function createTool(record: ToolRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.catalog.createTool(record)
    toolsByConnection.value = {
      ...toolsByConnection.value,
      [connectionId]: [...(toolsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function updateTool(toolId: string, record: ToolRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.catalog.updateTool(toolId, record)
    toolsByConnection.value = {
      ...toolsByConnection.value,
      [connectionId]: (toolsByConnection.value[connectionId] ?? []).map(item => item.id === toolId ? updated : item),
    }
    return updated
  }

  async function removeTool(toolId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.catalog.deleteTool(toolId)
    toolsByConnection.value = {
      ...toolsByConnection.value,
      [connectionId]: (toolsByConnection.value[connectionId] ?? []).filter(item => item.id !== toolId),
    }
  }

  return {
    snapshot,
    models,
    providerCredentials,
    tools,
    error,
    load,
    createTool,
    updateTool,
    removeTool,
  }
})
