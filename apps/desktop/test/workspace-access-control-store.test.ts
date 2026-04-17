// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import type {
  AccessExperienceResponse,
  AccessMemberSummary,
  AuthorizationSnapshot,
  MenuDefinition,
  WorkspaceConnectionRecord,
  WorkspaceSessionTokenEnvelope,
} from '@octopus/schema'

import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import * as tauriClient from '@/tauri/client'

const connection: WorkspaceConnectionRecord = {
  workspaceConnectionId: 'conn-local',
  workspaceId: 'ws-local',
  label: 'Local Workspace',
  baseUrl: 'http://127.0.0.1:43127',
  transportSecurity: 'loopback',
  authMode: 'session-token',
  status: 'connected',
}

const session: WorkspaceSessionTokenEnvelope = {
  workspaceConnectionId: 'conn-local',
  token: 'token-local',
  issuedAt: 1,
  session: {
    id: 'sess-local',
    workspaceId: 'ws-local',
    userId: 'user-owner',
    clientAppId: 'octopus-desktop',
    token: 'token-local',
    status: 'active',
    createdAt: 1,
    expiresAt: undefined,
  },
}

const authorization: AuthorizationSnapshot = {
  principal: {
    id: 'user-owner',
    username: 'owner',
    displayName: 'Workspace Owner',
    status: 'active',
    passwordState: 'set',
  },
  orgAssignments: [],
  effectiveRoleIds: ['role-system-owner'],
  effectiveRoles: [{
    id: 'role-system-owner',
    code: 'system.owner',
    name: 'Owner',
    description: 'Full workspace access.',
    source: 'system',
    editable: false,
    status: 'active',
    permissionCodes: [
      'access.users.read',
      'access.users.manage',
      'access.roles.read',
      'access.roles.manage',
      'access.org.read',
      'access.policies.read',
      'access.menus.read',
      'access.sessions.read',
    ],
  }],
  effectivePermissionCodes: [
    'access.users.read',
    'access.users.manage',
    'access.roles.read',
    'access.roles.manage',
    'access.org.read',
    'access.policies.read',
    'access.menus.read',
    'access.sessions.read',
  ],
  featureCodes: ['feature:menu-workspace-access-control'],
  visibleMenuIds: ['menu-workspace-access-control'],
  menuGates: [],
  resourceActionGrants: [],
}

const menuDefinitions: MenuDefinition[] = [{
  id: 'menu-workspace-access-control',
  parentId: undefined,
  label: '访问控制',
  routeName: 'workspace-access-control',
  source: 'main-sidebar',
  status: 'active',
  order: 100,
  featureCode: 'feature:menu-workspace-access-control',
}]

const experience: AccessExperienceResponse = {
  summary: {
    experienceLevel: 'team',
    memberCount: 2,
    hasOrgStructure: false,
    hasCustomRoles: false,
    hasAdvancedPolicies: false,
    hasMenuGovernance: false,
    hasResourceGovernance: false,
    recommendedLandingSection: 'members',
  },
  sectionGrants: [
    { section: 'members', allowed: true },
    { section: 'access', allowed: true },
    { section: 'governance', allowed: true },
  ],
  roleTemplates: [
    {
      code: 'owner',
      name: 'Owner',
      description: 'Manage the workspace.',
      managedRoleCodes: ['system.owner'],
      editable: false,
    },
    {
      code: 'admin',
      name: 'Admin',
      description: 'Operate the workspace.',
      managedRoleCodes: ['system.admin'],
      editable: false,
    },
  ],
  rolePresets: [
    {
      code: 'owner',
      name: 'Owner',
      description: 'Manage the workspace.',
      recommendedFor: 'Workspace operators',
      templateCodes: ['owner'],
      capabilityBundleCodes: ['workspace_governance'],
    },
    {
      code: 'admin',
      name: 'Admin',
      description: 'Operate the workspace.',
      recommendedFor: 'Workspace operators',
      templateCodes: ['admin'],
      capabilityBundleCodes: ['workspace_governance'],
    },
  ],
  capabilityBundles: [
    {
      code: 'workspace_governance',
      name: 'Workspace governance',
      description: 'Manage workspace governance settings.',
      permissionCodes: ['access.users.manage', 'access.roles.manage'],
    },
  ],
  counts: {
    auditEventCount: 1,
    customRoleCount: 0,
    dataPolicyCount: 0,
    menuPolicyCount: 0,
    orgUnitCount: 0,
    protectedResourceCount: 0,
    resourcePolicyCount: 0,
    sessionCount: 1,
  },
}

const members: AccessMemberSummary[] = [
  {
    user: {
      id: 'user-owner',
      username: 'owner',
      displayName: 'Workspace Owner',
      status: 'active',
      passwordState: 'set',
    },
    primaryPresetCode: 'owner',
    primaryPresetName: 'Owner',
    effectiveRoles: [{
      id: 'role-system-owner',
      code: 'system.owner',
      name: 'Owner',
      source: 'system',
    }],
    effectiveRoleNames: ['Owner'],
    hasOrgAssignments: false,
  },
  {
    user: {
      id: 'user-operator',
      username: 'operator',
      displayName: 'Workspace Operator',
      status: 'active',
      passwordState: 'set',
    },
    primaryPresetCode: 'admin',
    primaryPresetName: 'Admin',
    effectiveRoles: [{
      id: 'role-system-admin',
      code: 'system.admin',
      name: 'Admin',
      source: 'system',
    }],
    effectiveRoleNames: ['Admin'],
    hasOrgAssignments: false,
  },
]

describe('useWorkspaceAccessControlStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()

    const shell = useShellStore()
    shell.workspaceConnectionsState = [connection]
    shell.activeWorkspaceConnectionId = connection.workspaceConnectionId
    shell.workspaceSessionsState = {
      [connection.workspaceConnectionId]: session,
    }
  })

  it('loads authorization, experience, and member summaries without eagerly fetching governance datasets', async () => {
    const getCurrentAuthorization = vi.fn().mockResolvedValue(authorization)
    const listMenuDefinitions = vi.fn().mockResolvedValue(menuDefinitions)
    const getAccessExperience = vi.fn().mockResolvedValue(experience)
    const listMembers = vi.fn().mockResolvedValue(members)
    const listUsers = vi.fn().mockResolvedValue([])
    const listRoles = vi.fn().mockResolvedValue([])

    vi.spyOn(tauriClient, 'createWorkspaceClient').mockReturnValue({
      accessControl: {
        getCurrentAuthorization,
        listMenuDefinitions,
        getAccessExperience,
        listMembers,
        listUsers,
        listRoles,
      },
    } as unknown as ReturnType<typeof tauriClient.createWorkspaceClient>)

    const store = useWorkspaceAccessControlStore()
    await store.loadMembersData(connection.workspaceConnectionId)

    expect(getCurrentAuthorization).toHaveBeenCalledTimes(1)
    expect(listMenuDefinitions).toHaveBeenCalledTimes(1)
    expect(getAccessExperience).toHaveBeenCalledTimes(1)
    expect(listMembers).toHaveBeenCalledTimes(1)
    expect(listUsers).not.toHaveBeenCalled()
    expect(listRoles).not.toHaveBeenCalled()
    expect(store.members).toHaveLength(2)
    expect(store.membersByPresetCode.get('admin')?.[0]?.user.id).toBe('user-operator')
    expect(store.recommendedAccessSection).toBe('members')
  })

  it('loads governance datasets only when the governance layer is requested', async () => {
    const getCurrentAuthorization = vi.fn().mockResolvedValue(authorization)
    const listMenuDefinitions = vi.fn().mockResolvedValue(menuDefinitions)
    const getAccessExperience = vi.fn().mockResolvedValue(experience)
    const listMembers = vi.fn().mockResolvedValue(members)
    const listAudit = vi.fn().mockResolvedValue({ items: [], nextCursor: undefined })
    const listSessions = vi.fn().mockResolvedValue([])
    const listUsers = vi.fn().mockResolvedValue([members[0]?.user])
    const listOrgUnits = vi.fn().mockResolvedValue([])
    const listPositions = vi.fn().mockResolvedValue([])
    const listUserGroups = vi.fn().mockResolvedValue([])
    const listUserOrgAssignments = vi.fn().mockResolvedValue([])
    const listRoles = vi.fn().mockResolvedValue([authorization.effectiveRoles[0]])
    const listPermissionDefinitions = vi.fn().mockResolvedValue([])
    const listRoleBindings = vi.fn().mockResolvedValue([])
    const listDataPolicies = vi.fn().mockResolvedValue([])
    const listResourcePolicies = vi.fn().mockResolvedValue([])
    const listFeatureDefinitions = vi.fn().mockResolvedValue([])
    const listMenuGateResults = vi.fn().mockResolvedValue([])
    const listMenuPolicies = vi.fn().mockResolvedValue([])
    const listProtectedResources = vi.fn().mockResolvedValue([])

    vi.spyOn(tauriClient, 'createWorkspaceClient').mockReturnValue({
      accessControl: {
        getCurrentAuthorization,
        listMenuDefinitions,
        getAccessExperience,
        listMembers,
        listAudit,
        listSessions,
        listUsers,
        listOrgUnits,
        listPositions,
        listUserGroups,
        listUserOrgAssignments,
        listRoles,
        listPermissionDefinitions,
        listRoleBindings,
        listDataPolicies,
        listResourcePolicies,
        listFeatureDefinitions,
        listMenuGateResults,
        listMenuPolicies,
        listProtectedResources,
      },
    } as unknown as ReturnType<typeof tauriClient.createWorkspaceClient>)

    const store = useWorkspaceAccessControlStore()
    await store.loadGovernanceData(connection.workspaceConnectionId)

    expect(getAccessExperience).toHaveBeenCalledTimes(1)
    expect(listMembers).toHaveBeenCalledTimes(1)
    expect(listUsers).toHaveBeenCalledTimes(1)
    expect(listRoles).toHaveBeenCalledTimes(1)
    expect(listDataPolicies).toHaveBeenCalledTimes(1)
    expect(store.members).toHaveLength(2)
    expect(store.users).toHaveLength(1)
    expect(store.roles).toHaveLength(1)
  })

  it('derives sidebar access visibility from authorization permissions before experience loads', async () => {
    const getCurrentAuthorization = vi.fn().mockResolvedValue({
      ...authorization,
      effectiveRoleIds: ['role-auditor'],
      effectiveRoles: [{
        id: 'role-auditor',
        code: 'custom.auditor',
        name: 'Auditor',
        description: 'Review governance activity.',
        source: 'custom',
        editable: true,
        status: 'active',
        permissionCodes: ['audit.read'],
      }],
      effectivePermissionCodes: ['audit.read'],
      featureCodes: [],
      visibleMenuIds: [],
    } satisfies AuthorizationSnapshot)
    const listMenuDefinitions = vi.fn().mockResolvedValue(menuDefinitions)

    vi.spyOn(tauriClient, 'createWorkspaceClient').mockReturnValue({
      accessControl: {
        getCurrentAuthorization,
        listMenuDefinitions,
      },
    } as unknown as ReturnType<typeof tauriClient.createWorkspaceClient>)

    const store = useWorkspaceAccessControlStore()
    await store.ensureAuthorizationContext(connection.workspaceConnectionId)

    expect(store.experience).toBeNull()
    expect(store.sidebarAccessSectionGrants).toEqual({
      members: false,
      access: false,
      governance: true,
    })
    expect(store.canShowAccessControlNavigation).toBe(true)
  })

  it('assigns a preset through the lightweight members layer and refreshes experience and members', async () => {
    const getCurrentAuthorization = vi.fn().mockResolvedValue(authorization)
    const listMenuDefinitions = vi.fn().mockResolvedValue(menuDefinitions)
    const getAccessExperience = vi
      .fn()
      .mockResolvedValueOnce(experience)
      .mockResolvedValueOnce({
        ...experience,
        summary: {
          ...experience.summary,
          memberCount: 2,
        },
      })
    const listMembers = vi
      .fn()
      .mockResolvedValueOnce(members)
      .mockResolvedValueOnce([
        members[0],
        {
          ...members[1],
          primaryPresetCode: 'owner',
          primaryPresetName: 'Owner',
          effectiveRoleNames: ['Owner'],
        },
      ])
    const updateUserPreset = vi.fn().mockResolvedValue({
      ...members[1],
      primaryPresetCode: 'owner',
      primaryPresetName: 'Owner',
      effectiveRoleNames: ['Owner'],
    })

    vi.spyOn(tauriClient, 'createWorkspaceClient').mockReturnValue({
      accessControl: {
        getCurrentAuthorization,
        listMenuDefinitions,
        getAccessExperience,
        listMembers,
        updateUserPreset,
      },
    } as unknown as ReturnType<typeof tauriClient.createWorkspaceClient>)

    const store = useWorkspaceAccessControlStore()
    await store.loadMembersData(connection.workspaceConnectionId)
    await store.assignUserPreset('user-operator', { presetCode: 'owner' }, connection.workspaceConnectionId)

    expect(updateUserPreset).toHaveBeenCalledWith('user-operator', { presetCode: 'owner' })
    expect(getAccessExperience).toHaveBeenCalledTimes(2)
    expect(listMembers).toHaveBeenCalledTimes(2)
    expect(store.members.find(member => member.user.id === 'user-operator')?.primaryPresetCode).toBe('owner')
  })

  it('keeps low-noise governance empty for basic project access policies', async () => {
    const getCurrentAuthorization = vi.fn().mockResolvedValue(authorization)
    const listMenuDefinitions = vi.fn().mockResolvedValue(menuDefinitions)
    const getAccessExperience = vi.fn().mockResolvedValue({
      ...experience,
      summary: {
        ...experience.summary,
        experienceLevel: 'team',
        hasAdvancedPolicies: false,
        hasCustomRoles: false,
        hasMenuGovernance: false,
        hasOrgStructure: false,
        hasResourceGovernance: false,
      },
      counts: {
        ...experience.counts,
        dataPolicyCount: 1,
      },
    })
    const listMembers = vi.fn().mockResolvedValue(members)
    const listAudit = vi.fn().mockResolvedValue({ items: [], nextCursor: undefined })
    const listSessions = vi.fn().mockResolvedValue([])
    const listUsers = vi.fn().mockResolvedValue([members[0]?.user])
    const listOrgUnits = vi.fn().mockResolvedValue([])
    const listPositions = vi.fn().mockResolvedValue([])
    const listUserGroups = vi.fn().mockResolvedValue([])
    const listUserOrgAssignments = vi.fn().mockResolvedValue([])
    const listRoles = vi.fn().mockResolvedValue([authorization.effectiveRoles[0]])
    const listPermissionDefinitions = vi.fn().mockResolvedValue([])
    const listRoleBindings = vi.fn().mockResolvedValue([])
    const listDataPolicies = vi.fn().mockResolvedValue([{
      id: 'policy-user-operator-projects',
      name: 'Operator project access',
      subjectType: 'user',
      subjectId: 'user-operator',
      resourceType: 'project',
      scopeType: 'selected-projects',
      projectIds: ['proj-redesign'],
      tags: [],
      classifications: [],
      effect: 'allow',
    }])
    const listResourcePolicies = vi.fn().mockResolvedValue([])
    const listFeatureDefinitions = vi.fn().mockResolvedValue([])
    const listMenuGateResults = vi.fn().mockResolvedValue([])
    const listMenuPolicies = vi.fn().mockResolvedValue([])
    const listProtectedResources = vi.fn().mockResolvedValue([])

    vi.spyOn(tauriClient, 'createWorkspaceClient').mockReturnValue({
      accessControl: {
        getCurrentAuthorization,
        listMenuDefinitions,
        getAccessExperience,
        listMembers,
        listAudit,
        listSessions,
        listUsers,
        listOrgUnits,
        listPositions,
        listUserGroups,
        listUserOrgAssignments,
        listRoles,
        listPermissionDefinitions,
        listRoleBindings,
        listDataPolicies,
        listResourcePolicies,
        listFeatureDefinitions,
        listMenuGateResults,
        listMenuPolicies,
        listProtectedResources,
      },
    } as unknown as ReturnType<typeof tauriClient.createWorkspaceClient>)

    const store = useWorkspaceAccessControlStore()
    await store.loadGovernanceData(connection.workspaceConnectionId)

    expect(store.isGovernanceEmpty).toBe(true)
  })

  it('preserves null preset codes without creating preset buckets for them', async () => {
    const getCurrentAuthorization = vi.fn().mockResolvedValue(authorization)
    const listMenuDefinitions = vi.fn().mockResolvedValue(menuDefinitions)
    const getAccessExperience = vi.fn().mockResolvedValue(experience)
    const listMembers = vi.fn().mockResolvedValue([
      members[0],
      {
        user: {
          id: 'user-unassigned',
          username: 'unassigned',
          displayName: 'Unassigned Member',
          status: 'active',
          passwordState: 'set',
        },
        primaryPresetCode: null,
        primaryPresetName: 'No preset assigned',
        effectiveRoleNames: [],
        hasOrgAssignments: false,
      },
    ])

    vi.spyOn(tauriClient, 'createWorkspaceClient').mockReturnValue({
      accessControl: {
        getCurrentAuthorization,
        listMenuDefinitions,
        getAccessExperience,
        listMembers,
      },
    } as unknown as ReturnType<typeof tauriClient.createWorkspaceClient>)

    const store = useWorkspaceAccessControlStore()
    await store.loadMembersData(connection.workspaceConnectionId)

    expect(store.members.find(member => member.user.id === 'user-unassigned')?.primaryPresetCode).toBeNull()
    expect(Array.from(store.membersByPresetCode.keys())).toEqual(['owner'])
  })
})
