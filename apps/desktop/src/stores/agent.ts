import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  AgentRecord,
  ImportWorkspaceAgentBundleInput,
  ImportWorkspaceAgentBundlePreviewInput,
  ProjectAgentLinkInput,
  ProjectAgentLinkRecord,
  UpsertAgentInput,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

function withIntegrationSource(
  record: AgentRecord,
  link: ProjectAgentLinkRecord,
): AgentRecord {
  return {
    ...record,
    integrationSource: {
      kind: 'workspace-link',
      sourceId: link.agentId,
    },
  }
}

export const useAgentStore = defineStore('agent', () => {
  const agentsByConnection = ref<Record<string, AgentRecord[]>>({})
  const projectLinksByConnection = ref<Record<string, Record<string, ProjectAgentLinkRecord[]>>>({})
  const errors = ref<Record<string, string>>({})
  const requestTokens = ref<Record<string, number>>({})
  const projectLinkRequestTokens = ref<Record<string, number>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const agents = computed(() => agentsByConnection.value[activeConnectionId.value] ?? [])
  const workspaceAgents = computed(() => agents.value.filter(record => !record.projectId))
  const projectOwnedAgents = computed(() => agents.value.filter(record => record.projectId === workspaceStore.currentProjectId))
  const projectLinks = computed<Record<string, ProjectAgentLinkRecord[]>>(
    () => projectLinksByConnection.value[activeConnectionId.value] ?? {},
  )
  const currentProjectLinks = computed<ProjectAgentLinkRecord[]>(
    () => projectLinks.value[workspaceStore.currentProjectId ?? ''] ?? [],
  )
  const integratedProjectAgents = computed(() => {
    const linkMap = new Map(currentProjectLinks.value.map(link => [link.agentId, link]))
    return workspaceAgents.value
      .filter(record => linkMap.has(record.id))
      .map(record => withIntegrationSource(record, linkMap.get(record.id)!))
  })
  const projectAgents = computed(() => [
    ...projectOwnedAgents.value,
    ...integratedProjectAgents.value,
  ])
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
      errors.value = {
        ...errors.value,
        [connectionId]: '',
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

  async function loadProjectLinks(projectId: string, workspaceConnectionId?: string) {
    if (!projectId) {
      return
    }
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }
    const { client, connectionId } = resolvedClient
    const requestKey = `${connectionId}:${projectId}`
    const token = createWorkspaceRequestToken(projectLinkRequestTokens.value[requestKey] ?? 0)
    projectLinkRequestTokens.value[requestKey] = token
    try {
      const records = await client.agents.listProjectLinks(projectId)
      if (projectLinkRequestTokens.value[requestKey] !== token) {
        return
      }
      projectLinksByConnection.value = {
        ...projectLinksByConnection.value,
        [connectionId]: {
          ...(projectLinksByConnection.value[connectionId] ?? {}),
          [projectId]: records,
        },
      }
    } catch (cause) {
      if (projectLinkRequestTokens.value[requestKey] === token) {
        errors.value = {
          ...errors.value,
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load project agent links',
        }
      }
    }
  }

  async function create(input: UpsertAgentInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.agents.create(input)
    agentsByConnection.value = {
      ...agentsByConnection.value,
      [connectionId]: [...(agentsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function update(agentId: string, input: UpsertAgentInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.agents.update(agentId, input)
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
    projectLinksByConnection.value = {
      ...projectLinksByConnection.value,
      [connectionId]: Object.fromEntries(
        Object.entries(projectLinksByConnection.value[connectionId] ?? {}).map(([projectId, links]) => [
          projectId,
          links.filter(link => link.agentId !== agentId),
        ]),
      ),
    }
  }

  async function previewImportBundle(input: ImportWorkspaceAgentBundlePreviewInput) {
    const { client } = ensureWorkspaceClientForConnection()
    return await client.agents.previewImportBundle(input)
  }

  async function importBundle(input: ImportWorkspaceAgentBundleInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const result = await client.agents.importBundle(input)
    await load(connectionId)
    return result
  }

  async function linkProject(input: ProjectAgentLinkInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.agents.linkProject(input)
    projectLinksByConnection.value = {
      ...projectLinksByConnection.value,
      [connectionId]: {
        ...(projectLinksByConnection.value[connectionId] ?? {}),
        [input.projectId]: [
          ...((projectLinksByConnection.value[connectionId] ?? {})[input.projectId] ?? []).filter(link => link.agentId !== input.agentId),
          created,
        ],
      },
    }
    return created
  }

  async function unlinkProject(projectId: string, agentId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.agents.unlinkProject(projectId, agentId)
    projectLinksByConnection.value = {
      ...projectLinksByConnection.value,
      [connectionId]: {
        ...(projectLinksByConnection.value[connectionId] ?? {}),
        [projectId]: ((projectLinksByConnection.value[connectionId] ?? {})[projectId] ?? []).filter(link => link.agentId !== agentId),
      },
    }
  }

  return {
    agents,
    workspaceAgents,
    projectOwnedAgents,
    integratedProjectAgents,
    projectAgents,
    currentProjectLinks,
    error,
    load,
    loadProjectLinks,
    create,
    update,
    remove,
    previewImportBundle,
    importBundle,
    linkProject,
    unlinkProject,
  }
})
