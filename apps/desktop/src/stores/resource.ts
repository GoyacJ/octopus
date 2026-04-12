import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  CreateProjectPromotionRequestInput,
  CreateWorkspaceResourceFolderInput,
  CreateWorkspaceResourceInput,
  PromoteWorkspaceResourceInput,
  ProjectPromotionRequest,
  UpdateWorkspaceResourceInput,
  WorkspaceResourceChildrenRecord,
  WorkspaceResourceContentDocument,
  WorkspaceResourceImportInput,
  WorkspaceResourceRecord,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

type ResourceMap<T> = Record<string, Record<string, T>>

function upsertResourceRecord(records: WorkspaceResourceRecord[], record: WorkspaceResourceRecord) {
  const index = records.findIndex(item => item.id === record.id)
  if (index < 0) {
    return [...records, record]
  }
  return records.map(item => item.id === record.id ? record : item)
}

export const useResourceStore = defineStore('resource', () => {
  const workspaceResourcesByConnection = ref<Record<string, WorkspaceResourceRecord[]>>({})
  const projectResources = ref<Record<string, WorkspaceResourceRecord[]>>({})
  const detailByConnection = ref<ResourceMap<WorkspaceResourceRecord>>({})
  const contentByConnection = ref<ResourceMap<WorkspaceResourceContentDocument>>({})
  const childrenByConnection = ref<ResourceMap<WorkspaceResourceChildrenRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const workspaceResources = computed(() => workspaceResourcesByConnection.value[activeConnectionId.value] ?? [])
  const activeProjectResources = computed(() => projectResourcesFor(workspaceStore.currentProjectId))
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

  function projectKey(connectionId: string, projectId: string) {
    return `${connectionId}:${projectId}`
  }

  function projectResourcesFor(projectId?: string, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    if (!connectionId || !projectId) {
      return []
    }
    return projectResources.value[projectKey(connectionId, projectId)] ?? []
  }

  function cacheDetail(connectionId: string, record: WorkspaceResourceRecord) {
    detailByConnection.value = {
      ...detailByConnection.value,
      [connectionId]: {
        ...(detailByConnection.value[connectionId] ?? {}),
        [record.id]: record,
      },
    }
  }

  function cacheContent(connectionId: string, content: WorkspaceResourceContentDocument) {
    contentByConnection.value = {
      ...contentByConnection.value,
      [connectionId]: {
        ...(contentByConnection.value[connectionId] ?? {}),
        [content.resourceId]: content,
      },
    }
  }

  function cacheChildren(connectionId: string, resourceId: string, children: WorkspaceResourceChildrenRecord[]) {
    childrenByConnection.value = {
      ...childrenByConnection.value,
      [connectionId]: {
        ...(childrenByConnection.value[connectionId] ?? {}),
        [resourceId]: children,
      },
    }
  }

  function replaceWorkspaceResource(connectionId: string, record: WorkspaceResourceRecord) {
    cacheDetail(connectionId, record)
    workspaceResourcesByConnection.value = {
      ...workspaceResourcesByConnection.value,
      [connectionId]: upsertResourceRecord(workspaceResourcesByConnection.value[connectionId] ?? [], record),
    }
  }

  function replaceProjectResource(connectionId: string, projectId: string, record: WorkspaceResourceRecord) {
    cacheDetail(connectionId, record)
    const key = projectKey(connectionId, projectId)
    projectResources.value = {
      ...projectResources.value,
      [key]: upsertResourceRecord(projectResources.value[key] ?? [], record),
    }
  }

  function removeResourceRecord(connectionId: string, resourceId: string, projectId?: string) {
    workspaceResourcesByConnection.value = {
      ...workspaceResourcesByConnection.value,
      [connectionId]: (workspaceResourcesByConnection.value[connectionId] ?? []).filter(record => record.id !== resourceId),
    }

    if (projectId) {
      const key = projectKey(connectionId, projectId)
      projectResources.value = {
        ...projectResources.value,
        [key]: (projectResources.value[key] ?? []).filter(record => record.id !== resourceId),
      }
    } else {
      Object.keys(projectResources.value)
        .filter(key => key.startsWith(`${connectionId}:`))
        .forEach((key) => {
          projectResources.value = {
            ...projectResources.value,
            [key]: projectResources.value[key]!.filter(record => record.id !== resourceId),
          }
        })
    }

    const nextDetails = { ...(detailByConnection.value[connectionId] ?? {}) }
    delete nextDetails[resourceId]
    detailByConnection.value = {
      ...detailByConnection.value,
      [connectionId]: nextDetails,
    }

    const nextContents = { ...(contentByConnection.value[connectionId] ?? {}) }
    delete nextContents[resourceId]
    contentByConnection.value = {
      ...contentByConnection.value,
      [connectionId]: nextContents,
    }

    const nextChildren = { ...(childrenByConnection.value[connectionId] ?? {}) }
    delete nextChildren[resourceId]
    childrenByConnection.value = {
      ...childrenByConnection.value,
      [connectionId]: nextChildren,
    }
  }

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
      const cached = Object.fromEntries(records.map(record => [record.id, record]))
      detailByConnection.value = {
        ...detailByConnection.value,
        [connectionId]: {
          ...(detailByConnection.value[connectionId] ?? {}),
          ...cached,
        },
      }
      errors.value = {
        ...errors.value,
        [connectionId]: '',
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
        [projectKey(connectionId, projectId)]: records,
      }
      const cached = Object.fromEntries(records.map(record => [record.id, record]))
      detailByConnection.value = {
        ...detailByConnection.value,
        [connectionId]: {
          ...(detailByConnection.value[connectionId] ?? {}),
          ...cached,
        },
      }
      errors.value = {
        ...errors.value,
        [connectionId]: '',
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
    replaceWorkspaceResource(connectionId, record)
    return record
  }

  async function createProjectResource(projectId: string, input: CreateWorkspaceResourceInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.createProject(projectId, input)
    replaceProjectResource(connectionId, projectId, record)
    replaceWorkspaceResource(connectionId, record)
    return record
  }

  async function createProjectResourceFolder(projectId: string, input: CreateWorkspaceResourceFolderInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const records = await client.resources.createProjectFolder(projectId, input)
    records.forEach((record) => {
      replaceProjectResource(connectionId, projectId, record)
      replaceWorkspaceResource(connectionId, record)
    })
    return records
  }

  async function importWorkspaceResource(input: WorkspaceResourceImportInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.importWorkspace(input)
    replaceWorkspaceResource(connectionId, record)
    return record
  }

  async function importProjectResource(projectId: string, input: WorkspaceResourceImportInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.importProject(projectId, input)
    replaceProjectResource(connectionId, projectId, record)
    replaceWorkspaceResource(connectionId, record)
    return record
  }

  async function getResourceDetail(resourceId: string, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const cached = detailByConnection.value[connectionId]?.[resourceId]
    if (cached) {
      return cached
    }
    const record = await client.resources.getDetail(resourceId)
    cacheDetail(connectionId, record)
    return record
  }

  async function getResourceContent(resourceId: string, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const cached = contentByConnection.value[connectionId]?.[resourceId]
    if (cached) {
      return cached
    }
    const content = await client.resources.getContent(resourceId)
    cacheContent(connectionId, content)
    return content
  }

  async function loadResourceChildren(resourceId: string, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const cached = childrenByConnection.value[connectionId]?.[resourceId]
    if (cached) {
      return cached
    }
    const children = await client.resources.listChildren(resourceId)
    cacheChildren(connectionId, resourceId, children)
    return children
  }

  async function promoteResource(resourceId: string, input: PromoteWorkspaceResourceInput, projectId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.promote(resourceId, input)
    if (projectId ?? record.projectId) {
      replaceProjectResource(connectionId, projectId ?? record.projectId!, record)
    } else {
      replaceWorkspaceResource(connectionId, record)
    }
    replaceWorkspaceResource(connectionId, record)
    return record
  }

  async function submitProjectPromotionRequest(
    projectId: string,
    input: CreateProjectPromotionRequestInput,
  ): Promise<ProjectPromotionRequest> {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client } = resolvedClient
    return await client.projects.createPromotionRequest(projectId, input)
  }

  async function updateWorkspaceResource(resourceId: string, input: UpdateWorkspaceResourceInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.updateWorkspace(resourceId, input)
    replaceWorkspaceResource(connectionId, record)
    return record
  }

  async function updateProjectResource(projectId: string, resourceId: string, input: UpdateWorkspaceResourceInput) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    const record = await client.resources.updateProject(projectId, resourceId, input)
    replaceProjectResource(connectionId, projectId, record)
    replaceWorkspaceResource(connectionId, record)
    return record
  }

  async function deleteWorkspaceResource(resourceId: string) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    await client.resources.deleteWorkspace(resourceId)
    removeResourceRecord(connectionId, resourceId)
  }

  async function deleteProjectResource(projectId: string, resourceId: string) {
    const resolvedClient = resolveWorkspaceClientForConnection()
    if (!resolvedClient) {
      throw new Error('No active workspace connection')
    }
    const { client, connectionId } = resolvedClient
    await client.resources.deleteProject(projectId, resourceId)
    removeResourceRecord(connectionId, resourceId, projectId)
  }

  function getCachedDetail(resourceId: string, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    return detailByConnection.value[connectionId]?.[resourceId] ?? null
  }

  function getCachedContent(resourceId: string, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    return contentByConnection.value[connectionId]?.[resourceId] ?? null
  }

  function getCachedChildren(resourceId: string, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    return childrenByConnection.value[connectionId]?.[resourceId] ?? null
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const nextWorkspaceResources = { ...workspaceResourcesByConnection.value }
    const nextDetails = { ...detailByConnection.value }
    const nextContents = { ...contentByConnection.value }
    const nextChildren = { ...childrenByConnection.value }
    const nextErrors = { ...errors.value }
    const nextTokens = { ...requestTokens.value }
    delete nextWorkspaceResources[workspaceConnectionId]
    delete nextDetails[workspaceConnectionId]
    delete nextContents[workspaceConnectionId]
    delete nextChildren[workspaceConnectionId]
    delete nextErrors[workspaceConnectionId]
    delete nextTokens[workspaceConnectionId]
    workspaceResourcesByConnection.value = nextWorkspaceResources
    detailByConnection.value = nextDetails
    contentByConnection.value = nextContents
    childrenByConnection.value = nextChildren
    errors.value = nextErrors
    requestTokens.value = nextTokens

    const nextProjectResources = { ...projectResources.value }
    Object.keys(nextProjectResources)
      .filter(key => key.startsWith(`${workspaceConnectionId}:`))
      .forEach((key) => {
        delete nextProjectResources[key]
      })
    projectResources.value = nextProjectResources
  }

  return {
    workspaceResources,
    activeProjectResources,
    error,
    projectResourcesFor,
    getCachedDetail,
    getCachedContent,
    getCachedChildren,
    loadWorkspaceResources,
    loadProjectResources,
    createWorkspaceResource,
    createProjectResource,
    createProjectResourceFolder,
    importWorkspaceResource,
    importProjectResource,
    getResourceDetail,
    getResourceContent,
    loadResourceChildren,
    promoteResource,
    submitProjectPromotionRequest,
    updateWorkspaceResource,
    updateProjectResource,
    deleteWorkspaceResource,
    deleteProjectResource,
    clearWorkspaceScope,
  }
})
