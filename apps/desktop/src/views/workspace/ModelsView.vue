<script setup lang="ts">
import { watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiRecordCard, UiSectionHeading } from '@octopus/ui'

import { useCatalogStore } from '@/stores/catalog'
import { useShellStore } from '@/stores/shell'

const { t } = useI18n()
const catalogStore = useCatalogStore()
const shell = useShellStore()

watch(
  () => shell.activeWorkspaceConnectionId,
  (connectionId) => {
    if (connectionId) {
      void catalogStore.load(connectionId)
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="px-2">
      <UiSectionHeading :eyebrow="t('models.header.eyebrow')" :title="t('sidebar.navigation.models')" :subtitle="catalogStore.error || t('models.header.subtitle')" />
    </header>

    <section class="space-y-4 px-2">
      <h3 class="text-lg font-semibold text-text-primary">{{ t('models.catalog.title') }}</h3>
      <div v-if="catalogStore.models.length" class="grid gap-3 lg:grid-cols-2">
        <UiRecordCard
          v-for="model in catalogStore.models"
          :key="model.id"
          :title="model.label"
          :description="model.description"
        >
          <template #badges>
            <UiBadge :label="model.provider" subtle />
            <UiBadge :label="model.defaultPermission" subtle />
          </template>
          <template #meta>
            <span class="text-xs text-text-tertiary">{{ model.recommendedFor }}</span>
          </template>
        </UiRecordCard>
      </div>
      <UiEmptyState v-else :title="t('models.empty.title')" :description="t('models.empty.description')" />
    </section>

    <section class="space-y-4 px-2">
      <h3 class="text-lg font-semibold text-text-primary">{{ t('models.credentials.title') }}</h3>
      <div v-if="catalogStore.providerCredentials.length" class="grid gap-3 lg:grid-cols-2">
        <UiRecordCard
          v-for="credential in catalogStore.providerCredentials"
          :key="credential.id"
          :title="credential.name"
          :description="credential.baseUrl || credential.provider"
        >
          <template #badges>
            <UiBadge :label="credential.provider" subtle />
            <UiBadge :label="credential.status" subtle />
          </template>
        </UiRecordCard>
      </div>
      <UiEmptyState v-else :title="t('models.credentials.emptyTitle')" :description="t('models.credentials.emptyDescription')" />
    </section>
  </div>
</template>
