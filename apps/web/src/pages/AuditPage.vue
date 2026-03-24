<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { UiStatusBadge, UiSurfaceCard } from '@octopus/ui'

const { t } = useI18n()
</script>

<template>
  <UiSurfaceCard
    eyebrow="Audit"
    :title="t('pages.auditTitle')"
    :body="t('pages.auditBody')"
  >
    <UiStatusBadge
      :label="t('status.ready')"
      tone="accent"
    />
  </UiSurfaceCard>
</template>
