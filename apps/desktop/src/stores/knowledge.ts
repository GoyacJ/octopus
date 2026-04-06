import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type { KnowledgeRecord } from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

export const useKnowledgeStore = defineStore('knowledge', () => {
  const workspaceKnowledgeByConnection = ref<Record<string, KnowledgeRecord[]>>({})
  const projectKnowledge = ref<Record<string, KnowledgeRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const workspaceKnowledge = computed(() => workspaceKnowledgeByConnection.value[activeConnectionId.value] ?? [])
  const activeProjectKnowledge = computed(() => {
    if (!activeConnectionId.value || !workspaceStore.currentProjectId) {
      return []
    }
    return projectKnowledge.value[`${activeConnectionId.value}:${workspaceStore.currentProjectId}`] ?? []
  })
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')

  async function loadWorkspaceKnowledge(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const token = createWorkspaceRequestToken(requestTokens.value[connectionId] ?? 0)
    requestTokens.value[connectionId] = token
    try {
      const records = await client.knowledge.listWorkspace()
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      workspaceKnowledgeByConnection.value = {
        ...workspaceKnowledgeByConnection.value,
        [connectionId]: records,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load workspace knowledge',
        }
      }
    }
  }

  async function loadProjectKnowledge(projectId = workspaceStore.currentProjectId, workspaceConnectionId?: string) {
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
      const records = await client.knowledge.listProject(projectId)
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      projectKnowledge.value = {
        ...projectKnowledge.value,
        [`${connectionId}:${projectId}`]: records,
      }
    } catch (cause) {
      if (requestTokens.value[connectionId] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load project knowledge',
        }
      }
    }
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const nextWorkspaceKnowledge = { ...workspaceKnowledgeByConnection.value }
    const nextErrors = { ...errors.value }
    const nextTokens = { ...requestTokens.value }
    delete nextWorkspaceKnowledge[workspaceConnectionId]
    delete nextErrors[workspaceConnectionId]
    delete nextTokens[workspaceConnectionId]
    workspaceKnowledgeByConnection.value = nextWorkspaceKnowledge
    errors.value = nextErrors
    requestTokens.value = nextTokens
    Object.keys(projectKnowledge.value)
      .filter(key => key.startsWith(`${workspaceConnectionId}:`))
      .forEach((key) => {
        delete projectKnowledge.value[key]
      })
  }

  return {
    workspaceKnowledge,
    activeProjectKnowledge,
    error,
    loadWorkspaceKnowledge,
    loadProjectKnowledge,
    clearWorkspaceScope,
  }
})
