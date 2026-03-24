<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { UiStatusBadge, UiSurfaceCard } from '@octopus/ui'

const { t } = useI18n()
</script>

<template>
  <UiSurfaceCard
    eyebrow="Agents"
    :title="t('pages.agentsTitle')"
    :body="t('pages.agentsBody')"
  >
    <UiStatusBadge
      :label="t('status.ready')"
      tone="success"
    />
  </UiSurfaceCard>
</template>
