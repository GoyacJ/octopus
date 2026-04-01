<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { UiEmptyState, UiSectionHeading, UiSurface } from '@octopus/ui'

import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('models.header.eyebrow')"
      :title="t('models.header.title')"
      :subtitle="t('models.header.subtitle')"
    />

    <UiSurface :title="t('models.list.title')" :subtitle="t('models.list.subtitle')">
      <div v-if="workbench.workspaceModelCatalog.length" class="catalog-list">
        <div v-for="model in workbench.workspaceModelCatalog" :key="model.id" class="catalog-row">
          <div class="catalog-copy">
            <strong>{{ model.label }}</strong>
            <small>{{ model.provider }} · {{ model.recommendedFor }}</small>
          </div>
          <div class="catalog-meta">
            <span>{{ model.availability }}</span>
            <span>{{ model.defaultPermission }}</span>
          </div>
        </div>
      </div>
      <UiEmptyState v-else :title="t('models.list.emptyTitle')" :description="t('models.list.emptyDescription')" />
    </UiSurface>
  </section>
</template>

<style scoped>
.catalog-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.catalog-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.9rem;
  padding: 0.9rem 0.95rem;
  border-radius: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 72%, transparent);
}

.catalog-copy,
.catalog-meta {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.catalog-copy small,
.catalog-meta {
  color: var(--text-muted);
}

.catalog-meta {
  align-items: flex-end;
  text-transform: capitalize;
  font-size: 0.78rem;
}
</style>
