<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { UiStatusBadge, UiSurfaceCard } from '@octopus/ui'

const { t } = useI18n()
</script>

<template>
  <UiSurfaceCard
    eyebrow="Runs"
    :title="t('pages.runsTitle')"
    :body="t('pages.runsBody')"
  >
    <UiStatusBadge
      :label="t('status.ready')"
      tone="accent"
    />
  </UiSurfaceCard>
</template>
