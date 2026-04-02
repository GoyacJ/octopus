<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiField, UiSectionHeading, UiStatTile, UiSurface } from '@octopus/ui'

import { enumLabel, resolveCopy } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const snapshot = computed(() => workbench.projectDashboard)
const project = computed(() => snapshot.value.project)
const editing = ref(false)
const draft = reactive({
  name: '',
  goal: '',
  phase: '',
  summary: '',
})

watch(
  project,
  (value) => {
    if (!value || editing.value) {
      return
    }

    draft.name = value.name
    draft.goal = value.goal
    draft.phase = value.phase
    draft.summary = value.summary
  },
  { immediate: true },
)

function startEdit() {
  draft.name = project.value.name
  draft.goal = project.value.goal
  draft.phase = project.value.phase
  draft.summary = project.value.summary
  editing.value = true
}

function cancelEdit() {
  editing.value = false
  draft.name = project.value.name
  draft.goal = project.value.goal
  draft.phase = project.value.phase
  draft.summary = project.value.summary
}

function saveProject() {
  workbench.updateProjectDetails(project.value.id, draft)
  editing.value = false
}

const progressTone = computed(() => {
  if (snapshot.value.progress.blockerCount > 0) {
    return 'warning'
  }
  if (snapshot.value.progress.runStatus === 'completed') {
    return 'success'
  }
  return 'info'
})
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('projectDashboard.header.eyebrow')"
      :title="project.name"
      :subtitle="project.goal"
    />

    <UiSurface :title="t('projectDashboard.info.title')" :subtitle="t('projectDashboard.info.subtitle')">
      <div class="header-meta">
        <UiBadge :label="enumLabel('projectStatus', project.status)" :tone="project.status === 'active' ? 'success' : 'default'" />
        <UiBadge :label="project.phase" subtle />
      </div>

      <div v-if="editing" class="field-stack">
        <UiField :label="t('projectDashboard.fields.name')">
          <input v-model="draft.name" />
        </UiField>
        <UiField :label="t('projectDashboard.fields.goal')">
          <textarea v-model="draft.goal" rows="3" />
        </UiField>
        <UiField :label="t('projectDashboard.fields.phase')">
          <input v-model="draft.phase" />
        </UiField>
        <UiField :label="t('projectDashboard.fields.summary')">
          <textarea v-model="draft.summary" rows="4" />
        </UiField>
      </div>

      <div v-else class="project-info-grid">
        <div class="project-info-item">
          <span>{{ t('projectDashboard.fields.name') }}</span>
          <strong>{{ project.name }}</strong>
        </div>
        <div class="project-info-item">
          <span>{{ t('projectDashboard.fields.status') }}</span>
          <strong>{{ enumLabel('projectStatus', project.status) }}</strong>
        </div>
        <div class="project-info-item">
          <span>{{ t('projectDashboard.fields.phase') }}</span>
          <strong>{{ project.phase }}</strong>
        </div>
        <div class="project-info-item full">
          <span>{{ t('projectDashboard.fields.goal') }}</span>
          <strong>{{ project.goal }}</strong>
        </div>
        <div class="project-info-item full">
          <span>{{ t('projectDashboard.fields.summary') }}</span>
          <p>{{ project.summary }}</p>
        </div>
      </div>

      <div class="surface-actions">
        <button v-if="!editing" type="button" class="ghost-button" @click="startEdit">
          {{ t('common.edit') }}
        </button>
        <template v-else>
          <button type="button" class="ghost-button" @click="cancelEdit">
            {{ t('common.cancel') }}
          </button>
          <button type="button" class="primary-button" @click="saveProject">
            {{ t('common.save') }}
          </button>
        </template>
      </div>
    </UiSurface>

    <div class="surface-grid two">
      <UiSurface :title="t('projectDashboard.resources.title')" :subtitle="t('projectDashboard.resources.subtitle')">
        <div class="stat-grid triple">
          <UiStatTile
            v-for="metric in snapshot.resourceMetrics"
            :key="metric.label"
            :label="t(metric.label)"
            :value="metric.value"
          />
        </div>
      </UiSurface>

      <UiSurface :title="t('projectDashboard.progress.title')" :subtitle="t('projectDashboard.progress.subtitle')">
        <div class="stat-grid triple">
          <UiStatTile :label="t('projectDashboard.progress.phase')" :value="snapshot.progress.phase" />
          <UiStatTile :label="t('projectDashboard.progress.progress')" :value="`${snapshot.progress.progress}%`" :tone="progressTone" />
          <UiStatTile
            :label="t('projectDashboard.progress.runStatus')"
            :value="snapshot.progress.runStatus ? enumLabel('runStatus', snapshot.progress.runStatus) : t('common.na')"
          />
          <UiStatTile
            :label="t('projectDashboard.progress.currentStep')"
            :value="snapshot.progress.currentStep ? resolveCopy(snapshot.progress.currentStep) : t('common.na')"
          />
          <UiStatTile :label="t('projectDashboard.progress.blockers')" :value="String(snapshot.progress.blockerCount)" />
          <UiStatTile :label="t('projectDashboard.progress.pendingInbox')" :value="String(snapshot.progress.pendingInboxCount)" />
        </div>
      </UiSurface>
    </div>

    <UiSurface :title="t('projectDashboard.data.title')" :subtitle="t('projectDashboard.data.subtitle')">
      <div class="stat-grid triple">
        <UiStatTile
          v-for="metric in snapshot.dataMetrics"
          :key="metric.label"
          :label="t(metric.label)"
          :value="metric.value"
        />
      </div>

      <div class="surface-grid two">
        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('projectDashboard.activity.title') }}</strong>
          </div>
          <ul v-if="snapshot.activity.length" class="rank-list">
            <li v-for="activity in snapshot.activity" :key="activity.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ activity.title }}</strong>
                <small>{{ activity.description }}</small>
              </div>
              <span class="rank-value">{{ new Date(activity.timestamp).toLocaleString() }}</span>
            </li>
          </ul>
          <UiEmptyState
            v-else
            :title="t('projectDashboard.empty.activityTitle')"
            :description="t('projectDashboard.empty.activityDescription')"
          />
        </div>

        <div class="surface-subsection">
          <div class="subsection-header">
            <strong>{{ t('projectDashboard.data.conversationTop') }}</strong>
          </div>
          <ul v-if="snapshot.conversationTokenTop.length" class="rank-list">
            <li v-for="item in snapshot.conversationTokenTop" :key="item.id" class="rank-list-item">
              <div class="rank-copy">
                <strong>{{ item.label }}</strong>
                <small>{{ item.secondary }}</small>
              </div>
              <span class="rank-value">{{ item.value }}</span>
            </li>
          </ul>
          <UiEmptyState
            v-else
            :title="t('projectDashboard.empty.rankingTitle')"
            :description="t('projectDashboard.empty.rankingDescription')"
          />
        </div>
      </div>
    </UiSurface>
  </section>
</template>

<style scoped>
.header-meta,
.surface-actions,
.subsection-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.surface-actions {
  justify-content: flex-end;
}

.field-stack,
.surface-subsection {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.project-info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 1rem;
}

.project-info-item {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 1rem 1.05rem;
  border-radius: calc(var(--radius-l) + 1px);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 66%, transparent);
}

.project-info-item.full {
  grid-column: 1 / -1;
}

.project-info-item span,
.project-info-item p,
.rank-copy small {
  color: var(--text-secondary);
}

.project-info-item p {
  margin: 0;
  line-height: 1.6;
}

.stat-grid {
  display: grid;
  gap: 0.9rem;
}

.stat-grid.triple {
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
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
  padding: 0.95rem 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-l) + 1px);
  background: color-mix(in srgb, var(--bg-subtle) 66%, transparent);
}

.rank-copy {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}

.rank-value {
  color: var(--text-secondary);
  font-size: 0.85rem;
  white-space: nowrap;
}
</style>
