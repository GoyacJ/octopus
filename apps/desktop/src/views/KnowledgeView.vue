<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiSectionHeading, UiSurface } from '@octopus/ui'

import { enumLabel, resolveMockField } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const privateEntries = computed(() => workbench.projectKnowledge.filter((entry) => entry.kind === 'private'))
const sharedEntries = computed(() => workbench.projectKnowledge.filter((entry) => entry.kind === 'shared'))
const candidateEntries = computed(() => workbench.projectKnowledge.filter((entry) => entry.kind === 'candidate'))
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('knowledge.header.eyebrow')"
      :title="workbench.activeProject ? resolveMockField('project', workbench.activeProject.id, 'name', workbench.activeProject.name) : t('knowledge.header.titleFallback')"
      :subtitle="t('knowledge.header.subtitle')"
    />

    <div class="surface-grid three">
      <UiSurface :title="t('knowledge.sections.private.title')">
        <div v-if="privateEntries.length" class="entry-list">
          <article v-for="entry in privateEntries" :key="entry.id" class="knowledge-card">
            <div class="meta-row">
              <UiBadge :label="enumLabel('knowledgeStatus', entry.status)" subtle />
              <UiBadge :label="entry.ownerId ?? t('common.workspace')" tone="info" subtle />
            </div>
            <strong>{{ resolveMockField('knowledgeEntry', entry.id, 'title', entry.title) }}</strong>
            <p>{{ resolveMockField('knowledgeEntry', entry.id, 'summary', entry.summary) }}</p>
          </article>
        </div>
        <UiEmptyState v-else :title="t('knowledge.sections.private.emptyTitle')" :description="t('knowledge.sections.private.emptyDescription')" />
      </UiSurface>

      <UiSurface :title="t('knowledge.sections.shared.title')">
        <div v-if="sharedEntries.length" class="entry-list">
          <article v-for="entry in sharedEntries" :key="entry.id" class="knowledge-card">
            <div class="meta-row">
              <UiBadge :label="enumLabel('riskLevel', entry.trustLevel)" tone="success" />
              <UiBadge :label="enumLabel('knowledgeSourceType', entry.sourceType)" subtle />
            </div>
            <strong>{{ resolveMockField('knowledgeEntry', entry.id, 'title', entry.title) }}</strong>
            <p>{{ resolveMockField('knowledgeEntry', entry.id, 'summary', entry.summary) }}</p>
            <small>{{ t('common.lineage') }}: {{ entry.lineage.join(' → ') }}</small>
          </article>
        </div>
        <UiEmptyState v-else :title="t('knowledge.sections.shared.emptyTitle')" :description="t('knowledge.sections.shared.emptyDescription')" />
      </UiSurface>

      <UiSurface :title="t('knowledge.sections.candidates.title')">
        <div v-if="candidateEntries.length" class="entry-list">
          <article v-for="entry in candidateEntries" :key="entry.id" class="knowledge-card">
            <div class="meta-row">
              <UiBadge :label="t('knowledge.sections.candidates.badge')" tone="warning" />
              <UiBadge :label="entry.sourceId" subtle />
            </div>
            <strong>{{ resolveMockField('knowledgeEntry', entry.id, 'title', entry.title) }}</strong>
            <p>{{ resolveMockField('knowledgeEntry', entry.id, 'summary', entry.summary) }}</p>
            <small>{{ t('common.lineage') }}: {{ entry.lineage.join(' → ') }}</small>
          </article>
        </div>
        <UiEmptyState v-else :title="t('knowledge.sections.candidates.emptyTitle')" :description="t('knowledge.sections.candidates.emptyDescription')" />
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.entry-list {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}

.knowledge-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.knowledge-card p,
.knowledge-card small {
  color: var(--text-secondary);
  line-height: 1.6;
  overflow-wrap: anywhere;
}
</style>
