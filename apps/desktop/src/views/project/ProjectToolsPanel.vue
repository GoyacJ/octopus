<script setup lang="ts">
import type { WorkspaceToolCatalogEntry } from '@octopus/schema'
import { UiBadge, UiButton, UiEmptyState, UiRecordCard, UiSelect, UiStatusCallout } from '@octopus/ui'

import type { ToolPermissionSelection, ToolSection } from './useProjectSettings'

const props = defineProps<{
  toolSections: ToolSection[]
  toolPermissionOptions: Array<{ value: ToolPermissionSelection, label: string }>
  toolsError: string
  savingTools: boolean
  resolveToolSelection: (sourceKey: string) => string
  toolPermissionSummaryLabel: (entry: WorkspaceToolCatalogEntry) => string
}>()

const emit = defineEmits<{
  reset: []
  save: []
  updateToolPermission: [sourceKey: string, nextValue: string]
}>()
</script>

<template>
  <UiRecordCard
    :title="$t('projectSettings.tools.title')"
    :description="$t('projectSettings.tools.description')"
  >
    <template #eyebrow>
      {{ $t('projectSettings.tabs.tools') }}
    </template>

    <UiEmptyState
      v-if="!toolSections.length"
      :title="$t('projectSettings.tools.emptyTitle')"
      :description="$t('projectSettings.tools.emptyDescription')"
    />

    <div v-else class="space-y-6">
      <section
        v-for="section in toolSections"
        :key="section.kind"
        class="space-y-3"
      >
        <div class="text-[11px] font-semibold uppercase tracking-[0.22em] text-text-tertiary">
          {{ $t(`projectSettings.tools.groups.${section.kind}`) }}
        </div>

        <div class="space-y-3">
          <div
            v-for="entry in section.entries"
            :key="entry.sourceKey"
            class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
          >
            <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
              <div class="min-w-0 space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-semibold text-text-primary">{{ entry.name }}</span>
                  <UiBadge
                    v-if="entry.requiredPermission"
                    :label="$t(`tools.requiredPermissions.${entry.requiredPermission}`)"
                    subtle
                  />
                </div>
                <p class="text-xs leading-5 text-text-secondary">
                  {{ entry.description }}
                </p>
                <p class="text-[11px] text-text-tertiary">
                  {{ entry.sourceKey }}
                </p>
              </div>

              <div class="w-full max-w-[15rem] space-y-1">
                <UiSelect
                  :model-value="props.resolveToolSelection(entry.sourceKey)"
                  :options="toolPermissionOptions"
                  @update:model-value="emit('updateToolPermission', entry.sourceKey, $event)"
                />
                <div class="text-[11px] text-text-tertiary">
                  {{ props.toolPermissionSummaryLabel(entry) }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <UiStatusCallout v-if="toolsError" tone="error" :description="toolsError" />
    </div>

    <template #actions>
      <UiButton variant="ghost" :disabled="savingTools" @click="emit('reset')">
        {{ $t('common.reset') }}
      </UiButton>
      <UiButton :disabled="savingTools || !toolSections.length" @click="emit('save')">
        {{ $t('common.save') }}
      </UiButton>
    </template>
  </UiRecordCard>
</template>
