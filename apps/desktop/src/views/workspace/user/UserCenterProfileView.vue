<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiMetricCard, UiRecordCard } from '@octopus/ui'

import { useUserCenterStore } from '@/stores/user-center'

const { t } = useI18n()
const userCenterStore = useUserCenterStore()

const currentUser = computed(() => userCenterStore.currentUser)
const overview = computed(() => userCenterStore.overview)
const metrics = computed(() => [
  { id: 'roles', label: t('userCenter.profile.metrics.roleCount'), value: String(userCenterStore.currentRoleNames.length) },
  { id: 'permissions', label: t('userCenter.profile.metrics.permissionCount'), value: String(userCenterStore.permissions.length) },
  { id: 'menus', label: t('userCenter.profile.metrics.menuCount'), value: String(userCenterStore.currentEffectiveMenuIds.length) },
])
</script>

<template>
  <div class="space-y-8">
    <div v-if="currentUser" class="grid gap-4 md:grid-cols-3">
      <UiMetricCard v-for="metric in metrics" :key="metric.id" :label="metric.label" :value="metric.value" />
    </div>

    <UiRecordCard
      v-if="currentUser"
      :title="currentUser.displayName"
      :description="currentUser.username"
    >
      <template #badges>
        <UiBadge :label="currentUser.status" subtle />
        <UiBadge v-for="roleName in userCenterStore.currentRoleNames" :key="roleName" :label="roleName" subtle />
      </template>
      <template #meta>
        <span class="text-xs text-text-tertiary">{{ overview?.workspaceId }}</span>
      </template>
    </UiRecordCard>

    <div v-if="overview?.alerts.length" class="space-y-3">
      <UiRecordCard
        v-for="alert in overview.alerts"
        :key="alert.id"
        :title="alert.title"
        :description="alert.description"
      >
        <template #badges>
          <UiBadge :label="alert.severity" subtle />
        </template>
      </UiRecordCard>
    </div>

    <UiEmptyState v-if="!currentUser" :title="t('userCenter.profile.emptyTitle')" :description="t('userCenter.profile.emptyDescription')" />
  </div>
</template>
