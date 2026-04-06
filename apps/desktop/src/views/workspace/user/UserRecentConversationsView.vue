<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<template>
  <div class="p-6">
    <h1 class="text-2xl font-bold mb-4">{{ t('sidebar.workspaceMenu.items.conversations') }}</h1>
    <div class="bg-card rounded-lg border p-12 text-center text-text-tertiary">
      全局最近会话功能开发中...
    </div>
  </div>
</template>
