<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { UiEmptyState, UiSectionHeading, UiListRow, UiBadge } from '@octopus/ui'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()
</script>

<template>
  <div class="w-full flex flex-col gap-10 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0">
      <UiSectionHeading
        :eyebrow="t('models.header.eyebrow')"
        :title="t('models.header.title')"
        :subtitle="t('models.header.subtitle')"
      />
    </header>

    <main class="flex-1 px-2 space-y-6">
      <div class="space-y-1">
        <h3 class="text-lg font-bold text-text-primary">{{ t('models.list.title') }}</h3>
        <p class="text-[13px] text-text-secondary">{{ t('models.list.subtitle') }}</p>
      </div>

      <div v-if="workbench.workspaceModelCatalog.length" class="flex flex-col gap-1">
        <UiListRow
          v-for="model in workbench.workspaceModelCatalog"
          :key="model.id"
          :title="model.label"
          :subtitle="model.recommendedFor"
          :eyebrow="model.provider"
        >
          <template #meta>
            <UiBadge :label="model.availability" :tone="model.availability === 'healthy' ? 'success' : 'warning'" subtle />
            <UiBadge :label="model.defaultPermission" subtle />
          </template>
        </UiListRow>
      </div>
      <UiEmptyState 
        v-else 
        :title="t('models.list.emptyTitle')" 
        :description="t('models.list.emptyDescription')" 
      />
    </main>
  </div>
</template>
