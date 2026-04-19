import { describe, expect, it } from 'vitest'

import {
  canAccessProjectSettings,
  canReviewProjectDeletion,
  canShowProjectInShell,
  resolveProjectModulePermission,
} from '@/composables/project-governance'

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

  it('allows project settings access for owners and scoped governance reviewers', () => {
    const project = {
      ownerUserId: 'user-owner',
      memberUserIds: ['user-owner', 'user-member'],
    }

    expect(canAccessProjectSettings(project, 'user-owner', [], [])).toBe(true)
    expect(canAccessProjectSettings(project, 'user-member', ['project.manage'], [])).toBe(true)
    expect(canAccessProjectSettings(project, 'user-member', [], ['system.admin'])).toBe(true)
    expect(canAccessProjectSettings(project, 'user-member', [], [])).toBe(false)
  })

  it('keeps project shell navigation visible for members and governance reviewers', () => {
    const project = {
      ownerUserId: 'user-owner',
      memberUserIds: ['user-owner', 'user-member'],
    }

    expect(canShowProjectInShell(project, 'user-member', [], [])).toBe(true)
    expect(canShowProjectInShell(project, 'user-reviewer', ['project.manage'], [])).toBe(true)
    expect(canShowProjectInShell(project, 'user-reviewer', [], ['system.admin'])).toBe(true)
    expect(canShowProjectInShell(project, 'user-reviewer', [], [])).toBe(false)
  })

  it('allows delete review for project managers and system governance roles', () => {
    expect(canReviewProjectDeletion(['project.manage'], [])).toBe(true)
    expect(canReviewProjectDeletion([], ['system.owner'])).toBe(true)
    expect(canReviewProjectDeletion([], ['system.admin'])).toBe(true)
    expect(canReviewProjectDeletion([], [])).toBe(false)
  })
})
