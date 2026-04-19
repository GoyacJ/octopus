import { describe, expect, it } from 'vitest'

import {
  resolveGrantedAgentsWithExclusions,
  resolveGrantedTeamsWithExclusions,
  resolveGrantedToolEntriesWithExclusions,
  resolveProjectGrantedAgents,
  resolveProjectGrantedTeams,
  resolveProjectGrantedToolEntries,
} from '@/stores/project_setup'
import { parseProjectSettingsDocument } from '@/stores/project_settings'

describe('project setup selectors', () => {
  it('resolves granted tool entries from workspace baseline and project-owned assets minus exclusions', () => {
    const granted = resolveGrantedToolEntriesWithExclusions({
      projectId: 'proj-redesign',
      toolEntries: [
        {
          sourceKey: 'builtin:bash',
          enabled: true,
          ownerScope: 'workspace',
          ownerId: 'ws-local',
          kind: 'builtin',
          name: 'bash',
        },
        {
          sourceKey: 'builtin:read_file',
          enabled: false,
          ownerScope: 'workspace',
          ownerId: 'ws-local',
          kind: 'builtin',
          name: 'read_file',
        },
        {
          sourceKey: 'skill:project-custom',
          enabled: true,
          ownerScope: 'project',
          ownerId: 'proj-redesign',
          kind: 'skill',
          name: 'project-custom',
        },
      ] as any,
      excludedSourceKeys: ['builtin:bash'],
    })

    expect(granted.map(entry => entry.sourceKey)).toEqual(['skill:project-custom'])
  })

  it('resolves granted actors from workspace baseline and project-owned assets minus exclusions', () => {
    const grantedAgents = resolveGrantedAgentsWithExclusions({
      workspaceAgents: [
        { id: 'agent-workspace', status: 'active' },
        { id: 'agent-excluded', status: 'active' },
        { id: 'agent-inactive', status: 'archived' },
      ] as any,
      projectOwnedAgents: [
        { id: 'agent-project', projectId: 'proj-redesign', status: 'active' },
      ] as any,
      excludedAgentIds: ['agent-excluded'],
    })
    const grantedTeams = resolveGrantedTeamsWithExclusions({
      workspaceTeams: [
        { id: 'team-workspace', status: 'active' },
        { id: 'team-excluded', status: 'active' },
        { id: 'team-inactive', status: 'archived' },
      ] as any,
      projectOwnedTeams: [
        { id: 'team-project', projectId: 'proj-redesign', status: 'active' },
      ] as any,
      excludedTeamIds: ['team-excluded'],
    })

    expect(grantedAgents.map(agent => agent.id)).toEqual(['agent-project', 'agent-workspace'])
    expect(grantedTeams.map(team => team.id)).toEqual(['team-project', 'team-workspace'])
  })

  it('resolves granted tools, agents, and teams from current deltas instead of legacy enabled lists', () => {
    const projectSettings = parseProjectSettingsDocument({
      projectSettings: {
        tools: {
          enabledSourceKeys: ['builtin:bash'],
        },
        agents: {
          enabledAgentIds: ['agent-workspace'],
          enabledTeamIds: ['team-workspace'],
        },
      },
    } as any)

    const grantedTools = resolveProjectGrantedToolEntries(
      { id: 'proj-redesign' } as any,
      [
        {
          sourceKey: 'builtin:bash',
          enabled: true,
          ownerScope: 'workspace',
          ownerId: 'ws-local',
          kind: 'builtin',
          name: 'bash',
        },
        {
          sourceKey: 'builtin:read_file',
          enabled: true,
          ownerScope: 'workspace',
          ownerId: 'ws-local',
          kind: 'builtin',
          name: 'read_file',
        },
        {
          sourceKey: 'skill:project-custom',
          enabled: true,
          ownerScope: 'project',
          ownerId: 'proj-redesign',
          kind: 'skill',
          name: 'project-custom',
        },
      ] as any,
      projectSettings,
    )
    const grantedAgents = resolveProjectGrantedAgents(
      { id: 'proj-redesign' } as any,
      [
        { id: 'agent-workspace', status: 'active' },
        { id: 'agent-secondary', status: 'active' },
      ] as any,
      [
        { id: 'agent-project', projectId: 'proj-redesign', status: 'active' },
      ] as any,
      projectSettings,
    )
    const grantedTeams = resolveProjectGrantedTeams(
      { id: 'proj-redesign' } as any,
      [
        { id: 'team-workspace', status: 'active' },
        { id: 'team-secondary', status: 'active' },
      ] as any,
      [
        { id: 'team-project', projectId: 'proj-redesign', status: 'active' },
      ] as any,
      projectSettings,
    )

    expect(grantedTools.map(entry => entry.sourceKey)).toEqual([
      'builtin:bash',
      'builtin:read_file',
      'skill:project-custom',
    ])
    expect(grantedAgents.map(agent => agent.id)).toEqual([
      'agent-project',
      'agent-workspace',
      'agent-secondary',
    ])
    expect(grantedTeams.map(team => team.id)).toEqual([
      'team-project',
      'team-workspace',
      'team-secondary',
    ])
  })
})
