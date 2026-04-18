import { describe, expect, it } from 'vitest'

import { resolveEnabledProjectAgentIds } from '@/stores/project_settings'
import {
  buildProjectGrantState,
  resolveProjectToolSettings,
} from '@/stores/project_setup'

describe('project runtime semantics', () => {
  it('treats exclusion assignments as live workspace inheritance', () => {
    const grantState = buildProjectGrantState({
      id: 'proj-redesign',
      assignments: {
        tools: {
          excludedSourceKeys: ['mcp:ops'],
        },
        agents: {
          excludedAgentIds: ['agent-coder'],
          excludedTeamIds: [],
        },
      },
      ownerUserId: 'user-owner',
      memberUserIds: ['user-owner'],
    } as any, {
      workspaceToolSourceKeys: ['builtin:bash', 'mcp:ops'],
      workspaceAgentIds: ['agent-architect', 'agent-coder'],
      workspaceTeamIds: ['team-studio'],
    })

    expect(grantState.assignedToolSourceKeys).toEqual(['builtin:bash'])
    expect(grantState.assignedAgentIds).toEqual(['agent-architect'])
    expect(grantState.assignedTeamIds).toEqual(['team-studio'])
  })

  it('resolves enabled runtime selections from disabled arrays', () => {
    const resolvedTools = resolveProjectToolSettings(
      {
        tools: {
          disabledSourceKeys: ['mcp:ops'],
          overrides: {},
        },
      },
      [
        { sourceKey: 'builtin:bash' },
        { sourceKey: 'mcp:ops' },
      ] as any,
    )
    const resolvedActors = resolveEnabledProjectAgentIds(
      {
        agents: {
          disabledAgentIds: ['agent-coder'],
          disabledTeamIds: ['team-review'],
        },
      },
      ['agent-architect', 'agent-coder'],
      ['team-studio', 'team-review'],
    )

    expect(resolvedTools.enabledSourceKeys).toEqual(['builtin:bash'])
    expect(resolvedActors.enabledAgentIds).toEqual(['agent-architect'])
    expect(resolvedActors.enabledTeamIds).toEqual(['team-studio'])
  })
})
