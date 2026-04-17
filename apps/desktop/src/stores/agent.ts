import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  AgentRecord,
  ExportWorkspaceAgentBundleInput,
  ExportWorkspaceAgentBundleResult,
  ImportWorkspaceAgentBundleInput,
  ImportWorkspaceAgentBundleResult,
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
  const inflightLoads = new Map<string, Promise<void>>()

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const agents = computed(() => agentsByConnection.value[activeConnectionId.value] ?? [])
  const workspaceAgents = computed(() => agents.value.filter(record => !record.projectId))
  const workspaceOwnedAgents = computed(() =>
    workspaceAgents.value.filter(record => record.integrationSource?.kind !== 'builtin-template'),
  )
  const builtinTemplateAgents = computed(() =>
    workspaceAgents.value.filter(record => record.integrationSource?.kind === 'builtin-template'),
  )
  const projectOwnedAgents = computed(() => agents.value.filter(record => record.projectId === workspaceStore.currentProjectId))
  const currentProject = computed(() =>
    workspaceStore.projects.find(project => project.id === workspaceStore.currentProjectId) ?? null,
  )
  const assignedProjectAgentIds = computed(() =>
    currentProject.value?.assignments?.agents?.agentIds ?? [],
  )
  const projectLinks = computed<Record<string, ProjectAgentLinkRecord[]>>(
    () => projectLinksByConnection.value[activeConnectionId.value] ?? {},
  )
  const currentProjectLinks = computed<ProjectAgentLinkRecord[]>(
    () => projectLinks.value[workspaceStore.currentProjectId ?? ''] ?? [],
  )
  const integratedProjectAgents = computed(() => {
    const linkMap = new Map(currentProjectLinks.value.map(link => [link.agentId, link]))
    return workspaceOwnedAgents.value
      .filter(record => assignedProjectAgentIds.value.includes(record.id) || linkMap.has(record.id))
      .map(record => withIntegrationSource(record, linkMap.get(record.id) ?? {
        workspaceId: record.workspaceId,
        projectId: workspaceStore.currentProjectId,
        agentId: record.id,
        linkedAt: 0,
      }))
  })
  const assignedWorkspaceAgents = computed(() =>
    workspaceOwnedAgents.value.filter(record => assignedProjectAgentIds.value.includes(record.id)),
  )
  const assignedBuiltinAgents = computed(() =>
    builtinTemplateAgents.value.filter(record => assignedProjectAgentIds.value.includes(record.id)),
  )
  const effectiveProjectAgents = computed(() => {
    const merged = [
      ...projectOwnedAgents.value,
      ...assignedWorkspaceAgents.value,
      ...assignedBuiltinAgents.value,
    ]
    return merged.filter((record, index) => merged.findIndex(item => item.id === record.id) === index)
  })
  const projectAgents = computed(() => effectiveProjectAgents.value)
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

  async function ensureLoaded(
    workspaceConnectionId?: string,
    options: { force?: boolean } = {},
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return
    }

    const { connectionId } = resolvedClient
    if (!options.force && Object.prototype.hasOwnProperty.call(agentsByConnection.value, connectionId)) {
      return
    }

    const inflight = inflightLoads.get(connectionId)
    if (inflight && !options.force) {
      await inflight
      return
    }

    const task = load(connectionId)
    inflightLoads.set(connectionId, task)
    try {
      await task
    } finally {
      if (inflightLoads.get(connectionId) === task) {
        inflightLoads.delete(connectionId)
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

  async function previewImportBundle(input: ImportWorkspaceAgentBundlePreviewInput, projectId?: string) {
    const { client } = ensureWorkspaceClientForConnection()
    return await client.agents.previewImportBundle(input, projectId)
  }

  async function importBundle(input: ImportWorkspaceAgentBundleInput, projectId?: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const result = await client.agents.importBundle(input, projectId)
    await load(connectionId)
    if (projectId) {
      await loadProjectLinks(projectId, connectionId)
    }
    return result
  }

  async function exportBundle(input: ExportWorkspaceAgentBundleInput, projectId?: string): Promise<ExportWorkspaceAgentBundleResult> {
    const { client } = ensureWorkspaceClientForConnection()
    return await client.agents.exportBundle(input, projectId)
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

  async function copyToWorkspace(agentId: string): Promise<ImportWorkspaceAgentBundleResult> {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const result = await client.agents.copyToWorkspace(agentId)
    await load(connectionId)
    return result
  }

  async function copyToProject(projectId: string, agentId: string): Promise<ImportWorkspaceAgentBundleResult> {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const result = await client.agents.copyToProject(projectId, agentId)
    await Promise.all([
      load(connectionId),
      loadProjectLinks(projectId, connectionId),
    ])
    return result
  }

  return {
    agents,
    workspaceAgents,
    workspaceOwnedAgents,
    builtinTemplateAgents,
    projectOwnedAgents,
    integratedProjectAgents,
    assignedWorkspaceAgents,
    assignedBuiltinAgents,
    effectiveProjectAgents,
    projectAgents,
    currentProjectLinks,
    error,
    load,
    ensureLoaded,
    loadProjectLinks,
    create,
    update,
    remove,
    previewImportBundle,
    importBundle,
    exportBundle,
    linkProject,
    unlinkProject,
    copyToWorkspace,
    copyToProject,
  }
})
