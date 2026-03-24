<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { UiStatusBadge, UiSurfaceCard } from '@octopus/ui'

const { t } = useI18n()
</script>

<template>
  <UiSurfaceCard
    eyebrow="Workspaces"
    :title="t('pages.workspacesTitle')"
    :body="t('pages.workspacesBody')"
  >
    <UiStatusBadge
      :label="t('status.ready')"
      tone="success"
    />
  </UiSurfaceCard>
</template>
