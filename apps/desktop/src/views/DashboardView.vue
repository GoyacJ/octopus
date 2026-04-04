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
import { enumLabel } from '@/i18n/copy'
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
const activeWorkspaceName = computed(() =>
  workbench.activeWorkspace
    ? workbench.workspaceDisplayName(workbench.activeWorkspace.id)
    : t('dashboard.header.titleFallback'),
)
const activeWorkspaceDescription = computed(() =>
  workbench.activeWorkspace
    ? workbench.workspaceDisplayDescription(workbench.activeWorkspace.id)
    : t('dashboard.summary.subtitleFallback'),
)
const activeWorkspaceRoleSummary = computed(() =>
  workbench.activeWorkspace
    ? workbench.workspaceDisplayRoleSummary(workbench.activeWorkspace.id)
    : t('common.na'),
)
const activeWorkspaceMemberCountLabel = computed(() =>
  workbench.activeWorkspace
    ? workbench.workspaceDisplayMemberCountLabel(workbench.activeWorkspace.id)
    : workbench.workspaceDisplayMemberCountLabel(''),
)
const activeProjectSummary = computed(() =>
  workbench.activeProject
    ? workbench.projectDisplaySummary(workbench.activeProject.id)
    : t('dashboard.header.subtitleFallback'),
)
const activeProjectGoal = computed(() =>
  workbench.activeProject
    ? workbench.projectDisplayGoal(workbench.activeProject.id)
    : '',
)
const activeProjectRecentDecision = computed(() =>
  workbench.activeProject
    ? workbench.projectDisplayRecentDecision(workbench.activeProject.id)
    : t('dashboard.project.subtitleFallback'),
)
const activeProjectPhase = computed(() =>
  workbench.activeProject
    ? workbench.projectDisplayPhase(workbench.activeProject.id)
    : t('common.na'),
)
const activeProjectArtifactCountLabel = computed(() =>
  workbench.activeProject
    ? workbench.projectDisplayArtifactCountLabel(workbench.activeProject.id)
    : workbench.projectDisplayArtifactCountLabel(''),
)
const activeProjectConversationCountLabel = computed(() =>
  workbench.activeProject
    ? workbench.projectDisplayConversationCountLabel(workbench.activeProject.id)
    : workbench.projectDisplayConversationCountLabel(''),
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
  <div class="w-full flex flex-col gap-10 pb-12">
    <header class="px-2 shrink-0">
      <UiSectionHeading
        :eyebrow="t('dashboard.header.eyebrow')"
        :title="activeWorkspaceName"
        :subtitle="activeProjectSummary"
      />
    </header>

    <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 gap-4 px-2">
      <UiStatTile
        v-for="metric in workbench.workspaceDashboard.workspaceMetrics"
        :key="metric.label"
        :label="workbench.dashboardMetricLabel(metric)"
        :value="workbench.dashboardMetricValue(metric)"
        :tone="toneForMetric(metric.tone)"
      />
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-8 px-2">
      <UiSurface
        :title="t('dashboard.summary.title')"
        :subtitle="activeWorkspaceDescription"
      >
        <div class="flex flex-wrap gap-2 mb-3">
          <UiBadge
            :label="activeWorkspaceRoleSummary"
            tone="info"
            subtle
          />
          <UiBadge :label="activeWorkspaceMemberCountLabel" subtle />
        </div>
        <p class="text-text-secondary leading-relaxed text-[14px]">
          {{ activeProjectGoal }}
        </p>
        <div class="flex flex-wrap gap-3 mt-6">
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
        :subtitle="activeProjectRecentDecision"
      >
        <div class="flex flex-wrap gap-2 mb-3">
          <UiBadge
            :label="activeProjectPhase"
            tone="info"
            subtle
          />
          <UiBadge :label="activeProjectArtifactCountLabel" subtle />
          <UiBadge :label="activeProjectConversationCountLabel" subtle />
        </div>
        <p class="text-text-secondary leading-relaxed text-[14px]">
          {{ activeProjectSummary }}
        </p>
        <div class="flex flex-wrap gap-3 mt-6">
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

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-8 px-2 border-t border-border-subtle dark:border-white/[0.05] pt-10">
      <section class="space-y-4">
        <h3 class="text-lg font-bold text-text-primary px-1">{{ t('dashboard.highlights.title') }}</h3>
        <div class="grid gap-3 sm:grid-cols-2">
          <RouterLink
            v-for="highlight in workbench.workspaceDashboard.highlights"
            :key="highlight.id"
            :to="highlight.route"
            class="group flex flex-col gap-2 rounded-md border border-primary/50 dark:border-primary/50 bg-subtle/30 p-4 transition-all hover:bg-accent"
          >
            <div class="flex items-center justify-between gap-4">
              <strong class="text-sm font-bold text-text-primary line-clamp-1">{{ workbench.dashboardHighlightTitle(highlight) }}</strong>
              <ArrowRight :size="14" class="text-text-tertiary group-hover:text-primary transition-colors" />
            </div>
            <p class="text-[12px] text-text-secondary leading-relaxed line-clamp-2">{{ workbench.dashboardHighlightDescription(highlight) }}</p>
          </RouterLink>
        </div>
      </section>

      <section class="space-y-4">
        <h3 class="text-lg font-bold text-text-primary px-1">{{ t('dashboard.inbox.title') }}</h3>
        <div v-if="pendingInbox.length" class="flex flex-col gap-3">
          <UiInboxBlock
            v-for="item in pendingInbox"
            :key="item.id"
            :title="workbench.inboxItemDisplayTitle(item.id)"
            :description="workbench.inboxItemDisplayDescription(item.id)"
            :priority-label="enumLabel('riskLevel', item.priority)"
            :status-label="enumLabel('inboxStatus', item.status)"
            :impact="workbench.inboxItemDisplayImpact(item.id)"
            :risk-note="workbench.inboxItemDisplayRiskNote(item.id)"
            :status-heading="t('common.status')"
            :impact-heading="t('common.impact')"
            :risk-heading="t('common.risk')"
          />
        </div>
        <UiEmptyState v-else :title="t('dashboard.inbox.emptyTitle')" :description="t('dashboard.inbox.emptyDescription')" />
      </section>
    </div>

    <section class="space-y-6 px-2 border-t border-border-subtle dark:border-white/[0.05] pt-10">
      <h3 class="text-lg font-bold text-text-primary px-1">{{ t('dashboard.artifacts.title') }}</h3>
      <div v-if="workbench.activeConversationArtifacts.length" class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
        <UiArtifactBlock
          v-for="artifact in workbench.activeConversationArtifacts"
          :key="artifact.id"
          :title="workbench.artifactDisplayTitle(artifact.id)"
          :excerpt="workbench.artifactDisplayExcerpt(artifact.id)"
          :type-label="workbench.artifactDisplayTypeLabel(artifact.id)"
          :version-label="`v${artifact.version}`"
          :status-label="enumLabel('artifactStatus', artifact.status)"
        />
      </div>
      <UiEmptyState v-else :title="t('dashboard.artifacts.emptyTitle')" :description="t('dashboard.artifacts.emptyDescription')" />
    </section>
  </div>
</template>
