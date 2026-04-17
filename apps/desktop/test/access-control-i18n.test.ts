import { afterEach, describe, expect, it } from 'vitest'

import type { AccessMemberSummary, MenuDefinition, PermissionDefinition } from '@octopus/schema'

import i18n from '@/plugins/i18n'
import {
  createProjectAccessPolicyName,
  getAccessCapabilityBundleName,
  getAccessMemberEffectiveRoleNames,
  getAccessMemberPresetName,
  getAccessMenuLabel,
  getAccessPresetName,
  getAccessTemplateName,
  getAccessRoleName,
  getAuditActionLabel,
  getAuditResourceLabel,
  getPermissionDisplayDescription,
  getPermissionDisplayName,
  getResourceActionLabel,
} from '@/views/workspace/access-control/display-i18n'

const originalLocale = i18n.global.locale.value

describe('access-control display i18n', () => {
  afterEach(() => {
    i18n.global.locale.value = originalLocale
  })

  it('localizes system presets, templates, bundles, roles, and menu labels by stable codes', () => {
    const menu = {
      id: 'menu-workspace-access-control',
      parentId: undefined,
      label: '访问控制',
      routeName: 'workspace-access-control',
      source: 'main-sidebar',
      status: 'active',
      order: 100,
      featureCode: 'feature:menu-workspace-access-control',
    } satisfies MenuDefinition

    i18n.global.locale.value = 'zh-CN'
    expect(getAccessPresetName('owner', 'Owner')).toBe('所有者')
    expect(getAccessTemplateName('auditor', 'Auditor')).toBe('审计员')
    expect(getAccessCapabilityBundleName('workspace_governance', 'Workspace Governance')).toBe('工作区治理')
    expect(getAccessRoleName({
      code: 'system.owner',
      name: 'Owner',
      source: 'system',
    })).toBe('所有者')
    expect(getAccessMenuLabel(menu)).toBe('访问控制')

    i18n.global.locale.value = 'en-US'
    expect(getAccessPresetName('owner', 'Owner')).toBe('Owner')
    expect(getAccessTemplateName('auditor', 'Auditor')).toBe('Auditor')
    expect(getAccessCapabilityBundleName('workspace_governance', 'Workspace Governance')).toBe('Workspace Governance')
    expect(getAccessRoleName({
      code: 'system.owner',
      name: 'Owner',
      source: 'system',
    })).toBe('Owner')
    expect(getAccessMenuLabel(menu)).toBe('Access Control')
  })

  it('localizes system member roles while preserving custom role names', () => {
    const member = {
      user: {
        id: 'user-owner',
        username: 'owner',
        displayName: 'Workspace Owner',
        status: 'active',
        passwordState: 'set',
      },
      primaryPresetCode: 'owner',
      primaryPresetName: 'Owner',
      effectiveRoles: [
        {
          id: 'role-system-owner',
          code: 'system.owner',
          name: 'Owner',
          source: 'system',
        },
        {
          id: 'role-qa-lead',
          code: 'custom.qa-lead',
          name: 'QA Lead',
          source: 'custom',
        },
      ],
      effectiveRoleNames: ['Owner', 'QA Lead'],
      hasOrgAssignments: false,
    } as unknown as AccessMemberSummary

    i18n.global.locale.value = 'zh-CN'
    expect(getAccessMemberPresetName(member)).toBe('所有者')
    expect(getAccessMemberEffectiveRoleNames(member)).toEqual(['所有者', 'QA Lead'])

    i18n.global.locale.value = 'en-US'
    expect(getAccessMemberPresetName(member)).toBe('Owner')
    expect(getAccessMemberEffectiveRoleNames(member)).toEqual(['Owner', 'QA Lead'])
  })

  it('localizes permission, resource action, and audit display values with raw fallbacks', () => {
    const permission = {
      code: 'tool.mcp.bind-credential',
      name: 'Tool MCP Bind Credential',
      description: 'Bind credentials for MCP tools.',
      category: 'tool',
      resourceType: 'tool.mcp',
      actions: ['bind-credential'],
    } satisfies PermissionDefinition

    i18n.global.locale.value = 'zh-CN'
    expect(getPermissionDisplayName(permission)).toBe('MCP 工具绑定凭据')
    expect(getPermissionDisplayDescription(permission)).toBe('允许为 MCP 工具绑定凭据。')
    expect(getResourceActionLabel('invoke')).toBe('调用')
    expect(getResourceActionLabel('sync-tenant')).toBe('sync-tenant')
    expect(getAuditActionLabel('access.users.preset.update')).toBe('更新成员访问预设')
    expect(getAuditResourceLabel('access.user:user-operator')).toBe('成员：user-operator')

    i18n.global.locale.value = 'en-US'
    expect(getPermissionDisplayName(permission)).toBe('MCP Tools Credential Binding')
    expect(getPermissionDisplayDescription(permission)).toBe('Allows credential binding for MCP tools.')
    expect(getResourceActionLabel('invoke')).toBe('Invoke')
    expect(getAuditActionLabel('access.users.preset.update')).toBe('Update member preset')
    expect(getAuditResourceLabel('access.user:user-operator')).toBe('Member: user-operator')
  })

  it('builds localized names for generated project-access policies', () => {
    i18n.global.locale.value = 'zh-CN'
    expect(createProjectAccessPolicyName('Workspace Operator')).toBe('Workspace Operator 项目访问')

    i18n.global.locale.value = 'en-US'
    expect(createProjectAccessPolicyName('Workspace Operator')).toBe('Workspace Operator project access')
  })
})
