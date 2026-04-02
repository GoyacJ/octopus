<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'
import { 
  UiArtifactBlock, 
  UiBadge, 
  UiEmptyState, 
  UiInboxBlock, 
  UiSectionHeading, 
  UiStatTile, 
  UiSurface,
  UiButton
} from '@octopus/ui'
import { ArrowRight, MessageSquare, Library, Activity } from 'lucide-vue-next'
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
  <div class="mx-auto flex max-w-[1400px] flex-col gap-8 pb-12">
    <UiSectionHeading
      :eyebrow="t('dashboard.header.eyebrow')"
      :title="workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name) : t('dashboard.header.titleFallback')"
      :subtitle="workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'summary', workbench.activeProject.summary) : t('dashboard.header.subtitleFallback')"
    />

    <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
      <UiStatTile
        v-for="metric in workbench.workspaceDashboard.workspaceMetrics"
        :key="metric.label"
        :label="resolveCopy(metric.label)"
        :value="resolveCopy(metric.value)"
        :tone="toneForMetric(metric.tone)"
      />
    </div>

    <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
      <UiSurface
        :title="t('dashboard.summary.title')"
        :subtitle="workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'description', workbench.activeWorkspace.description) : t('dashboard.summary.subtitleFallback')"
      >
        <div class="flex flex-wrap gap-2 mb-2">
          <UiBadge
            :label="workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'roleSummary', workbench.activeWorkspace.roleSummary) : t('common.na')"
            tone="info"
          />
          <UiBadge :label="countLabel('common.members', workbench.activeWorkspace?.memberCount ?? 0)" subtle />
        </div>
        <p class="text-text-secondary leading-relaxed text-sm">
          {{ workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'goal', workbench.activeProject.goal) : '' }}
        </p>
        <div class="flex flex-wrap gap-3 mt-4">
          <UiButton 
            variant="secondary" 
            size="sm"
            as="RouterLink"
            :to="{ name: 'agents', params: { workspaceId: workbench.currentWorkspaceId } }"
          >
            {{ t('dashboard.summary.openAgentCenter') }}
          </UiButton>
          <UiButton 
            variant="ghost" 
            size="sm"
            as="RouterLink"
            :to="{ name: 'agents', params: { workspaceId: workbench.currentWorkspaceId }, query: { kind: 'team' } }"
          >
            {{ t('dashboard.summary.openTeamCenter') }}
          </UiButton>
        </div>
      </UiSurface>

      <UiSurface
        :title="t('dashboard.project.title')"
        :subtitle="workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'recentDecision', workbench.activeProject.recentDecision) : t('dashboard.project.subtitleFallback')"
      >
        <div class="flex flex-wrap gap-2 mb-2">
          <UiBadge
            :label="workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'phase', workbench.activeProject.phase) : t('common.na')"
            tone="info"
          />
          <UiBadge :label="countLabel('common.artifacts', workbench.activeProject?.artifactIds.length ?? 0)" subtle />
          <UiBadge :label="countLabel('common.conversations', workbench.activeProject?.conversationIds.length ?? 0)" subtle />
        </div>
        <p class="text-text-secondary leading-relaxed text-sm">
          {{ workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'summary', workbench.activeProject.summary) : '' }}
        </p>
        <div class="flex flex-wrap gap-3 mt-4">
          <UiButton 
            size="sm"
            as="RouterLink"
            :to="conversationTarget"
          >
            <MessageSquare :size="16" />
            {{ t('dashboard.project.openConversation') }}
          </UiButton>
          <UiButton 
            variant="ghost" 
            size="sm"
            as="RouterLink"
            :to="{ name: 'knowledge', params: { workspaceId: workbench.currentWorkspaceId, projectId: workbench.currentProjectId } }"
          >
            <Library :size="16" />
            {{ t('dashboard.project.knowledge') }}
          </UiButton>
          <UiButton 
            variant="ghost" 
            size="sm"
            as="RouterLink"
            :to="{ name: 'trace', params: { workspaceId: workbench.currentWorkspaceId, projectId: workbench.currentProjectId } }"
          >
            <Activity :size="16" />
            {{ t('dashboard.project.trace') }}
          </UiButton>
        </div>
      </UiSurface>
    </div>

    <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
      <UiSurface :title="t('dashboard.highlights.title')" :subtitle="t('dashboard.highlights.subtitle')">
        <div class="flex flex-col gap-3">
          <RouterLink
            v-for="highlight in workbench.workspaceDashboard.highlights"
            :key="highlight.id"
            :to="highlight.route"
            class="group flex flex-col gap-2 rounded-[calc(var(--radius-lg)+1px)] border border-border bg-subtle/35 p-4 transition-all duration-fast ease-apple hover:border-primary/20 hover:bg-surface hover:shadow-sm"
          >
            <div class="flex items-center justify-between gap-4">
              <strong class="text-sm font-semibold text-text-primary line-clamp-1">{{ resolveCopy(highlight.title) }}</strong>
              <ArrowRight :size="14" class="text-text-tertiary group-hover:text-primary transition-colors" />
            </div>
            <p class="text-xs text-text-secondary leading-relaxed line-clamp-2">{{ resolveCopy(highlight.description) }}</p>
          </RouterLink>
        </div>
      </UiSurface>

      <UiSurface :title="t('dashboard.inbox.title')" :subtitle="t('dashboard.inbox.subtitle')">
        <div v-if="pendingInbox.length" class="flex flex-col gap-3">
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
      <div v-if="workbench.activeConversationArtifacts.length" class="grid grid-cols-1 md:grid-cols-2 gap-5">
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
  </div>
</template>