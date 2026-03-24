<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { UiStatusBadge, UiSurfaceCard } from '@octopus/ui'

const { t } = useI18n()
</script>

<template>
  <UiSurfaceCard
    eyebrow="Inbox"
    :title="t('pages.inboxTitle')"
    :body="t('pages.inboxBody')"
  >
    <UiStatusBadge
      :label="t('status.ready')"
      tone="warning"
    />
  </UiSurfaceCard>
</template>
