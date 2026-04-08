import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { AutomationRecord } from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

export const useAutomationStore = defineStore('automation', () => {
  const recordsByConnection = ref<Record<string, AutomationRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const automations = computed(() => recordsByConnection.value[activeConnectionId.value] ?? [])
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
      const records = await client.automations.list()
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      recordsByConnection.value = {
        ...recordsByConnection.value,
        [connectionId]: records,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load automations',
        }
      }
    }
  }

  async function create(record: AutomationRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.automations.create(record)
    recordsByConnection.value = {
      ...recordsByConnection.value,
      [connectionId]: [...(recordsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function update(automationId: string, record: AutomationRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.automations.update(automationId, record)
    recordsByConnection.value = {
      ...recordsByConnection.value,
      [connectionId]: (recordsByConnection.value[connectionId] ?? []).map(item => item.id === automationId ? updated : item),
    }
    return updated
  }

  async function remove(automationId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.automations.delete(automationId)
    recordsByConnection.value = {
      ...recordsByConnection.value,
      [connectionId]: (recordsByConnection.value[connectionId] ?? []).filter(item => item.id !== automationId),
    }
  }

  return {
    automations,
    error,
    load,
    create,
    update,
    remove,
  }
})
