<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiRecordCard, UiSectionHeading } from '@octopus/ui'

import { enumLabel, resolveMockField } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()
</script>

<template>
  <div class="w-full flex flex-col gap-10 pb-20">
    <header class="px-2 shrink-0">
      <UiSectionHeading
        :eyebrow="t('automations.placeholder.eyebrow')"
        :title="t('automations.placeholder.title')"
        :subtitle="t('automations.placeholder.subtitle')"
      />
    </header>

    <main class="flex-1 px-2 space-y-6">
      <div class="space-y-1 border-b border-border-subtle pb-4">
        <h3 class="text-lg font-bold text-text-primary">{{ t('automations.list.title') }}</h3>
        <p class="text-[13px] text-text-secondary">{{ t('automations.list.subtitle') }}</p>
      </div>

      <div v-if="workbench.workspaceAutomations.length" data-testid="automations-record-list" class="grid gap-4 sm:grid-cols-2">
        <UiRecordCard
          v-for="automation in workbench.workspaceAutomations"
          :key="automation.id"
          :test-id="`automation-record-${automation.id}`"
          :title="resolveMockField('automation', automation.id, 'title', automation.title)"
          :description="resolveMockField('automation', automation.id, 'description', automation.description)"
        >
          <template #badges>
            <UiBadge :label="enumLabel('automationStatus', automation.status)" :tone="automation.status === 'active' ? 'success' : 'warning'" />
            <UiBadge :label="resolveMockField('automation', automation.id, 'cadence', automation.cadence)" subtle />
          </template>
          <template #meta>
            <span class="text-[12px] text-text-secondary">{{ resolveMockField('automation', automation.id, 'output', automation.output) }}</span>
          </template>
        </UiRecordCard>
      </div>
      
      <UiEmptyState 
        v-else 
        :title="t('automations.list.emptyTitle')" 
        :description="t('automations.list.emptyDescription')" 
      />
    </main>
  </div>
</template>
