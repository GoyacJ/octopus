import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  CreateWorkspaceResourceFolderInput,
  CreateWorkspaceResourceInput,
  UpdateWorkspaceResourceInput,
  WorkspaceResourceRecord,
} from '@octopus/schema'

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

  async function createWorkspaceResource(input: CreateWorkspaceResourceInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.createWorkspace(input)
    workspaceResourcesByConnection.value = {
      ...workspaceResourcesByConnection.value,
      [connectionId]: [...(workspaceResourcesByConnection.value[connectionId] ?? []), record],
    }
    return record
  }

  async function createProjectResource(projectId: string, input: CreateWorkspaceResourceInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.createProject(projectId, input)
    const key = `${connectionId}:${projectId}`
    projectResources.value = {
      ...projectResources.value,
      [key]: [...(projectResources.value[key] ?? []), record],
    }
    return record
  }

  async function createProjectResourceFolder(projectId: string, input: CreateWorkspaceResourceFolderInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const records = await client.resources.createProjectFolder(projectId, input)
    const key = `${connectionId}:${projectId}`
    projectResources.value = {
      ...projectResources.value,
      [key]: [...(projectResources.value[key] ?? []), ...records],
    }
    return records
  }

  async function updateWorkspaceResource(resourceId: string, input: UpdateWorkspaceResourceInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.updateWorkspace(resourceId, input)
    const resources = workspaceResourcesByConnection.value[connectionId] ?? []
    workspaceResourcesByConnection.value = {
      ...workspaceResourcesByConnection.value,
      [connectionId]: resources.map(r => r.id === resourceId ? record : r),
    }
    return record
  }

  async function updateProjectResource(projectId: string, resourceId: string, input: UpdateWorkspaceResourceInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.updateProject(projectId, resourceId, input)
    const key = `${connectionId}:${projectId}`
    const resources = projectResources.value[key] ?? []
    projectResources.value = {
      ...projectResources.value,
      [key]: resources.map(r => r.id === resourceId ? record : r),
    }
    return record
  }

  async function deleteWorkspaceResource(resourceId: string) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    await client.resources.deleteWorkspace(resourceId)
    const resources = workspaceResourcesByConnection.value[connectionId] ?? []
    workspaceResourcesByConnection.value = {
      ...workspaceResourcesByConnection.value,
      [connectionId]: resources.filter(r => r.id !== resourceId),
    }
  }

  async function deleteProjectResource(projectId: string, resourceId: string) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    await client.resources.deleteProject(projectId, resourceId)
    const key = `${connectionId}:${projectId}`
    const resources = projectResources.value[key] ?? []
    projectResources.value = {
      ...projectResources.value,
      [key]: resources.filter(r => r.id !== resourceId),
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
    createWorkspaceResource,
    createProjectResource,
    createProjectResourceFolder,
    updateWorkspaceResource,
    updateProjectResource,
    deleteWorkspaceResource,
    deleteProjectResource,
    clearWorkspaceScope,
  }
})
