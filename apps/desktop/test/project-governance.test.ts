import { describe, expect, it } from 'vitest'

import { resolveProjectModulePermission } from '@/composables/project-governance'

describe('project governance', () => {
  it('supports task module permissions from workspace defaults and project overrides', () => {
    expect(resolveProjectModulePermission({
      projectDefaultPermissions: {
        agents: 'allow',
        resources: 'allow',
        tools: 'allow',
        knowledge: 'allow',
        tasks: 'deny',
      },
    }, {
      permissionOverrides: {
        agents: 'inherit',
        resources: 'inherit',
        tools: 'inherit',
        knowledge: 'inherit',
        tasks: 'inherit',
      },
    }, 'tasks')).toBe('deny')

    expect(resolveProjectModulePermission({
      projectDefaultPermissions: {
        agents: 'allow',
        resources: 'allow',
        tools: 'allow',
        knowledge: 'allow',
        tasks: 'deny',
      },
    }, {
      permissionOverrides: {
        agents: 'inherit',
        resources: 'inherit',
        tools: 'inherit',
        knowledge: 'inherit',
        tasks: 'allow',
      },
    }, 'tasks')).toBe('allow')
  })
})
