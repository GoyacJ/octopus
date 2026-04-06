import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { TeamRecord } from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

export const useTeamStore = defineStore('team', () => {
  const teamsByConnection = ref<Record<string, TeamRecord[]>>({})
  const errors = ref<Record<string, string>>({})
  const requestTokens = ref<Record<string, number>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const teams = computed(() => teamsByConnection.value[activeConnectionId.value] ?? [])
  const workspaceTeams = computed(() => teams.value.filter(record => !record.projectId))
  const projectTeams = computed(() => teams.value.filter(record => record.projectId === workspaceStore.currentProjectId))
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
      const records = await client.teams.list()
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      teamsByConnection.value = {
        ...teamsByConnection.value,
        [connectionId]: records,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load teams',
        }
      }
    }
  }

  async function create(record: TeamRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.teams.create(record)
    teamsByConnection.value = {
      ...teamsByConnection.value,
      [connectionId]: [...(teamsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function update(teamId: string, record: TeamRecord) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.teams.update(teamId, record)
    teamsByConnection.value = {
      ...teamsByConnection.value,
      [connectionId]: (teamsByConnection.value[connectionId] ?? []).map(item => item.id === teamId ? updated : item),
    }
    return updated
  }

  async function remove(teamId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.teams.delete(teamId)
    teamsByConnection.value = {
      ...teamsByConnection.value,
      [connectionId]: (teamsByConnection.value[connectionId] ?? []).filter(item => item.id !== teamId),
    }
  }

  return {
    teams,
    workspaceTeams,
    projectTeams,
    error,
    load,
    create,
    update,
    remove,
  }
})
