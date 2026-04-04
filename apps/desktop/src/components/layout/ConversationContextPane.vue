<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import {
  Brain,
  BookOpen,
  FileText,
  FolderTree,
  Sparkles,
  Waypoints,
  Wrench,
} from 'lucide-vue-next'

import { UiBadge, UiButton, UiEmptyState, UiNavCardList, UiSurface, UiTextarea, UiTimelineList } from '@octopus/ui'

import type { ConversationDetailFocus } from '@octopus/schema'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const activeConversation = computed(() => workbench.activeConversation)
const selectedArtifact = computed(() =>
  workbench.activeConversationArtifacts.find((artifact: { id: string }) => artifact.id === shell.selectedArtifactId)
  ?? workbench.activeConversationArtifacts[0],
)
const artifactDraft = ref('')

const sectionItems = computed(() => [
  { id: 'summary', label: t('conversation.detail.sections.summary'), icon: Sparkles },
  { id: 'memories', label: t('conversation.detail.sections.memories'), icon: Brain },
  { id: 'artifacts', label: t('conversation.detail.sections.artifacts'), icon: FileText },
  { id: 'knowledge', label: t('conversation.detail.sections.knowledge'), icon: BookOpen },
  { id: 'resources', label: t('conversation.detail.sections.resources'), icon: FolderTree },
  { id: 'tools', label: t('conversation.detail.sections.tools'), icon: Wrench },
  { id: 'timeline', label: t('conversation.detail.sections.timeline'), icon: Waypoints },
] as const)

const expandedSectionNavItems = computed(() =>
  sectionItems.value.map((section) => ({
    id: section.id,
    label: section.label,
    icon: section.icon,
    active: shell.detailFocus === section.id,
  })),
)

const artifactItems = computed(() =>
  workbench.activeConversationArtifacts.map((artifact) => ({
    id: artifact.id,
    label: workbench.artifactDisplayTitle(artifact.id),
    helper: workbench.artifactDisplayExcerpt(artifact.id),
    badge: enumLabel('artifactStatus', artifact.status),
    icon: FileText,
    active: shell.selectedArtifactId === artifact.id,
  })),
)

const timelineItems = computed(() =>
  workbench.activeConversationTimeline.map((trace) => ({
    id: trace.id,
    title: workbench.traceDisplayTitle(trace.id),
    description: workbench.traceDisplayDetail(trace.id),
    helper: trace.actor,
    timestamp: formatDateTime(trace.timestamp),
  })),
)

watch(
  selectedArtifact,
  (artifact) => {
    artifactDraft.value = artifact ? workbench.artifactDisplayContent(artifact.id) : ''
  },
  { immediate: true },
)

function updateQuery(detail: ConversationDetailFocus, artifactId?: string) {
  void router.replace({
    query: {
      ...route.query,
      detail,
      ...(artifactId ? { artifact: artifactId } : {}),
    },
  })
}

function setDetail(detail: string) {
  const nextDetail = detail as ConversationDetailFocus
  shell.setDetailFocus(nextDetail)
  shell.setRightSidebarCollapsed(false)
  updateQuery(nextDetail, nextDetail === 'artifacts' ? shell.selectedArtifactId || undefined : undefined)
}

function openArtifact(artifactId: string) {
  shell.selectArtifact(artifactId)
  shell.setDetailFocus('artifacts')
  shell.setRightSidebarCollapsed(false)
  updateQuery('artifacts', artifactId)
}

function saveArtifactDraft() {
  if (!selectedArtifact.value) return
  workbench.updateArtifactContent(selectedArtifact.value.id, artifactDraft.value)
}

function requestReview() {
  if (!selectedArtifact.value) return
  workbench.requestArtifactReview(selectedArtifact.value.id)
  shell.setDetailFocus('timeline')
  shell.setRightSidebarCollapsed(false)
  updateQuery('timeline', selectedArtifact.value.id)
}
</script>

<template>
  <div class="h-full flex flex-col bg-sidebar border-l border-border-subtle dark:border-white/[0.05]">
    <!-- Rail View (Collapsed State: 48px width) -->
    <aside
      v-if="shell.rightSidebarCollapsed"
      class="flex h-full w-[48px] flex-col items-center gap-3 py-6"
      data-testid="conversation-detail-rail"
    >
      <nav class="flex w-full flex-col items-center gap-3" data-testid="conversation-detail-rail-nav">
        <button
          v-for="item in expandedSectionNavItems"
          :key="item.id"
          class="flex h-9 w-9 items-center justify-center rounded-lg transition-all"
          :class="item.active
            ? 'bg-primary/10 text-primary shadow-xs'
            : 'text-text-tertiary hover:bg-accent hover:text-text-secondary'"
          :title="item.label"
          :data-testid="`conversation-detail-rail-section-${item.id}`"
          @click="setDetail(item.id)"
        >
          <component :is="item.icon" :size="18" />
        </button>
      </nav>
    </aside>

    <!-- Panel View (Expanded State: 360px width) -->
    <aside v-else class="flex h-full flex-col overflow-hidden w-[360px]" data-testid="conversation-detail-panel">
      <!-- Fixed Header Area -->
      <div class="flex shrink-0 items-center px-4 h-11 border-b border-border-subtle dark:border-white/[0.05] bg-sidebar/80 backdrop-blur-md sticky top-0 z-10">
        <span class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">{{ t('conversation.detail.title') }}</span>
      </div>

      <!-- Fixed Navigation Tabs Area -->
      <nav class="flex shrink-0 flex-wrap gap-1 border-b border-border-subtle dark:border-white/[0.05] p-2 bg-sidebar/50 sticky top-[44px] z-10">
        <button
          v-for="item in expandedSectionNavItems"
          :key="item.id"
          class="flex items-center gap-2 rounded px-2.5 py-1.5 text-[11px] font-medium transition-all"
          :class="item.active 
            ? 'bg-background shadow-xs text-text-primary border border-border-subtle' 
            : 'text-text-tertiary hover:bg-accent hover:text-text-secondary border border-transparent'"
          @click="setDetail(item.id)"
        >
          <component :is="item.icon" :size="14" />
          {{ item.label }}
        </button>
      </nav>

      <!-- Independent Content Scroll Area -->
      <div class="scroll-y flex-1 p-4 bg-background/30">
        <div v-if="shell.detailFocus === 'summary' && activeConversation" class="flex flex-col gap-6">
          <div class="space-y-1">
            <h3 class="text-sm font-bold text-text-primary">{{ t('conversation.detail.summary.title') }}</h3>
            <p class="text-[12px] text-text-secondary">{{ t('conversation.detail.summary.subtitle') }}</p>
          </div>

          <div class="space-y-4">
            <div class="space-y-1.5">
              <span class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary">{{ t('common.goal') }}</span>
              <p class="text-[13px] leading-relaxed text-text-secondary bg-subtle/30 p-3 rounded-md border border-border-subtle dark:border-white/[0.08]">{{ workbench.conversationDisplayGoal(activeConversation.id) }}</p>
            </div>
            
            <div class="space-y-1.5">
              <span class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary">{{ t('conversation.detail.summary.statusNote') }}</span>
              <p class="text-[13px] leading-relaxed text-text-secondary bg-subtle/30 p-3 rounded-md border border-border-subtle dark:border-white/[0.08]">{{ workbench.conversationDisplayStatusNote(activeConversation.id) }}</p>
            </div>

            <div class="space-y-1.5">
              <span class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary">{{ t('common.constraints') }}</span>
              <ul class="space-y-2 bg-subtle/30 p-3 rounded-md border border-border-subtle dark:border-white/[0.08]">
                <li
                  v-for="(constraint, index) in workbench.conversationDisplayConstraints(activeConversation.id)"
                  :key="`${activeConversation.id}-constraint-${index}`"
                  class="flex gap-2 text-[12px] leading-relaxed text-text-secondary"
                >
                  <span class="text-text-tertiary opacity-50">•</span>
                  {{ constraint }}
                </li>
              </ul>
            </div>
          </div>
        </div>

        <div v-else-if="shell.detailFocus === 'memories'" class="space-y-4">
          <div v-if="workbench.activeConversationMemories.length" class="space-y-3">
            <div
              v-for="memory in workbench.activeConversationMemories"
              :key="memory.id"
              class="bg-background rounded-md border border-border-subtle dark:border-white/[0.08] p-3 space-y-2 shadow-xs"
            >
              <div class="flex flex-wrap items-center gap-2">
                <UiBadge :label="memory.source === 'agent' ? t('conversation.detail.memories.agentSource') : t('conversation.detail.memories.conversationSource')" subtle />
              </div>
              <strong class="block text-[13px] font-bold text-text-primary">{{ memory.title }}</strong>
              <p class="text-[12px] leading-relaxed text-text-secondary">{{ memory.summary }}</p>
              <small class="block text-[10px] text-text-tertiary">{{ formatDateTime(memory.createdAt) }}</small>
            </div>
          </div>
          <UiEmptyState
            v-else
            :title="t('conversation.detail.memories.emptyTitle')"
            :description="t('conversation.detail.memories.emptyDescription')"
          />
        </div>

        <template v-else-if="shell.detailFocus === 'artifacts'">
          <div class="space-y-4">
            <UiNavCardList
              v-if="artifactItems.length"
              :items="artifactItems"
              @select="openArtifact"
            />
            <UiEmptyState
              v-else
              :title="t('conversation.detail.artifacts.emptyTitle')"
              :description="t('conversation.detail.artifacts.emptyDescription')"
            />

            <div v-if="selectedArtifact" class="pt-4 border-t border-border-subtle dark:border-white/[0.05] space-y-4">
              <div class="flex items-center justify-between">
                <h4 class="text-[13px] font-bold text-text-primary">{{ workbench.artifactDisplayTitle(selectedArtifact.id) }}</h4>
                <UiBadge :label="`v${selectedArtifact.version}`" subtle />
              </div>
              <UiTextarea v-model="artifactDraft" class="bg-subtle/30 font-mono text-[12px]" :rows="12" />
              <div class="flex gap-2">
                <UiButton variant="ghost" size="sm" class="flex-1" @click="saveArtifactDraft">{{ t('common.saveDraft') }}</UiButton>
                <UiButton variant="primary" size="sm" class="flex-1" @click="requestReview">{{ t('common.requestReview') }}</UiButton>
              </div>
            </div>
          </div>
        </template>

        <div v-else-if="shell.detailFocus === 'knowledge'" class="space-y-3">
          <div v-if="workbench.activeConversationKnowledge.length" class="space-y-2">
            <div
              v-for="entry in workbench.activeConversationKnowledge"
              :key="entry.id"
              class="bg-background rounded-md border border-border-subtle dark:border-white/[0.08] p-3 transition-colors hover:border-border-strong"
            >
              <div class="flex items-center gap-2 mb-2">
                <UiBadge :label="enumLabel('knowledgeStatus', entry.status)" subtle />
                <UiBadge :label="enumLabel('knowledgeSourceType', entry.sourceType)" subtle />
              </div>
              <strong class="block text-[13px] font-bold text-text-primary mb-1">{{ workbench.knowledgeEntryDisplayTitle(entry.id) }}</strong>
              <p class="text-[12px] leading-relaxed text-text-secondary line-clamp-2">{{ workbench.knowledgeEntryDisplaySummary(entry.id) }}</p>
            </div>
          </div>
          <UiEmptyState
            v-else
            :title="t('conversation.detail.knowledge.emptyTitle')"
            :description="t('conversation.detail.knowledge.emptyDescription')"
          />
        </div>

        <div v-else-if="shell.detailFocus === 'resources'" class="space-y-3">
          <div v-if="workbench.activeConversationResources.length" class="space-y-2">
            <div
              v-for="resource in workbench.activeConversationResources"
              :key="resource.id"
              class="flex items-center gap-3 bg-background rounded-md border border-border-subtle dark:border-white/[0.08] p-3"
            >
              <component :is="resource.kind === 'folder' ? FolderTree : FileText" :size="16" class="text-text-tertiary" />
              <div class="min-w-0 flex-1">
                <strong class="block text-[13px] font-bold text-text-primary truncate">{{ resource.name }}</strong>
                <span class="text-[11px] text-text-tertiary">{{ resource.sizeLabel }}</span>
              </div>
            </div>
          </div>
          <UiEmptyState
            v-else
            :title="t('conversation.detail.resources.emptyTitle')"
            :description="t('conversation.detail.resources.emptyDescription')"
          />
        </div>

        <div v-else-if="shell.detailFocus === 'tools'" class="space-y-3">
          <div v-if="workbench.activeConversationToolStats.length" class="space-y-2">
            <div
              v-for="tool in workbench.activeConversationToolStats"
              :key="tool.toolId"
              class="flex items-center justify-between bg-background rounded-md border border-border-subtle dark:border-white/[0.08] p-3"
            >
              <div class="min-w-0">
                <strong class="block text-[13px] font-bold text-text-primary truncate">{{ tool.label }}</strong>
                <span class="text-[11px] text-text-tertiary">{{ tool.kind }}</span>
              </div>
              <UiBadge :label="String(tool.count)" subtle />
            </div>
          </div>
          <UiEmptyState
            v-else
            :title="t('conversation.detail.tools.emptyTitle')"
            :description="t('conversation.detail.tools.emptyDescription')"
          />
        </div>

        <div v-else class="space-y-4">
          <UiTimelineList
            v-if="timelineItems.length"
            test-id="conversation-detail-timeline"
            density="compact"
            :items="timelineItems"
          />
          <UiEmptyState
            v-else
            :title="t('conversation.detail.timeline.emptyTitle')"
            :description="t('conversation.detail.timeline.emptyDescription')"
          />
        </div>
      </div>
    </aside>
  </div>
</template>
