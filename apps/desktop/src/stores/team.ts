import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ProjectTeamLinkInput,
  ProjectTeamLinkRecord,
  TeamRecord,
  UpsertTeamInput,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useWorkspaceStore } from './workspace'

function withIntegrationSource(
  record: TeamRecord,
  link: ProjectTeamLinkRecord,
): TeamRecord {
  return {
    ...record,
    integrationSource: {
      kind: 'workspace-link',
      sourceId: link.teamId,
    },
  }
}

export const useTeamStore = defineStore('team', () => {
  const teamsByConnection = ref<Record<string, TeamRecord[]>>({})
  const projectLinksByConnection = ref<Record<string, Record<string, ProjectTeamLinkRecord[]>>>({})
  const errors = ref<Record<string, string>>({})
  const requestTokens = ref<Record<string, number>>({})
  const projectLinkRequestTokens = ref<Record<string, number>>({})

  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const teams = computed(() => teamsByConnection.value[activeConnectionId.value] ?? [])
  const workspaceTeams = computed(() => teams.value.filter(record => !record.projectId))
  const projectOwnedTeams = computed(() => teams.value.filter(record => record.projectId === workspaceStore.currentProjectId))
  const projectLinks = computed<Record<string, ProjectTeamLinkRecord[]>>(
    () => projectLinksByConnection.value[activeConnectionId.value] ?? {},
  )
  const currentProjectLinks = computed<ProjectTeamLinkRecord[]>(
    () => projectLinks.value[workspaceStore.currentProjectId ?? ''] ?? [],
  )
  const integratedProjectTeams = computed(() => {
    const linkMap = new Map(currentProjectLinks.value.map(link => [link.teamId, link]))
    return workspaceTeams.value
      .filter(record => linkMap.has(record.id))
      .map(record => withIntegrationSource(record, linkMap.get(record.id)!))
  })
  const projectTeams = computed(() => [
    ...projectOwnedTeams.value,
    ...integratedProjectTeams.value,
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
      const records = await client.teams.list()
      if (requestTokens.value[connectionId] !== token) {
        return
      }
      teamsByConnection.value = {
        ...teamsByConnection.value,
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
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load teams',
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
      const records = await client.teams.listProjectLinks(projectId)
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
          [connectionId]: cause instanceof Error ? cause.message : 'Failed to load project team links',
        }
      }
    }
  }

  async function create(input: UpsertTeamInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.teams.create(input)
    teamsByConnection.value = {
      ...teamsByConnection.value,
      [connectionId]: [...(teamsByConnection.value[connectionId] ?? []), created],
    }
    return created
  }

  async function update(teamId: string, input: UpsertTeamInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const updated = await client.teams.update(teamId, input)
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
    projectLinksByConnection.value = {
      ...projectLinksByConnection.value,
      [connectionId]: Object.fromEntries(
        Object.entries(projectLinksByConnection.value[connectionId] ?? {}).map(([projectId, links]) => [
          projectId,
          links.filter(link => link.teamId !== teamId),
        ]),
      ),
    }
  }

  async function linkProject(input: ProjectTeamLinkInput) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    const created = await client.teams.linkProject(input)
    projectLinksByConnection.value = {
      ...projectLinksByConnection.value,
      [connectionId]: {
        ...(projectLinksByConnection.value[connectionId] ?? {}),
        [input.projectId]: [
          ...((projectLinksByConnection.value[connectionId] ?? {})[input.projectId] ?? []).filter(link => link.teamId !== input.teamId),
          created,
        ],
      },
    }
    return created
  }

  async function unlinkProject(projectId: string, teamId: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection()
    await client.teams.unlinkProject(projectId, teamId)
    projectLinksByConnection.value = {
      ...projectLinksByConnection.value,
      [connectionId]: {
        ...(projectLinksByConnection.value[connectionId] ?? {}),
        [projectId]: ((projectLinksByConnection.value[connectionId] ?? {})[projectId] ?? []).filter(link => link.teamId !== teamId),
      },
    }
  }

  return {
    teams,
    workspaceTeams,
    projectOwnedTeams,
    integratedProjectTeams,
    projectTeams,
    currentProjectLinks,
    error,
    load,
    loadProjectLinks,
    create,
    update,
    remove,
    linkProject,
    unlinkProject,
  }
})
