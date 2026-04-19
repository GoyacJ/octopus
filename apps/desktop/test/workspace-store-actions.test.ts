// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'
import * as tauriClient from '@/tauri/client'
import { installWorkspaceApiFixture } from './support/workspace-fixture'
import { updateFixtureProjectSettings } from './support/workspace-fixture-project-settings'

describe('useWorkspaceStore actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    window.localStorage.clear()
    vi.restoreAllMocks()
    installWorkspaceApiFixture()
  })

  async function prepareWorkspaceStore() {
    const shell = useShellStore()
    await shell.bootstrap('ws-local', 'proj-redesign', [])
    const workspace = useWorkspaceStore()
    await workspace.bootstrap()
    return { shell, workspace }
  }

  it('updates workspace settings through the shared workspace client and syncs overview state', async () => {
    const { workspace } = await prepareWorkspaceStore()

    const updated = await workspace.updateWorkspace({
      name: 'Workspace HQ',
      mappedDirectory: '/Users/goya/Workspace HQ',
    })

    expect(updated?.name).toBe('Workspace HQ')
    expect(updated?.mappedDirectory).toBe('/Users/goya/Workspace HQ')
    expect(workspace.activeWorkspace?.name).toBe('Workspace HQ')
    expect(workspace.activeOverview?.workspace.name).toBe('Workspace HQ')
    expect(workspace.activeOverview?.workspace.mappedDirectory).toBe('/Users/goya/Workspace HQ')
  })

  it('archives a project without resubmitting legacy assignments payloads', async () => {
    const { workspace } = await prepareWorkspaceStore()

    const baseImplementation = vi.mocked(tauriClient.createWorkspaceClient).getMockImplementation()
    expect(baseImplementation).toBeTypeOf('function')

    const capturedUpdates: unknown[] = []
    vi.mocked(tauriClient.createWorkspaceClient).mockImplementation((context) => {
      const client = baseImplementation!(context)
      return {
        ...client,
        projects: {
          ...client.projects,
          async update(projectId, input) {
            capturedUpdates.push(input)
            return await client.projects.update(projectId, input)
          },
        },
      }
    })

    await workspace.archiveProject('proj-redesign')

    expect(capturedUpdates).toHaveLength(1)
    expect(capturedUpdates[0]).toMatchObject({
      name: 'Desktop Redesign',
      description: 'Real workspace API migration for the desktop surface.',
      resourceDirectory: 'data/projects/proj-redesign/resources',
      status: 'archived',
    })
    expect(capturedUpdates[0]).not.toHaveProperty('assignments')
  })

  it('saves runtime tool settings without echoing legacy enabled source keys', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }
        updateFixtureProjectSettings(state, 'proj-redesign', current => ({
          ...current,
          tools: {
            ...(current.tools ?? { disabledSourceKeys: [], overrides: {} }),
            enabledSourceKeys: ['builtin:bash'],
          } as any,
        }))
      },
    })

    const { workspace } = await prepareWorkspaceStore()
    await workspace.loadProjectRuntimeConfig('proj-redesign', true)

    const baseImplementation = vi.mocked(tauriClient.createWorkspaceClient).getMockImplementation()
    expect(baseImplementation).toBeTypeOf('function')

    const capturedPatches: unknown[] = []
    vi.mocked(tauriClient.createWorkspaceClient).mockImplementation((context) => {
      const client = baseImplementation!(context)
      return {
        ...client,
        runtime: {
          ...client.runtime,
          async saveProjectConfig(projectId, patch) {
            capturedPatches.push(patch.patch)
            return await client.runtime.saveProjectConfig(projectId, patch)
          },
        },
      }
    })

    await workspace.saveProjectToolSettings('proj-redesign', {
      disabledSourceKeys: ['builtin:bash'],
      overrides: {},
    })

    expect(capturedPatches).toHaveLength(1)
    expect(capturedPatches[0]).toMatchObject({
      projectSettings: {
        tools: {
          disabledSourceKeys: ['builtin:bash'],
          overrides: {},
        },
      },
    })
    expect(capturedPatches[0]).not.toHaveProperty('projectSettings.tools.enabledSourceKeys')
  })

  it('saves runtime actor settings without echoing legacy enabled actor lists', async () => {
    installWorkspaceApiFixture({
      stateTransform(state, connection) {
        if (connection.workspaceId !== 'ws-local') {
          return
        }
        updateFixtureProjectSettings(state, 'proj-redesign', current => ({
          ...current,
          agents: {
            ...(current.agents ?? { disabledAgentIds: [], disabledTeamIds: [] }),
            enabledAgentIds: ['agent-architect'],
            enabledTeamIds: ['team-studio'],
          } as any,
        }))
      },
    })

    const { workspace } = await prepareWorkspaceStore()
    await workspace.loadProjectRuntimeConfig('proj-redesign', true)

    const baseImplementation = vi.mocked(tauriClient.createWorkspaceClient).getMockImplementation()
    expect(baseImplementation).toBeTypeOf('function')

    const capturedPatches: unknown[] = []
    vi.mocked(tauriClient.createWorkspaceClient).mockImplementation((context) => {
      const client = baseImplementation!(context)
      return {
        ...client,
        runtime: {
          ...client.runtime,
          async saveProjectConfig(projectId, patch) {
            capturedPatches.push(patch.patch)
            return await client.runtime.saveProjectConfig(projectId, patch)
          },
        },
      }
    })

    await workspace.saveProjectAgentSettings('proj-redesign', {
      disabledAgentIds: ['agent-coder'],
      disabledTeamIds: ['team-redesign'],
    })

    expect(capturedPatches).toHaveLength(1)
    expect(capturedPatches[0]).toMatchObject({
      projectSettings: {
        agents: {
          disabledAgentIds: ['agent-coder'],
          disabledTeamIds: ['team-redesign'],
        },
      },
    })
    expect(capturedPatches[0]).not.toHaveProperty('projectSettings.agents.enabledAgentIds')
    expect(capturedPatches[0]).not.toHaveProperty('projectSettings.agents.enabledTeamIds')
  })
})
