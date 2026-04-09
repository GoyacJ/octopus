<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { UiEmptyState, UiPanelFrame } from '@octopus/ui'

const { t } = useI18n()
</script>

<template>
  <div data-testid="user-center-recent-conversations">
    <UiPanelFrame
      variant="subtle"
      padding="lg"
      :title="t('userCenter.recentConversations.title')"
      :subtitle="t('userCenter.recentConversations.emptyState')"
    >
      <UiEmptyState
        :title="t('userCenter.recentConversations.title')"
        :description="t('userCenter.recentConversations.emptyState')"
      />
    </UiPanelFrame>
  </div>
</template>
