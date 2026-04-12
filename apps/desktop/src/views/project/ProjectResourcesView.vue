<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { FolderIcon, Trash2Icon, UploadIcon } from 'lucide-vue-next'
import type { UpdateWorkspaceResourceInput, WorkspaceResourceRecord } from '@octopus/schema'

import {
  UiBadge,
  UiButton,
  UiCodeEditor,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiListRow,
  UiPageHeader,
  UiPageShell,
  UiPagination,
  UiSelect,
  UiStatusCallout,
  UiSwitch,
  UiToolbarRow,
} from '@octopus/ui'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { isProjectOwner, resolveProjectActorUserId } from '@/composables/project-governance'
import { usePagination } from '@/composables/usePagination'
import { pickResourceFile, pickResourceFolder } from '@/tauri/client'
import { useResourceStore } from '@/stores/resource'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'
import { useProjectResourceNotifications } from './useProjectResourceNotifications'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()
const resourceStore = useResourceStore()
const notifications = useProjectResourceNotifications()

const searchQuery = ref('')
const scopeFilter = ref<'all' | 'personal' | 'project' | 'workspace'>('all')
const visibilityFilter = ref<'all' | 'private' | 'public'>('all')
const detailLoading = ref(false)
const togglingResourceIds = ref<string[]>([])
const updatingVisibility = ref(false)
const promotingSelected = ref(false)
const deletingSelected = ref(false)
const deleteDialogOpen = ref(false)
const deleteTarget = ref<WorkspaceResourceRecord | null>(null)

const projectId = computed(() => typeof route.params.projectId === 'string' ? route.params.projectId : '')
const projectRecord = computed(() => workspaceStore.projects.find(project => project.id === projectId.value) ?? null)
const projectResources = computed(() => resourceStore.projectResourcesFor(projectId.value))
const selectedResourceId = computed(() => typeof route.query.resourceId === 'string' ? route.query.resourceId : '')
const selectedResource = computed(() =>
  (selectedResourceId.value ? resourceStore.getCachedDetail(selectedResourceId.value) : null)
  ?? projectResources.value.find(resource => resource.id === selectedResourceId.value)
  ?? null,
)
const selectedContent = computed(() =>
  selectedResourceId.value ? resourceStore.getCachedContent(selectedResourceId.value) : null,
)
const selectedChildren = computed(() =>
  selectedResourceId.value ? (resourceStore.getCachedChildren(selectedResourceId.value) ?? []) : [],
)
const resourceDirectory = computed(() => projectRecord.value?.resourceDirectory?.trim() || t('common.na'))
const currentProjectActorUserId = computed(() =>
  resolveProjectActorUserId(
    workspaceAccessControlStore.currentUser?.id,
    workspaceAccessControlStore.loading ? undefined : shell.activeWorkspaceSession?.session.userId,
  ),
)
const canSubmitPromotion = computed(() =>
  Boolean(projectRecord.value)
  && Boolean(currentProjectActorUserId.value)
  && isProjectOwner(projectRecord.value, currentProjectActorUserId.value),
)

const scopeOptions = computed(() => [
  { label: t('resources.filters.allScopes'), value: 'all' },
  { label: enumLabel('resourceScope', 'personal'), value: 'personal' },
  { label: enumLabel('resourceScope', 'project'), value: 'project' },
  { label: enumLabel('resourceScope', 'workspace'), value: 'workspace' },
])

const visibilityOptions = computed(() => [
  { label: t('resources.filters.allVisibility'), value: 'all' },
  { label: enumLabel('resourceVisibility', 'private'), value: 'private' },
  { label: enumLabel('resourceVisibility', 'public'), value: 'public' },
])

const detailVisibilityOptions = computed(() => [
  { label: enumLabel('resourceVisibility', 'private'), value: 'private' },
  { label: enumLabel('resourceVisibility', 'public'), value: 'public' },
])

const filteredResources = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()

  return projectResources.value.filter((resource) => {
    if (scopeFilter.value !== 'all' && resource.scope !== scopeFilter.value) {
      return false
    }
    if (visibilityFilter.value !== 'all' && resource.visibility !== visibilityFilter.value) {
      return false
    }
    if (!query) {
      return true
    }

    return [
      resource.name,
      resource.location ?? '',
      resource.kind,
      resource.scope,
      resource.visibility,
      resource.status,
      ...resource.tags,
    ].join(' ').toLowerCase().includes(query)
  })
})

const pagination = usePagination(filteredResources, {
  pageSize: 6,
  resetOn: [searchQuery, scopeFilter, visibilityFilter],
})
const pagedResources = computed(() => pagination.pagedItems.value)
const currentPage = computed(() => pagination.currentPage.value)
const pageCount = computed(() => pagination.pageCount.value)
const totalFilteredResources = computed(() => filteredResources.value.length)
const previewKind = computed(() => selectedResource.value?.previewKind ?? selectedContent.value?.previewKind ?? '')
const previewContentType = computed(() => {
  const directContentType = selectedContent.value?.contentType?.trim() || selectedResource.value?.contentType?.trim()
  if (directContentType) {
    return directContentType
  }

  const fileName = selectedContent.value?.fileName || selectedResource.value?.name || ''
  const extension = fileName.includes('.')
    ? fileName.slice(fileName.lastIndexOf('.') + 1).toLowerCase()
    : ''
  const extensionMap: Record<string, string> = {
    txt: 'text/plain',
    md: 'text/markdown',
    markdown: 'text/markdown',
    json: 'application/json',
    csv: 'text/csv',
    png: 'image/png',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    gif: 'image/gif',
    webp: 'image/webp',
    svg: 'image/svg+xml',
    pdf: 'application/pdf',
    mp3: 'audio/mpeg',
    wav: 'audio/wav',
    ogg: 'audio/ogg',
    mp4: 'video/mp4',
    webm: 'video/webm',
    mov: 'video/quicktime',
  }

  if (extension && extensionMap[extension]) {
    return extensionMap[extension]
  }

  switch (previewKind.value) {
    case 'markdown':
      return 'text/markdown'
    case 'text':
    case 'code':
      return 'text/plain'
    case 'pdf':
      return 'application/pdf'
    default:
      return ''
  }
})
const previewSrc = computed(() => {
  const dataBase64 = selectedContent.value?.dataBase64
  const contentType = previewContentType.value
  if (!dataBase64 || !contentType) {
    return ''
  }
  return `data:${contentType};base64,${dataBase64}`
})
const selectedVisibility = computed(() => selectedResource.value?.visibility ?? 'public')

watch(
  () => [shell.activeWorkspaceConnectionId, projectId.value],
  ([connectionId, nextProjectId]) => {
    if (typeof connectionId === 'string' && connectionId && nextProjectId) {
      void resourceStore.loadProjectResources(nextProjectId)
    }
  },
  { immediate: true },
)

watch(
  () => selectedResourceId.value,
  async (resourceId) => {
    if (!resourceId || !shell.activeWorkspaceConnectionId) {
      return
    }

    detailLoading.value = true
    try {
      const detail = await resourceStore.getResourceDetail(resourceId)
      if (detail.previewKind === 'folder') {
        await resourceStore.loadResourceChildren(resourceId)
        return
      }
      await resourceStore.getResourceContent(resourceId)
    } catch {
      // Keep the workbench responsive when the connection is not ready yet.
    } finally {
      detailLoading.value = false
    }
  },
  { immediate: true },
)

function resourceBadgeLabel(group: string, value?: string | null) {
  return enumLabel(group, value)
}

function isSelectedResource(resourceId: string) {
  return selectedResourceId.value === resourceId
}

function isStatusUpdating(resourceId: string) {
  return togglingResourceIds.value.includes(resourceId)
}

function resourceLocation(resource: NonNullable<typeof selectedResource.value> | (typeof projectResources.value)[number]) {
  return resource.location || resource.storagePath || resource.origin || t('common.na')
}

function shouldShowStatusBadge(status?: string | null) {
  return status === 'attention'
}

function formatByteSize(byteSize?: number | null) {
  if (typeof byteSize !== 'number' || Number.isNaN(byteSize)) {
    return t('common.na')
  }
  if (byteSize < 1024) {
    return `${byteSize} B`
  }
  if (byteSize < 1024 * 1024) {
    return `${(byteSize / 1024).toFixed(1)} KB`
  }
  return `${(byteSize / (1024 * 1024)).toFixed(1)} MB`
}

async function selectResource(resourceId: string) {
  await router.replace({
    query: {
      ...route.query,
      resourceId,
    },
  })
}

function nextPromoteScope(scope: string) {
  if (scope === 'personal') {
    return 'project'
  }
  if (scope === 'project') {
    return 'workspace'
  }
  return null
}

async function updateResource(resource: WorkspaceResourceRecord, input: UpdateWorkspaceResourceInput) {
  if (resource.projectId) {
    return await resourceStore.updateProjectResource(resource.projectId, resource.id, input)
  }
  return await resourceStore.updateWorkspaceResource(resource.id, input)
}

async function deleteResource(resource: WorkspaceResourceRecord) {
  if (resource.projectId) {
    await resourceStore.deleteProjectResource(resource.projectId, resource.id)
    return
  }
  await resourceStore.deleteWorkspaceResource(resource.id)
}

async function handleToggleStatus(
  resource: (typeof projectResources.value)[number],
  nextEnabled: boolean,
) {
  togglingResourceIds.value = [...togglingResourceIds.value, resource.id]
  try {
    await updateResource(resource, {
      status: nextEnabled ? 'healthy' : 'attention',
    })
    await notifications.notifyStatusChanged(resource.name, nextEnabled)
  } catch (error) {
    const message = error instanceof Error ? error.message : t('resources.notifications.fallbackError')
    await notifications.notifyStatusChangeFailed(resource.name, message)
  } finally {
    togglingResourceIds.value = togglingResourceIds.value.filter(id => id !== resource.id)
  }
}

async function handleVisibilityChange(nextVisibility: string) {
  if (!selectedResource.value || nextVisibility === selectedResource.value.visibility) {
    return
  }
  const resource = selectedResource.value
  updatingVisibility.value = true
  try {
    await updateResource(resource, {
      visibility: nextVisibility as 'private' | 'public',
    })
    await notifications.notifyVisibilityChanged(
      resource.name,
      nextVisibility as 'private' | 'public',
    )
  } catch (error) {
    const message = error instanceof Error ? error.message : t('resources.notifications.fallbackError')
    await notifications.notifyVisibilityChangeFailed(resource.name, message)
  } finally {
    updatingVisibility.value = false
  }
}

async function handleDeleteSelected() {
  if (!selectedResource.value) {
    return
  }
  deleteTarget.value = selectedResource.value
  deleteDialogOpen.value = true
}

async function confirmDeleteSelected() {
  if (!deleteTarget.value) {
    return
  }
  const resource = deleteTarget.value
  deletingSelected.value = true
  try {
    const resourceId = resource.id
    const deletedResourceName = resource.name
    await deleteResource(resource)
    if (selectedResourceId.value === resourceId) {
      const nextQuery = { ...route.query }
      delete nextQuery.resourceId
      await router.replace({ query: nextQuery })
    }
    deleteDialogOpen.value = false
    deleteTarget.value = null
    await notifications.notifyDeleteSuccess(deletedResourceName)
  } catch (error) {
    const message = error instanceof Error ? error.message : t('resources.notifications.fallbackError')
    await notifications.notifyDeleteFailed(resource.name, message)
  } finally {
    deletingSelected.value = false
  }
}

async function handlePromoteSelected() {
  if (!selectedResource.value) {
    return
  }
  const resource = selectedResource.value
  const nextScope = nextPromoteScope(resource.scope)
  if (!nextScope || !projectId.value) {
    return
  }
  promotingSelected.value = true
  try {
    if (nextScope === 'workspace') {
      await resourceStore.submitProjectPromotionRequest(projectId.value, {
        assetType: 'resource',
        assetId: resource.id,
      })
      await notifications.notifyPromoteSubmitted(resource.name)
    } else {
      await resourceStore.promoteResource(resource.id, { scope: nextScope }, projectId.value)
      await notifications.notifyPromoteSuccess(resource.name, nextScope)
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : t('resources.notifications.fallbackError')
    await notifications.notifyPromoteFailed(resource.name, message)
  } finally {
    promotingSelected.value = false
  }
}

function normalizeFolderImport(entries: Awaited<ReturnType<typeof pickResourceFolder>>) {
  const normalizedEntries = (entries ?? []).map(entry => ({
    ...entry,
    relativePath: entry.relativePath.replace(/\\/g, '/'),
  }))
  const topLevelNames = Array.from(new Set(
    normalizedEntries
      .map(entry => entry.relativePath.split('/')[0])
      .filter((value): value is string => Boolean(value)),
  ))
  const rootDirName = topLevelNames.length === 1 ? (topLevelNames[0] ?? '') : ''
  const files = rootDirName
    ? normalizedEntries.map(entry => ({
        ...entry,
        relativePath: entry.relativePath.startsWith(`${rootDirName}/`)
          ? entry.relativePath.slice(rootDirName.length + 1)
          : entry.relativePath,
      }))
    : normalizedEntries

  return {
    name: rootDirName || files[0]?.fileName || 'uploaded-folder',
    rootDirName: rootDirName || undefined,
    files,
  }
}

async function handleUploadFile() {
  if (!projectId.value) {
    return
  }
  try {
    const payload = await pickResourceFile()
    if (!payload) {
      return
    }
    const record = await resourceStore.importProjectResource(projectId.value, {
      name: payload.fileName,
      scope: 'project',
      visibility: 'public',
      files: [
        {
          ...payload,
          relativePath: payload.fileName,
        },
      ],
    })
    await selectResource(record.id)
    await notifications.notifyUploadSuccess('file', record.name, record.storagePath || resourceDirectory.value)
  } catch (error) {
    const message = error instanceof Error ? error.message : t('resources.notifications.fallbackError')
    await notifications.notifyUploadFailed('file', message)
  }
}

async function handleUploadFolder() {
  if (!projectId.value) {
    return
  }
  try {
    const entries = await pickResourceFolder()
    if (!entries?.length) {
      return
    }
    const normalized = normalizeFolderImport(entries)
    const record = await resourceStore.importProjectResource(projectId.value, {
      name: normalized.name,
      rootDirName: normalized.rootDirName,
      scope: 'project',
      visibility: 'public',
      files: normalized.files,
    })
    await selectResource(record.id)
    await notifications.notifyUploadSuccess('folder', record.name, record.storagePath || resourceDirectory.value)
  } catch (error) {
    const message = error instanceof Error ? error.message : t('resources.notifications.fallbackError')
    await notifications.notifyUploadFailed('folder', message)
  }
}

function closeDeleteDialog() {
  if (deletingSelected.value) {
    return
  }
  deleteDialogOpen.value = false
  deleteTarget.value = null
}

function editorLanguage() {
  if (!selectedResource.value) {
    return 'plaintext'
  }
  if (selectedResource.value.previewKind === 'markdown') {
    return 'markdown'
  }
  if (selectedResource.value.contentType?.includes('json') || selectedResource.value.name.endsWith('.json')) {
    return 'json'
  }
  if (selectedResource.value.previewKind === 'code') {
    return 'code'
  }
  return 'plaintext'
}
</script>

<template>
  <UiPageShell width="wide" test-id="project-resources-view">
    <UiPageHeader
      :eyebrow="t('resources.header.eyebrow')"
      :title="projectRecord?.name ?? t('resources.header.projectTitleFallback')"
      :description="projectRecord?.description || t('resources.header.subtitle')"
    />

    <div
      data-testid="project-resource-directory"
      class="flex flex-col gap-2 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3 shadow-xs md:flex-row md:items-center md:justify-between"
    >
      <div class="text-[12px] font-semibold text-text-secondary">
        {{ t('resources.mappingDirectory.label') }}
      </div>
      <code
        class="truncate rounded-[var(--radius-xs)] bg-subtle px-2 py-1 font-mono text-[12px] text-text-primary"
        :title="projectRecord?.resourceDirectory || ''"
      >
        {{ resourceDirectory }}
      </code>
    </div>

    <UiStatusCallout
      v-if="resourceStore.error"
      tone="error"
      :description="resourceStore.error"
    />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedResource)"
      :detail-title="selectedResource?.name"
      :detail-subtitle="selectedResource ? resourceLocation(selectedResource) : t('common.na')"
      :empty-detail-title="t('resources.detail.emptyTitle')"
      :empty-detail-description="t('resources.detail.emptyDescription')"
      detail-class="xl:min-w-[420px]"
    >
      <template #toolbar>
        <UiToolbarRow test-id="project-resources-toolbar">
          <template #search>
            <UiInput
              v-model="searchQuery"
              :placeholder="t('resources.filters.searchPlaceholder')"
            />
          </template>

          <template #filters>
            <UiField :label="t('resources.filters.allScopes')" class="w-full md:w-[180px]">
              <UiSelect v-model="scopeFilter" :options="scopeOptions" />
            </UiField>
            <UiField :label="t('resources.filters.allVisibility')" class="w-full md:w-[180px]">
              <UiSelect v-model="visibilityFilter" :options="visibilityOptions" />
            </UiField>
          </template>

          <template #actions>
            <UiButton
              variant="outline"
              size="sm"
              data-testid="project-resources-upload-file"
              @click="handleUploadFile"
            >
              <UploadIcon :size="14" />
              {{ t('resources.actions.uploadFile') }}
            </UiButton>
            <UiButton
              variant="outline"
              size="sm"
              data-testid="project-resources-upload-folder"
              @click="handleUploadFolder"
            >
              <FolderIcon :size="14" />
              {{ t('resources.actions.uploadFolder') }}
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <div class="space-y-3">
          <div v-if="pagedResources.length" class="space-y-2">
            <div
              v-for="resource in pagedResources"
              :key="resource.id"
              class="rounded-[var(--radius-l)]"
              @click="selectResource(resource.id)"
            >
              <UiListRow
                :title="resource.name"
                :subtitle="resourceLocation(resource)"
                :eyebrow="resourceBadgeLabel('resourceScope', resource.scope)"
                interactive
                :active="isSelectedResource(resource.id)"
              >
                <template #meta>
                  <UiBadge :label="resourceBadgeLabel('resourceKind', resource.kind)" subtle />
                  <UiBadge :label="resourceBadgeLabel('resourceVisibility', resource.visibility)" subtle />
                  <UiBadge
                    v-if="shouldShowStatusBadge(resource.status)"
                    :label="resourceBadgeLabel('resourceStatus', resource.status)"
                    subtle
                  />
                  <span class="text-xs text-text-tertiary">{{ formatDateTime(resource.updatedAt) }}</span>
                </template>

                <template #actions>
                  <div class="flex items-center gap-2" @click.stop>
                    <UiSwitch
                      :model-value="resource.status !== 'attention'"
                      :disabled="isStatusUpdating(resource.id)"
                      :data-testid="`project-resource-status-toggle-${resource.id}`"
                      @update:model-value="handleToggleStatus(resource, Boolean($event))"
                    >
                      <span class="sr-only">{{ t('resources.list.toggleStatus', { name: resource.name }) }}</span>
                    </UiSwitch>
                  </div>
                </template>
              </UiListRow>
            </div>
          </div>

          <UiEmptyState
            v-else
            :title="t('resources.empty.title')"
            :description="t('resources.empty.description')"
          />

          <UiPagination
            v-if="totalFilteredResources"
            :page="currentPage"
            :page-count="pageCount"
            :previous-label="t('resources.pagination.previous')"
            :next-label="t('resources.pagination.next')"
            :meta-label="t('resources.pagination.meta', { count: totalFilteredResources })"
            hide-page-info
            root-test-id="project-resources-pagination"
            @update:page="pagination.setPage"
          />
        </div>
      </template>

      <template #detail>
        <section
          v-if="selectedResource"
          data-testid="project-resource-detail"
          class="space-y-4"
        >
          <div class="flex flex-wrap items-center gap-2">
            <UiBadge :label="resourceBadgeLabel('resourceKind', selectedResource.kind)" subtle />
            <UiBadge :label="resourceBadgeLabel('resourceScope', selectedResource.scope)" subtle />
            <UiBadge :label="resourceBadgeLabel('resourceVisibility', selectedResource.visibility)" subtle />
            <UiBadge
              v-if="shouldShowStatusBadge(selectedResource.status)"
              :label="resourceBadgeLabel('resourceStatus', selectedResource.status)"
              subtle
            />
            <UiBadge :label="t(`resources.preview.${selectedResource.previewKind}`)" subtle />
          </div>

          <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_280px]">
            <div class="grid gap-3 rounded-[var(--radius-l)] border border-border bg-subtle px-3 py-3 text-sm text-text-secondary">
              <div>{{ t('resources.detail.location') }}: {{ resourceLocation(selectedResource) }}</div>
              <div>{{ t('resources.detail.owner') }}: {{ selectedResource.ownerUserId }}</div>
              <div>{{ t('resources.detail.updatedAt') }}: {{ formatDateTime(selectedResource.updatedAt) }}</div>
              <div>{{ t('resources.detail.byteSize') }}: {{ formatByteSize(selectedContent?.byteSize ?? selectedResource.byteSize) }}</div>
              <div>{{ t('resources.detail.contentType') }}: {{ previewContentType || t('common.na') }}</div>
            </div>

            <div class="space-y-3 rounded-[var(--radius-l)] border border-border bg-surface px-3 py-3">
              <div class="text-[12px] font-semibold text-text-secondary">
                {{ t('resources.detail.actions') }}
              </div>

              <UiField :label="t('resources.detail.visibility')">
                <UiSelect
                  :model-value="selectedVisibility"
                  :options="detailVisibilityOptions"
                  :disabled="updatingVisibility"
                  data-testid="project-resource-detail-visibility"
                  @update:model-value="handleVisibilityChange"
                />
              </UiField>

              <div class="flex flex-wrap gap-2">
                <UiButton
                  v-if="nextPromoteScope(selectedResource.scope) && canSubmitPromotion"
                  size="sm"
                  variant="outline"
                  :disabled="promotingSelected"
                  data-testid="project-resource-detail-promote"
                  @click="handlePromoteSelected"
                >
                  {{ t(`resources.actions.promoteTo.${nextPromoteScope(selectedResource.scope)}`) }}
                </UiButton>
                <UiButton
                  size="sm"
                  variant="destructive"
                  :disabled="deletingSelected"
                  data-testid="project-resource-detail-delete"
                  @click="handleDeleteSelected"
                >
                  <Trash2Icon :size="14" />
                  {{ t('resources.actions.delete') }}
                </UiButton>
              </div>
            </div>
          </div>

          <UiStatusCallout
            v-if="detailLoading"
            :description="t('resources.preview.loading')"
          />

          <UiCodeEditor
            v-else-if="selectedContent && ['text', 'code', 'markdown'].includes(selectedResource.previewKind)"
            readonly
            :language="editorLanguage()"
            :model-value="selectedContent.textContent || ''"
          />

          <div
            v-else-if="selectedContent && selectedResource.previewKind === 'image' && previewSrc"
            class="overflow-hidden rounded-[var(--radius-l)] border border-border bg-surface p-2"
          >
            <img
              :src="previewSrc"
              :alt="selectedResource.name"
              class="max-h-[420px] w-full object-contain"
              data-testid="project-resource-image-preview"
            >
          </div>

          <div
            v-else-if="selectedContent && selectedResource.previewKind === 'pdf' && previewSrc"
            class="overflow-hidden rounded-[var(--radius-l)] border border-border bg-surface"
          >
            <iframe :src="previewSrc" class="h-[520px] w-full" :title="selectedResource.name" />
          </div>

          <audio
            v-else-if="selectedContent && selectedResource.previewKind === 'audio' && previewSrc"
            controls
            class="w-full"
            :src="previewSrc"
          />

          <video
            v-else-if="selectedContent && selectedResource.previewKind === 'video' && previewSrc"
            controls
            class="w-full rounded-[var(--radius-l)] border border-border"
            :src="previewSrc"
          />

          <div v-else-if="selectedResource.previewKind === 'folder'" class="space-y-2">
            <div
              v-for="child in selectedChildren"
              :key="child.relativePath"
              class="flex items-center justify-between gap-3 rounded-[var(--radius-l)] border border-border bg-surface px-3 py-3"
            >
              <div class="min-w-0">
                <div class="truncate text-sm font-semibold text-text-primary">{{ child.name }}</div>
                <div class="truncate font-mono text-[12px] text-text-secondary">{{ child.relativePath }}</div>
              </div>
              <div class="flex flex-wrap items-center gap-2">
                <UiBadge :label="resourceBadgeLabel('resourcePreviewKind', child.previewKind)" subtle />
                <span class="text-xs text-text-tertiary">{{ formatByteSize(child.byteSize) }}</span>
                <span class="text-xs text-text-tertiary">{{ formatDateTime(child.updatedAt) }}</span>
              </div>
            </div>

            <UiEmptyState
              v-if="!selectedChildren.length"
              :title="t('resources.preview.folderEmptyTitle')"
              :description="t('resources.preview.folderEmptyDescription')"
            />
          </div>

          <div v-else-if="selectedContent && selectedResource.previewKind === 'url'" class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-4">
            <div class="text-sm font-semibold text-text-primary">{{ t('resources.preview.url') }}</div>
            <a
              class="mt-2 block break-all text-sm text-primary underline-offset-2 hover:underline"
              :href="selectedContent.externalUrl || selectedResource.location"
              target="_blank"
              rel="noreferrer"
            >
              {{ selectedContent.externalUrl || selectedResource.location }}
            </a>
          </div>

          <div
            v-else
            data-testid="project-resource-preview-fallback"
            class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-4 text-sm text-text-secondary"
          >
            {{ t('resources.preview.unavailable') }}
          </div>
        </section>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      v-model:open="deleteDialogOpen"
      :title="t('resources.deleteDialog.title')"
      :description="t('resources.deleteDialog.description')"
      content-test-id="project-resource-delete-dialog"
    >
      <div class="space-y-2">
        <div class="text-sm font-semibold text-text-primary">
          {{ deleteTarget?.name || t('common.na') }}
        </div>
        <div class="break-all font-mono text-[12px] text-text-secondary">
          {{ deleteTarget ? resourceLocation(deleteTarget) : t('common.na') }}
        </div>
      </div>

      <template #footer>
        <UiButton variant="ghost" :disabled="deletingSelected" @click="closeDeleteDialog">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :disabled="deletingSelected"
          data-testid="project-resource-delete-confirm"
          @click="confirmDeleteSelected"
        >
          {{ t('resources.deleteDialog.confirm') }}
        </UiButton>
      </template>
    </UiDialog>
  </UiPageShell>
</template>
