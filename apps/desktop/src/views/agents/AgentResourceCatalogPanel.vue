<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type { CapabilityManagementEntry } from '@octopus/schema'
import { UiBadge, UiEmptyState, UiInput, UiPagination, UiRecordCard, UiToolbarRow } from '@octopus/ui'

const props = defineProps<{
  query: string
  total: number
  page: number
  pageCount: number
  pagedEntries: CapabilityManagementEntry[]
}>()

const emit = defineEmits<{
  'update:query': [value: string]
  'update:page': [value: number]
}>()

const { t } = useI18n()

function ownerScopeLabel(ownerScope: CapabilityManagementEntry['ownerScope']) {
  if (!ownerScope) {
    return t('common.na')
  }
  const translationKey = `tools.ownerScopes.${ownerScope}`
  const translated = t(translationKey)
  return translated === translationKey ? ownerScope : translated
}
</script>

<template>
  <section class="space-y-4">
    <UiToolbarRow>
      <template #search>
        <UiInput
          :model-value="query"
          :placeholder="t('tools.search.placeholder')"
          class="max-w-md"
          @update:model-value="emit('update:query', String($event))"
        />
      </template>
      <template #actions>
        <span class="text-[12px] text-text-tertiary">
          {{ t('tools.summary.results', { count: total, total }) }}
        </span>
      </template>
    </UiToolbarRow>

    <div v-if="total" class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
      <UiRecordCard
        v-for="entry in pagedEntries"
        :key="entry.id"
        :title="entry.name"
        :description="entry.description"
      >
        <template #eyebrow>
          {{ t(`tools.tabs.${entry.kind}`) }}
        </template>

        <template #badges>
          <UiBadge v-if="entry.ownerScope" :label="ownerScopeLabel(entry.ownerScope)" subtle />
          <UiBadge v-if="entry.ownerLabel" :label="entry.ownerLabel" subtle />
          <UiBadge v-if="entry.disabled" :label="t('tools.states.disabled')" tone="warning" />
          <UiBadge v-if="entry.kind === 'mcp' && entry.toolNames.length" :label="`${entry.toolNames.length} tools`" subtle />
        </template>

        <div class="space-y-2">
          <p class="break-all text-[12px] text-text-secondary">
            {{ entry.displayPath }}
          </p>
          <p v-if="entry.ownerScope || entry.ownerLabel" class="text-[11px] text-text-tertiary">
            {{ t('tools.detail.source') }}:
            {{ entry.ownerScope ? ownerScopeLabel(entry.ownerScope) : t('common.na') }}
            <template v-if="entry.ownerLabel">
              · {{ entry.ownerLabel }}
            </template>
          </p>
          <div v-if="entry.consumers?.length" class="flex flex-wrap gap-1.5">
            <UiBadge
              v-for="consumer in entry.consumers"
              :key="`${entry.id}-${consumer.kind}-${consumer.id}`"
              :label="consumer.name"
              subtle
            />
          </div>
        </div>
      </UiRecordCard>
    </div>

    <UiEmptyState
      v-else
      :title="t('tools.empty.title')"
      :description="t('tools.empty.description')"
    />

    <UiPagination
      v-if="pageCount > 1"
      :page="page"
      :page-count="pageCount"
      :meta-label="t('tools.summary.results', { count: total, total })"
      @update:page="emit('update:page', $event)"
    />
  </section>
</template>
