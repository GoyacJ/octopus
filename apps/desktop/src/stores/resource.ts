import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { WorkspaceResourceRecord } from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

export const useResourceStore = defineStore('resource', () => {
  const workspaceResourcesByConnection = ref<Record<string, WorkspaceResourceRecord[]>>({})
  const projectResources = ref<Record<string, WorkspaceResourceRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const workspaceResources = computed(() => workspaceResourcesByConnection.value[activeConnectionId.value] ?? [])
  const activeProjectResources = computed(() => {
    if (!activeConnectionId.value || !workspaceStore.currentProjectId) {
      return []
    }
    return projectResources.value[`${activeConnectionId.value}:${workspaceStore.currentProjectId}`] ?? []
  })
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

  async function loadWorkspaceResources(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    try {
      const records = await client.resources.listWorkspace()
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      workspaceResourcesByConnection.value = {
        ...workspaceResourcesByConnection.value,
        [connectionId]: records,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load workspace resources',
        }
      }
    }
  }

  async function loadProjectResources(projectId = workspaceStore.currentProjectId, workspaceConnectionId?: string) {
    if (!projectId) {
      return
    }
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    try {
      const records = await client.resources.listProject(projectId)
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      projectResources.value = {
        ...projectResources.value,
        [`${connectionId}:${projectId}`]: records,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load project resources',
        }
      }
    }
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const nextWorkspaceResources = { ...workspaceResourcesByConnection.value }
    const nextErrors = { ...errors.value }
    const nextTokens = { ...requestTokens.value }
    delete nextWorkspaceResources[workspaceConnectionId]
    delete nextErrors[workspaceConnectionId]
    delete nextTokens[workspaceConnectionId]
    workspaceResourcesByConnection.value = nextWorkspaceResources
    errors.value = nextErrors
    requestTokens.value = nextTokens
    Object.keys(projectResources.value)
      .filter(key => key.startsWith(`${workspaceConnectionId}:`))
      .forEach((key) => {
        delete projectResources.value[key]
      })
  }

  return {
    workspaceResources,
    activeProjectResources,
    error,
    loadWorkspaceResources,
    loadProjectResources,
    clearWorkspaceScope,
  }
})
