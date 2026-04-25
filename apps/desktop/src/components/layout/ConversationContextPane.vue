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
  cn,
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
    class="h-full border-l border-border/30 bg-sidebar/30 backdrop-blur-xl transition-all duration-500"
    :class="shell.rightSidebarCollapsed ? 'w-[64px]' : 'w-[400px]'"
  >
    <div v-if="shell.rightSidebarCollapsed" class="flex h-full flex-col items-center gap-4 py-4 bg-black/10">
      <UiButton
        data-testid="conversation-context-expand"
        variant="ghost"
        size="icon"
        class="h-10 w-10 rounded-xl border border-border/40 bg-subtle/50 text-text-tertiary hover:bg-primary/10 hover:text-primary transition-all"
        @click="shell.toggleRightSidebar()"
      >
        <PanelRight :size="20" />
      </UiButton>

      <div class="h-px w-8 bg-border/30" />

      <div class="flex flex-col gap-3">
        <button
          v-for="item in sectionItems"
          :key="item.id"
          :class="cn(
            'flex size-10 items-center justify-center rounded-xl transition-all',
            shell.workbenchMode === item.id ? 'bg-primary text-primary-foreground shadow-lg shadow-primary/20' : 'bg-black/10 text-text-tertiary hover:bg-black/20 hover:text-text-secondary'
          )"
          @click="setWorkbenchMode(item.id)"
        >
          <component :is="item.icon" :size="18" />
        </button>
      </div>
    </div>

    <div v-else class="flex h-full flex-col overflow-hidden">
      <!-- Header -->
      <div
        data-testid="conversation-context-header"
        class="flex items-center justify-between border-b border-border/50 bg-black/20 px-5 py-4"
      >
        <div class="flex items-center gap-3">
           <div class="size-2 rounded-full bg-primary shadow-[0_0_8px_var(--color-primary)] animate-pulse" />
           <div class="text-[11px] font-extrabold uppercase tracking-[0.2em] text-text-primary">{{ t('conversation.detail.title') }}</div>
        </div>
        <UiButton
          data-testid="conversation-context-collapse"
          variant="ghost"
          size="icon"
          class="h-8 w-8 rounded-lg text-text-tertiary hover:bg-black/20"
          @click="shell.toggleRightSidebar()"
        >
          <PanelRight :size="16" />
        </UiButton>
      </div>

      <!-- Navigation -->
      <nav class="flex p-3 bg-black/5">
        <div class="flex w-full rounded-xl bg-black/20 p-1 border border-border/30">
          <button
            v-for="item in sectionItems"
            :key="item.id"
            :class="cn(
              'flex-1 flex items-center justify-center gap-2 py-2 text-[11px] font-bold uppercase tracking-tight transition-all rounded-lg',
              shell.workbenchMode === item.id ? 'bg-primary text-primary-foreground shadow-sm' : 'text-text-tertiary hover:text-text-secondary'
            )"
            @click="setWorkbenchMode(item.id)"
          >
            <component :is="item.icon" :size="14" />
            <span class="hidden sm:inline">{{ item.label }}</span>
          </button>
        </div>
      </nav>

      <!-- Content Area -->
      <div class="flex-1 overflow-y-auto p-5 scroll-y space-y-6">
        <UiEmptyState
          v-if="!hasActiveConversation"
          data-testid="conversation-context-empty-state"
          :title="t('conversation.detail.empty.title')"
          :description="t('conversation.detail.empty.description')"
          class="bg-black/5 rounded-2xl"
        />

        <div v-else-if="shell.workbenchMode === 'deliverable'" class="space-y-6" v-auto-animate>
          <UiEmptyState
            v-if="!selectedConversationDeliverable"
            :title="t('conversation.detail.deliverables.emptyTitle')"
            class="bg-black/5 rounded-2xl"
          />

          <template v-else>
            <UiSurface
              variant="glass"
              padding="md"
              :title="selectedDeliverableDetail?.title ?? selectedConversationDeliverable.title"
              highlight-border
            >
              <template #actions>
                <UiBadge
                  :label="`v${artifactStore.resolvedSelectedVersion ?? selectedConversationDeliverable.latestVersion}`"
                  class="bg-primary/10 text-primary border-primary/20 font-mono"
                />
              </template>

              <div class="space-y-5">
                <div v-if="deliverableOptions.length > 1" class="space-y-2">
                  <div class="text-[10px] font-bold uppercase tracking-widest text-text-tertiary opacity-60">
                    {{ t('conversation.detail.deliverables.switchTitle') }}
                  </div>
                  <UiSelect
                    class="bg-black/20 border-border/40"
                    :model-value="selectedConversationDeliverable.id"
                    :options="deliverableOptions"
                    @update:model-value="selectConversationDeliverable"
                  />
                </div>

                <div class="grid grid-cols-2 gap-4">
                   <UiStatTile :label="t('conversation.detail.deliverables.updatedAt')" :value="formatDateTime(selectedDeliverableDetail?.updatedAt ?? selectedConversationDeliverable.updatedAt)" tone="default" class="p-3" />
                   <UiStatTile :label="t('conversation.detail.deliverables.contentType')" :value="selectedDeliverableContent?.contentType ?? 'Markdown'" tone="default" class="p-3" />
                </div>

                <div class="flex flex-wrap gap-2 pt-2 border-t border-border/30">
                  <UiButton
                    v-if="canPromoteSelectedDeliverable"
                    size="sm"
                    class="flex-1"
                    :disabled="promotingDeliverable"
                    @click="promoteSelectedDeliverable"
                  >
                    {{ t('deliverables.actions.promote') }}
                  </UiButton>

                  <UiButton
                    size="sm"
                    variant="outline"
                    class="flex-1 bg-surface/50"
                    :disabled="forkingDeliverable"
                    @click="forkSelectedDeliverable"
                  >
                    {{ t('deliverables.actions.fork') }}
                  </UiButton>
                </div>
              </div>
            </UiSurface>

            <ArtifactVersionList
              :versions="selectedDeliverableVersions"
              :selected-version="artifactStore.resolvedSelectedVersion"
              class="rounded-2xl border-border/30 bg-black/10"
              @select="selectDeliverableVersion"
            />

            <ArtifactPreviewPanel
              :key="`${selectedConversationDeliverable.id}:${artifactStore.resolvedSelectedVersion ?? 'none'}:${isEditingDeliverable ? 'edit' : 'view'}`"
              :content="selectedDeliverableContent"
              :draft="selectedDeliverableDraft"
              :editing="isEditingDeliverable"
              class="rounded-2xl border border-border/30 bg-black/5"
              @edit="beginEditingDeliverable"
              @cancel="cancelDeliverableEditing"
              @save="saveDeliverableVersion"
              @update-draft="updateDeliverableDraft"
            />
          </template>
        </div>

        <div v-else-if="shell.workbenchMode === 'context'" class="space-y-6" v-auto-animate>
          <UiSurface
            variant="glass-strong"
            padding="md"
            title="Session Overview"
          >
            <div class="space-y-3">
              <div class="text-sm font-bold text-text-primary tracking-tight">{{ runtime.activeSession?.summary.title }}</div>
              <div class="flex flex-wrap gap-2">
                 <UiBadge :label="`${usageSummary} Tokens`" class="bg-primary/10 text-primary border-primary/20" />
                 <UiBadge :label="runtime.activeRunStatusLabel" tone="warning" />
              </div>
            </div>
          </UiSurface>

          <div class="grid grid-cols-2 gap-3">
            <UiStatTile
              v-for="card in summaryCards.slice(1)"
              :key="card.label"
              :label="card.label"
              :value="card.value"
              class="p-4"
            />
          </div>

          <UiSurface variant="glass" padding="md" title="Memories & Knowledge">
             <div v-if="selectedMemory.length" class="space-y-2">
                <div v-for="entry in selectedMemory" :key="entry.memoryId" class="p-3 rounded-xl bg-black/20 border border-border/30">
                   <div class="text-[12px] font-bold text-text-primary">{{ entry.title }}</div>
                   <div class="text-[10px] text-text-tertiary mt-1 uppercase font-bold tracking-tight">{{ entry.kind }} · {{ entry.freshnessState }}</div>
                </div>
             </div>
             <UiEmptyState v-else compact title="No active memories" class="bg-black/5 py-8" />
          </UiSurface>
        </div>

        <div v-else-if="shell.workbenchMode === 'ops'" class="space-y-6" v-auto-animate>
          <div class="grid grid-cols-2 gap-3">
            <UiStatTile
              v-for="card in opsCards"
              :key="card.label"
              :label="card.label"
              :value="card.value"
              class="p-4"
            />
          </div>

          <UiSurface variant="glass" padding="md" title="Runtime Operations">
            <UiTimelineList
              v-if="timelineItems.length"
              :items="timelineItems.slice(0, 5)"
              density="compact"
            />
            <UiEmptyState v-else compact title="No operations logged" class="bg-black/5 py-8" />
          </UiSurface>
        </div>
      </div>
    </div>
  </aside>
</template>
