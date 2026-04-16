<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiInput,
  UiListDetailWorkspace,
  UiListRow,
  UiPageHeader,
  UiPageShell,
  UiStatusCallout,
  UiToolbarRow,
} from '@octopus/ui'

import ArtifactPreviewPanel from '@/components/conversation/ArtifactPreviewPanel.vue'
import ArtifactVersionList from '@/components/conversation/ArtifactVersionList.vue'
import { isProjectOwner, resolveProjectActorUserId } from '@/composables/project-governance'
import { enumLabel, formatDateTime } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useArtifactStore } from '@/stores/artifact'
import { useKnowledgeStore } from '@/stores/knowledge'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()
const artifactStore = useArtifactStore()
const knowledgeStore = useKnowledgeStore()

const searchQuery = ref('')
const promotingSelected = ref(false)
const forkingSelected = ref(false)
const actionStatus = ref('')

const projectId = computed(() => typeof route.params.projectId === 'string' ? route.params.projectId : '')
const projectRecord = computed(() => workspaceStore.projects.find(project => project.id === projectId.value) ?? null)
const deliverables = computed(() => artifactStore.activeProjectDeliverables)
const selectedDeliverableId = computed(() => typeof route.query.deliverable === 'string' ? route.query.deliverable : '')
const selectedVersionFromQuery = computed<number | null>(() => {
  if (typeof route.query.version !== 'string') {
    return null
  }
  const version = Number(route.query.version)
  return Number.isInteger(version) && version > 0 ? version : null
})
const selectedDeliverable = computed(() =>
  deliverables.value.find(deliverable => deliverable.id === selectedDeliverableId.value) ?? null,
)
const selectedDeliverableDetail = computed(() =>
  artifactStore.selectedDeliverableDetail?.id === selectedDeliverable.value?.id
    ? artifactStore.selectedDeliverableDetail
    : null,
)
const selectedDeliverableVersions = computed(() =>
  selectedDeliverableDetail.value?.id === selectedDeliverable.value?.id
    ? artifactStore.selectedDeliverableVersions
    : [],
)
const selectedDeliverableContent = computed(() =>
  selectedDeliverableDetail.value?.id === selectedDeliverable.value?.id
    ? artifactStore.selectedDeliverableContent
    : null,
)
const resolvedVersion = computed(() => selectedVersionFromQuery.value ?? artifactStore.resolvedSelectedVersion ?? null)
const filteredDeliverables = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  if (!query) {
    return deliverables.value
  }

  return deliverables.value.filter((deliverable) =>
    [
      deliverable.title,
      deliverable.id,
      deliverable.contentType ?? '',
      deliverable.status,
      deliverable.promotionState,
    ].join(' ').toLowerCase().includes(query),
  )
})
const currentProjectActorUserId = computed(() =>
  resolveProjectActorUserId(
    workspaceAccessControlStore.currentUser?.id,
    workspaceAccessControlStore.loading ? undefined : shell.activeWorkspaceSession?.session.userId,
  ),
)
const canPromoteSelected = computed(() =>
  Boolean(selectedDeliverable.value)
  && Boolean(projectRecord.value)
  && Boolean(currentProjectActorUserId.value)
  && isProjectOwner(projectRecord.value, currentProjectActorUserId.value)
  && selectedDeliverableDetail.value?.promotionState !== 'promoted',
)
const conversationTarget = computed(() => {
  const deliverable = selectedDeliverableDetail.value ?? selectedDeliverable.value
  if (!deliverable?.conversationId) {
    return null
  }
  return createProjectConversationTarget(deliverable.workspaceId, deliverable.projectId, deliverable.conversationId)
})

watch(
  () => [shell.activeWorkspaceConnectionId, projectId.value],
  async ([connectionId, nextProjectId]) => {
    if (!connectionId || !nextProjectId) {
      return
    }
    await artifactStore.loadProjectDeliverables(nextProjectId)
  },
  { immediate: true },
)

watch(
  () => [deliverables.value, selectedDeliverableId.value] as const,
  async ([records, queryDeliverableId]) => {
    if (!records.length) {
      return
    }
    if (queryDeliverableId && records.some(record => record.id === queryDeliverableId)) {
      return
    }
    await replaceDeliverableQuery(records[0]?.id ?? '')
  },
  { immediate: true },
)

watch(
  () => [selectedDeliverable.value?.id ?? '', selectedVersionFromQuery.value] as const,
  ([deliverableId, version]) => {
    if (!deliverableId) {
      return
    }
    shell.selectDeliverable(deliverableId, version ?? undefined)
  },
  { immediate: true },
)

watch(
  () => [shell.activeWorkspaceConnectionId, selectedDeliverable.value?.id ?? '', selectedVersionFromQuery.value] as const,
  async ([connectionId, deliverableId, version]) => {
    if (!connectionId || !deliverableId) {
      return
    }
    await artifactStore.ensureDeliverableState(deliverableId, version ?? undefined)
  },
  { immediate: true },
)

function isSelectedDeliverable(deliverableId: string) {
  return selectedDeliverable.value?.id === deliverableId
}

async function replaceDeliverableQuery(deliverableId: string, version?: number | null) {
  await router.replace({
    query: {
      ...route.query,
      deliverable: deliverableId || undefined,
      version: version ? String(version) : undefined,
    },
  })
}

async function selectDeliverableRow(deliverableId: string) {
  if (!deliverableId || deliverableId === selectedDeliverable.value?.id) {
    return
  }
  actionStatus.value = ''
  await replaceDeliverableQuery(deliverableId)
}

async function selectDeliverableVersion(version: number) {
  if (!selectedDeliverable.value) {
    return
  }
  actionStatus.value = ''
  await replaceDeliverableQuery(selectedDeliverable.value.id, version)
}

async function promoteSelectedDeliverable() {
  if (!selectedDeliverable.value || !projectId.value || promotingSelected.value || !canPromoteSelected.value) {
    return
  }

  promotingSelected.value = true
  actionStatus.value = ''
  try {
    const record = await artifactStore.promoteDeliverable(selectedDeliverable.value.id)
    if (!record) {
      return
    }
    await knowledgeStore.loadProjectKnowledge(projectId.value)
    actionStatus.value = t('deliverables.status.promoted')
  } finally {
    promotingSelected.value = false
  }
}

async function forkSelectedDeliverable() {
  if (!selectedDeliverable.value || !projectId.value || forkingSelected.value) {
    return
  }

  forkingSelected.value = true
  actionStatus.value = ''
  try {
    const conversation = await artifactStore.forkDeliverable(
      projectId.value,
      selectedDeliverable.value.title,
      selectedDeliverable.value.id,
    )
    if (!conversation) {
      return
    }
    await router.push(
      createProjectConversationTarget(
        conversation.workspaceId,
        conversation.projectId,
        conversation.id,
      ),
    )
  } finally {
    forkingSelected.value = false
  }
}
</script>

<template>
  <UiPageShell width="wide" test-id="project-deliverables-view">
    <UiPageHeader
      :eyebrow="t('deliverables.header.eyebrow')"
      :title="projectRecord?.name ?? t('deliverables.header.titleFallback')"
      :description="projectRecord?.description || t('deliverables.header.subtitle')"
    >
      <template #actions>
        <UiInput
          v-model="searchQuery"
          :placeholder="t('deliverables.filters.searchPlaceholder')"
          class="w-full md:w-[320px]"
        />
      </template>
    </UiPageHeader>

    <UiStatusCallout
      v-if="artifactStore.error"
      tone="error"
      :description="artifactStore.error"
    />

    <UiStatusCallout
      v-else-if="actionStatus"
      tone="success"
      :description="actionStatus"
    />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedDeliverable)"
      :detail-title="selectedDeliverable?.title"
      :detail-subtitle="selectedDeliverable?.id ?? t('common.na')"
      :empty-detail-title="t('deliverables.detail.emptyTitle')"
      :empty-detail-description="t('deliverables.detail.emptyDescription')"
      detail-class="xl:min-w-[440px]"
    >
      <template #toolbar>
        <UiToolbarRow test-id="project-deliverables-toolbar">
          <template #search>
            <UiInput
              v-model="searchQuery"
              :placeholder="t('deliverables.filters.searchPlaceholder')"
            />
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <div class="space-y-3">
          <div v-if="filteredDeliverables.length" class="space-y-2">
            <div
              v-for="deliverable in filteredDeliverables"
              :key="deliverable.id"
              class="rounded-[var(--radius-l)]"
              @click="selectDeliverableRow(deliverable.id)"
            >
              <UiListRow
                :title="deliverable.title"
                :subtitle="deliverable.id"
                :eyebrow="t('deliverables.list.eyebrow', { version: deliverable.latestVersion })"
                interactive
                :active="isSelectedDeliverable(deliverable.id)"
              >
                <template #meta>
                  <UiBadge :label="enumLabel('deliverablePromotionState', deliverable.promotionState)" subtle />
                  <UiBadge :label="enumLabel('artifactStatus', deliverable.status)" subtle />
                  <span class="text-xs text-text-tertiary">{{ formatDateTime(deliverable.updatedAt) }}</span>
                </template>
              </UiListRow>
            </div>
          </div>

          <UiEmptyState
            v-else
            :title="t('deliverables.empty.title')"
            :description="t('deliverables.empty.description')"
          />
        </div>
      </template>

      <template #detail>
        <section
          v-if="selectedDeliverable"
          data-testid="project-deliverable-detail"
          class="space-y-4"
        >
          <div class="flex flex-wrap items-center gap-2">
            <UiBadge
              :label="enumLabel('resourcePreviewKind', selectedDeliverableContent?.previewKind ?? selectedDeliverableDetail?.previewKind ?? selectedDeliverable.previewKind)"
              subtle
            />
            <UiBadge :label="enumLabel('artifactStatus', selectedDeliverable.status)" subtle />
            <UiBadge :label="enumLabel('deliverablePromotionState', selectedDeliverableDetail?.promotionState ?? selectedDeliverable.promotionState)" subtle />
            <UiBadge :label="`v${resolvedVersion ?? selectedDeliverable.latestVersion}`" subtle />
          </div>

          <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_280px]">
            <div class="grid gap-3 rounded-[var(--radius-l)] border border-border bg-subtle px-3 py-3 text-sm text-text-secondary">
              <div>{{ t('deliverables.detail.updatedAt') }}: {{ formatDateTime(selectedDeliverableDetail?.updatedAt ?? selectedDeliverable.updatedAt) }}</div>
              <div>{{ t('deliverables.detail.contentType') }}: {{ selectedDeliverableContent?.contentType ?? selectedDeliverableDetail?.contentType ?? selectedDeliverable.contentType ?? t('common.na') }}</div>
              <div>{{ t('deliverables.detail.sourceConversation') }}: {{ selectedDeliverableDetail?.conversationId ?? selectedDeliverable.conversationId ?? t('common.na') }}</div>
            </div>

            <div class="space-y-3 rounded-[var(--radius-l)] border border-border bg-surface px-3 py-3">
              <div class="text-[12px] font-semibold text-text-secondary">
                {{ t('deliverables.detail.actionsTitle') }}
              </div>

              <div class="flex flex-wrap gap-2">
                <UiButton
                  v-if="canPromoteSelected"
                  size="sm"
                  variant="outline"
                  :disabled="promotingSelected"
                  data-testid="project-deliverable-promote"
                  @click="promoteSelectedDeliverable"
                >
                  {{ t('deliverables.actions.promote') }}
                </UiButton>

                <UiButton
                  size="sm"
                  variant="outline"
                  :disabled="forkingSelected"
                  data-testid="project-deliverable-fork"
                  @click="forkSelectedDeliverable"
                >
                  {{ t('deliverables.actions.fork') }}
                </UiButton>

                <RouterLink
                  v-if="conversationTarget"
                  class="inline-flex items-center text-sm font-medium text-primary hover:underline"
                  :to="conversationTarget"
                >
                  {{ t('deliverables.detail.openConversation') }}
                </RouterLink>
              </div>
            </div>
          </div>

          <ArtifactVersionList
            :versions="selectedDeliverableVersions"
            :selected-version="resolvedVersion"
            :loading="artifactStore.loading && !selectedDeliverableVersions.length"
            @select="selectDeliverableVersion"
          />

          <UiStatusCallout
            v-if="artifactStore.loading && !selectedDeliverableContent"
            :description="t('conversation.detail.deliverables.loadingPreview')"
          />

          <ArtifactPreviewPanel
            :key="`${selectedDeliverable.id}:${resolvedVersion ?? 'none'}`"
            :content="selectedDeliverableContent"
            :error="artifactStore.error"
          />
        </section>
      </template>
    </UiListDetailWorkspace>
  </UiPageShell>
</template>
