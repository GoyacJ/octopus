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
import { UiBadge, UiButton, UiEmptyState, UiInput, UiPagination, UiSectionHeading, UiSurface } from '@octopus/ui'

import { usePagination } from '@/composables/usePagination'
import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

type ResourceViewMode = 'list' | 'grid'

const PAGE_SIZE = 5

const { t } = useI18n()
const router = useRouter()
const workbench = useWorkbenchStore()

const viewMode = ref<ResourceViewMode>('list')
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

const resources = computed(() => workbench.projectResources)
const normalizedSearchQuery = computed(() => searchQuery.value.trim().toLowerCase())
const filteredResources = computed(() => {
  if (!normalizedSearchQuery.value) {
    return resources.value
  }

  return resources.value.filter((resource) => {
    const haystack = [
      resourceLabel(resource),
      resource.location ?? '',
      resource.kind,
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
  { label: t('resources.kinds.file'), value: String(resources.value.filter((resource) => resource.kind === 'file').length) },
  { label: t('resources.kinds.folder'), value: String(resources.value.filter((resource) => resource.kind === 'folder').length) },
  { label: t('resources.kinds.artifact'), value: String(resources.value.filter((resource) => resource.kind === 'artifact').length) },
  { label: t('resources.kinds.url'), value: String(resources.value.filter((resource) => resource.kind === 'url').length) },
])

const {
  currentPage,
  pageCount: totalPages,
  pagedItems: paginatedResources,
  setPage,
} = usePagination(filteredResources, {
  pageSize: PAGE_SIZE,
  resetOn: [searchQuery, viewMode],
})

const paginationSummaryLabel = computed(() => t('resources.pagination.summary', {
  page: currentPage.value,
  total: totalPages.value,
}))

function resourceLabel(resource: ProjectResource): string {
  return resolveMockField('projectResource', resource.id, 'name', resource.name)
}

function resourceKindLabel(resource: ProjectResource): string {
  return t(`resources.kinds.${resource.kind}`)
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

function toggleAddMenu() {
  addMenuOpen.value = !addMenuOpen.value
}

function createFile() {
  workbench.createProjectResource('file')
  addMenuOpen.value = false
}

function createFolder() {
  workbench.createProjectResource('folder')
  addMenuOpen.value = false
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
  <section class="resources-page section-stack">
    <UiSectionHeading
      :eyebrow="t('resources.header.eyebrow')"
      :title="t('resources.header.title')"
    />

    <div class="resource-stats-grid">
      <UiSurface v-for="stat in resourceStats" :key="stat.label" class="resource-stat-card" padding="sm">
        <small>{{ stat.label }}</small>
        <strong>{{ stat.value }}</strong>
      </UiSurface>
    </div>

    <UiSurface>
      <div class="resource-toolbar">
        <div class="resource-search-shell">
          <Search :size="16" class="resource-search-icon" />
          <UiInput
            v-model="searchQuery"
            class="resource-search-input"
            data-testid="resources-search-input"
            :placeholder="t('resources.search.placeholder')"
          />
        </div>

        <div class="resource-toolbar-actions">
          <div class="resource-view-toggle">
            <UiButton
              variant="ghost"
              size="sm"
              :class="viewMode === 'list' ? 'active' : ''"
              data-testid="resources-view-list"
              @click="viewMode = 'list'"
            >
              <List :size="16" />
              <span>{{ t('resources.views.list') }}</span>
            </UiButton>
            <UiButton
              variant="ghost"
              size="sm"
              :class="viewMode === 'grid' ? 'active' : ''"
              data-testid="resources-view-grid"
              @click="viewMode = 'grid'"
            >
              <LayoutGrid :size="16" />
              <span>{{ t('resources.views.grid') }}</span>
            </UiButton>
          </div>

          <div class="resource-add-shell">
            <UiButton data-testid="resources-add-trigger" @click="toggleAddMenu">
              <Plus :size="16" />
              <span>{{ t('resources.actions.add') }}</span>
            </UiButton>

            <div v-if="addMenuOpen" class="resource-add-menu" data-testid="resources-add-menu">
              <button type="button" class="resource-menu-item" data-testid="resources-add-file" @click="createFile">
                {{ t('resources.actions.uploadFile') }}
              </button>
              <button type="button" class="resource-menu-item" data-testid="resources-add-folder" @click="createFolder">
                {{ t('resources.actions.uploadFolder') }}
              </button>
              <button type="button" class="resource-menu-item" data-testid="resources-add-url" @click="openCreateUrlModal">
                {{ t('resources.actions.addUrl') }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div v-if="paginatedResources.length" class="resource-shell">
        <div v-if="viewMode === 'list'" class="resource-list" data-testid="resources-list">
          <article
            v-for="resource in paginatedResources"
            :key="resource.id"
            class="resource-row"
            :data-testid="`resource-item-${resource.id}`"
          >
            <div class="resource-main">
              <span class="resource-icon">
                <component :is="resourceIcon(resource)" :size="18" />
              </span>
              <div class="resource-copy">
                <strong>{{ resourceLabel(resource) }}</strong>
                <small>{{ resourceSecondaryText(resource) }}</small>
              </div>
            </div>

            <div class="resource-side">
              <div class="resource-meta">
                <UiBadge :label="resourceKindLabel(resource)" subtle />
                <span>{{ resourceMetaLabel(resource) }}</span>
              </div>
              <div class="resource-actions">
                <UiButton
                  variant="ghost"
                  size="sm"
                  class="resource-action-button"
                  :data-testid="`resource-preview-${resource.id}`"
                  @click="openPreview(resource.id)"
                >
                  <Eye :size="14" />
                </UiButton>
                <UiButton
                  variant="ghost"
                  size="sm"
                  class="resource-action-button"
                  :data-testid="`resource-edit-${resource.id}`"
                  @click="openEdit(resource.id)"
                >
                  <Pencil :size="14" />
                </UiButton>
                <UiButton
                  variant="ghost"
                  size="sm"
                  class="resource-action-button resource-action-danger"
                  :data-testid="`resource-delete-${resource.id}`"
                  @click="openDelete(resource.id)"
                >
                  <Trash2 :size="14" />
                </UiButton>
              </div>
            </div>
          </article>
        </div>

        <div v-else class="resource-grid" data-testid="resources-grid">
          <article
            v-for="resource in paginatedResources"
            :key="resource.id"
            class="resource-card"
            :data-testid="`resource-item-${resource.id}`"
          >
            <div class="resource-card-topline">
              <span class="resource-icon resource-icon-large">
                <component :is="resourceIcon(resource)" :size="20" />
              </span>
              <UiBadge :label="resourceKindLabel(resource)" subtle />
            </div>
            <div class="resource-copy">
              <strong>{{ resourceLabel(resource) }}</strong>
              <small>{{ resourceSecondaryText(resource) }}</small>
            </div>
            <div class="resource-card-footer">
              <span>{{ resourceMetaLabel(resource) }}</span>
              <div class="resource-actions">
                <UiButton
                  variant="ghost"
                  size="sm"
                  class="resource-action-button"
                  :data-testid="`resource-preview-${resource.id}`"
                  @click="openPreview(resource.id)"
                >
                  <Eye :size="14" />
                </UiButton>
                <UiButton
                  variant="ghost"
                  size="sm"
                  class="resource-action-button"
                  :data-testid="`resource-edit-${resource.id}`"
                  @click="openEdit(resource.id)"
                >
                  <Pencil :size="14" />
                </UiButton>
                <UiButton
                  variant="ghost"
                  size="sm"
                  class="resource-action-button resource-action-danger"
                  :data-testid="`resource-delete-${resource.id}`"
                  @click="openDelete(resource.id)"
                >
                  <Trash2 :size="14" />
                </UiButton>
              </div>
            </div>
          </article>
        </div>

        <UiPagination
          :page="currentPage"
          :page-count="totalPages"
          :previous-label="t('resources.pagination.previous')"
          :next-label="t('resources.pagination.next')"
          :summary-label="paginationSummaryLabel"
          previous-button-test-id="resources-pagination-prev"
          next-button-test-id="resources-pagination-next"
          @update:page="setPage"
        >
          <template #previousIcon>
            <ChevronLeft :size="16" />
          </template>
          <template #nextIcon>
            <ChevronRight :size="16" />
          </template>
        </UiPagination>
      </div>

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
    </UiSurface>

    <div v-if="creatingUrl" class="resource-modal-shell">
      <button type="button" class="resource-modal-backdrop" @click="closeCreateUrlModal" />
      <section class="resource-modal-card">
        <div class="resource-modal-copy">
          <strong>{{ t('resources.editForm.urlTitle') }}</strong>
        </div>
        <label class="resource-field-stack">
          <span>{{ t('resources.editForm.nameLabel') }}</span>
          <UiInput
            v-model="createUrlNameDraft"
            data-testid="resource-url-name-input"
            :placeholder="t('resources.editForm.namePlaceholder')"
          />
        </label>
        <label class="resource-field-stack">
          <span>{{ t('resources.editForm.locationLabel') }}</span>
          <UiInput
            v-model="createUrlLocationDraft"
            data-testid="resource-url-location-input"
            :placeholder="t('resources.editForm.locationPlaceholder')"
          />
        </label>
        <div class="resource-modal-actions">
          <UiButton variant="ghost" @click="closeCreateUrlModal">{{ t('resources.actions.cancel') }}</UiButton>
          <UiButton data-testid="resource-url-confirm" @click="submitUrlResource">{{ t('resources.editForm.confirmCreate') }}</UiButton>
        </div>
      </section>
    </div>

    <div v-if="previewResource" class="resource-modal-shell">
      <button type="button" class="resource-modal-backdrop" @click="closePreview" />
      <section class="resource-modal-card resource-preview-card" data-testid="resource-preview-modal">
        <div class="resource-preview-heading">
          <div class="resource-modal-copy">
            <strong>{{ t('resources.preview.title') }}</strong>
            <p>{{ resourceLabel(previewResource) }}</p>
          </div>
          <UiButton variant="ghost" size="sm" data-testid="resource-preview-close" @click="closePreview">
            <span>{{ t('resources.actions.cancel') }}</span>
          </UiButton>
        </div>
        <div class="resource-preview-meta">
          <UiBadge :label="resourceKindLabel(previewResource)" subtle />
          <span>{{ t('resources.preview.size') }}: {{ resourceMetaLabel(previewResource) }}</span>
          <span>{{ t('resources.preview.linkedConversations') }}: {{ previewResource.linkedConversationIds.length }}</span>
        </div>
        <div class="resource-preview-content">
          <p><strong>{{ t('resources.preview.location') }}:</strong> {{ resourceSecondaryText(previewResource) }}</p>
          <p><strong>{{ t('resources.preview.tags') }}:</strong> {{ previewResource.tags.join(', ') || t('common.na') }}</p>
          <p v-if="previewArtifact"><strong>{{ t('resources.preview.kind') }}:</strong> {{ previewArtifact.type }}</p>
          <pre v-if="previewArtifact" class="resource-artifact-preview">{{ previewArtifact.content }}</pre>
          <a
            v-if="previewResource.kind === 'url' && previewResource.location"
            class="resource-preview-link"
            :href="previewResource.location"
            target="_blank"
            rel="noreferrer"
          >
            <ExternalLink :size="16" />
            <span>{{ t('resources.actions.openLink') }}</span>
          </a>
        </div>
      </section>
    </div>

    <div v-if="editingResource" class="resource-modal-shell">
      <button type="button" class="resource-modal-backdrop" @click="closeEdit" />
      <section class="resource-modal-card" data-testid="resource-edit-modal">
        <div class="resource-modal-copy">
          <strong>{{ t('resources.editForm.renameTitle') }}</strong>
          <p>{{ resourceKindLabel(editingResource) }}</p>
        </div>
        <label class="resource-field-stack">
          <span>{{ t('resources.editForm.nameLabel') }}</span>
          <UiInput
            v-model="editNameDraft"
            data-testid="resource-edit-name-input"
            :placeholder="t('resources.editForm.namePlaceholder')"
          />
        </label>
        <label v-if="editingResource.kind === 'url'" class="resource-field-stack">
          <span>{{ t('resources.editForm.locationLabel') }}</span>
          <UiInput
            v-model="editLocationDraft"
            data-testid="resource-edit-location-input"
            :placeholder="t('resources.editForm.locationPlaceholder')"
          />
        </label>
        <div class="resource-modal-actions">
          <UiButton variant="ghost" @click="closeEdit">{{ t('resources.actions.cancel') }}</UiButton>
          <UiButton data-testid="resource-edit-confirm" @click="saveEdit">{{ t('resources.editForm.confirmSave') }}</UiButton>
        </div>
      </section>
    </div>

    <div v-if="deletingResource" class="resource-modal-shell">
      <button type="button" class="resource-modal-backdrop" @click="closeDelete" />
      <section class="resource-modal-card" data-testid="resource-delete-modal">
        <div class="resource-modal-copy">
          <strong>{{ t('resources.deleteDialog.title') }}</strong>
          <p>{{ deletingResource ? resourceLabel(deletingResource) : '' }}</p>
        </div>
        <p class="resource-delete-copy">{{ t('resources.deleteDialog.description') }}</p>
        <div class="resource-modal-actions">
          <UiButton variant="ghost" @click="closeDelete">{{ t('resources.actions.cancel') }}</UiButton>
          <UiButton variant="destructive" data-testid="resource-delete-confirm" @click="confirmDelete">{{ t('resources.deleteDialog.confirm') }}</UiButton>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.resources-page,
.resource-copy,
.resource-modal-copy,
.resource-preview-content,
.resource-field-stack {
  display: flex;
  flex-direction: column;
}

.resource-stats-grid,
.resource-toolbar,
.resource-toolbar-actions,
.resource-view-toggle,
.resource-main,
.resource-side,
.resource-meta,
.resource-actions,
.resource-card-topline,
.resource-card-footer,
.resource-modal-actions,
.resource-preview-heading,
.resource-preview-meta {
  display: flex;
  align-items: center;
}

.resource-toolbar,
.resource-row,
.resource-preview-heading {
  justify-content: space-between;
}

.resource-stats-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 1rem;
}

.resource-stat-card {
  gap: 0.35rem;
}

.resource-stat-card small,
.resource-copy small,
.resource-modal-copy p,
.resource-delete-copy,
.resource-preview-meta,
.resource-meta,
.resource-card-footer {
  color: var(--text-secondary);
}

.resource-stat-card strong {
  font-size: 1.3rem;
  line-height: 1;
}

.resource-toolbar {
  gap: 1rem;
  flex-wrap: wrap;
}

.resource-search-shell {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  flex: 1;
  min-width: min(100%, 18rem);
  padding: 0.15rem 0.2rem 0.15rem 0.9rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-lg) + 2px);
  background: color-mix(in srgb, var(--bg-subtle) 70%, transparent);
}

.resource-search-icon {
  color: var(--text-tertiary);
}

.resource-search-input {
  border: 0;
  background: transparent;
  box-shadow: none;
}

.resource-search-input:focus-visible {
  box-shadow: none;
}

.resource-toolbar-actions,
.resource-view-toggle,
.resource-actions,
.resource-modal-actions,
.resource-preview-meta {
  gap: 0.6rem;
}

.resource-view-toggle .active {
  border-color: color-mix(in srgb, var(--brand-primary) 28%, var(--border-strong));
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
  color: var(--text-primary);
}

.resource-add-shell {
  position: relative;
}

.resource-add-menu {
  position: absolute;
  top: calc(100% + 0.45rem);
  right: 0;
  z-index: 2;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  min-width: 12rem;
  padding: 0.45rem;
  border-radius: calc(var(--radius-lg) + 2px);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: var(--bg-surface);
  box-shadow: var(--shadow-md);
}

.resource-menu-item {
  display: flex;
  align-items: center;
  min-height: 2.5rem;
  padding: 0 0.85rem;
  border-radius: var(--radius-m);
  transition: background-color var(--duration-fast) var(--ease-apple);
}

.resource-menu-item:hover {
  background: color-mix(in srgb, var(--bg-subtle) 88%, transparent);
}

.resource-shell {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  margin-top: 1rem;
}

.resource-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.resource-row,
.resource-card {
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-lg) + 2px);
  transition: border-color var(--duration-fast) var(--ease-apple), transform var(--duration-fast) var(--ease-apple), box-shadow var(--duration-fast) var(--ease-apple);
}

.resource-row:hover,
.resource-card:hover {
  border-color: color-mix(in srgb, var(--brand-primary) 24%, var(--border-strong));
  transform: translateY(-1px);
  box-shadow: var(--shadow-sm);
}

.resource-row {
  display: flex;
  gap: 1rem;
  padding: 1rem;
  background: color-mix(in srgb, var(--bg-subtle) 70%, transparent);
}

.resource-main {
  gap: 0.85rem;
  min-width: 0;
}

.resource-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.35rem;
  height: 2.35rem;
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
  color: var(--brand-primary);
  flex-shrink: 0;
}

.resource-icon-large {
  width: 2.85rem;
  height: 2.85rem;
}

.resource-copy,
.resource-modal-copy,
.resource-preview-content {
  gap: 0.3rem;
  min-width: 0;
}

.resource-copy strong,
.resource-modal-copy strong {
  overflow-wrap: anywhere;
}

.resource-side {
  gap: 1rem;
  flex-shrink: 0;
}

.resource-meta {
  flex-direction: column;
  align-items: flex-end;
  gap: 0.35rem;
  font-size: 0.82rem;
}

.resource-action-button {
  min-width: 2.25rem;
  padding: 0;
}

.resource-action-danger {
  color: var(--status-error);
}

.resource-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 0.9rem;
}

.resource-card {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  min-width: 0;
  padding: 1rem;
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 10%, transparent), transparent 42%),
    var(--bg-surface);
}

.resource-card-topline,
.resource-card-footer {
  justify-content: space-between;
}

.resource-card-footer {
  margin-top: auto;
  font-size: 0.82rem;
}

.resource-modal-shell {
  position: fixed;
  inset: 0;
  z-index: 30;
}

.resource-modal-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(10, 15, 30, 0.42);
}

.resource-modal-card {
  position: relative;
  z-index: 1;
  width: min(32rem, calc(100vw - 2rem));
  margin: 10vh auto 0;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1.15rem;
  border-radius: calc(var(--radius-xl) + 2px);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: var(--bg-surface);
  box-shadow: var(--shadow-lg);
}

.resource-preview-card {
  width: min(40rem, calc(100vw - 2rem));
}

.resource-field-stack {
  gap: 0.45rem;
}

.resource-field-stack span {
  font-size: 0.82rem;
  font-weight: 700;
}

.resource-preview-meta {
  flex-wrap: wrap;
  font-size: 0.82rem;
}

.resource-preview-content {
  line-height: 1.6;
}

.resource-artifact-preview {
  margin: 0;
  padding: 0.95rem;
  border-radius: calc(var(--radius-lg) + 1px);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 80%, transparent);
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 0.82rem;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.resource-preview-link {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  align-self: flex-start;
  padding: 0.7rem 0.9rem;
  border-radius: var(--radius-m);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 72%, transparent);
}

@media (max-width: 1040px) {
  .resource-stats-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 880px) {
  .resource-toolbar-actions,
  .resource-side {
    width: 100%;
  }

  .resource-toolbar-actions {
    justify-content: space-between;
  }

  .resource-row {
    flex-direction: column;
  }

  .resource-side {
    justify-content: space-between;
  }
}

@media (max-width: 640px) {
  .resource-stats-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
