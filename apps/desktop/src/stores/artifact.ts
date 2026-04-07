import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { ArtifactRecord } from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

export const useArtifactStore = defineStore('artifact', () => {
  const workspaceArtifactsByConnection = ref<Record<string, ArtifactRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const workspaceArtifacts = computed(() => workspaceArtifactsByConnection.value[activeConnectionId.value] ?? [])
  const activeProjectArtifacts = computed(() => {
    if (!activeConnectionId.value || !workspaceStore.currentProjectId) {
      return []
    }
    return workspaceArtifacts.value.filter(artifact => !artifact.projectId || artifact.projectId === workspaceStore.currentProjectId)
  })
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

  async function loadWorkspaceArtifacts(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    try {
      const records = await client.artifacts.listWorkspace()
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      workspaceArtifactsByConnection.value = {
        ...workspaceArtifactsByConnection.value,
        [connectionId]: records,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load workspace artifacts',
        }
      }
    }
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const nextArtifacts = { ...workspaceArtifactsByConnection.value }
    const nextErrors = { ...errors.value }
    const nextTokens = { ...requestTokens.value }
    delete nextArtifacts[workspaceConnectionId]
    delete nextErrors[workspaceConnectionId]
    delete nextTokens[workspaceConnectionId]
    workspaceArtifactsByConnection.value = nextArtifacts
    errors.value = nextErrors
    requestTokens.value = nextTokens
  }

  return {
    workspaceArtifacts,
    activeProjectArtifacts,
    error,
    loadWorkspaceArtifacts,
    clearWorkspaceScope,
  }
})
