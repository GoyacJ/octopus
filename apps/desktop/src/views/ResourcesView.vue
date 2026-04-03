<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import {
  ChevronLeft,
  ChevronRight,
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
  UiPagination,
  UiRecordCard,
  UiSectionHeading,
  UiSelectionMenu,
  UiTabs,
} from '@octopus/ui'

import { usePagination } from '@/composables/usePagination'
import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

type ResourceViewMode = 'list' | 'grid'
type ResourceTab = 'all' | 'source' | 'generated'

const PAGE_SIZE = 12

const { t } = useI18n()
const router = useRouter()
const workbench = useWorkbenchStore()

const viewMode = ref<ResourceViewMode>('list')
const activeTab = ref<ResourceTab>('all')
const searchQuery = ref('')
const addMenuOpen = ref(false)
const previewResourceId = ref('')
const editingResourceId = ref('')
const deletingResourceId = ref('')
const creatingUrl = ref(false)
const editNameDraft = ref('')
const editLocationDraft = ref('')
const createUrlNameDraft = ref('')
const createUrlLocationDraft = ref('')
const fileInputRef = ref<HTMLInputElement | null>(null)
const folderInputRef = ref<HTMLInputElement | null>(null)

const resources = computed(() => workbench.projectResources)
const normalizedSearchQuery = computed(() => searchQuery.value.trim().toLowerCase())
const tabItems = computed(() => [
  { value: 'all', label: t('resources.tabs.all') },
  { value: 'source', label: t('resources.tabs.source') },
  { value: 'generated', label: t('resources.tabs.generated') },
])
const addMenuSections = computed(() => [
  {
    id: 'resource-create',
    items: [
      { id: 'file', label: t('resources.actions.uploadFile'), icon: FileText, testId: 'resources-add-file' },
      { id: 'folder', label: t('resources.actions.uploadFolder'), icon: FolderOpen, testId: 'resources-add-folder' },
      { id: 'url', label: t('resources.actions.addUrl'), icon: Link2, testId: 'resources-add-url' },
    ],
  },
])
const filteredResources = computed(() => {
  return resources.value.filter((resource) => {
    if (activeTab.value !== 'all' && resource.origin !== activeTab.value) {
      return false
    }

    if (!normalizedSearchQuery.value) {
      return true
    }

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
  {
    label: t('resources.stats.artifact'),
    value: String(resources.value.filter((resource) => resource.kind === 'artifact').length),
  },
])

const {
  currentPage,
  pageCount: totalPages,
  pagedItems: paginatedResources,
  setPage,
} = usePagination(filteredResources, {
  pageSize: PAGE_SIZE,
  resetOn: [searchQuery, viewMode, activeTab],
})

function resourceLabel(resource: ProjectResource): string {
  return resolveMockField('projectResource', resource.id, 'name', resource.name)
}

function resourceKindLabel(resource: ProjectResource): string {
  return t(`resources.kinds.${resource.kind}`)
}

function resourceOriginLabel(resource: ProjectResource): string {
  return t(`resources.origins.${resource.origin}`)
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

function supportsBrowserPicker(): boolean {
  return typeof window !== 'undefined' && !/jsdom/i.test(window.navigator.userAgent)
}

function createLocationFromName(name: string): string {
  return `/selected/${name}`
}

function createFile() {
  addMenuOpen.value = false
  if (!supportsBrowserPicker()) {
    workbench.createProjectResource('file')
    return
  }

  fileInputRef.value?.click()
}

function createFolder() {
  addMenuOpen.value = false
  if (!supportsBrowserPicker()) {
    workbench.createProjectResource('folder')
    return
  }

  folderInputRef.value?.click()
}

function handleAddAction(action: string) {
  if (action === 'file') {
    createFile()
    return
  }

  if (action === 'folder') {
    createFolder()
    return
  }

  openCreateUrlModal()
}

function handleFileSelection(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) {
    input.value = ''
    return
  }

  workbench.createProjectResource('file', {
    name: file.name,
    location: createLocationFromName(file.name),
    origin: 'source',
  })
  input.value = ''
}

function handleFolderSelection(event: Event) {
  const input = event.target as HTMLInputElement
  const files = input.files
  const firstFile = files?.[0] as (File & { webkitRelativePath?: string }) | undefined
  if (!firstFile) {
    input.value = ''
    return
  }

  const folderName = firstFile.webkitRelativePath?.split('/').filter(Boolean)[0] ?? firstFile.name
  workbench.createProjectResource('folder', {
    name: folderName,
    location: createLocationFromName(folderName),
    origin: 'source',
  })
  input.value = ''
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
  closeCreateUrlModal()
}

function openPreview(resourceId: string) {
  previewResourceId.value = resourceId
}

function closePreview() {
  previewResourceId.value = ''
}

async function openEdit(resourceId: string) {
  const resource = resources.value.find((item) => item.id === resourceId)
  if (!resource) {
    return
  }

  if (resource.kind === 'artifact') {
    const conversationId = resource.linkedConversationIds[0]
    if (!conversationId) {
      return
    }

    await router.push({
      ...createProjectConversationTarget(workbench.currentWorkspaceId, workbench.currentProjectId, conversationId),
      query: {
        detail: 'resources',
        artifact: resource.sourceArtifactId ?? resource.id,
      },
    })
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
  closeDelete()

  if (previewResourceId.value === resource.id) {
    closePreview()
  }
}
</script>

<template>
  <div class="w-full flex flex-col gap-6 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0 space-y-6">
      <UiSectionHeading
        :eyebrow="t('resources.header.eyebrow')"
        :title="t('resources.header.title')"
      />
      
      <div class="grid gap-3 sm:grid-cols-2 md:grid-cols-4">
        <UiMetricCard
          v-for="stat in resourceStats"
          :key="stat.label"
          :label="stat.label"
          :value="stat.value"
          tone="muted"
        />
      </div>
    </header>

    <input ref="fileInputRef" class="hidden" type="file" @change="handleFileSelection" >
    <input ref="folderInputRef" class="hidden" type="file" webkitdirectory directory multiple @change="handleFolderSelection" >

    <div class="px-2 flex flex-wrap items-center justify-between gap-4 border-b border-border-subtle pb-4">
      <UiTabs v-model="activeTab" :tabs="tabItems" />
      
      <div class="flex items-center gap-2">
        <div class="relative w-64">
          <Search :size="14" class="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-tertiary" />
          <UiInput v-model="searchQuery" class="pl-8 bg-subtle/30 h-8 text-[13px]" :placeholder="t('resources.search.placeholder')" />
        </div>

        <div class="h-4 w-px bg-border-subtle mx-1" />

        <div class="flex items-center gap-1 bg-subtle/50 rounded-md p-0.5 border border-border-subtle">
          <UiButton variant="ghost" size="icon" class="h-6 w-6 rounded" :class="viewMode === 'list' ? 'bg-background shadow-sm text-text-primary' : 'text-text-tertiary'" @click="viewMode = 'list'">
            <List :size="14" />
          </UiButton>
          <UiButton variant="ghost" size="icon" class="h-6 w-6 rounded" :class="viewMode === 'grid' ? 'bg-background shadow-sm text-text-primary' : 'text-text-tertiary'" @click="viewMode = 'grid'">
            <LayoutGrid :size="14" />
          </UiButton>
        </div>

        <UiSelectionMenu
          v-model:open="addMenuOpen"
          :title="t('resources.actions.add')"
          :sections="addMenuSections"
          @select="handleAddAction"
        >
          <template #trigger>
            <UiButton variant="primary" class="h-8">
              <Plus :size="16" />
              {{ t('resources.actions.add') }}
            </UiButton>
          </template>
        </UiSelectionMenu>
      </div>
    </div>

    <main class="flex-1 overflow-y-auto min-h-0 px-2 space-y-6">
      <template v-if="paginatedResources.length">
        <div v-if="viewMode === 'list'" class="flex flex-col gap-1">
          <UiListRow
            v-for="resource in paginatedResources"
            :key="resource.id"
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
              <div class="opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1 bg-background/80 backdrop-blur-sm rounded pl-2">
                <UiButton variant="ghost" size="icon" class="h-7 w-7 text-text-secondary" @click.stop="openEdit(resource.id)"><Pencil :size="14" /></UiButton>
                <UiButton variant="ghost" size="icon" class="h-7 w-7 text-destructive hover:bg-destructive/10" @click.stop="openDelete(resource.id)"><Trash2 :size="14" /></UiButton>
              </div>
            </template>
          </UiListRow>
        </div>

        <div v-else class="grid gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
          <UiRecordCard
            v-for="resource in paginatedResources"
            :key="resource.id"
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
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <UiButton variant="ghost" size="icon" class="h-6 w-6" @click.stop="openEdit(resource.id)"><Pencil :size="12" /></UiButton>
                <UiButton variant="ghost" size="icon" class="h-6 w-6 text-destructive hover:bg-destructive/10" @click.stop="openDelete(resource.id)"><Trash2 :size="12" /></UiButton>
              </div>
            </template>
          </UiRecordCard>
        </div>
      </template>

      <UiEmptyState
        v-else-if="normalizedSearchQuery"
        :title="t('resources.filteredEmpty.title')"
        :description="t('resources.filteredEmpty.description')"
      />
      <UiEmptyState
        v-else
        :title="t('resources.empty.emptyTitle')"
        :description="t('resources.empty.emptyDescription')"
      />

      <div v-if="totalPages > 1" class="pt-4 flex justify-center border-t border-border-subtle mt-8">
        <UiPagination :page="currentPage" :page-count="totalPages" @update:page="setPage" />
      </div>
    </main>

    <!-- Modals -->
    <UiDialog :open="creatingUrl" :title="t('resources.editForm.urlTitle')" @update:open="(o) => { if(!o) closeCreateUrlModal() }">
      <div class="grid gap-4">
        <label class="grid gap-2 text-[13px] font-medium text-text-secondary">
          <span>{{ t('resources.editForm.nameLabel') }}</span>
          <UiInput v-model="createUrlNameDraft" :placeholder="t('resources.editForm.namePlaceholder')" />
        </label>
        <label class="grid gap-2 text-[13px] font-medium text-text-secondary">
          <span>{{ t('resources.editForm.locationLabel') }}</span>
          <UiInput v-model="createUrlLocationDraft" :placeholder="t('resources.editForm.locationPlaceholder')" />
        </label>
      </div>
      <template #footer>
        <UiButton variant="ghost" @click="closeCreateUrlModal">{{ t('resources.actions.cancel') }}</UiButton>
        <UiButton variant="primary" @click="submitUrlResource">{{ t('resources.editForm.confirmCreate') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog :open="Boolean(previewResource)" :title="t('resources.preview.title')" class="max-w-2xl" @update:open="(o) => { if(!o) closePreview() }">
      <div v-if="previewResource" class="space-y-6">
        <div class="flex items-center justify-between">
          <h3 class="text-xl font-bold text-text-primary">{{ resourceLabel(previewResource) }}</h3>
          <div class="flex gap-2">
            <UiBadge :label="resourceKindLabel(previewResource)" subtle />
            <UiBadge :label="resourceOriginLabel(previewResource)" :tone="previewResource.origin === 'generated' ? 'info' : 'default'" subtle />
          </div>
        </div>

        <div class="grid gap-4 sm:grid-cols-2 text-[13px] border-y border-border-subtle py-4">
          <div><strong class="block text-text-tertiary text-[10px] uppercase mb-1">{{ t('resources.preview.size') }}</strong><span>{{ resourceMetaLabel(previewResource) }}</span></div>
          <div><strong class="block text-text-tertiary text-[10px] uppercase mb-1">{{ t('resources.preview.linkedConversations') }}</strong><span>{{ previewResource.linkedConversationIds.length }}</span></div>
          <div><strong class="block text-text-tertiary text-[10px] uppercase mb-1">{{ t('resources.preview.location') }}</strong><span class="break-all">{{ resourceSecondaryText(previewResource) }}</span></div>
          <div><strong class="block text-text-tertiary text-[10px] uppercase mb-1">{{ t('resources.preview.tags') }}</strong><span>{{ previewResource.tags.join(', ') || t('common.na') }}</span></div>
        </div>

        <div v-if="previewArtifact" class="bg-subtle/30 rounded-md border border-border-subtle p-3">
          <pre class="text-[12px] text-text-secondary whitespace-pre-wrap font-mono">{{ previewArtifact.content }}</pre>
        </div>

        <a v-if="previewResource.kind === 'url' && previewResource.location" class="inline-flex items-center gap-1.5 text-sm font-medium text-primary hover:underline" :href="previewResource.location" target="_blank">
          <ExternalLink :size="14" /> {{ t('resources.actions.openLink') }}
        </a>
      </div>
    </UiDialog>

    <UiDialog :open="Boolean(editingResource)" :title="t('resources.editForm.renameTitle')" @update:open="(o) => { if(!o) closeEdit() }">
      <div v-if="editingResource" class="grid gap-4">
        <label class="grid gap-2 text-[13px] font-medium text-text-secondary">
          <span>{{ t('resources.editForm.nameLabel') }}</span>
          <UiInput v-model="editNameDraft" />
        </label>
        <label v-if="editingResource.kind === 'url'" class="grid gap-2 text-[13px] font-medium text-text-secondary">
          <span>{{ t('resources.editForm.locationLabel') }}</span>
          <UiInput v-model="editLocationDraft" />
        </label>
      </div>
      <template #footer>
        <UiButton variant="ghost" @click="closeEdit">{{ t('resources.actions.cancel') }}</UiButton>
        <UiButton variant="primary" @click="saveEdit">{{ t('resources.editForm.confirmSave') }}</UiButton>
      </template>
    </UiDialog>

    <UiDialog :open="Boolean(deletingResource)" :title="t('resources.deleteDialog.title')" @update:open="(o) => { if(!o) closeDelete() }">
      <p class="text-[14px] text-text-secondary">{{ t('resources.deleteDialog.description') }}</p>
      <template #footer>
        <UiButton variant="ghost" @click="closeDelete">{{ t('resources.actions.cancel') }}</UiButton>
        <UiButton variant="destructive" @click="confirmDelete">{{ t('resources.deleteDialog.confirm') }}</UiButton>
      </template>
    </UiDialog>
  </div>
</template>
