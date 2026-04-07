<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<template>
  <div class="p-6">
    <h1 class="mb-4 text-2xl font-bold">{{ t('userCenter.recentConversations.title') }}</h1>
    <div class="bg-card rounded-lg border p-12 text-center text-text-tertiary">
      {{ t('userCenter.recentConversations.emptyState') }}
    </div>
  </div>
</template>
