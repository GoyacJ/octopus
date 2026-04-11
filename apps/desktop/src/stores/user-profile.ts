import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ChangeCurrentUserPasswordRequest,
  ChangeCurrentUserPasswordResponse,
  AccessUserRecord,
  RuntimeConfigValidationResult,
  RuntimeEffectiveConfig,
  UpdateCurrentUserProfileRequest,
  UserRecordSummary,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
  ensureWorkspaceClientForConnection,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import {
  createRuntimeConfigDraftsFromConfig,
  parseRuntimeConfigDraft,
} from './runtime-config'
import { useShellStore } from './shell'
import { useWorkspaceAccessControlStore } from './workspace-access-control'

interface UserProfileState {
  workspaceId: string
  currentUser: UserRecordSummary | null
  alerts: UserProfileAlert[]
}

interface UserProfileAlert {
  id: string
  severity: 'low' | 'medium' | 'high'
  title: string
  description: string
}

function emptyProfileState(): UserProfileState {
  return {
    workspaceId: '',
    currentUser: null,
    alerts: [],
  }
}

function toUserSummary(user: AccessUserRecord, previous?: UserRecordSummary | null): UserRecordSummary {
  return {
    id: user.id,
    username: user.username,
    displayName: user.displayName,
    avatar: previous?.avatar,
    status: user.status as UserRecordSummary['status'],
    passwordState: user.passwordState as UserRecordSummary['passwordState'],
  }
}

function buildProfileAlerts(user: AccessUserRecord): UserProfileAlert[] {
  if (user.passwordState === 'reset-required') {
    return [{
      id: 'password-reset-required',
      severity: 'high',
      title: '需要重置密码',
      description: '当前账号需要在个人中心完成密码重置后继续使用。',
    }]
  }

  if (user.passwordState === 'temporary') {
    return [{
      id: 'password-temporary',
      severity: 'medium',
      title: '临时密码仍在使用',
      description: '建议尽快修改临时密码，避免继续依赖初始化口令。',
    }]
  }

  return []
}

export const useUserProfileStore = defineStore('user-profile', () => {
  const profilesByConnection = ref<Record<string, UserProfileState>>({})
  const runtimeConfigsByConnection = ref<Record<string, RuntimeEffectiveConfig>>({})
  const runtimeDraftsByConnection = ref<Record<string, string>>({})
  const runtimeValidationByConnection = ref<Record<string, RuntimeConfigValidationResult | null>>({})
  const runtimeLoadingByConnection = ref<Record<string, boolean>>({})
  const runtimeSavingByConnection = ref<Record<string, boolean>>({})
  const runtimeValidatingByConnection = ref<Record<string, boolean>>({})
  const runtimeErrorsByConnection = ref<Record<string, string>>({})
  const profileSavingByConnection = ref<Record<string, boolean>>({})
  const profileErrorsByConnection = ref<Record<string, string>>({})
  const passwordSavingByConnection = ref<Record<string, boolean>>({})
  const passwordErrorsByConnection = ref<Record<string, string>>({})

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const profileState = computed(() => profilesByConnection.value[activeConnectionId.value] ?? emptyProfileState())
  const workspaceId = computed(() => profileState.value.workspaceId)
  const currentUser = computed(() => profileState.value.currentUser)
  const alerts = computed(() => profileState.value.alerts)
  const runtimeConfig = computed(() => runtimeConfigsByConnection.value[activeConnectionId.value] ?? null)
  const runtimeDraft = computed(() => runtimeDraftsByConnection.value[activeConnectionId.value] ?? '{}')
  const runtimeValidation = computed(() => runtimeValidationByConnection.value[activeConnectionId.value] ?? null)
  const runtimeLoading = computed(() => runtimeLoadingByConnection.value[activeConnectionId.value] ?? false)
  const runtimeSaving = computed(() => runtimeSavingByConnection.value[activeConnectionId.value] ?? false)
  const runtimeValidating = computed(() => runtimeValidatingByConnection.value[activeConnectionId.value] ?? false)
  const runtimeError = computed(() => runtimeErrorsByConnection.value[activeConnectionId.value] ?? '')
  const profileSaving = computed(() => profileSavingByConnection.value[activeConnectionId.value] ?? false)
  const profileError = computed(() => profileErrorsByConnection.value[activeConnectionId.value] ?? '')
  const passwordSaving = computed(() => passwordSavingByConnection.value[activeConnectionId.value] ?? false)
  const passwordError = computed(() => passwordErrorsByConnection.value[activeConnectionId.value] ?? '')

  function updateCurrentUserState(connectionId: string, current: UserRecordSummary) {
    profilesByConnection.value = {
      ...profilesByConnection.value,
      [connectionId]: {
        ...(profilesByConnection.value[connectionId] ?? emptyProfileState()),
        currentUser: current,
      },
    }
  }

  async function load(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { connectionId } = resolvedClient
    const accessControlStore = useWorkspaceAccessControlStore()
    if (connectionId !== activeConnectionId.value || !accessControlStore.currentUser) {
      await accessControlStore.load(connectionId)
    }

    const current = accessControlStore.currentUser
    if (!current) {
      return null
    }

    const workspaceId = useShellStore().workspaceConnections
      .find(connection => connection.workspaceConnectionId === connectionId)
      ?.workspaceId ?? ''
    const previous = profilesByConnection.value[connectionId]?.currentUser ?? null
    profilesByConnection.value = {
      ...profilesByConnection.value,
      [connectionId]: {
        workspaceId,
        currentUser: toUserSummary(current, previous),
        alerts: buildProfileAlerts(current),
      },
    }
    return profilesByConnection.value[connectionId].currentUser
  }

  async function updateCurrentUserProfile(input: UpdateCurrentUserProfileRequest, workspaceConnectionId?: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    profileSavingByConnection.value = {
      ...profileSavingByConnection.value,
      [connectionId]: true,
    }
    profileErrorsByConnection.value = {
      ...profileErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const updated = await client.profile.updateCurrentUserProfile(input)
      updateCurrentUserState(connectionId, updated)
      await useWorkspaceAccessControlStore().reloadAll(connectionId)
      return updated
    } catch (cause) {
      profileErrorsByConnection.value = {
        ...profileErrorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to update profile',
      }
      throw cause
    } finally {
      profileSavingByConnection.value = {
        ...profileSavingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  async function changeCurrentUserPassword(input: ChangeCurrentUserPasswordRequest, workspaceConnectionId?: string) {
    const { client, connectionId } = ensureWorkspaceClientForConnection(workspaceConnectionId)
    passwordSavingByConnection.value = {
      ...passwordSavingByConnection.value,
      [connectionId]: true,
    }
    passwordErrorsByConnection.value = {
      ...passwordErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const response = await client.profile.changeCurrentUserPassword(input)
      const activeUser = profilesByConnection.value[connectionId]?.currentUser
      if (activeUser) {
        updateCurrentUserState(connectionId, {
          ...activeUser,
          passwordState: response.passwordState as ChangeCurrentUserPasswordResponse['passwordState'],
        })
      }
      await useWorkspaceAccessControlStore().reloadAll(connectionId)
      return response
    } catch (cause) {
      passwordErrorsByConnection.value = {
        ...passwordErrorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to update password',
      }
      throw cause
    } finally {
      passwordSavingByConnection.value = {
        ...passwordSavingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  function setCurrentUserRuntimeDraft(value: string, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    if (!connectionId) {
      return
    }
    runtimeDraftsByConnection.value = {
      ...runtimeDraftsByConnection.value,
      [connectionId]: value,
    }
  }

  async function loadCurrentUserRuntimeConfig(force = false, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    if (runtimeConfigsByConnection.value[connectionId] && !force) {
      return runtimeConfigsByConnection.value[connectionId]
    }

    runtimeLoadingByConnection.value = {
      ...runtimeLoadingByConnection.value,
      [connectionId]: true,
    }
    runtimeErrorsByConnection.value = {
      ...runtimeErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const config = await client.runtime.getUserConfig()
      runtimeConfigsByConnection.value = {
        ...runtimeConfigsByConnection.value,
        [connectionId]: config,
      }
      runtimeDraftsByConnection.value = {
        ...runtimeDraftsByConnection.value,
        [connectionId]: createRuntimeConfigDraftsFromConfig(config).user,
      }
      runtimeValidationByConnection.value = {
        ...runtimeValidationByConnection.value,
        [connectionId]: null,
      }
      return config
    } catch (cause) {
      runtimeErrorsByConnection.value = {
        ...runtimeErrorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to load user runtime config',
      }
      return null
    } finally {
      runtimeLoadingByConnection.value = {
        ...runtimeLoadingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  async function validateCurrentUserRuntimeConfig(workspaceConnectionId?: string): Promise<RuntimeConfigValidationResult> {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return {
        valid: false,
        errors: ['Active workspace connection is unavailable'],
        warnings: [],
      }
    }

    const { client, connectionId } = resolvedClient
    runtimeValidatingByConnection.value = {
      ...runtimeValidatingByConnection.value,
      [connectionId]: true,
    }
    runtimeErrorsByConnection.value = {
      ...runtimeErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const patch = parseRuntimeConfigDraft('user', runtimeDraftsByConnection.value[connectionId] ?? '{}')
      const result = await client.runtime.validateUserConfig(patch)
      runtimeValidationByConnection.value = {
        ...runtimeValidationByConnection.value,
        [connectionId]: result,
      }
      return result
    } catch (cause) {
      const result = {
        valid: false,
        errors: [cause instanceof Error ? cause.message : 'Failed to validate user runtime config'],
        warnings: [],
      } satisfies RuntimeConfigValidationResult
      runtimeValidationByConnection.value = {
        ...runtimeValidationByConnection.value,
        [connectionId]: result,
      }
      runtimeErrorsByConnection.value = {
        ...runtimeErrorsByConnection.value,
        [connectionId]: result.errors[0] ?? '',
      }
      return result
    } finally {
      runtimeValidatingByConnection.value = {
        ...runtimeValidatingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  async function saveCurrentUserRuntimeConfig(workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    const validation = await validateCurrentUserRuntimeConfig(connectionId)
    if (!validation.valid) {
      return null
    }

    runtimeSavingByConnection.value = {
      ...runtimeSavingByConnection.value,
      [connectionId]: true,
    }
    runtimeErrorsByConnection.value = {
      ...runtimeErrorsByConnection.value,
      [connectionId]: '',
    }

    try {
      const patch = parseRuntimeConfigDraft('user', runtimeDraftsByConnection.value[connectionId] ?? '{}')
      const config = await client.runtime.saveUserConfig(patch)
      runtimeConfigsByConnection.value = {
        ...runtimeConfigsByConnection.value,
        [connectionId]: config,
      }
      runtimeDraftsByConnection.value = {
        ...runtimeDraftsByConnection.value,
        [connectionId]: createRuntimeConfigDraftsFromConfig(config).user,
      }
      runtimeValidationByConnection.value = {
        ...runtimeValidationByConnection.value,
        [connectionId]: config.validation,
      }
      return config
    } catch (cause) {
      runtimeErrorsByConnection.value = {
        ...runtimeErrorsByConnection.value,
        [connectionId]: cause instanceof Error ? cause.message : 'Failed to save user runtime config',
      }
      return null
    } finally {
      runtimeSavingByConnection.value = {
        ...runtimeSavingByConnection.value,
        [connectionId]: false,
      }
    }
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const clearRecord = <T>(record: Record<string, T>) => {
      const next = { ...record }
      delete next[workspaceConnectionId]
      return next
    }

    profilesByConnection.value = clearRecord(profilesByConnection.value)
    runtimeConfigsByConnection.value = clearRecord(runtimeConfigsByConnection.value)
    runtimeDraftsByConnection.value = clearRecord(runtimeDraftsByConnection.value)
    runtimeValidationByConnection.value = clearRecord(runtimeValidationByConnection.value)
    runtimeLoadingByConnection.value = clearRecord(runtimeLoadingByConnection.value)
    runtimeSavingByConnection.value = clearRecord(runtimeSavingByConnection.value)
    runtimeValidatingByConnection.value = clearRecord(runtimeValidatingByConnection.value)
    runtimeErrorsByConnection.value = clearRecord(runtimeErrorsByConnection.value)
    profileSavingByConnection.value = clearRecord(profileSavingByConnection.value)
    profileErrorsByConnection.value = clearRecord(profileErrorsByConnection.value)
    passwordSavingByConnection.value = clearRecord(passwordSavingByConnection.value)
    passwordErrorsByConnection.value = clearRecord(passwordErrorsByConnection.value)
  }

  return {
    workspaceId,
    currentUser,
    alerts,
    runtimeConfig,
    runtimeDraft,
    runtimeValidation,
    runtimeLoading,
    runtimeSaving,
    runtimeValidating,
    runtimeError,
    profileSaving,
    profileError,
    passwordSaving,
    passwordError,
    load,
    updateCurrentUserProfile,
    changeCurrentUserPassword,
    setCurrentUserRuntimeDraft,
    loadCurrentUserRuntimeConfig,
    validateCurrentUserRuntimeConfig,
    saveCurrentUserRuntimeConfig,
    clearWorkspaceScope,
  }
})
