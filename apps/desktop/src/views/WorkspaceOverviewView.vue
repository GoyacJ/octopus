<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

import { UiEmptyState, UiSectionHeading, UiStatTile, UiSurface } from '@octopus/ui'

import { createProjectConversationTarget, createProjectDashboardTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const snapshot = computed(() => workbench.workspaceOverview)
const currentProject = computed(() =>
  snapshot.value.projectId
    ? workbench.projects.find((project) => project.id === snapshot.value.projectId)
    : undefined,
)
const currentConversationId = computed(() => currentProject.value?.conversationIds[0])
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('overview.header.eyebrow')"
      :title="workbench.activeWorkspace?.name ?? t('overview.header.titleFallback')"
      :subtitle="workbench.activeWorkspace?.description ?? t('overview.header.subtitleFallback')"
    />

    <UiSurface :title="t('overview.sections.user.title')" :subtitle="t('overview.sections.user.subtitle')">
      <div class="stat-grid triple">
        <UiStatTile
          v-for="metric in snapshot.userMetrics"
          :key="metric.label"
          :label="t(metric.label)"
          :value="metric.value"
        />
      </div>

      <div class="surface-subsection">
        <div class="subsection-header">
          <strong>{{ t('overview.sections.user.activity') }}</strong>
        </div>
        <ul v-if="snapshot.userActivity.length" class="rank-list">
          <li v-for="activity in snapshot.userActivity" :key="activity.id" class="rank-list-item">
            <div class="rank-copy">
              <strong>{{ activity.title }}</strong>
              <small>{{ activity.description }}</small>
            </div>
            <span class="rank-value">{{ new Date(activity.timestamp).toLocaleString() }}</span>
          </li>
        </ul>
        <UiEmptyState
          v-else
          :title="t('overview.empty.activityTitle')"
          :description="t('overview.empty.activityDescription')"
        />
      </div>
    </UiSurface>

    <UiSurface :title="t('overview.sections.project.title')" :subtitle="t('overview.sections.project.subtitle')">
      <div v-if="currentProject" class="surface-subsection">
        <div class="surface-actions">
          <RouterLink class="ghost-button" :to="createProjectDashboardTarget(currentProject.workspaceId, currentProject.id)">
            {{ t('overview.sections.project.openDashboard') }}
          </RouterLink>
          <RouterLink
            class="primary-button"
            :to="createProjectConversationTarget(currentProject.workspaceId, currentProject.id, currentConversationId)"
          >
            {{ t('overview.sections.project.openConversation') }}
          </RouterLink>
        </div>

        <div class="stat-grid triple">
          <UiStatTile
            v-for="metric in snapshot.projectSummary.metrics"
            :key="metric.label"
            :label="t(metric.label)"
            :value="metric.value"
          />
        </div>
      </div>

      <UiEmptyState
        v-else
        :title="t('overview.empty.projectTitle')"
        :description="t('overview.empty.projectDescription')"
      />

      <div class="surface-grid two">
        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('overview.sections.project.activity') }}</strong>
          </div>
          <ul v-if="snapshot.projectSummary.activity.length" class="rank-list">
            <li v-for="activity in snapshot.projectSummary.activity" :key="activity.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ activity.title }}</strong>
                <small>{{ activity.description }}</small>
              </div>
              <span class="rank-value">{{ new Date(activity.timestamp).toLocaleString() }}</span>
            </li>
          </ul>
          <UiEmptyState
            v-else
            :title="t('overview.empty.activityTitle')"
            :description="t('overview.empty.activityDescription')"
          />
        </div>

        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('overview.sections.project.conversationTop') }}</strong>
          </div>
          <ul v-if="snapshot.projectSummary.conversationTokenTop.length" class="rank-list">
            <li v-for="item in snapshot.projectSummary.conversationTokenTop" :key="item.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ item.label }}</strong>
                <small>{{ item.secondary }}</small>
              </div>
              <span class="rank-value">{{ item.value }}</span>
            </li>
          </ul>
          <UiEmptyState
            v-else
            :title="t('overview.empty.rankingTitle')"
            :description="t('overview.empty.rankingDescription')"
          />
        </div>
      </div>
    </UiSurface>

    <UiSurface :title="t('overview.sections.workspace.title')" :subtitle="t('overview.sections.workspace.subtitle')">
      <div class="stat-grid quad">
        <UiStatTile
          v-for="metric in snapshot.workspaceMetrics"
          :key="metric.label"
          :label="t(metric.label)"
          :value="metric.value"
        />
      </div>

      <div class="surface-grid two">
        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('overview.sections.workspace.projectRanking') }}</strong>
          </div>
          <ul class="rank-list">
            <li v-for="item in snapshot.projectTokenTop" :key="item.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ item.label }}</strong>
                <small>{{ item.secondary }}</small>
              </div>
              <span class="rank-value">{{ item.value }}</span>
            </li>
          </ul>
        </div>

        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('overview.sections.workspace.agentRanking') }}</strong>
          </div>
          <ul class="rank-list">
            <li v-for="item in snapshot.agentUsage" :key="item.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ item.label }}</strong>
                <small>{{ item.secondary }}</small>
              </div>
              <span class="rank-value">{{ item.value }}</span>
            </li>
          </ul>
        </div>

        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('overview.sections.workspace.teamRanking') }}</strong>
          </div>
          <ul class="rank-list">
            <li v-for="item in snapshot.teamUsage" :key="item.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ item.label }}</strong>
                <small>{{ item.secondary }}</small>
              </div>
              <span class="rank-value">{{ item.value }}</span>
            </li>
          </ul>
        </div>

        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('overview.sections.workspace.toolRanking') }}</strong>
          </div>
          <ul class="rank-list">
            <li v-for="item in snapshot.toolUsage" :key="item.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ item.label }}</strong>
              </div>
              <span class="rank-value">{{ item.value }}</span>
            </li>
          </ul>
        </div>

        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('overview.sections.workspace.modelRanking') }}</strong>
          </div>
          <ul class="rank-list">
            <li v-for="item in snapshot.modelUsage" :key="item.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ item.label }}</strong>
                <small>{{ item.secondary }}</small>
              </div>
              <span class="rank-value">{{ item.value }}</span>
            </li>
          </ul>
        </div>
      </div>
    </UiSurface>
  </section>
</template>

<style scoped>
.surface-subsection {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.surface-subsection + .surface-subsection {
  margin-top: 1rem;
}

.surface-actions,
.subsection-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.stat-grid {
  display: grid;
  gap: 0.9rem;
}

.stat-grid.triple {
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
}

.stat-grid.quad {
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
}

.rank-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.rank-list-item {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem 0.95rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-l);
  background: color-mix(in srgb, var(--bg-subtle) 66%, transparent);
}

.rank-copy {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.rank-copy small {
  color: var(--text-secondary);
  line-height: 1.5;
}

.rank-value {
  color: var(--text-secondary);
  font-size: 0.85rem;
  white-space: nowrap;
}
</style>
