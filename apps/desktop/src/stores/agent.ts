import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { AgentRecord } from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

export const useAgentStore = defineStore('agent', () => {
  const agentsByConnection = ref<Record<string, AgentRecord[]>>({})
  const errors = ref<Record<string, string>>({})
  const requestTokens = ref<Record<string, number>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const agents = computed(() => agentsByConnection.value[activeConnectionId.value] ?? [])
  const workspaceAgents = computed(() => agents.value.filter(record => !record.projectId))
  const projectAgents = computed(() => agents.value.filter(record => record.projectId === workspaceStore.currentProjectId))
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
      const records = await client.agents.list()
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      agentsByConnection.value = {
        ...agentsByConnection.value,
        [connectionId]: records,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load agents',
        }
      }
    }
  }

  async function create(record: AgentRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.agents.create(record)
    agentsByConnection.value = {
      ...agentsByConnection.value,
      [connectionId]: [...(agentsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function update(agentId: string, record: AgentRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.agents.update(agentId, record)
    agentsByConnection.value = {
      ...agentsByConnection.value,
      [connectionId]: (agentsByConnection.value[connectionId] ?? []).map(item => item.id === agentId ? updated : item),
    }
    return updated
  }

  async function remove(agentId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.agents.delete(agentId)
    agentsByConnection.value = {
      ...agentsByConnection.value,
      [connectionId]: (agentsByConnection.value[connectionId] ?? []).filter(item => item.id !== agentId),
    }
  }

  return {
    agents,
    workspaceAgents,
    projectAgents,
    error,
    load,
    create,
    update,
    remove,
  }
})
