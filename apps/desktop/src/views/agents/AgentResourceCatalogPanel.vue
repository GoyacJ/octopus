<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type { WorkspaceToolCatalogEntry } from '@octopus/schema'
import { UiBadge, UiEmptyState, UiInput, UiPagination, UiRecordCard, UiToolbarRow } from '@octopus/ui'

const props = defineProps<{
  query: string
  total: number
  page: number
  pageCount: number
  pagedEntries: WorkspaceToolCatalogEntry[]
}>()

const emit = defineEmits<{
  'update:query': [value: string]
  'update:page': [value: number]
}>()

const { t } = useI18n()
</script>

<template>
  <section class="space-y-4">
    <UiToolbarRow>
      <template #search>
        <UiInput
          :model-value="query"
          placeholder="搜索名称、使用者或来源"
          class="max-w-md"
          @update:model-value="emit('update:query', String($event))"
        />
      </template>
      <template #actions>
        <span class="text-[12px] text-text-tertiary">
          {{ `共 ${total} 项` }}
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
          <UiBadge v-if="entry.ownerLabel" :label="entry.ownerLabel" subtle />
          <UiBadge v-if="entry.disabled" :label="t('tools.states.disabled')" tone="warning" />
          <UiBadge v-if="entry.kind === 'mcp' && entry.toolNames.length" :label="`${entry.toolNames.length} tools`" subtle />
        </template>

        <div class="space-y-2">
          <p class="break-all text-[12px] text-text-secondary">
            {{ entry.displayPath }}
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
      title="暂无资源"
      description="当前分组下没有可展示的资源。"
    />

    <UiPagination
      v-if="pageCount > 1"
      :page="page"
      :page-count="pageCount"
      :meta-label="`共 ${total} 项`"
      @update:page="emit('update:page', $event)"
    />
  </section>
</template>
