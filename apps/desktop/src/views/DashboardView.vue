<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

import { UiArtifactBlock, UiBadge, UiEmptyState, UiInboxBlock, UiSectionHeading, UiStatTile, UiSurface } from '@octopus/ui'

import { countLabel, enumLabel, resolveCopy, resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()
const conversationTarget = computed(() =>
  createProjectConversationTarget(
    workbench.currentWorkspaceId,
    workbench.currentProjectId,
    workbench.currentConversationId,
  ),
)

const pendingInbox = computed(() => workbench.workspaceInbox.filter((item) => item.status === 'pending'))

function toneForMetric(tone?: string): 'default' | 'success' | 'warning' | 'error' | 'info' {
  if (tone === 'success' || tone === 'warning' || tone === 'error' || tone === 'info') {
    return tone
  }

  return 'default'
}
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('dashboard.header.eyebrow')"
      :title="workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name) : t('dashboard.header.titleFallback')"
      :subtitle="workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'summary', workbench.activeProject.summary) : t('dashboard.header.subtitleFallback')"
    />

    <div class="surface-grid three">
      <UiStatTile
        v-for="metric in workbench.workspaceDashboard.workspaceMetrics"
        :key="metric.label"
        :label="resolveCopy(metric.label)"
        :value="resolveCopy(metric.value)"
        :tone="toneForMetric(metric.tone)"
      />
    </div>

    <div class="surface-grid two">
      <UiSurface
        :title="t('dashboard.summary.title')"
        :subtitle="workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'description', workbench.activeWorkspace.description) : t('dashboard.summary.subtitleFallback')"
      >
        <div class="meta-row">
          <UiBadge
            :label="workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'roleSummary', workbench.activeWorkspace.roleSummary) : t('common.na')"
            tone="info"
          />
          <UiBadge :label="countLabel('common.members', workbench.activeWorkspace?.memberCount ?? 0)" subtle />
        </div>
        <p class="summary-copy">
          {{ workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'goal', workbench.activeProject.goal) : '' }}
        </p>
        <div class="action-row">
          <RouterLink :to="{ name: 'agents', params: { workspaceId: workbench.currentWorkspaceId } }" class="secondary-button">
            {{ t('dashboard.summary.openAgentCenter') }}
          </RouterLink>
          <RouterLink :to="{ name: 'agents', params: { workspaceId: workbench.currentWorkspaceId }, query: { kind: 'team' } }" class="ghost-button">
            {{ t('dashboard.summary.openTeamCenter') }}
          </RouterLink>
        </div>
      </UiSurface>

      <UiSurface
        :title="t('dashboard.project.title')"
        :subtitle="workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'recentDecision', workbench.activeProject.recentDecision) : t('dashboard.project.subtitleFallback')"
      >
        <div class="meta-row">
          <UiBadge
            :label="workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'phase', workbench.activeProject.phase) : t('common.na')"
            tone="info"
          />
          <UiBadge :label="countLabel('common.artifacts', workbench.activeProject?.artifactIds.length ?? 0)" subtle />
          <UiBadge :label="countLabel('common.conversations', workbench.activeProject?.conversationIds.length ?? 0)" subtle />
        </div>
        <p class="summary-copy">{{ workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'summary', workbench.activeProject.summary) : '' }}</p>
        <div class="action-row">
          <RouterLink
            class="primary-button"
            :to="conversationTarget"
          >
            {{ t('dashboard.project.openConversation') }}
          </RouterLink>
          <RouterLink
            class="ghost-button"
            :to="{ name: 'knowledge', params: { workspaceId: workbench.currentWorkspaceId, projectId: workbench.currentProjectId } }"
          >
            {{ t('dashboard.project.knowledge') }}
          </RouterLink>
          <RouterLink
            class="ghost-button"
            :to="{ name: 'trace', params: { workspaceId: workbench.currentWorkspaceId, projectId: workbench.currentProjectId } }"
          >
            {{ t('dashboard.project.trace') }}
          </RouterLink>
        </div>
      </UiSurface>
    </div>

    <div class="surface-grid two">
      <UiSurface :title="t('dashboard.highlights.title')" :subtitle="t('dashboard.highlights.subtitle')">
        <div class="panel-list">
          <RouterLink
            v-for="highlight in workbench.workspaceDashboard.highlights"
            :key="highlight.id"
            :to="highlight.route"
            class="highlight-link"
          >
            <strong>{{ resolveCopy(highlight.title) }}</strong>
            <p>{{ resolveCopy(highlight.description) }}</p>
          </RouterLink>
        </div>
      </UiSurface>

      <UiSurface :title="t('dashboard.inbox.title')" :subtitle="t('dashboard.inbox.subtitle')">
        <div v-if="pendingInbox.length" class="panel-list">
          <UiInboxBlock
            v-for="item in pendingInbox"
            :key="item.id"
            :title="resolveMockField('inboxItem', item.id, 'title', resolveCopy(item.title))"
            :description="resolveMockField('inboxItem', item.id, 'description', resolveCopy(item.description))"
            :priority-label="enumLabel('riskLevel', item.priority)"
            :status-label="enumLabel('inboxStatus', item.status)"
            :impact="resolveMockField('inboxItem', item.id, 'impact', resolveCopy(item.impact))"
            :risk-note="resolveMockField('inboxItem', item.id, 'riskNote', resolveCopy(item.riskNote))"
            :status-heading="t('common.status')"
            :impact-heading="t('common.impact')"
            :risk-heading="t('common.risk')"
          />
        </div>
        <UiEmptyState v-else :title="t('dashboard.inbox.emptyTitle')" :description="t('dashboard.inbox.emptyDescription')" />
      </UiSurface>
    </div>

    <UiSurface :title="t('dashboard.artifacts.title')" :subtitle="t('dashboard.artifacts.subtitle')">
      <div v-if="workbench.activeConversationArtifacts.length" class="surface-grid two">
        <UiArtifactBlock
          v-for="artifact in workbench.activeConversationArtifacts"
          :key="artifact.id"
          :title="resolveMockField('artifact', artifact.id, 'title', artifact.title)"
          :excerpt="resolveMockField('artifact', artifact.id, 'excerpt', artifact.excerpt)"
          :type-label="resolveMockField('artifact', artifact.id, 'type', artifact.type)"
          :version-label="`v${artifact.version}`"
          :status-label="enumLabel('artifactStatus', artifact.status)"
        />
      </div>
      <UiEmptyState v-else :title="t('dashboard.artifacts.emptyTitle')" :description="t('dashboard.artifacts.emptyDescription')" />
    </UiSurface>
  </section>
</template>

<style scoped>
.summary-copy,
.highlight-link p {
  color: var(--text-secondary);
  line-height: 1.6;
}

.highlight-link {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
  padding: 0.95rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.highlight-link strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}
</style>
