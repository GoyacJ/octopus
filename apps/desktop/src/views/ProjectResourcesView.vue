<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  Eye,
  ExternalLink,
  FileText,
  FolderOpen,
  LayoutGrid,
  Link2,
  List,
  Pencil,
  Plus,
  Search,
  Trash2,
} from 'lucide-vue-next'

import type { Artifact, ProjectResource } from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiDialog,
  UiEmptyState,
  UiInput,
  UiListRow,
  UiMetricCard,
  UiRecordCard,
  UiSectionHeading,
  UiTabs,
} from '@octopus/ui'

import { usePagination } from '@/composables/usePagination'
import { useWorkbenchStore } from '@/stores/workbench'

const PAGE_SIZE = 5
const { t } = useI18n()
const workbench = useWorkbenchStore()

const viewMode = ref<'list' | 'grid'>('list')
const activeTab = ref<'all' | 'source' | 'generated'>('all')
const searchQuery = ref('')
const addMenuOpen = ref(false)
const previewResourceId = ref('')
const editingResourceId = ref('')
const deletingResourceId = ref('')
const creatingUrl = ref(false)
const createUrlNameDraft = ref('')
const createUrlLocationDraft = ref('')
const editNameDraft = ref('')
const editLocationDraft = ref('')

const resources = computed(() => workbench.projectResources)
const normalizedSearchQuery = computed(() => searchQuery.value.trim().toLowerCase())
const tabItems = computed(() => [
  { value: 'all', label: t('resources.tabs.all') },
  { value: 'source', label: t('resources.tabs.source') },
  { value: 'generated', label: t('resources.tabs.generated') },
])

const filteredResources = computed(() => {
  return resources.value.filter((resource) => {
    if (activeTab.value !== 'all' && resource.origin !== activeTab.value) return false
    if (!normalizedSearchQuery.value) return true

    const haystack = [
      resourceLabel(resource),
      resource.location ?? '',
      resource.kind,
      resource.origin,
      ...resource.tags,
    ].join(' ').toLowerCase()

    return haystack.includes(normalizedSearchQuery.value)
  })
})

const previewResource = computed(() =>
  resources.value.find((resource) => resource.id === previewResourceId.value),
)
const editingResource = computed(() =>
  resources.value.find((resource) => resource.id === editingResourceId.value),
)
const deletingResource = computed(() =>
  resources.value.find((resource) => resource.id === deletingResourceId.value),
)
const previewArtifact = computed<Artifact | undefined>(() => {
  const resource = previewResource.value
  if (!resource || resource.kind !== 'artifact') {
    return undefined
  }

  return workbench.artifacts.find((artifact) => artifact.id === (resource.sourceArtifactId ?? resource.id))
})

const resourceStats = computed(() => [
  {
    label: t('resources.stats.total'),
    value: String(resources.value.length),
  },
  {
    label: t('resources.stats.source'),
    value: String(resources.value.filter((resource) => resource.origin === 'source').length),
  },
  {
    label: t('resources.stats.generated'),
    value: String(resources.value.filter((resource) => resource.origin === 'generated').length),
  },
])

const {
  currentPage,
  pageCount,
  pagedItems,
  isFirstPage,
  isLastPage,
  nextPage,
  previousPage,
  resetPage,
} = usePagination(filteredResources, {
  pageSize: PAGE_SIZE,
  resetOn: [searchQuery, activeTab, viewMode],
})

function resourceLabel(resource: ProjectResource): string {
  return workbench.projectResourceDisplayName(resource.id)
}

function resourceKindLabel(resource: ProjectResource): string {
  return t(`resources.kinds.${resource.kind}`)
}

function resourceOriginLabel(resource: ProjectResource): string {
  return resource.origin === 'generated' ? t('resources.tabs.generated') : t('resources.tabs.source')
}

function resourceIcon(resource: ProjectResource) {
  if (resource.kind === 'folder') {
    return FolderOpen
  }

  if (resource.kind === 'url') {
    return Link2
  }

  return FileText
}

function resourceSecondaryText(resource: ProjectResource): string {
  return resource.location || resourceKindLabel(resource)
}

function resourceMetaLabel(resource: ProjectResource): string {
  return resource.sizeLabel || t('common.na')
}

function openCreateUrlModal() {
  createUrlNameDraft.value = ''
  createUrlLocationDraft.value = ''
  creatingUrl.value = true
  addMenuOpen.value = false
}

function closeCreateUrlModal() {
  creatingUrl.value = false
  createUrlNameDraft.value = ''
  createUrlLocationDraft.value = ''
}

function submitUrlResource() {
  if (!createUrlNameDraft.value.trim() || !createUrlLocationDraft.value.trim()) {
    return
  }

  workbench.createProjectResource('url', {
    name: createUrlNameDraft.value,
    location: createUrlLocationDraft.value,
    origin: 'source',
  })
  resetPage()
  closeCreateUrlModal()
}

function toggleAddMenu() {
  addMenuOpen.value = !addMenuOpen.value
}

function createFile() {
  addMenuOpen.value = false
  workbench.createProjectResource('file')
}

function createFolder() {
  addMenuOpen.value = false
  workbench.createProjectResource('folder')
}

function createUrl() {
  openCreateUrlModal()
}

function openPreview(resourceId: string) {
  previewResourceId.value = resourceId
}

function closePreview() {
  previewResourceId.value = ''
}

function openEdit(resourceId: string) {
  const resource = resources.value.find((item) => item.id === resourceId)
  if (!resource) {
    return
  }

  editingResourceId.value = resourceId
  editNameDraft.value = resourceLabel(resource)
  editLocationDraft.value = resource.location ?? ''
}

function closeEdit() {
  editingResourceId.value = ''
  editNameDraft.value = ''
  editLocationDraft.value = ''
}

function saveEdit() {
  if (!editingResource.value || !editNameDraft.value.trim()) {
    return
  }

  workbench.updateProjectResource(editingResource.value.id, {
    name: editNameDraft.value,
    location: editingResource.value.kind === 'url' ? editLocationDraft.value : undefined,
  })
  closeEdit()
}

function openDelete(resourceId: string) {
  deletingResourceId.value = resourceId
}

function closeDelete() {
  deletingResourceId.value = ''
}

function confirmDelete() {
  const resource = deletingResource.value
  if (!resource) {
    return
  }

  workbench.removeProjectResource(resource.id)
  if (previewResourceId.value === resource.id) {
    closePreview()
  }
  closeDelete()
}
</script>

<template>
  <div class="w-full flex flex-col gap-6 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0 space-y-6">
      <UiSectionHeading :eyebrow="t('resources.header.eyebrow')" :title="'项目资源中心'" />
      <div class="grid gap-3 sm:grid-cols-2 md:grid-cols-4">
        <UiMetricCard :label="t('resources.stats.total')" :value="resources.length" tone="muted" />
        <UiMetricCard :label="'原始资源'" :value="resources.filter((resource) => resource.origin === 'source').length" tone="muted" />
        <UiMetricCard :label="t('resources.stats.generated')" :value="resources.filter((resource) => resource.origin === 'generated').length" tone="muted" />
      </div>
    </header>

    <div data-testid="resources-toolbar" class="px-2 flex flex-wrap items-center justify-between gap-4 border-b border-border-subtle pb-4">
      <UiTabs v-model="activeTab" :tabs="tabItems" />
      <div class="flex items-center gap-2">
        <div class="relative w-64">
          <Search :size="14" class="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-tertiary" />
          <UiInput
            v-model="searchQuery"
            data-testid="resources-search-input"
            class="pl-8 bg-subtle/30 h-8 text-[13px]"
            :placeholder="t('resources.search.placeholder')"
          />
        </div>

        <div class="flex items-center gap-1 bg-subtle/50 rounded-md p-0.5 border border-border-subtle dark:border-white/[0.08]">
          <UiButton
            data-testid="resources-view-list"
            variant="ghost"
            size="icon"
            class="h-6 w-6 rounded"
            :class="viewMode === 'list' ? 'bg-background shadow-sm text-text-primary' : 'text-text-tertiary'"
            @click="viewMode = 'list'"
          >
            <List :size="14" />
          </UiButton>
          <UiButton
            data-testid="resources-view-grid"
            variant="ghost"
            size="icon"
            class="h-6 w-6 rounded"
            :class="viewMode === 'grid' ? 'bg-background shadow-sm text-text-primary' : 'text-text-tertiary'"
            @click="viewMode = 'grid'"
          >
            <LayoutGrid :size="14" />
          </UiButton>
        </div>

        <div class="relative">
          <UiButton data-testid="resources-add-trigger" variant="primary" class="h-8" @click="toggleAddMenu">
            <Plus :size="16" />
            {{ t('resources.actions.add') }}
          </UiButton>
          <div
            v-if="addMenuOpen"
            data-testid="resources-add-menu"
            class="absolute right-0 top-full z-10 mt-2 min-w-40 rounded-md border border-border-subtle bg-background p-1 shadow-lg"
          >
            <button data-testid="resources-add-file" type="button" class="flex w-full items-center gap-2 rounded px-3 py-2 text-left text-sm hover:bg-accent" @click="createFile">
              <FileText :size="14" />
              {{ t('resources.actions.uploadFile') }}
            </button>
            <button data-testid="resources-add-folder" type="button" class="flex w-full items-center gap-2 rounded px-3 py-2 text-left text-sm hover:bg-accent" @click="createFolder">
              <FolderOpen :size="14" />
              {{ t('resources.actions.uploadFolder') }}
            </button>
            <button data-testid="resources-add-url" type="button" class="flex w-full items-center gap-2 rounded px-3 py-2 text-left text-sm hover:bg-accent" @click="createUrl">
              <Link2 :size="14" />
              {{ t('resources.actions.addUrl') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <main class="flex-1 overflow-y-auto min-h-0 px-2 space-y-6">
      <template v-if="pagedItems.length">
        <div v-if="viewMode === 'list'" class="flex flex-col gap-1">
          <div v-for="resource in pagedItems" :key="resource.id" :data-testid="`resource-item-${resource.id}`">
            <UiListRow
              :title="resourceLabel(resource)"
              :subtitle="resourceSecondaryText(resource)"
              interactive
              @click="openPreview(resource.id)"
            >
            <template #meta>
              <div class="flex items-center gap-2 text-[11px] text-text-tertiary font-medium">
                <component :is="resourceIcon(resource)" :size="12" />
                <span>{{ resourceMetaLabel(resource) }}</span>
                <span>·</span>
                <span>{{ resourceKindLabel(resource) }}</span>
              </div>
            </template>

            <template #actions>
              <div class="flex items-center gap-1">
                <UiButton :data-testid="`resource-preview-${resource.id}`" variant="ghost" size="icon" class="h-7 w-7" @click.stop="openPreview(resource.id)">
                  <Eye :size="14" />
                </UiButton>
                <UiButton :data-testid="`resource-edit-${resource.id}`" variant="ghost" size="icon" class="h-7 w-7" @click.stop="openEdit(resource.id)">
                  <Pencil :size="14" />
                </UiButton>
                <UiButton :data-testid="`resource-delete-${resource.id}`" variant="ghost" size="icon" class="h-7 w-7 text-destructive hover:bg-destructive/10" @click.stop="openDelete(resource.id)">
                  <Trash2 :size="14" />
                </UiButton>
              </div>
            </template>
            </UiListRow>
          </div>
        </div>

        <div v-else data-testid="resources-grid" class="grid gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
          <UiRecordCard
            v-for="resource in pagedItems"
            :key="resource.id"
            :test-id="`resource-item-${resource.id}`"
            :title="resourceLabel(resource)"
            :description="resourceSecondaryText(resource)"
            interactive
            @click="openPreview(resource.id)"
          >
            <template #leading>
              <component :is="resourceIcon(resource)" :size="18" class="text-primary opacity-80" />
            </template>
            <template #badges>
              <UiBadge :label="resourceOriginLabel(resource)" :tone="resource.origin === 'generated' ? 'info' : 'default'" subtle />
            </template>
            <template #meta>
              <span class="text-[11px] font-bold uppercase tracking-wider text-text-tertiary">{{ resourceMetaLabel(resource) }}</span>
            </template>
            <template #actions>
              <div class="flex items-center gap-1">
                <UiButton :data-testid="`resource-preview-${resource.id}`" variant="ghost" size="icon" class="h-6 w-6" @click.stop="openPreview(resource.id)">
                  <Eye :size="12" />
                </UiButton>
                <UiButton :data-testid="`resource-edit-${resource.id}`" variant="ghost" size="icon" class="h-6 w-6" @click.stop="openEdit(resource.id)">
                  <Pencil :size="12" />
                </UiButton>
                <UiButton :data-testid="`resource-delete-${resource.id}`" variant="ghost" size="icon" class="h-6 w-6 text-destructive hover:bg-destructive/10" @click.stop="openDelete(resource.id)">
                  <Trash2 :size="12" />
                </UiButton>
              </div>
            </template>
          </UiRecordCard>
        </div>
      </template>

      <UiEmptyState
        v-else
        :title="t('resources.empty.emptyTitle')"
        :description="t('resources.empty.emptyDescription')"
      />

      <div class="mt-8 flex items-center justify-center gap-4 border-t border-border-subtle pt-4">
        <UiButton data-testid="resources-pagination-prev" variant="ghost" size="sm" :disabled="isFirstPage" @click="previousPage">
          {{ t('resources.pagination.previous') }}
        </UiButton>
        <span>{{ t('resources.pagination.summary', { page: currentPage, total: pageCount }) }}</span>
        <UiButton data-testid="resources-pagination-next" variant="ghost" size="sm" :disabled="isLastPage" @click="nextPage">
          {{ t('resources.pagination.next') }}
        </UiButton>
      </div>
    </main>

    <UiDialog :open="creatingUrl" content-test-id="resource-url-modal" @update:open="(open) => { if (!open) closeCreateUrlModal() }">
      <div class="grid gap-4">
        <UiInput v-model="createUrlNameDraft" data-testid="resource-url-name-input" :placeholder="t('resources.editForm.nameLabel')" />
        <UiInput v-model="createUrlLocationDraft" data-testid="resource-url-location-input" :placeholder="t('resources.editForm.locationLabel')" />
      </div>
      <template #footer>
        <UiButton variant="ghost" @click="closeCreateUrlModal">{{ t('resources.actions.cancel') }}</UiButton>
        <UiButton data-testid="resource-url-confirm" variant="primary" @click="submitUrlResource">{{ t('resources.editForm.confirmCreate') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog :open="Boolean(previewResource)" content-test-id="resource-preview-modal" @update:open="(open) => { if (!open) closePreview() }">
      <div v-if="previewResource" class="space-y-6">
        <div class="flex items-center justify-between">
          <h3 class="text-xl font-bold text-text-primary">{{ resourceLabel(previewResource) }}</h3>
          <div class="flex gap-2">
            <UiBadge :label="resourceKindLabel(previewResource)" subtle />
            <UiBadge :label="resourceOriginLabel(previewResource)" :tone="previewResource.origin === 'generated' ? 'info' : 'default'" subtle />
          </div>
        </div>

        <div class="grid gap-4 sm:grid-cols-2 text-[13px] border-y border-border-subtle py-4">
          <div><strong class="block text-text-tertiary text-[10px] uppercase mb-1">Size</strong><span>{{ resourceMetaLabel(previewResource) }}</span></div>
          <div><strong class="block text-text-tertiary text-[10px] uppercase mb-1">Links</strong><span>{{ previewResource.linkedConversationIds.length }}</span></div>
          <div><strong class="block text-text-tertiary text-[10px] uppercase mb-1">Location</strong><span class="break-all">{{ resourceSecondaryText(previewResource) }}</span></div>
          <div><strong class="block text-text-tertiary text-[10px] uppercase mb-1">Tags</strong><span>{{ previewResource.tags.join(', ') || t('common.na') }}</span></div>
        </div>

        <div v-if="previewArtifact" class="bg-subtle/30 rounded-md border border-border-subtle p-3">
          <pre class="text-[12px] text-text-secondary whitespace-pre-wrap font-mono">{{ previewArtifact.content }}</pre>
        </div>

        <a v-if="previewResource.kind === 'url' && previewResource.location" class="inline-flex items-center gap-1.5 text-sm font-medium text-primary hover:underline" :href="previewResource.location" target="_blank">
          <ExternalLink :size="14" /> Open link
        </a>
      </div>
      <template #footer>
        <UiButton data-testid="resource-preview-close" variant="ghost" @click="closePreview">{{ t('resources.actions.cancel') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog :open="Boolean(editingResource)" content-test-id="resource-edit-modal" @update:open="(open) => { if (!open) closeEdit() }">
      <div v-if="editingResource" class="grid gap-4">
        <UiInput v-model="editNameDraft" data-testid="resource-edit-name-input" :placeholder="t('resources.editForm.nameLabel')" />
        <UiInput v-if="editingResource.kind === 'url'" v-model="editLocationDraft" data-testid="resource-edit-location-input" :placeholder="t('resources.editForm.locationLabel')" />
      </div>
      <template #footer>
        <UiButton variant="ghost" @click="closeEdit">{{ t('resources.actions.cancel') }}</UiButton>
        <UiButton data-testid="resource-edit-confirm" variant="primary" @click="saveEdit">{{ t('resources.editForm.confirmSave') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog :open="Boolean(deletingResource)" content-test-id="resource-delete-modal" @update:open="(open) => { if (!open) closeDelete() }">
      <p class="text-[14px] text-text-secondary">{{ t('resources.deleteDialog.description') }}</p>
      <template #footer>
        <UiButton variant="ghost" @click="closeDelete">{{ t('resources.actions.cancel') }}</UiButton>
        <UiButton data-testid="resource-delete-confirm" variant="destructive" @click="confirmDelete">{{ t('resources.deleteDialog.confirm') }}</UiButton>
      </template>
    </UiDialog>
  </div>
</template>
