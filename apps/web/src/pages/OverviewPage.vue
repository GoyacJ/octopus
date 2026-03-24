<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { UiStatTile, UiStatusBadge, UiSurfaceCard } from '@octopus/ui'

const { t } = useI18n()
</script>

<template>
  <div class="grid gap-4">
    <UiSurfaceCard
      eyebrow="Phase 2"
      :title="t('pages.overviewTitle')"
      :body="t('pages.overviewBody')"
    >
      <UiStatusBadge :label="t('status.ready')" />
    </UiSurfaceCard>

    <div class="grid gap-4 md:grid-cols-3">
      <UiStatTile
        label="Runs"
        value="01"
        caption="`run -> approval -> resume` vertical slice is the next milestone."
      />
      <UiStatTile
        label="Inbox"
        value="01"
        caption="Ask-user and approval interactions converge into one surface."
      />
      <UiStatTile
        label="Audit"
        value="01"
        caption="Timeline and audit stay first-class, not an afterthought."
      />
    </div>
  </div>
</template>
