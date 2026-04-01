<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import {
  Brain,
  BookOpen,
  FileText,
  FolderTree,
  PanelRightClose,
  Sparkles,
  Waypoints,
  Wrench,
} from 'lucide-vue-next'

import { UiBadge, UiEmptyState, UiSurface, UiTraceBlock } from '@octopus/ui'

import { enumLabel, formatDateTime, resolveCopy, resolveMockField, resolveMockList } from '@/i18n/copy'
import type { ConversationDetailFocus } from '@/stores/shell'
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

watch(
  selectedArtifact,
  (artifact) => {
    artifactDraft.value = artifact ? resolveMockField('artifact', artifact.id, 'content', artifact.content) : ''
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
  if (!selectedArtifact.value) {
    return
  }

  workbench.updateArtifactContent(selectedArtifact.value.id, artifactDraft.value)
}

function requestReview() {
  if (!selectedArtifact.value) {
    return
  }

  workbench.requestArtifactReview(selectedArtifact.value.id)
  shell.setDetailFocus('timeline')
  shell.setRightSidebarCollapsed(false)
  updateQuery('timeline', selectedArtifact.value.id)
}
</script>

<template>
  <aside
    v-if="shell.rightSidebarCollapsed"
    class="detail-rail"
    data-testid="conversation-detail-rail"
  >
    <button
      type="button"
      class="detail-rail-toggle"
      data-testid="conversation-detail-rail-toggle"
      :title="t('conversation.detail.actions.expand')"
      @click="shell.toggleRightSidebar()"
    >
      <PanelRightClose :size="18" />
    </button>
    <div class="detail-rail-divider" aria-hidden="true" />
    <button
      v-for="section in sectionItems"
      :key="section.id"
      type="button"
      class="detail-rail-link"
      :data-testid="`conversation-detail-rail-section-${section.id}`"
      :class="{ active: shell.detailFocus === section.id }"
      :title="section.label"
      @click="setDetail(section.id)"
    >
      <component :is="section.icon" :size="18" />
    </button>
  </aside>

  <aside v-else class="detail-panel" data-testid="conversation-detail-panel">
    <div class="detail-toolbar">
      <strong>{{ t('conversation.detail.title') }}</strong>
      <button
        type="button"
        class="detail-rail-toggle"
        data-testid="conversation-detail-collapse"
        :title="t('conversation.detail.actions.collapse')"
        @click="shell.toggleRightSidebar()"
      >
        <PanelRightClose :size="18" />
      </button>
    </div>

    <div class="detail-section-nav" role="tablist">
      <button
        v-for="section in sectionItems"
        :key="section.id"
        type="button"
        class="detail-section-button"
        :data-testid="`conversation-detail-section-${section.id}`"
        :class="{ active: shell.detailFocus === section.id }"
        @click="setDetail(section.id)"
      >
        <component :is="section.icon" :size="16" />
        <span>{{ section.label }}</span>
      </button>
    </div>

    <div class="detail-content scroll-y">
      <UiSurface
        v-if="shell.detailFocus === 'summary' && activeConversation"
        class="detail-surface"
        :title="t('conversation.detail.summary.title')"
        :subtitle="t('conversation.detail.summary.subtitle')"
      >
        <div class="detail-summary-grid">
          <div class="detail-copy">
            <strong>{{ t('common.goal') }}</strong>
            <p>{{ resolveMockField('conversation', activeConversation.id, 'currentGoal', activeConversation.currentGoal) }}</p>
          </div>
          <div class="detail-copy">
            <strong>{{ t('conversation.detail.summary.statusNote') }}</strong>
            <p>{{ resolveCopy(activeConversation.statusNote) }}</p>
          </div>
        </div>
        <div class="detail-copy">
          <strong>{{ t('common.constraints') }}</strong>
          <ul>
            <li
              v-for="(constraint, index) in resolveMockList('conversation', activeConversation.id, 'constraints', activeConversation.constraints)"
              :key="`${activeConversation.id}-constraint-${index}`"
            >
              {{ constraint }}
            </li>
          </ul>
        </div>
        <div class="detail-summary-grid">
          <div class="detail-copy">
            <strong>{{ t('conversation.detail.summary.currentStep') }}</strong>
            <p>{{ resolveCopy(workbench.activeRun?.currentStep) }}</p>
          </div>
          <div class="detail-copy">
            <strong>{{ t('conversation.detail.summary.updatedAt') }}</strong>
            <p>{{ formatDateTime(workbench.activeRun?.updatedAt) }}</p>
          </div>
        </div>
      </UiSurface>

      <UiSurface
        v-else-if="shell.detailFocus === 'memories'"
        class="detail-surface"
        :title="t('conversation.detail.memories.title')"
        :subtitle="t('conversation.detail.memories.subtitle')"
      >
        <div v-if="workbench.activeConversationMemories.length" class="panel-list">
          <article
            v-for="memory in workbench.activeConversationMemories"
            :key="memory.id"
            class="panel-card"
          >
            <div class="meta-row">
              <UiBadge :label="memory.source === 'agent' ? t('conversation.detail.memories.agentSource') : t('conversation.detail.memories.conversationSource')" subtle />
              <UiBadge v-if="memory.ownerId" :label="memory.ownerId" subtle />
            </div>
            <strong>{{ memory.title }}</strong>
            <p>{{ memory.summary }}</p>
            <small>{{ formatDateTime(memory.createdAt) }}</small>
          </article>
        </div>
        <UiEmptyState
          v-else
          :title="t('conversation.detail.memories.emptyTitle')"
          :description="t('conversation.detail.memories.emptyDescription')"
        />
      </UiSurface>

      <template v-else-if="shell.detailFocus === 'artifacts'">
        <UiSurface class="detail-surface" :title="t('conversation.detail.artifacts.title')" :subtitle="t('conversation.detail.artifacts.subtitle')">
          <div v-if="workbench.activeConversationArtifacts.length" class="resource-list">
            <button
              v-for="artifact in workbench.activeConversationArtifacts"
              :key="artifact.id"
              type="button"
              class="resource-card"
              :class="{ active: shell.selectedArtifactId === artifact.id }"
              @click="openArtifact(artifact.id)"
            >
              <div class="resource-copy">
                <strong>{{ resolveMockField('artifact', artifact.id, 'title', artifact.title) }}</strong>
                <small>{{ resolveMockField('artifact', artifact.id, 'excerpt', artifact.excerpt) }}</small>
              </div>
              <UiBadge :label="enumLabel('artifactStatus', artifact.status)" subtle />
            </button>
          </div>
          <UiEmptyState
            v-else
            :title="t('conversation.detail.artifacts.emptyTitle')"
            :description="t('conversation.detail.artifacts.emptyDescription')"
          />
        </UiSurface>

        <UiSurface
          v-if="selectedArtifact"
          class="detail-surface"
          :title="resolveMockField('artifact', selectedArtifact.id, 'title', selectedArtifact.title)"
          :subtitle="t('common.updatedAt', { time: formatDateTime(selectedArtifact.updatedAt) })"
        >
          <div class="meta-row">
            <UiBadge :label="resolveMockField('artifact', selectedArtifact.id, 'type', selectedArtifact.type)" subtle />
            <UiBadge :label="`v${selectedArtifact.version}`" subtle />
          </div>
          <textarea v-model="artifactDraft" rows="10" />
          <div class="action-row">
            <button type="button" class="secondary-button" @click="saveArtifactDraft">{{ t('common.saveDraft') }}</button>
            <button type="button" class="primary-button" @click="requestReview">{{ t('common.requestReview') }}</button>
          </div>
        </UiSurface>
      </template>

      <UiSurface
        v-else-if="shell.detailFocus === 'knowledge'"
        class="detail-surface"
        :title="t('conversation.detail.knowledge.title')"
        :subtitle="t('conversation.detail.knowledge.subtitle')"
      >
        <div v-if="workbench.activeConversationKnowledge.length" class="panel-list">
          <article
            v-for="entry in workbench.activeConversationKnowledge"
            :key="entry.id"
            class="panel-card"
          >
            <div class="meta-row">
              <UiBadge :label="enumLabel('knowledgeStatus', entry.status)" subtle />
              <UiBadge :label="enumLabel('knowledgeSourceType', entry.sourceType)" subtle />
            </div>
            <strong>{{ resolveMockField('knowledgeEntry', entry.id, 'title', entry.title) }}</strong>
            <p>{{ resolveMockField('knowledgeEntry', entry.id, 'summary', entry.summary) }}</p>
          </article>
        </div>
        <UiEmptyState
          v-else
          :title="t('conversation.detail.knowledge.emptyTitle')"
          :description="t('conversation.detail.knowledge.emptyDescription')"
        />
      </UiSurface>

      <UiSurface
        v-else-if="shell.detailFocus === 'resources'"
        class="detail-surface"
        :title="t('conversation.detail.resources.title')"
        :subtitle="t('conversation.detail.resources.subtitle')"
      >
        <div v-if="workbench.activeConversationResources.length" class="panel-list">
          <article
            v-for="resource in workbench.activeConversationResources"
            :key="resource.id"
            class="panel-card"
          >
            <div class="meta-row">
              <UiBadge :label="resource.kind" subtle />
              <UiBadge :label="resource.sizeLabel ?? t('common.na')" subtle />
            </div>
            <strong>{{ resource.name }}</strong>
            <p>{{ resource.location ?? t('common.na') }}</p>
          </article>
        </div>
        <UiEmptyState
          v-else
          :title="t('conversation.detail.resources.emptyTitle')"
          :description="t('conversation.detail.resources.emptyDescription')"
        />
      </UiSurface>

      <UiSurface
        v-else-if="shell.detailFocus === 'tools'"
        class="detail-surface"
        :title="t('conversation.detail.tools.title')"
        :subtitle="t('conversation.detail.tools.subtitle')"
      >
        <div v-if="workbench.activeConversationToolStats.length" class="panel-list">
          <article
            v-for="tool in workbench.activeConversationToolStats"
            :key="tool.toolId"
            class="panel-card"
          >
            <div class="meta-row">
              <UiBadge :label="tool.kind" subtle />
              <UiBadge :label="t('conversation.detail.tools.callCount', { count: tool.count })" subtle />
            </div>
            <strong>{{ tool.label }}</strong>
            <p>{{ tool.toolId }}</p>
          </article>
        </div>
        <UiEmptyState
          v-else
          :title="t('conversation.detail.tools.emptyTitle')"
          :description="t('conversation.detail.tools.emptyDescription')"
        />
      </UiSurface>

      <UiSurface
        v-else
        class="detail-surface"
        :title="t('conversation.detail.timeline.title')"
        :subtitle="t('conversation.detail.timeline.subtitle')"
      >
        <div v-if="workbench.activeConversationTimeline.length" class="panel-list">
          <UiTraceBlock
            v-for="trace in workbench.activeConversationTimeline"
            :key="trace.id"
            :title="resolveMockField('traceRecord', trace.id, 'title', trace.title)"
            :detail="resolveMockField('traceRecord', trace.id, 'detail', trace.detail)"
            :actor="trace.actor"
            :timestamp-label="formatDateTime(trace.timestamp)"
            :tone="trace.status"
          />
        </div>
        <UiEmptyState
          v-else
          :title="t('conversation.detail.timeline.emptyTitle')"
          :description="t('conversation.detail.timeline.emptyDescription')"
        />
      </UiSurface>
    </div>
  </aside>
</template>

<style scoped>
.detail-panel,
.detail-copy,
.resource-copy {
  display: flex;
  flex-direction: column;
}

.detail-rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.35rem;
  min-height: 0;
  height: 100%;
  max-height: 100%;
  width: 100%;
  padding: 0.15rem 0 0;
  border-left: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: transparent;
}

.detail-panel {
  display: grid;
  gap: 0.85rem;
  grid-template-rows: auto auto minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  height: 100%;
  max-height: 100%;
  max-width: 100%;
  padding: 0.15rem 0 0 0.95rem;
  border-left: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  overflow-x: hidden;
}

.detail-panel > * {
  min-width: 0;
}

.detail-content {
  min-height: 0;
  padding-right: 0.35rem;
  padding-bottom: 0.25rem;
  overflow-x: hidden;
  overscroll-behavior: contain;
}

.detail-content > * + * {
  margin-top: 0.85rem;
}

.detail-toolbar,
.detail-rail-toggle,
.detail-rail-link,
.detail-section-button,
.meta-row,
.detail-summary-grid,
.action-row,
.resource-card {
  display: flex;
}

.detail-toolbar,
.meta-row,
.action-row {
  align-items: center;
}

.meta-row,
.action-row {
  flex-wrap: wrap;
}

.detail-toolbar {
  justify-content: space-between;
  gap: 0.75rem;
  padding-bottom: 0.65rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
}

.detail-rail-toggle,
.detail-rail-link,
.detail-section-button {
  justify-content: center;
  border-radius: 0.78rem;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-secondary);
}

.detail-rail-toggle,
.detail-rail-link {
  width: 2.25rem;
  height: 2.25rem;
  align-items: center;
}

.detail-section-button {
  align-items: center;
  gap: 0.45rem;
  padding: 0.55rem 0.75rem;
}

.detail-rail-link.active,
.detail-section-button.active,
.resource-card.active {
  border-color: color-mix(in srgb, var(--brand-primary) 48%, var(--border-subtle));
  background: color-mix(in srgb, var(--brand-primary) 12%, var(--bg-surface));
  color: var(--text-primary);
}

.detail-rail-divider {
  width: 1px;
  height: 0.75rem;
  background: color-mix(in srgb, var(--border-subtle) 92%, transparent);
  margin: 0.1rem 0 0.2rem;
}

.detail-section-nav {
  display: flex;
  gap: 0.45rem;
  flex-wrap: wrap;
  min-width: 0;
}

.detail-summary-grid {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.detail-copy {
  gap: 0.25rem;
  min-width: 0;
}

.detail-copy p,
.detail-copy li,
.detail-copy strong,
.panel-card p,
.panel-card strong,
.panel-card small,
.resource-copy strong,
.resource-copy small {
  color: var(--text-secondary);
  line-height: 1.55;
  overflow-wrap: anywhere;
}

.detail-copy strong,
.panel-card strong,
.resource-copy strong {
  color: var(--text-primary);
}

.panel-list,
.resource-list {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.panel-card,
.resource-card {
  min-width: 0;
  padding: 0.9rem;
  border-radius: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.panel-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.resource-card {
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  text-align: left;
}

textarea {
  width: 100%;
  min-width: 0;
  max-width: 100%;
  border-radius: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 72%, transparent);
  color: var(--text-primary);
  padding: 0.85rem 0.95rem;
  resize: vertical;
}

ul {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding-left: 1rem;
}

@media (max-width: 960px) {
  .detail-summary-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
