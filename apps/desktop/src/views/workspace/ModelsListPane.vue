<script setup lang="ts">
import { enumLabel } from '@/i18n/copy'
import type { CatalogConfiguredModelRow } from '@/stores/catalog'
import { UiBadge, UiEmptyState, UiPagination, UiRecordCard } from '@octopus/ui'

const props = defineProps<{
  pagedRows: CatalogConfiguredModelRow[]
  selectedConfiguredModelId: string
  filteredRowsLength: number
  page: number
  pageCount: number
  t: (key: string, params?: Record<string, unknown>) => string
}>()

const t = props.t

const emit = defineEmits<{
  'update:page': [value: number]
  selectRow: [row: CatalogConfiguredModelRow]
}>()

function credentialSourceLabel(row: CatalogConfiguredModelRow) {
  const value = row.credentialDisplayLabel
  return typeof value === 'string' && value.trim()
    ? value
    : props.t('models.security.sources.missing')
}

function credentialHealthLabel(row: CatalogConfiguredModelRow) {
  const value = row.credentialHealthLabel
  return typeof value === 'string' && value.trim()
    ? value
    : props.t('models.security.states.missing')
}

function quotaLabel(row: CatalogConfiguredModelRow) {
  if (!row.totalTokens) {
    return props.t('models.quota.unlimited')
  }

  return row.quotaExhausted
    ? props.t('models.quota.exhausted')
    : props.t('models.quota.available')
}
</script>

<template>
  <section data-testid="workspace-models-list-pane" class="space-y-3">
    <div v-if="pagedRows.length" class="space-y-2">
      <div
        v-for="row in pagedRows"
        :key="row.configuredModelId"
        :data-testid="`models-list-row-${row.configuredModelId}`"
        class="rounded-[var(--radius-l)]"
        @click="emit('selectRow', row)"
      >
        <UiRecordCard
          :title="row.name"
          :description="row.description || `${row.modelLabel} · ${row.providerLabel}`"
          :active="props.selectedConfiguredModelId === row.configuredModelId"
          interactive
          layout="compact"
        >
          <template #eyebrow>
            {{ row.providerLabel }}
          </template>

          <template #badges>
            <UiBadge
              :label="row.enabled ? t('models.states.enabled') : t('models.states.disabled')"
              :tone="row.enabled ? 'success' : 'warning'"
            />
            <UiBadge v-if="row.hasDiagnostics" :label="t('models.list.hasDiagnostics')" tone="warning" />
          </template>

          <template #secondary>
            <UiBadge :label="credentialSourceLabel(row)" subtle />
            <UiBadge :label="credentialHealthLabel(row)" subtle />
            <UiBadge
              v-for="surface in row.surfaces"
              :key="surface"
              :label="enumLabel('modelSurface', surface)"
              subtle
            />
          </template>

          <template #meta>
            <div class="flex flex-wrap items-center gap-3 text-xs text-text-tertiary">
              <span>{{ t('models.list.usedTokens', { count: row.usedTokens.toLocaleString() }) }}</span>
              <span>{{ t('models.list.tokenQuota', { count: row.totalTokens?.toLocaleString() ?? t('models.quota.unlimited') }) }}</span>
              <span>{{ quotaLabel(row) }}</span>
            </div>
          </template>
        </UiRecordCard>
      </div>
    </div>

    <UiEmptyState
      v-else
      :title="t('models.empty.title')"
      :description="t('models.empty.description')"
    />

    <UiPagination
      :page="page"
      data-testid="models-pagination"
      root-test-id="models-pagination"
      :page-count="pageCount"
      :summary-label="t('models.pagination.summary', { count: filteredRowsLength, page, total: pageCount })"
      @update:page="emit('update:page', $event)"
    />
  </section>
</template>
