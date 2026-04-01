<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
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
  ChevronLeft,
  ChevronRight,
} from 'lucide-vue-next'

import type { Artifact, ProjectResource } from '@octopus/schema'
import { UiBadge, UiEmptyState, UiPagination, UiSectionHeading, UiSurface } from '@octopus/ui'

import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { usePagination } from '@/composables/usePagination'
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
  if (resource.kind === 'artifact') {
    return resource.location || resourceKindLabel(resource)
  }

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
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('resources.header.eyebrow')"
      :title="t('resources.header.title')"
    />

    <UiSurface>
      <div class="toolbar">
        <label class="search-shell">
          <Search :size="16" />
          <input
            v-model="searchQuery"
            data-testid="resources-search-input"
            :placeholder="t('resources.search.placeholder')"
          >
        </label>

        <div class="toolbar-actions">
          <div class="view-toggle">
            <button
              type="button"
              class="ghost-button"
              :class="{ active: viewMode === 'list' }"
              data-testid="resources-view-list"
              @click="viewMode = 'list'"
            >
              <List :size="16" />
              <span>{{ t('resources.views.list') }}</span>
            </button>
            <button
              type="button"
              class="ghost-button"
              :class="{ active: viewMode === 'grid' }"
              data-testid="resources-view-grid"
              @click="viewMode = 'grid'"
            >
              <LayoutGrid :size="16" />
              <span>{{ t('resources.views.grid') }}</span>
            </button>
          </div>

          <div class="add-menu-shell">
            <button
              type="button"
              class="primary-button"
              data-testid="resources-add-trigger"
              @click="toggleAddMenu"
            >
              <Plus :size="16" />
              <span>{{ t('resources.actions.add') }}</span>
            </button>

            <div v-if="addMenuOpen" class="menu-card" data-testid="resources-add-menu">
              <button
                type="button"
                class="menu-item"
                data-testid="resources-add-file"
                @click="createFile"
              >
                {{ t('resources.actions.uploadFile') }}
              </button>
              <button
                type="button"
                class="menu-item"
                data-testid="resources-add-folder"
                @click="createFolder"
              >
                {{ t('resources.actions.uploadFolder') }}
              </button>
              <button
                type="button"
                class="menu-item"
                data-testid="resources-add-url"
                @click="openCreateUrlModal"
              >
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
                <button
                  type="button"
                  class="ghost-button action-button"
                  :data-testid="`resource-preview-${resource.id}`"
                  @click="openPreview(resource.id)"
                >
                  <Eye :size="14" />
                </button>
                <button
                  type="button"
                  class="ghost-button action-button"
                  :data-testid="`resource-edit-${resource.id}`"
                  @click="openEdit(resource.id)"
                >
                  <Pencil :size="14" />
                </button>
                <button
                  type="button"
                  class="ghost-button action-button danger"
                  :data-testid="`resource-delete-${resource.id}`"
                  @click="openDelete(resource.id)"
                >
                  <Trash2 :size="14" />
                </button>
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
            <div class="card-topline">
              <span class="resource-icon large">
                <component :is="resourceIcon(resource)" :size="20" />
              </span>
              <UiBadge :label="resourceKindLabel(resource)" subtle />
            </div>
            <div class="card-copy">
              <strong>{{ resourceLabel(resource) }}</strong>
              <small>{{ resourceSecondaryText(resource) }}</small>
            </div>
            <div class="card-footer">
              <span>{{ resourceMetaLabel(resource) }}</span>
              <div class="resource-actions">
                <button
                  type="button"
                  class="ghost-button action-button"
                  :data-testid="`resource-preview-${resource.id}`"
                  @click="openPreview(resource.id)"
                >
                  <Eye :size="14" />
                </button>
                <button
                  type="button"
                  class="ghost-button action-button"
                  :data-testid="`resource-edit-${resource.id}`"
                  @click="openEdit(resource.id)"
                >
                  <Pencil :size="14" />
                </button>
                <button
                  type="button"
                  class="ghost-button action-button danger"
                  :data-testid="`resource-delete-${resource.id}`"
                  @click="openDelete(resource.id)"
                >
                  <Trash2 :size="14" />
                </button>
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

    <div v-if="creatingUrl" class="modal-shell">
      <button type="button" class="modal-backdrop" @click="closeCreateUrlModal" />
      <section class="modal-card">
        <div class="modal-copy">
          <strong>{{ t('resources.editForm.urlTitle') }}</strong>
        </div>
        <label class="field-stack">
          <span>{{ t('resources.editForm.nameLabel') }}</span>
          <input
            v-model="createUrlNameDraft"
            data-testid="resource-url-name-input"
            :placeholder="t('resources.editForm.namePlaceholder')"
          >
        </label>
        <label class="field-stack">
          <span>{{ t('resources.editForm.locationLabel') }}</span>
          <input
            v-model="createUrlLocationDraft"
            data-testid="resource-url-location-input"
            :placeholder="t('resources.editForm.locationPlaceholder')"
          >
        </label>
        <div class="modal-actions">
          <button type="button" class="ghost-button" @click="closeCreateUrlModal">{{ t('resources.actions.cancel') }}</button>
          <button
            type="button"
            class="primary-button"
            data-testid="resource-url-confirm"
            @click="submitUrlResource"
          >
            {{ t('resources.editForm.confirmCreate') }}
          </button>
        </div>
      </section>
    </div>

    <div v-if="previewResource" class="modal-shell">
      <button type="button" class="modal-backdrop" @click="closePreview" />
      <section class="modal-card preview-card" data-testid="resource-preview-modal">
        <div class="split-heading">
          <div class="modal-copy">
            <strong>{{ t('resources.preview.title') }}</strong>
            <p>{{ resourceLabel(previewResource) }}</p>
          </div>
          <button type="button" class="ghost-button action-button" data-testid="resource-preview-close" @click="closePreview">
            <span>{{ t('resources.actions.cancel') }}</span>
          </button>
        </div>
        <div class="meta-row">
          <UiBadge :label="resourceKindLabel(previewResource)" subtle />
          <span>{{ t('resources.preview.size') }}: {{ resourceMetaLabel(previewResource) }}</span>
          <span>{{ t('resources.preview.linkedConversations') }}: {{ previewResource.linkedConversationIds.length }}</span>
        </div>
        <div class="preview-content">
          <p><strong>{{ t('resources.preview.location') }}:</strong> {{ resourceSecondaryText(previewResource) }}</p>
          <p><strong>{{ t('resources.preview.tags') }}:</strong> {{ previewResource.tags.join(', ') || t('common.na') }}</p>
          <p v-if="previewArtifact"><strong>{{ t('resources.preview.kind') }}:</strong> {{ previewArtifact.type }}</p>
          <pre v-if="previewArtifact" class="artifact-preview">{{ previewArtifact.content }}</pre>
          <a
            v-if="previewResource.kind === 'url' && previewResource.location"
            class="secondary-button preview-link"
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

    <div v-if="editingResource" class="modal-shell">
      <button type="button" class="modal-backdrop" @click="closeEdit" />
      <section class="modal-card" data-testid="resource-edit-modal">
        <div class="modal-copy">
          <strong>{{ t('resources.editForm.renameTitle') }}</strong>
          <p>{{ resourceKindLabel(editingResource) }}</p>
        </div>
        <label class="field-stack">
          <span>{{ t('resources.editForm.nameLabel') }}</span>
          <input
            v-model="editNameDraft"
            data-testid="resource-edit-name-input"
            :placeholder="t('resources.editForm.namePlaceholder')"
          >
        </label>
        <label v-if="editingResource.kind === 'url'" class="field-stack">
          <span>{{ t('resources.editForm.locationLabel') }}</span>
          <input
            v-model="editLocationDraft"
            data-testid="resource-edit-location-input"
            :placeholder="t('resources.editForm.locationPlaceholder')"
          >
        </label>
        <div class="modal-actions">
          <button type="button" class="ghost-button" @click="closeEdit">{{ t('resources.actions.cancel') }}</button>
          <button
            type="button"
            class="primary-button"
            data-testid="resource-edit-confirm"
            @click="saveEdit"
          >
            {{ t('resources.editForm.confirmSave') }}
          </button>
        </div>
      </section>
    </div>

    <div v-if="deletingResource" class="modal-shell">
      <button type="button" class="modal-backdrop" @click="closeDelete" />
      <section class="modal-card" data-testid="resource-delete-modal">
        <div class="modal-copy">
          <strong>{{ t('resources.deleteDialog.title') }}</strong>
          <p>{{ deletingResource ? resourceLabel(deletingResource) : '' }}</p>
        </div>
        <p class="delete-copy">{{ t('resources.deleteDialog.description') }}</p>
        <div class="modal-actions">
          <button type="button" class="ghost-button" @click="closeDelete">{{ t('resources.actions.cancel') }}</button>
          <button type="button" class="danger-button" data-testid="resource-delete-confirm" @click="confirmDelete">
            {{ t('resources.deleteDialog.confirm') }}
          </button>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.toolbar,
.toolbar-actions,
.view-toggle,
.search-shell,
.resource-main,
.resource-side,
.resource-meta,
.resource-actions,
.card-topline,
.card-footer,
.pagination-row,
.modal-actions {
  display: flex;
  align-items: center;
}

.toolbar,
.resource-row,
.pagination-row,
.split-heading {
  justify-content: space-between;
}

.toolbar {
  gap: 1rem;
  flex-wrap: wrap;
}

.search-shell {
  flex: 1;
  gap: 0.65rem;
  min-width: min(100%, 18rem);
  padding: 0 0.95rem;
  min-height: 2.9rem;
  border-radius: var(--radius-m);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.search-shell input,
.field-stack input {
  width: 100%;
  min-width: 0;
  border: none;
  background: transparent;
  color: inherit;
  outline: none;
}

.toolbar-actions {
  gap: 0.75rem;
  margin-left: auto;
}

.view-toggle {
  gap: 0.5rem;
}

.view-toggle .ghost-button.active {
  border-color: color-mix(in srgb, var(--brand-primary) 30%, var(--border-subtle));
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
}

.add-menu-shell {
  position: relative;
}

.menu-card {
  position: absolute;
  top: calc(100% + 0.45rem);
  right: 0;
  z-index: 2;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  min-width: 12rem;
  padding: 0.45rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
  box-shadow: var(--shadow-md);
}

.menu-item {
  display: flex;
  align-items: center;
  min-height: 2.5rem;
  padding: 0 0.85rem;
  border-radius: var(--radius-m);
  transition: background-color var(--duration-fast) var(--ease-apple);
}

.menu-item:hover {
  background: color-mix(in srgb, var(--bg-subtle) 88%, transparent);
}

.resource-shell {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.resource-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.resource-row {
  display: flex;
  gap: 1rem;
  padding: 0.95rem 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 72%, transparent);
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

.resource-icon.large {
  width: 2.85rem;
  height: 2.85rem;
}

.resource-copy,
.card-copy,
.modal-copy,
.preview-content,
.field-stack {
  display: flex;
  flex-direction: column;
}

.resource-copy,
.card-copy,
.modal-copy,
.preview-content {
  gap: 0.3rem;
  min-width: 0;
}

.resource-copy strong,
.card-copy strong,
.modal-copy strong {
  overflow-wrap: anywhere;
}

.resource-copy small,
.card-copy small,
.modal-copy p,
.delete-copy {
  color: var(--text-secondary);
  line-height: 1.5;
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
  color: var(--text-secondary);
  font-size: 0.82rem;
}

.resource-actions {
  gap: 0.35rem;
}

.action-button {
  min-height: 2.35rem;
  min-width: 2.35rem;
  padding: 0;
}

.action-button.danger {
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
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 10%, transparent), transparent 42%),
    var(--bg-surface);
}

.card-topline,
.card-footer {
  justify-content: space-between;
  gap: 0.75rem;
}

.card-footer {
  margin-top: auto;
  color: var(--text-secondary);
  font-size: 0.82rem;
}

.pagination-row {
  gap: 1rem;
  flex-wrap: wrap;
}

.pagination-summary {
  color: var(--text-secondary);
  font-size: 0.82rem;
}

.modal-shell {
  position: fixed;
  inset: 0;
  z-index: 30;
}

.modal-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(10, 15, 30, 0.42);
}

.modal-card {
  position: relative;
  z-index: 1;
  width: min(32rem, calc(100vw - 2rem));
  margin: 10vh auto 0;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1.15rem;
  border-radius: var(--radius-xl);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
  box-shadow: var(--shadow-md);
}

.preview-card {
  width: min(40rem, calc(100vw - 2rem));
}

.field-stack {
  gap: 0.45rem;
}

.field-stack span {
  font-size: 0.82rem;
  font-weight: 700;
}

.field-stack input {
  min-height: 2.75rem;
  padding: 0 0.9rem;
  border-radius: var(--radius-m);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.modal-actions {
  justify-content: flex-end;
  gap: 0.65rem;
  flex-wrap: wrap;
}

.artifact-preview {
  margin: 0;
  padding: 0.95rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 80%, transparent);
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 0.82rem;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.preview-link {
  align-self: flex-start;
}

@media (max-width: 880px) {
  .toolbar-actions,
  .resource-side,
  .pagination-row {
    width: 100%;
  }

  .toolbar-actions,
  .pagination-row {
    justify-content: space-between;
  }

  .resource-row {
    flex-direction: column;
  }

  .resource-side {
    justify-content: space-between;
  }
}
</style>
