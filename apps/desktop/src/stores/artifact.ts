import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ArtifactVersionReference,
  ConversationRecord,
  CreateDeliverableVersionInput,
  DeliverableDetail,
  DeliverableSummary,
  DeliverableVersionContent,
  DeliverableVersionSummary,
  KnowledgeRecord,
  RuntimeSessionDetail,
} from '@octopus/schema'

import * as tauriClient from '@/tauri/client'

import {
  activeWorkspaceConnectionId,
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'
import { useShellStore } from './shell'
import { useWorkspaceStore } from './workspace'

function projectScopeKey(connectionId: string, projectId: string): string {
  return `${connectionId}:project:${projectId}`
}

function deliverableScopeKey(connectionId: string, deliverableId: string): string {
  return `${connectionId}:deliverable:${deliverableId}`
}

function deliverableVersionScopeKey(
  connectionId: string,
  deliverableId: string,
  version: number,
): string {
  return `${deliverableScopeKey(connectionId, deliverableId)}:version:${version}`
}

function buildDeliverableRefs(records: DeliverableSummary[]): ArtifactVersionReference[] {
  return records.map(record => record.latestVersionRef ?? {
    artifactId: record.id,
    title: record.title,
    version: record.latestVersion,
    previewKind: record.previewKind,
    contentType: record.contentType,
    updatedAt: record.updatedAt,
  })
}

function uniqueArtifactIds(detail: RuntimeSessionDetail): string[] {
  const ids = new Set<string>()

  for (const ref of detail.run.artifactRefs ?? []) {
    ids.add(ref.artifactId)
  }
  for (const message of detail.messages) {
    for (const artifactRef of message.artifacts ?? []) {
      ids.add(typeof artifactRef === 'string' ? artifactRef : artifactRef.artifactId)
    }
  }

  return [...ids]
}

function deliverablePromotionPriority(state: DeliverableSummary['promotionState']): number {
  switch (state) {
    case 'candidate':
      return 0
    case 'promoted':
      return 1
    default:
      return 2
  }
}

export const useArtifactStore = defineStore('artifact', () => {
  const projectDeliverablesByScope = ref<Record<string, DeliverableSummary[]>>({})
  const deliverableDetailsByScope = ref<Record<string, DeliverableDetail>>({})
  const deliverableVersionsByScope = ref<Record<string, DeliverableVersionSummary[]>>({})
  const deliverableContentsByScope = ref<Record<string, DeliverableVersionContent>>({})
  const draftTextByScope = ref<Record<string, string>>({})
  const requestTokens = ref<Record<string, number>>({})
  const loadingScopes = ref<Record<string, boolean>>({})
  const savingScopes = ref<Record<string, boolean>>({})
  const errors = ref<Record<string, string>>({})

  const shell = useShellStore()
  const workspaceStore = useWorkspaceStore()
  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const activeProjectScope = computed(() => (
    activeConnectionId.value && workspaceStore.currentProjectId
      ? projectScopeKey(activeConnectionId.value, workspaceStore.currentProjectId)
      : ''
  ))
  const selectedDeliverableScope = computed(() => (
    activeConnectionId.value && shell.selectedDeliverableId
      ? deliverableScopeKey(activeConnectionId.value, shell.selectedDeliverableId)
      : ''
  ))
  const resolvedSelectedVersion = computed(() => (
    shell.selectedDeliverableVersion
    ?? selectedDeliverableDetail.value?.latestVersion
    ?? selectedDeliverable.value?.latestVersion
    ?? null
  ))
  const selectedDeliverableContentScope = computed(() => (
    activeConnectionId.value && shell.selectedDeliverableId && resolvedSelectedVersion.value
      ? deliverableVersionScopeKey(
          activeConnectionId.value,
          shell.selectedDeliverableId,
          resolvedSelectedVersion.value,
        )
      : ''
  ))

  const activeProjectDeliverables = computed(() => (
    activeProjectScope.value ? projectDeliverablesByScope.value[activeProjectScope.value] ?? [] : []
  ))
  const selectedDeliverable = computed(() => (
    activeProjectDeliverables.value.find(deliverable => deliverable.id === shell.selectedDeliverableId) ?? null
  ))
  const selectedDeliverableDetail = computed(() => (
    selectedDeliverableScope.value
      ? deliverableDetailsByScope.value[selectedDeliverableScope.value] ?? null
      : null
  ))
  const selectedDeliverableVersions = computed(() => (
    selectedDeliverableScope.value
      ? deliverableVersionsByScope.value[selectedDeliverableScope.value] ?? []
      : []
  ))
  const selectedDeliverableContent = computed(() => (
    selectedDeliverableContentScope.value
      ? deliverableContentsByScope.value[selectedDeliverableContentScope.value] ?? null
      : null
  ))
  const selectedDeliverableDraft = computed(() => (
    selectedDeliverableScope.value
      ? draftTextByScope.value[selectedDeliverableScope.value]
        ?? selectedDeliverableContent.value?.textContent
        ?? ''
      : ''
  ))
  const loading = computed(() => (
    (activeProjectScope.value && loadingScopes.value[activeProjectScope.value])
    || (selectedDeliverableScope.value && loadingScopes.value[selectedDeliverableScope.value])
    || (selectedDeliverableContentScope.value && loadingScopes.value[selectedDeliverableContentScope.value])
    || false
  ))
  const saving = computed(() => (
    selectedDeliverableScope.value ? savingScopes.value[selectedDeliverableScope.value] ?? false : false
  ))
  const error = computed(() => {
    const scopedError = selectedDeliverableScope.value
      ? errors.value[selectedDeliverableScope.value]
      : undefined
    if (scopedError) {
      return scopedError
    }
    return activeProjectScope.value ? errors.value[activeProjectScope.value] ?? '' : ''
  })

  function setScopeLoading(scope: string, loading: boolean) {
    loadingScopes.value = {
      ...loadingScopes.value,
      [scope]: loading,
    }
  }

  function setScopeSaving(scope: string, saving: boolean) {
    savingScopes.value = {
      ...savingScopes.value,
      [scope]: saving,
    }
  }

  function setScopeError(scope: string, message: string) {
    errors.value = {
      ...errors.value,
      [scope]: message,
    }
  }

  function clearScopeError(scope: string) {
    if (!errors.value[scope]) {
      return
    }

    const nextErrors = { ...errors.value }
    delete nextErrors[scope]
    errors.value = nextErrors
  }

  async function loadProjectDeliverables(projectId?: string, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    const nextProjectId = projectId ?? workspaceStore.currentProjectId
    if (!resolvedClient || !nextProjectId) {
      return []
    }

    const { client, connectionId } = resolvedClient
    const scope = projectScopeKey(connectionId, nextProjectId)
    const token = createWorkspaceRequestToken(requestTokens.value[scope] ?? 0)
    requestTokens.value[scope] = token
    setScopeLoading(scope, true)
    clearScopeError(scope)

    try {
      const records = (await client.projects.listDeliverables(nextProjectId))
        .slice()
        .sort((left, right) => {
          const promotionPriority = deliverablePromotionPriority(left.promotionState)
            - deliverablePromotionPriority(right.promotionState)
          if (promotionPriority !== 0) {
            return promotionPriority
          }
          return right.updatedAt - left.updatedAt
        })
      if (requestTokens.value[scope] !== token) {
        return records
      }

      projectDeliverablesByScope.value = {
        ...projectDeliverablesByScope.value,
        [scope]: records,
      }
      shell.hydrateDeliverableSelection(buildDeliverableRefs(records))
      return records
    } catch (cause) {
      if (requestTokens.value[scope] === token) {
        setScopeError(
          scope,
          cause instanceof Error ? cause.message : 'Failed to load project deliverables',
        )
      }
      return []
    } finally {
      if (requestTokens.value[scope] === token) {
        setScopeLoading(scope, false)
      }
    }
  }

  async function loadDeliverableDetail(deliverableId: string, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    const scope = deliverableScopeKey(connectionId, deliverableId)
    const token = createWorkspaceRequestToken(requestTokens.value[scope] ?? 0)
    requestTokens.value[scope] = token
    setScopeLoading(scope, true)
    clearScopeError(scope)

    try {
      const detail = await client.runtime.getDeliverableDetail(deliverableId)
      if (requestTokens.value[scope] !== token) {
        return detail
      }

      deliverableDetailsByScope.value = {
        ...deliverableDetailsByScope.value,
        [scope]: detail,
      }
      return detail
    } catch (cause) {
      if (requestTokens.value[scope] === token) {
        setScopeError(
          scope,
          cause instanceof Error ? cause.message : 'Failed to load deliverable detail',
        )
      }
      return null
    } finally {
      if (requestTokens.value[scope] === token) {
        setScopeLoading(scope, false)
      }
    }
  }

  async function loadDeliverableVersions(deliverableId: string, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return []
    }

    const { client, connectionId } = resolvedClient
    const scope = deliverableScopeKey(connectionId, deliverableId)
    const token = createWorkspaceRequestToken(requestTokens.value[`${scope}:versions`] ?? 0)
    requestTokens.value[`${scope}:versions`] = token
    setScopeLoading(scope, true)
    clearScopeError(scope)

    try {
      const versions = (await client.runtime.listDeliverableVersions(deliverableId))
        .slice()
        .sort((left, right) => right.version - left.version)
      if (requestTokens.value[`${scope}:versions`] !== token) {
        return versions
      }

      deliverableVersionsByScope.value = {
        ...deliverableVersionsByScope.value,
        [scope]: versions,
      }
      return versions
    } catch (cause) {
      if (requestTokens.value[`${scope}:versions`] === token) {
        setScopeError(
          scope,
          cause instanceof Error ? cause.message : 'Failed to load deliverable versions',
        )
      }
      return []
    } finally {
      if (requestTokens.value[`${scope}:versions`] === token) {
        setScopeLoading(scope, false)
      }
    }
  }

  async function loadDeliverableVersionContent(
    deliverableId: string,
    version: number,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient) {
      return null
    }

    const { client, connectionId } = resolvedClient
    const scope = deliverableVersionScopeKey(connectionId, deliverableId, version)
    const token = createWorkspaceRequestToken(requestTokens.value[scope] ?? 0)
    requestTokens.value[scope] = token
    setScopeLoading(scope, true)

    try {
      const content = await client.runtime.getDeliverableVersionContent(deliverableId, version)
      if (requestTokens.value[scope] !== token) {
        return content
      }

      deliverableContentsByScope.value = {
        ...deliverableContentsByScope.value,
        [scope]: content,
      }
      return content
    } catch (cause) {
      if (requestTokens.value[scope] === token) {
        setScopeError(
          deliverableScopeKey(connectionId, deliverableId),
          cause instanceof Error ? cause.message : 'Failed to load deliverable content',
        )
      }
      return null
    } finally {
      if (requestTokens.value[scope] === token) {
        setScopeLoading(scope, false)
      }
    }
  }

  async function ensureDeliverableState(
    deliverableId = shell.selectedDeliverableId,
    version = resolvedSelectedVersion.value,
    workspaceConnectionId?: string,
  ) {
    if (!deliverableId) {
      return null
    }

    const [detail, versions] = await Promise.all([
      loadDeliverableDetail(deliverableId, workspaceConnectionId),
      loadDeliverableVersions(deliverableId, workspaceConnectionId),
    ])

    const targetVersion = version
      ?? detail?.latestVersion
      ?? versions[0]?.version
      ?? null
    if (targetVersion) {
      shell.setSelectedDeliverableVersion(targetVersion)
      await loadDeliverableVersionContent(deliverableId, targetVersion, workspaceConnectionId)
    }

    return detail
  }

  function updateDraft(nextValue: string, deliverableId = shell.selectedDeliverableId) {
    if (!selectedDeliverableScope.value || !deliverableId) {
      return
    }

    draftTextByScope.value = {
      ...draftTextByScope.value,
      [selectedDeliverableScope.value]: nextValue,
    }
  }

  function resetDraft(deliverableId = shell.selectedDeliverableId) {
    if (!selectedDeliverableScope.value || !deliverableId) {
      return
    }

    const nextDrafts = { ...draftTextByScope.value }
    delete nextDrafts[selectedDeliverableScope.value]
    draftTextByScope.value = nextDrafts
  }

  async function saveDraftAsVersion(
    input: Partial<Omit<CreateDeliverableVersionInput, 'parentVersion' | 'textContent'>> & {
      title?: string
      textContent?: string
      parentVersion?: number
    } = {},
    deliverableId = shell.selectedDeliverableId,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient || !deliverableId) {
      return null
    }

    const { client, connectionId } = resolvedClient
    const scope = deliverableScopeKey(connectionId, deliverableId)
    const currentContent = selectedDeliverableContent.value
    const currentDetail = selectedDeliverableDetail.value
    const parentVersion = input.parentVersion
      ?? resolvedSelectedVersion.value
      ?? currentDetail?.latestVersion
    const textContent = input.textContent
      ?? draftTextByScope.value[scope]
      ?? currentContent?.textContent
      ?? ''
    const previewKind = input.previewKind ?? currentContent?.previewKind ?? currentDetail?.previewKind

    if (!previewKind) {
      return null
    }

    setScopeSaving(scope, true)
    clearScopeError(scope)
    try {
      const detail = await client.runtime.createDeliverableVersion(
        deliverableId,
        {
          title: input.title ?? currentDetail?.title ?? currentContent?.fileName ?? 'Untitled deliverable',
          previewKind,
          textContent,
          parentVersion,
        },
        tauriClient.createIdempotencyKey(`deliverable-version-${connectionId}-${deliverableId}`),
      )
      deliverableDetailsByScope.value = {
        ...deliverableDetailsByScope.value,
        [scope]: detail,
      }
      shell.selectDeliverable(deliverableId, detail.latestVersion)
      resetDraft(deliverableId)
      await Promise.all([
        loadProjectDeliverables(detail.projectId, connectionId),
        loadDeliverableVersions(deliverableId, connectionId),
        loadDeliverableVersionContent(deliverableId, detail.latestVersion, connectionId),
      ])
      return detail
    } catch (cause) {
      setScopeError(
        scope,
        cause instanceof Error ? cause.message : 'Failed to save deliverable version',
      )
      return null
    } finally {
      setScopeSaving(scope, false)
    }
  }

  async function promoteDeliverable(deliverableId = shell.selectedDeliverableId, workspaceConnectionId?: string) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    const detail = selectedDeliverableDetail.value
    if (!resolvedClient || !deliverableId || !detail) {
      return null
    }

    const { client, connectionId } = resolvedClient
    const scope = deliverableScopeKey(connectionId, deliverableId)
    setScopeSaving(scope, true)
    clearScopeError(scope)

    try {
      const record = await client.runtime.promoteDeliverable(
        deliverableId,
        {
          title: detail.title,
          summary: `Promoted from ${detail.title}`,
          kind: 'shared',
        },
        tauriClient.createIdempotencyKey(`deliverable-promote-${connectionId}-${deliverableId}`),
      )
      await Promise.all([
        loadProjectDeliverables(detail.projectId, connectionId),
        loadDeliverableDetail(deliverableId, connectionId),
      ])
      return record as KnowledgeRecord
    } catch (cause) {
      setScopeError(
        scope,
        cause instanceof Error ? cause.message : 'Failed to promote deliverable',
      )
      return null
    } finally {
      setScopeSaving(scope, false)
    }
  }

  async function forkDeliverable(
    projectId: string,
    title?: string,
    deliverableId = shell.selectedDeliverableId,
    workspaceConnectionId?: string,
  ) {
    const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
    if (!resolvedClient || !deliverableId) {
      return null
    }

    const { client, connectionId } = resolvedClient
    const scope = deliverableScopeKey(connectionId, deliverableId)
    setScopeSaving(scope, true)
    clearScopeError(scope)

    try {
      return await client.runtime.forkDeliverable(
        deliverableId,
        {
          projectId,
          title,
        },
        tauriClient.createIdempotencyKey(`deliverable-fork-${connectionId}-${deliverableId}`),
      ) as ConversationRecord
    } catch (cause) {
      setScopeError(
        scope,
        cause instanceof Error ? cause.message : 'Failed to fork deliverable',
      )
      return null
    } finally {
      setScopeSaving(scope, false)
    }
  }

  async function syncConversationDeliverables(detail: RuntimeSessionDetail, workspaceConnectionId?: string) {
    const connectionId = workspaceConnectionId ?? activeConnectionId.value
    if (!connectionId || !detail.summary.projectId) {
      return
    }

    const projectScope = projectScopeKey(connectionId, detail.summary.projectId)
    let records = projectDeliverablesByScope.value[projectScope]
    if (!records?.length) {
      records = await loadProjectDeliverables(detail.summary.projectId, connectionId)
    }

    const knownRefs = buildDeliverableRefs(records ?? [])
    const refsById = new Map(knownRefs.map(ref => [ref.artifactId, ref]))
    const runtimeRefs = (detail.run.artifactRefs ?? [])
      .filter(ref => ref.artifactId)
    const messageRefs = uniqueArtifactIds(detail)
      .map(artifactId => refsById.get(artifactId))
      .filter((ref): ref is ArtifactVersionReference => Boolean(ref))

    const nextRefs = [...runtimeRefs, ...messageRefs]
      .filter((ref, index, items) => items.findIndex(candidate => candidate.artifactId === ref.artifactId) === index)

    shell.hydrateDeliverableSelection(nextRefs)
    if (shell.selectedDeliverableId) {
      await ensureDeliverableState(shell.selectedDeliverableId, shell.selectedDeliverableVersion, connectionId)
    }
  }

  function clearWorkspaceScope(workspaceConnectionId: string) {
    const projectDeliverables = { ...projectDeliverablesByScope.value }
    const deliverableDetails = { ...deliverableDetailsByScope.value }
    const deliverableVersions = { ...deliverableVersionsByScope.value }
    const deliverableContents = { ...deliverableContentsByScope.value }
    const drafts = { ...draftTextByScope.value }
    const nextErrors = { ...errors.value }
    const nextTokens = { ...requestTokens.value }
    const nextLoading = { ...loadingScopes.value }
    const nextSaving = { ...savingScopes.value }

    for (const bucket of [
      projectDeliverables,
      deliverableDetails,
      deliverableVersions,
      deliverableContents,
      drafts,
      nextErrors,
      nextTokens,
      nextLoading,
      nextSaving,
    ]) {
      for (const key of Object.keys(bucket)) {
        if (key.startsWith(`${workspaceConnectionId}:`)) {
          delete bucket[key]
        }
      }
    }

    projectDeliverablesByScope.value = projectDeliverables
    deliverableDetailsByScope.value = deliverableDetails
    deliverableVersionsByScope.value = deliverableVersions
    deliverableContentsByScope.value = deliverableContents
    draftTextByScope.value = drafts
    errors.value = nextErrors
    requestTokens.value = nextTokens
    loadingScopes.value = nextLoading
    savingScopes.value = nextSaving
  }

  return {
    activeProjectDeliverables,
    selectedDeliverable,
    selectedDeliverableDetail,
    selectedDeliverableVersions,
    selectedDeliverableContent,
    selectedDeliverableDraft,
    resolvedSelectedVersion,
    loading,
    saving,
    error,
    loadProjectDeliverables,
    loadDeliverableDetail,
    loadDeliverableVersions,
    loadDeliverableVersionContent,
    ensureDeliverableState,
    updateDraft,
    resetDraft,
    saveDraftAsVersion,
    promoteDeliverable,
    forkDeliverable,
    syncConversationDeliverables,
    clearWorkspaceScope,
  }
})
