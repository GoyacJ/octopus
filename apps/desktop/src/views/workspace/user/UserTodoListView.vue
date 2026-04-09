<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { UiEmptyState, UiPanelFrame } from '@octopus/ui'

const { t } = useI18n()
</script>

<template>
  <div data-testid="user-center-todos-view">
    <UiPanelFrame
      variant="subtle"
      padding="lg"
      :title="t('userCenter.todos.title')"
      :subtitle="t('userCenter.todos.emptyState')"
    >
      <UiEmptyState
        :title="t('userCenter.todos.title')"
        :description="t('userCenter.todos.emptyState')"
      />
    </UiPanelFrame>
  </div>
</template>
