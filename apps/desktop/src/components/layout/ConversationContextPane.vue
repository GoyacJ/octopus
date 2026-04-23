<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { FileText, FolderTree, Link2, PanelRight, ShieldAlert, Wrench } from 'lucide-vue-next'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiInput,
  UiInspectorPanel,
  UiListRow,
  UiSelect,
  UiSkeleton,
  UiStatTile,
  UiStatusCallout,
  UiTimelineList,
} from '@octopus/ui'

import type { ConversationWorkbenchMode } from '@octopus/schema'

import ArtifactPreviewPanel from '@/components/conversation/ArtifactPreviewPanel.vue'
import ArtifactVersionList from '@/components/conversation/ArtifactVersionList.vue'
import { isProjectOwner, resolveProjectActorUserId } from '@/composables/project-governance'
import { enumLabel, formatDateTime } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useArtifactStore } from '@/stores/artifact'
import { useKnowledgeStore } from '@/stores/knowledge'
import { useResourceStore } from '@/stores/resource'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const runtime = useRuntimeStore()
const resourceStore = useResourceStore()
const artifactStore = useArtifactStore()
const knowledgeStore = useKnowledgeStore()
const workspaceStore = useWorkspaceStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()

const resourceFilter = ref('')
const isEditingDeliverable = ref(false)
const saveStatus = ref('')
const deliverableActionStatus = ref('')
const promotingDeliverable = ref(false)
const forkingDeliverable = ref(false)

const sectionItems = computed(() => [
  { id: 'deliverable', label: t('conversation.detail.sections.deliverable'), icon: FileText },
  { id: 'context', label: t('conversation.detail.sections.context'), icon: FolderTree },
  { id: 'ops', label: t('conversation.detail.sections.ops'), icon: Wrench },
] as const)

function resolveMessageDeliverableId(artifact: string | { artifactId: string }) {
  return typeof artifact === 'string' ? artifact : artifact.artifactId
}

const timelineItems = computed(() =>
  runtime.activeTrace.map(trace => ({
    id: trace.id,
    title: trace.title,
    description: trace.detail,
    helper: trace.actor,
    timestamp: formatDateTime(trace.timestamp),
  })),
)
const conversationResourceIds = computed(() => [...new Set(runtime.activeMessages.flatMap(message => message.resourceIds ?? []))])
const conversationArtifactIds = computed(() => [
  ...new Set([
    ...(runtime.activeRun?.deliverableRefs ?? []).map(resolveMessageDeliverableId),
    ...runtime.activeMessages.flatMap(message => (message.deliverableRefs ?? []).map(resolveMessageDeliverableId)),
  ]),
])
const conversationResources = computed(() =>
  resourceStore.activeProjectResources.filter(resource => conversationResourceIds.value.includes(resource.id)),
)
const conversationArtifacts = computed(() =>
  artifactStore.activeProjectDeliverables.filter(artifact => conversationArtifactIds.value.includes(artifact.id)),
)
const selectedProjectDeliverable = computed(() =>
  artifactStore.activeProjectDeliverables.find(artifact => artifact.id === shell.selectedDeliverableId) ?? null,
)
const deliverableOptions = computed(() =>
  conversationArtifacts.value.map(artifact => ({
    label: artifact.title,
    value: artifact.id,
  })),
)
const localizedArtifactStatus = computed(() =>
  new Map(
    [
      ...conversationArtifacts.value,
      ...(selectedProjectDeliverable.value ? [selectedProjectDeliverable.value] : []),
    ].map(artifact => [artifact.id, enumLabel('artifactStatus', artifact.status)]),
  ),
)
const localizedResourceKind = computed(() =>
  new Map(conversationResources.value.map(resource => [resource.id, enumLabel('projectResourceKind', resource.kind)])),
)
const localizedResourceOrigin = computed(() =>
  new Map(conversationResources.value.map(resource => [resource.id, enumLabel('projectResourceOrigin', resource.origin)])),
)
const filteredConversationResources = computed(() => {
  const query = resourceFilter.value.trim().toLowerCase()
  return conversationResources.value.filter((resource) => {
    if (!query) {
      return true
    }
    return [resource.name, resource.id, resource.location ?? '', resource.origin, resource.kind, resource.sourceArtifactId ?? '']
      .join(' ')
      .toLowerCase()
      .includes(query)
  })
})
const workspaceId = computed(() => typeof route.params.workspaceId === 'string' ? route.params.workspaceId : '')
const conversationId = computed(() => typeof route.params.conversationId === 'string' ? route.params.conversationId : '')
const selectedConversationDeliverable = computed(() =>
  conversationArtifacts.value.find(artifact => artifact.id === shell.selectedDeliverableId)
  ?? selectedProjectDeliverable.value
  ?? conversationArtifacts.value[0]
  ?? null,
)
const projectId = computed(() => typeof route.params.projectId === 'string' ? route.params.projectId : '')
const hasConversationContext = computed(() => Boolean(workspaceId.value && projectId.value))
const hasActiveConversation = computed(() => Boolean(workspaceId.value && projectId.value && conversationId.value))
const projectRecord = computed(() => workspaceStore.projects.find(project => project.id === projectId.value) ?? null)
const selectedDeliverableDetail = computed(() => {
  const detail = artifactStore.selectedDeliverableDetail
  if (!detail || detail.id !== selectedConversationDeliverable.value?.id) {
    return null
  }
  return detail
})
const selectedDeliverableVersions = computed(() =>
  selectedDeliverableDetail.value?.id === selectedConversationDeliverable.value?.id
    ? artifactStore.selectedDeliverableVersions
    : [],
)
const selectedDeliverableContent = computed(() =>
  selectedDeliverableDetail.value?.id === selectedConversationDeliverable.value?.id
    ? artifactStore.selectedDeliverableContent
    : null,
)
const selectedDeliverableDraft = computed(() =>
  selectedDeliverableDetail.value?.id === selectedConversationDeliverable.value?.id
    ? artifactStore.selectedDeliverableDraft
    : '',
)
const visibleConversationResources = computed(() =>
  selectedConversationDeliverable.value
    ? filteredConversationResources.value.filter(resource => resource.sourceArtifactId === selectedConversationDeliverable.value.id)
    : filteredConversationResources.value,
)
const localizedPromotionState = computed(() =>
  selectedDeliverableDetail.value
    ? enumLabel('deliverablePromotionState', selectedDeliverableDetail.value.promotionState)
    : '',
)
const currentProjectActorUserId = computed(() =>
  resolveProjectActorUserId(
    workspaceAccessControlStore.currentUser?.id,
    workspaceAccessControlStore.loading ? undefined : shell.activeWorkspaceSession?.session.userId,
  ),
)
const canPromoteSelectedDeliverable = computed(() =>
  Boolean(selectedConversationDeliverable.value)
  && Boolean(projectRecord.value)
  && Boolean(currentProjectActorUserId.value)
  && isProjectOwner(projectRecord.value, currentProjectActorUserId.value)
  && selectedDeliverableDetail.value?.promotionState !== 'promoted',
)
const summaryCards = computed(() => [
  {
    label: t('trace.stats.status'),
    value: runtime.activeRunStatusLabel,
  },
  {
    label: t('conversation.detail.summary.cards.messages'),
    value: String(runtime.activeMessages.length),
  },
  {
    label: t('conversation.detail.summary.cards.artifacts'),
    value: String(conversationArtifacts.value.length),
  },
  {
    label: t('conversation.detail.summary.cards.resources'),
    value: String(conversationResources.value.length),
  },
])
const memorySelectionSummary = computed(() => runtime.activeSession?.memorySelectionSummary ?? runtime.activeSession?.summary.memorySelectionSummary ?? null)
const selectedMemory = computed(() => runtime.activeRun?.selectedMemory ?? [])
const freshnessSummary = computed(() => runtime.activeRun?.freshnessSummary ?? null)
const pendingMemoryProposal = computed(() => runtime.pendingMemoryProposal)
const toolEntries = computed(() => {
  const entries = new Map<string, { toolId: string, label: string, kind: string, count: number }>()

  for (const message of runtime.activeMessages) {
    for (const toolCall of message.toolCalls ?? []) {
      const current = entries.get(toolCall.toolId)
      if (current) {
        current.count += toolCall.count
        continue
      }

      entries.set(toolCall.toolId, {
        toolId: toolCall.toolId,
        label: toolCall.label,
        kind: toolCall.kind,
        count: toolCall.count,
      })
    }
  }

  return [...entries.values()].sort((left, right) => right.count - left.count)
})
const opsCallouts = computed(() => {
  const callouts: Array<{ title: string, description: string, tone: 'warning' | 'info' }> = []

  if (runtime.pendingApproval) {
    callouts.push({
      title: runtime.pendingApproval.summary,
      description: runtime.pendingApproval.detail,
      tone: 'warning',
    })
  }

  if (runtime.pendingMediation) {
    callouts.push({
      title: runtime.pendingMediation.summary ?? t('common.na'),
      description: runtime.pendingMediation.detail ?? t('common.na'),
      tone: runtime.pendingMediation.requiresApproval || runtime.pendingMediation.requiresAuth ? 'warning' : 'info',
    })
  } else if (runtime.authTarget) {
    callouts.push({
      title: runtime.authTarget.summary,
      description: runtime.authTarget.detail,
      tone: 'warning',
    })
  }

  return callouts
})
const opsCards = computed(() => [
  {
    label: t('trace.stats.status'),
    value: runtime.activeRunStatusLabel,
  },
  {
    label: t('conversation.detail.ops.currentStep'),
    value: runtime.activeRunCurrentStepLabel || t('common.na'),
  },
  {
    label: t('conversation.detail.ops.toolCount'),
    value: String(toolEntries.value.length),
  },
  {
    label: t('conversation.detail.ops.timelineCount'),
    value: String(timelineItems.value.length),
  },
])
const usageSummary = computed(() =>
  runtime.activeMessages.reduce((total, message) => total + (message.usage?.totalTokens ?? 0), 0),
)

watch(
  () => [
    selectedConversationDeliverable.value?.id ?? '',
    shell.selectedDeliverableVersion ?? null,
    shell.workbenchMode,
  ] as const,
  async ([deliverableId, version, mode]) => {
    if (!deliverableId || mode !== 'deliverable') {
      return
    }
    await artifactStore.ensureDeliverableState(deliverableId, version ?? undefined)
  },
  { immediate: true },
)

watch(
  () => `${selectedConversationDeliverable.value?.id ?? ''}:${artifactStore.resolvedSelectedVersion ?? 'none'}`,
  (next, previous) => {
    if (next === previous) {
      return
    }
    isEditingDeliverable.value = false
    saveStatus.value = ''
    deliverableActionStatus.value = ''
    if (selectedConversationDeliverable.value?.id) {
      artifactStore.resetDraft(selectedConversationDeliverable.value.id)
    }
  },
)

function createConversationQueryTarget(query: Record<string, string | undefined>) {
  if (!hasConversationContext.value) {
    return null
  }

  return {
    ...createProjectConversationTarget(
      workspaceId.value,
      projectId.value,
      conversationId.value || undefined,
    ),
    query: {
      ...route.query,
      ...query,
    },
  }
}

async function pushConversationQuery(query: Record<string, string | undefined>) {
  if (!hasActiveConversation.value) {
    return
  }

  const target = createConversationQueryTarget(query)
  if (!target) {
    return
  }

  await router.replace(target)
}

function openResource() {
  if (!hasConversationContext.value) {
    return
  }

  void router.push({
    name: 'project-resources',
    params: {
      workspaceId: workspaceId.value,
      projectId: projectId.value,
    },
  })
}

function setWorkbenchMode(mode: string) {
  shell.setWorkbenchMode(mode as ConversationWorkbenchMode)
  shell.setRightSidebarCollapsed(false)
  if (!hasActiveConversation.value) {
    return
  }
  void pushConversationQuery({
    mode,
    deliverable: shell.selectedDeliverableId || undefined,
    version: shell.selectedDeliverableVersion ? String(shell.selectedDeliverableVersion) : undefined,
  })
}

function sectionButtonClass(itemId: ConversationWorkbenchMode): string {
  return [
    'h-auto rounded-[var(--radius-s)] border border-transparent px-2.5 py-1.5 text-[11px]',
    shell.workbenchMode === itemId
      ? 'border-border bg-subtle text-text-primary'
      : 'text-text-tertiary hover:bg-subtle hover:text-text-secondary',
    !hasActiveConversation.value ? 'cursor-not-allowed opacity-50 hover:bg-transparent hover:text-text-tertiary' : '',
  ].filter(Boolean).join(' ')
}

function sectionIconButtonClass(itemId: ConversationWorkbenchMode): string {
  return [
    'h-9 w-9 rounded-[var(--radius-s)] border px-0',
    shell.workbenchMode === itemId
      ? 'border-border bg-subtle text-text-primary'
      : 'border-transparent text-text-tertiary hover:border-border hover:bg-subtle hover:text-text-secondary',
    !hasActiveConversation.value ? 'cursor-not-allowed opacity-50 hover:border-transparent hover:bg-transparent hover:text-text-tertiary' : '',
  ].filter(Boolean).join(' ')
}

function selectConversationDeliverable(deliverableId: string) {
  if (!deliverableId) {
    return
  }
  isEditingDeliverable.value = false
  saveStatus.value = ''
  deliverableActionStatus.value = ''
  shell.selectDeliverable(deliverableId)
  void pushConversationQuery({
    mode: 'deliverable',
    deliverable: deliverableId,
    version: undefined,
  })
}

async function selectDeliverableVersion(version: number) {
  if (!selectedConversationDeliverable.value) {
    return
  }

  isEditingDeliverable.value = false
  saveStatus.value = ''
  deliverableActionStatus.value = ''
  shell.setSelectedDeliverableVersion(version)
  await artifactStore.ensureDeliverableVersionContent(selectedConversationDeliverable.value.id, version)
  await pushConversationQuery({
    mode: 'deliverable',
    deliverable: selectedConversationDeliverable.value.id,
    version: String(version),
  })
}

function beginEditingDeliverable() {
  saveStatus.value = ''
  isEditingDeliverable.value = true
}

function updateDeliverableDraft(value: string) {
  artifactStore.updateDraft(value)
}

function cancelDeliverableEditing() {
  if (selectedConversationDeliverable.value?.id) {
    artifactStore.resetDraft(selectedConversationDeliverable.value.id)
  }
  saveStatus.value = ''
  isEditingDeliverable.value = false
}

async function saveDeliverableVersion() {
  if (!selectedConversationDeliverable.value) {
    return
  }

  const detail = await artifactStore.saveDraftAsVersion({}, selectedConversationDeliverable.value.id)
  if (!detail) {
    return
  }

  isEditingDeliverable.value = false
  saveStatus.value = t('conversation.detail.deliverables.savedVersion', { version: detail.latestVersion })
  await pushConversationQuery({
    mode: 'deliverable',
    deliverable: selectedConversationDeliverable.value.id,
    version: String(detail.latestVersion),
  })
}

async function promoteSelectedDeliverable() {
  if (!selectedConversationDeliverable.value || !projectId.value || promotingDeliverable.value || !canPromoteSelectedDeliverable.value) {
    return
  }

  promotingDeliverable.value = true
  deliverableActionStatus.value = ''
  try {
    const record = await artifactStore.promoteDeliverable(selectedConversationDeliverable.value.id)
    if (!record) {
      return
    }
    await knowledgeStore.loadProjectKnowledge(projectId.value)
    deliverableActionStatus.value = t('deliverables.status.promoted')
  } finally {
    promotingDeliverable.value = false
  }
}

async function forkSelectedDeliverable() {
  if (!selectedConversationDeliverable.value || !projectId.value || forkingDeliverable.value) {
    return
  }

  forkingDeliverable.value = true
  deliverableActionStatus.value = ''
  try {
    const conversation = await artifactStore.forkDeliverable(
      projectId.value,
      selectedConversationDeliverable.value.title,
      selectedConversationDeliverable.value.id,
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
    forkingDeliverable.value = false
  }
}
</script>

<template>
  <aside
    data-testid="conversation-context-pane"
    class="h-full border-l border-border bg-surface"
    :class="shell.rightSidebarCollapsed ? 'w-[48px]' : 'w-[360px]'"
  >
    <div v-if="shell.rightSidebarCollapsed" class="flex h-full flex-col items-center gap-2 py-3">
      <UiButton
        data-testid="conversation-context-expand"
        variant="ghost"
        size="icon"
        class="h-9 w-9 rounded-[var(--radius-s)] border border-border bg-subtle text-text-tertiary hover:bg-surface hover:text-text-primary"
        :title="t('common.expand')"
        @click="shell.toggleRightSidebar()"
      >
        <PanelRight :size="18" />
      </UiButton>

      <div class="h-px w-6 bg-border-subtle" />

      <UiButton
        v-for="item in sectionItems"
        :key="item.id"
        :data-testid="`conversation-context-section-${item.id}`"
        variant="ghost"
        size="icon"
        :class="sectionIconButtonClass(item.id)"
        :title="item.label"
        :disabled="!hasActiveConversation"
        @click="setWorkbenchMode(item.id)"
      >
        <component :is="item.icon" :size="18" />
      </UiButton>
    </div>

    <div v-else class="flex h-full flex-col overflow-hidden">
      <div
        data-testid="conversation-context-header"
        class="flex items-center justify-between border-b border-border bg-subtle px-4 py-3"
      >
        <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">{{ t('conversation.detail.title') }}</div>
        <UiButton
          data-testid="conversation-context-collapse"
          variant="ghost"
          size="icon"
          class="h-7 w-7 rounded-[var(--radius-s)] border border-transparent text-text-tertiary hover:border-border hover:bg-surface hover:text-text-primary"
          :title="t('common.collapse')"
          @click="shell.toggleRightSidebar()"
        >
          <PanelRight :size="14" />
        </UiButton>
      </div>

      <nav class="flex flex-wrap gap-1 border-b border-border p-2">
        <UiButton
          v-for="item in sectionItems"
          :key="item.id"
          :data-testid="`conversation-context-section-${item.id}`"
          variant="ghost"
          size="sm"
          :class="sectionButtonClass(item.id)"
          :disabled="!hasActiveConversation"
          @click="setWorkbenchMode(item.id)"
        >
          {{ item.label }}
        </UiButton>
      </nav>

      <div class="flex-1 overflow-y-auto p-4">
        <UiEmptyState
          v-if="!hasActiveConversation"
          data-testid="conversation-context-empty-state"
          :title="t('conversation.detail.empty.title')"
          :description="t('conversation.detail.empty.description')"
          class="min-h-full"
        />

        <div v-else-if="shell.workbenchMode === 'deliverable'" class="flex min-h-full flex-col gap-4">
          <UiEmptyState
            v-if="!selectedConversationDeliverable"
            :title="t('conversation.detail.deliverables.emptyTitle')"
            :description="t('conversation.detail.deliverables.emptyDescription')"
          />

          <template v-else>
            <UiInspectorPanel
              data-testid="conversation-deliverable-overview"
              :title="selectedDeliverableDetail?.title ?? selectedConversationDeliverable.title"
              :subtitle="selectedConversationDeliverable.id"
              class="overflow-hidden"
            >
              <template #actions>
                <div class="flex flex-wrap justify-end gap-2">
                  <UiBadge
                    :label="enumLabel('resourcePreviewKind', selectedDeliverableContent?.previewKind ?? selectedDeliverableDetail?.previewKind ?? selectedConversationDeliverable.previewKind)"
                    subtle
                  />
                  <UiBadge
                    :label="localizedArtifactStatus.get(selectedConversationDeliverable.id) ?? selectedConversationDeliverable.status"
                    subtle
                  />
                  <UiBadge
                    :label="`v${artifactStore.resolvedSelectedVersion ?? selectedConversationDeliverable.latestVersion}`"
                    subtle
                  />
                  <UiBadge
                    v-if="localizedPromotionState"
                    :label="localizedPromotionState"
                    subtle
                  />
                </div>
              </template>

              <div class="space-y-4">
                <div v-if="deliverableOptions.length > 1" class="space-y-2">
                  <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">
                    {{ t('conversation.detail.deliverables.switchTitle') }}
                  </div>
                  <UiSelect
                    data-testid="deliverable-selector"
                    :model-value="selectedConversationDeliverable.id"
                    :options="deliverableOptions"
                    @update:model-value="selectConversationDeliverable"
                  />
                </div>

                <div class="grid gap-3 text-xs text-text-secondary sm:grid-cols-2">
                  <div>
                    <div class="font-medium text-text-tertiary">{{ t('conversation.detail.deliverables.updatedAt') }}</div>
                    <div class="mt-1 text-text-primary">
                      {{ formatDateTime(selectedDeliverableDetail?.updatedAt ?? selectedConversationDeliverable.updatedAt) }}
                    </div>
                  </div>
                  <div>
                    <div class="font-medium text-text-tertiary">{{ t('conversation.detail.deliverables.contentType') }}</div>
                    <div class="mt-1 text-text-primary">
                      {{ selectedDeliverableContent?.contentType ?? selectedDeliverableDetail?.contentType ?? selectedConversationDeliverable.contentType ?? t('common.na') }}
                    </div>
                  </div>
                </div>
              </div>

              <div
                data-testid="conversation-deliverable-actions"
                class="space-y-3 border-t border-border pt-4"
              >
                <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">
                  {{ t('deliverables.detail.actionsTitle') }}
                </div>

                <div class="flex flex-wrap gap-2">
                  <UiButton
                    v-if="canPromoteSelectedDeliverable"
                    size="sm"
                    variant="outline"
                    :disabled="promotingDeliverable"
                    data-testid="conversation-deliverable-promote"
                    @click="promoteSelectedDeliverable"
                  >
                    {{ t('deliverables.actions.promote') }}
                  </UiButton>

                  <UiButton
                    size="sm"
                    variant="outline"
                    :disabled="forkingDeliverable"
                    data-testid="conversation-deliverable-fork"
                    @click="forkSelectedDeliverable"
                  >
                    {{ t('deliverables.actions.fork') }}
                  </UiButton>
                </div>

                <UiStatusCallout
                  v-if="deliverableActionStatus"
                  :description="deliverableActionStatus"
                />
              </div>
            </UiInspectorPanel>

            <ArtifactVersionList
              :versions="selectedDeliverableVersions"
              :selected-version="artifactStore.resolvedSelectedVersion"
              :loading="artifactStore.loading && !selectedDeliverableVersions.length"
              @select="selectDeliverableVersion"
            />

            <div
              v-if="artifactStore.loading && !selectedDeliverableContent"
              data-testid="deliverable-preview-skeleton"
              class="space-y-3 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-4"
            >
              <UiSkeleton variant="line" :count="3" />
              <UiSkeleton variant="card" :count="1" />
            </div>

            <ArtifactPreviewPanel
              v-else
              :key="`${selectedConversationDeliverable.id}:${artifactStore.resolvedSelectedVersion ?? 'none'}:${isEditingDeliverable ? 'edit' : 'view'}`"
              :content="selectedDeliverableContent"
              :draft="selectedDeliverableDraft"
              :editing="isEditingDeliverable"
              :saving="artifactStore.saving"
              :error="artifactStore.error"
              :save-status="saveStatus"
              @edit="beginEditingDeliverable"
              @cancel="cancelDeliverableEditing"
              @save="saveDeliverableVersion"
              @update-draft="updateDeliverableDraft"
            />
          </template>
        </div>

        <div v-else-if="shell.workbenchMode === 'context'" class="space-y-4">
          <UiInspectorPanel
            data-testid="conversation-context-summary"
            :title="t('conversation.detail.summary.title')"
            :subtitle="t('conversation.detail.summary.subtitle')"
          >
            <div
              data-testid="conversation-context-summary-body"
              class="space-y-2"
            >
              <div class="text-sm text-text-primary">{{ runtime.activeSession?.summary.title ?? t('common.na') }}</div>
              <div class="text-xs text-text-secondary">{{ runtime.activeRunCurrentStepLabel }}</div>
              <div class="text-xs text-text-secondary">
                {{ t('conversation.detail.summary.tokenUsage') }}: {{ usageSummary }}
              </div>
            </div>
          </UiInspectorPanel>

          <div class="grid grid-cols-2 gap-3">
            <UiStatTile
              v-for="card in summaryCards"
              :key="card.label"
              :label="card.label"
              :value="card.value"
            />
          </div>

          <UiInspectorPanel
            v-if="selectedConversationDeliverable"
            :title="t('conversation.detail.context.promotionTitle')"
          >
            <template #actions>
              <UiBadge
                v-if="localizedPromotionState"
                :label="localizedPromotionState"
                subtle
              />
            </template>

            <div class="space-y-2 text-sm text-text-primary">
              <div>{{ t('conversation.detail.context.lineageSession') }}: {{ selectedDeliverableDetail?.sessionId ?? t('common.na') }}</div>
              <div>{{ t('conversation.detail.context.lineageRun') }}: {{ selectedDeliverableDetail?.runId ?? t('common.na') }}</div>
              <div>{{ t('conversation.detail.context.lineageMessage') }}: {{ selectedDeliverableDetail?.sourceMessageId ?? t('common.na') }}</div>
              <div>{{ t('conversation.detail.context.knowledgeLink') }}: {{ selectedDeliverableDetail?.promotionKnowledgeId ?? t('common.na') }}</div>
            </div>
          </UiInspectorPanel>

          <UiInspectorPanel
            v-if="freshnessSummary"
            data-testid="conversation-context-freshness"
            title="Freshness"
          >
            <template #actions>
              <UiBadge :label="freshnessSummary.freshnessRequired ? 'Required' : 'Optional'" subtle />
            </template>
            <div class="space-y-4">
              <div class="grid grid-cols-2 gap-3">
                <UiStatTile label="Fresh" :value="String(freshnessSummary.freshCount)" />
                <UiStatTile label="Stale" :value="String(freshnessSummary.staleCount)" />
              </div>
            </div>
          </UiInspectorPanel>

          <UiInspectorPanel
            v-if="pendingMemoryProposal"
            data-testid="conversation-memory-proposal"
            title="Memory Proposal"
            :subtitle="pendingMemoryProposal.summary"
          >
            <template #actions>
              <UiBadge :label="pendingMemoryProposal.proposalState" subtle />
            </template>
            <div class="space-y-3">
              <div class="text-sm font-semibold text-text-primary">{{ pendingMemoryProposal.title }}</div>
              <div class="flex flex-wrap gap-2">
                <UiBadge :label="pendingMemoryProposal.kind" subtle />
                <UiBadge :label="pendingMemoryProposal.scope" subtle />
              </div>
            </div>
          </UiInspectorPanel>

          <UiInspectorPanel
            :title="t('conversation.detail.memories.title')"
          >
            <div v-if="memorySelectionSummary" class="grid grid-cols-2 gap-3">
              <UiStatTile label="Selected" :value="String(memorySelectionSummary.selectedCount)" />
              <UiStatTile label="Ignored" :value="String(memorySelectionSummary.ignoredCount)" />
            </div>

            <div v-if="selectedMemory.length" class="space-y-2">
              <UiListRow
                v-for="entry in selectedMemory"
                :key="entry.memoryId"
                :title="entry.title"
                :subtitle="entry.summary"
              >
                <template #meta>
                  <UiBadge :label="entry.kind" subtle />
                  <UiBadge :label="entry.freshnessState" subtle />
                </template>
              </UiListRow>
            </div>

            <UiEmptyState
              v-else
              :title="t('conversation.detail.memories.emptyTitle')"
              :description="t('conversation.detail.memories.emptyDescription')"
            />
          </UiInspectorPanel>

          <UiInspectorPanel
            :title="t('conversation.detail.resources.title')"
            :subtitle="t('conversation.detail.resources.emptyDescription')"
          >
            <template #actions>
              <UiButton size="sm" variant="ghost" @click="openResource">{{ t('conversation.detail.resources.openFullPage') }}</UiButton>
            </template>

            <UiInput v-model="resourceFilter" :placeholder="t('conversation.detail.resources.filterPlaceholder')" />

            <div v-if="visibleConversationResources.length" class="space-y-2">
              <UiListRow
                v-for="resource in visibleConversationResources"
                :key="resource.id"
                :title="resource.name"
                :subtitle="resource.location || resource.origin"
              >
                <template #meta>
                  <div class="flex flex-wrap items-center gap-2">
                    <UiBadge :label="localizedResourceKind.get(resource.id) ?? resource.kind" subtle />
                    <UiBadge :label="localizedResourceOrigin.get(resource.id) ?? resource.origin" subtle />
                    <span v-if="resource.sourceArtifactId" class="inline-flex items-center gap-1 text-xs text-text-tertiary">
                      <Link2 :size="12" />
                      {{ resource.sourceArtifactId }}
                    </span>
                  </div>
                </template>
              </UiListRow>
            </div>

            <UiEmptyState
              v-else
              :title="t('conversation.detail.deliverables.noLinkedResourcesTitle')"
              :description="t('conversation.detail.deliverables.noLinkedResourcesDescription')"
            />
          </UiInspectorPanel>
        </div>

        <div v-else-if="shell.workbenchMode === 'ops'" class="space-y-4">
          <div v-if="opsCallouts.length" class="space-y-3">
            <UiStatusCallout
              v-for="(callout, index) in opsCallouts"
              :key="`${callout.title}:${index}`"
              :tone="callout.tone"
              :title="callout.title"
              :description="callout.description"
            >
              <div class="flex items-center gap-2 text-xs font-semibold">
                <ShieldAlert :size="13" class="shrink-0" />
                {{ t('conversation.detail.ops.pendingTitle') }}
              </div>
            </UiStatusCallout>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <UiStatTile
              v-for="card in opsCards"
              :key="card.label"
              :label="card.label"
              :value="card.value"
            />
          </div>

          <UiInspectorPanel
            data-testid="conversation-ops-tools"
            :title="t('conversation.detail.tools.title')"
            :subtitle="t('conversation.detail.tools.emptyDescription')"
          >
            <div class="space-y-2">
              <div v-if="toolEntries.length" class="space-y-2">
                <UiListRow
                  v-for="tool in toolEntries"
                  :key="tool.toolId"
                  :title="tool.label"
                  :subtitle="tool.toolId"
                >
                  <template #meta>
                    <div class="flex items-center gap-2">
                      <UiBadge :label="tool.kind" subtle />
                      <span class="text-xs text-text-tertiary">×{{ tool.count }}</span>
                    </div>
                  </template>
                </UiListRow>
              </div>

              <UiEmptyState
                v-else
                :title="t('conversation.detail.tools.emptyTitle')"
                :description="t('conversation.detail.tools.emptyDescription')"
              />
            </div>
          </UiInspectorPanel>

          <UiInspectorPanel
            data-testid="conversation-ops-timeline"
            :title="t('conversation.detail.timeline.title')"
            :subtitle="t('conversation.detail.timeline.emptyDescription')"
          >
            <div class="space-y-3">
              <UiTimelineList
                v-if="timelineItems.length"
                :items="timelineItems"
              />

              <UiEmptyState
                v-else
                :title="t('conversation.detail.timeline.emptyTitle')"
                :description="t('conversation.detail.timeline.emptyDescription')"
              />
            </div>
          </UiInspectorPanel>
        </div>

        <UiEmptyState
          v-else
          :title="t('conversation.detail.emptyTitle')"
          :description="t('conversation.detail.emptyDescription')"
        />
      </div>
    </div>
  </aside>
</template>
