<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { Brain, FileText, FolderTree, Link2, PanelRight, Sparkles, Waypoints, Wrench } from 'lucide-vue-next'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiInput,
  UiListRow,
  UiPanelFrame,
  UiStatTile,
  UiTimelineList,
} from '@octopus/ui'

import type { ConversationDetailFocus } from '@octopus/schema'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useResourceStore } from '@/stores/resource'
import { useArtifactStore } from '@/stores/artifact'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const runtime = useRuntimeStore()
const resourceStore = useResourceStore()
const artifactStore = useArtifactStore()
const artifactFilter = ref('')
const resourceFilter = ref('')

const sectionItems = computed(() => [
  { id: 'summary', label: t('conversation.detail.sections.summary'), icon: Sparkles },
  { id: 'memories', label: t('conversation.detail.sections.memories'), icon: Brain },
  { id: 'artifacts', label: t('conversation.detail.sections.artifacts'), icon: FileText },
  { id: 'resources', label: t('conversation.detail.sections.resources'), icon: FolderTree },
  { id: 'tools', label: t('conversation.detail.sections.tools'), icon: Wrench },
  { id: 'timeline', label: t('conversation.detail.sections.timeline'), icon: Waypoints },
] as const)

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
const conversationArtifactIds = computed(() => [...new Set(runtime.activeMessages.flatMap(message => message.artifacts ?? []))])
const conversationResources = computed(() =>
  resourceStore.activeProjectResources.filter(resource => conversationResourceIds.value.includes(resource.id)),
)
const conversationArtifacts = computed(() =>
  artifactStore.activeProjectArtifacts.filter(artifact => conversationArtifactIds.value.includes(artifact.id)),
)
const localizedArtifactStatus = computed(() =>
  new Map(conversationArtifacts.value.map(artifact => [artifact.id, enumLabel('artifactStatus', artifact.status)])),
)
const localizedResourceKind = computed(() =>
  new Map(conversationResources.value.map(resource => [resource.id, enumLabel('projectResourceKind', resource.kind)])),
)
const localizedResourceOrigin = computed(() =>
  new Map(conversationResources.value.map(resource => [resource.id, enumLabel('projectResourceOrigin', resource.origin)])),
)
const filteredConversationArtifacts = computed(() => {
  const query = artifactFilter.value.trim().toLowerCase()
  return conversationArtifacts.value.filter((artifact) => {
    if (!query) {
      return true
    }
    return [artifact.title, artifact.id, artifact.status, artifact.contentType ?? '']
      .join(' ')
      .toLowerCase()
      .includes(query)
  })
})
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
const selectedArtifact = computed(() =>
  filteredConversationArtifacts.value.find(artifact => artifact.id === shell.selectedArtifactId) ?? filteredConversationArtifacts.value[0] ?? null,
)
const selectedArtifactSourceResources = computed(() =>
  filteredConversationResources.value.filter(resource => resource.sourceArtifactId === selectedArtifact.value?.id),
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
const memoryEntries = computed(() =>
  runtime.activeMessages
    .filter(message => message.senderType === 'agent')
    .slice(-4)
    .map(message => ({
      id: message.id,
      title: message.actorId ?? message.senderId,
      detail: message.content,
      source: message.actorKind === 'team' ? t('conversation.detail.memories.agentSource') : t('conversation.detail.memories.conversationSource'),
    })),
)
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
const usageSummary = computed(() =>
  runtime.activeMessages.reduce((total, message) => total + (message.usage?.totalTokens ?? 0), 0),
)

async function pushConversationQuery(query: Record<string, string | undefined>) {
  await router.replace({
    name: 'project-conversation',
    params: route.params,
    query: {
      ...route.query,
      ...query,
    },
  })
}

function openArtifact(artifactId: string) {
  shell.selectArtifact(artifactId)
  void pushConversationQuery({
    detail: 'artifacts',
    artifact: artifactId,
  })
}

function openResource() {
  void router.push({
    name: 'project-resources',
    params: route.params,
  })
}

function setDetail(detail: string) {
  shell.setDetailFocus(detail as ConversationDetailFocus)
  shell.setRightSidebarCollapsed(false)
}
</script>

<template>
  <aside
    data-testid="conversation-context-pane"
    class="h-full border-l border-border bg-surface"
    :class="shell.rightSidebarCollapsed ? 'w-[48px]' : 'w-[360px]'"
  >
    <div v-if="shell.rightSidebarCollapsed" class="flex h-full flex-col items-center gap-3 py-4">
      <UiButton
        variant="ghost"
        size="icon"
        class="h-9 w-9 rounded-lg text-text-tertiary hover:bg-muted/80 hover:text-text-primary"
        :title="t('common.expand')"
        @click="shell.toggleRightSidebar()"
      >
        <PanelRight :size="18" />
      </UiButton>

      <div class="h-px w-6 bg-border-subtle" />

      <UiButton
        v-for="item in sectionItems"
        :key="item.id"
        variant="ghost"
        size="icon"
        class="h-9 w-9 rounded-lg"
        :title="item.label"
        @click="setDetail(item.id)"
      >
        <component :is="item.icon" :size="18" />
      </UiButton>
    </div>

    <div v-else class="flex h-full flex-col overflow-hidden">
      <div class="flex items-center justify-between border-b border-border px-4 py-3">
        <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">{{ t('conversation.detail.title') }}</div>
        <UiButton
          variant="ghost"
          size="icon"
          class="h-6 w-6 text-text-tertiary hover:bg-muted/80 hover:text-text-primary"
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
          variant="ghost"
          size="sm"
          class="h-auto rounded-[var(--radius-s)] border border-transparent px-2.5 py-1.5 text-[11px]"
          :class="shell.detailFocus === item.id ? 'border-border bg-surface text-text-primary shadow-xs' : 'text-text-tertiary hover:bg-subtle hover:text-text-secondary'"
          @click="setDetail(item.id)"
        >
          {{ item.label }}
        </UiButton>
      </nav>

      <div class="flex-1 overflow-y-auto p-4">
        <div v-if="shell.detailFocus === 'summary'" class="space-y-4">
          <UiPanelFrame variant="subtle" padding="md">
            <div class="text-xs text-text-secondary">{{ t('conversation.detail.summary.title') }}</div>
            <div class="mt-2 text-sm text-text-primary">{{ runtime.activeSession?.summary.title ?? t('common.na') }}</div>
            <div class="mt-2 text-xs text-text-secondary">{{ runtime.activeRunCurrentStepLabel }}</div>
          </UiPanelFrame>
          <div class="grid grid-cols-2 gap-3">
            <UiStatTile
              v-for="card in summaryCards"
              :key="card.label"
              :label="card.label"
              :value="card.value"
            />
          </div>
          <UiPanelFrame variant="subtle" padding="md">
            <div class="text-xs text-text-secondary">{{ t('conversation.detail.summary.tokenUsage') }}</div>
            <div class="mt-2 text-sm text-text-primary">{{ usageSummary }}</div>
          </UiPanelFrame>
        </div>

        <div v-else-if="shell.detailFocus === 'memories'" class="space-y-4">
          <div v-if="memoryEntries.length" class="space-y-2">
            <UiListRow
              v-for="entry in memoryEntries"
              :key="entry.id"
              :title="entry.title"
              :subtitle="entry.detail"
            >
              <template #meta>
                <UiBadge :label="entry.source" subtle />
              </template>
            </UiListRow>
          </div>
          <UiEmptyState
            v-else
            :title="t('conversation.detail.memories.emptyTitle')"
            :description="t('conversation.detail.memories.emptyDescription')"
          />
        </div>

        <div v-else-if="shell.detailFocus === 'artifacts'" class="space-y-4">
          <UiPanelFrame variant="subtle" padding="sm">
            <UiInput v-model="artifactFilter" :placeholder="t('conversation.detail.artifacts.filterPlaceholder')" />
          </UiPanelFrame>
          <div v-if="selectedArtifact" class="space-y-4">
            <UiPanelFrame variant="raised" padding="md">
              <div class="flex items-start justify-between gap-3">
                <div class="space-y-1">
                  <div class="text-sm font-semibold text-text-primary">{{ selectedArtifact.title }}</div>
                  <div class="text-xs text-text-secondary">{{ selectedArtifact.id }}</div>
                </div>
                <div class="flex flex-wrap gap-2">
                  <UiBadge :label="localizedArtifactStatus.get(selectedArtifact.id) ?? selectedArtifact.status" subtle />
                  <UiBadge :label="`v${selectedArtifact.latestVersion}`" subtle />
                </div>
              </div>
              <div class="mt-3 grid gap-3 text-xs text-text-secondary sm:grid-cols-2">
                <div>
                  <div class="font-medium text-text-tertiary">{{ t('conversation.detail.artifacts.updatedAt') }}</div>
                  <div class="mt-1 text-text-primary">{{ formatDateTime(selectedArtifact.updatedAt) }}</div>
                </div>
                <div>
                  <div class="font-medium text-text-tertiary">{{ t('conversation.detail.artifacts.contentType') }}</div>
                  <div class="mt-1 text-text-primary">{{ selectedArtifact.contentType ?? t('common.na') }}</div>
                </div>
              </div>
            </UiPanelFrame>

            <div class="space-y-2">
              <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">{{ t('conversation.detail.artifacts.listTitle') }}</div>
              <UiListRow
                v-for="artifact in filteredConversationArtifacts"
                :key="artifact.id"
                :title="artifact.title"
                :subtitle="artifact.id"
                @click="openArtifact(artifact.id)"
              >
                <template #meta>
                  <UiBadge :label="localizedArtifactStatus.get(artifact.id) ?? artifact.status" subtle />
                  <span class="text-xs text-text-tertiary">v{{ artifact.latestVersion }}</span>
                </template>
              </UiListRow>
            </div>

            <div class="space-y-2">
              <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">{{ t('conversation.detail.artifacts.linkedResourcesTitle') }}</div>
              <UiListRow
                v-for="resource in selectedArtifactSourceResources"
                :key="resource.id"
                :title="resource.name"
                :subtitle="resource.location || resource.origin"
              >
                <template #meta>
                  <UiBadge :label="localizedResourceKind.get(resource.id) ?? resource.kind" subtle />
                </template>
              </UiListRow>
              <UiEmptyState
                v-if="!selectedArtifactSourceResources.length"
                :title="t('conversation.detail.artifacts.noLinkedResourcesTitle')"
                :description="t('conversation.detail.artifacts.noLinkedResourcesDescription')"
              />
            </div>
          </div>
          <UiEmptyState
            v-else
            :title="t('conversation.detail.artifacts.emptyTitle')"
            :description="t('conversation.detail.artifacts.emptyDescription')"
          />
        </div>

        <div v-else-if="shell.detailFocus === 'resources'" class="space-y-4">
          <UiPanelFrame variant="subtle" padding="sm" class="space-y-3">
            <UiInput v-model="resourceFilter" :placeholder="t('conversation.detail.resources.filterPlaceholder')" />
            <UiButton size="sm" variant="ghost" @click="openResource">{{ t('conversation.detail.resources.openFullPage') }}</UiButton>
          </UiPanelFrame>
          <div v-if="filteredConversationResources.length" class="space-y-2">
            <UiListRow
              v-for="resource in filteredConversationResources"
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
            :title="t('conversation.detail.resources.emptyTitle')"
            :description="t('conversation.detail.resources.emptyDescription')"
          />
        </div>

        <div v-else-if="shell.detailFocus === 'tools'" class="space-y-4">
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

        <div v-else-if="shell.detailFocus === 'timeline'" class="space-y-4">
          <UiTimelineList v-if="timelineItems.length" :items="timelineItems" />
          <UiEmptyState v-else :title="t('conversation.detail.timeline.emptyTitle')" :description="t('conversation.detail.timeline.emptyDescription')" />
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
