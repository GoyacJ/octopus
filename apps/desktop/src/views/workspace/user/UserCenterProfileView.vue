<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiButton, UiEmptyState, UiInfoCard, UiMetricCard, UiTimelineList } from '@octopus/ui'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const snapshot = computed(() => workbench.currentUserProfileSnapshot)
const maskedPassword = computed(() => {
  if (!workbench.currentUser) {
    return '********'
  }

  if (workbench.currentUser.passwordState === 'reset-required') {
    return t('userCenter.profile.passwordResetRequired')
  }

  if (workbench.currentUser.passwordState === 'temporary') {
    return t('userCenter.profile.passwordTemporary')
  }

  return '********'
})

const securityMetrics = computed(() => [
  {
    id: 'permissionCount',
    label: t('userCenter.profile.metrics.permissionCount'),
    value: String(snapshot.value.permissionCount),
    helper: t('userCenter.profile.metrics.permissionHelper'),
  },
  {
    id: 'menuCount',
    label: t('userCenter.profile.metrics.menuCount'),
    value: String(snapshot.value.menuCount),
    helper: t('userCenter.profile.metrics.menuHelper'),
  },
  {
    id: 'roleCount',
    label: t('userCenter.profile.metrics.roleCount'),
    value: String(snapshot.value.roleNames.filter((name) => name !== t('userCenter.common.noRoles')).length),
    helper: t('userCenter.profile.metrics.roleHelper'),
  },
])

const timelineItems = computed(() =>
  snapshot.value.recentActivity.map((activity) => ({
    id: activity.id,
    title: activity.title,
    description: activity.description,
    timestamp: formatDateTime(activity.timestamp),
  })),
)

function resetPassword() {
  if (!workbench.currentUser) {
    return
  }

  workbench.resetUserPassword(workbench.currentUser.id)
}
</script>

<template>
  <div class="space-y-12">
    
    <!-- Profile Hero -->
    <section class="flex flex-col md:flex-row md:items-start justify-between gap-8 border-b border-border-subtle pb-8">
      <div class="flex min-w-0 items-center gap-5">
        <div class="flex size-20 shrink-0 items-center justify-center rounded-2xl bg-primary/10 text-2xl font-bold text-primary">
          {{ workbench.currentUser?.avatar ?? '--' }}
        </div>
        <div class="space-y-2.5">
          <div>
            <h3 class="text-2xl font-bold text-text-primary leading-none">{{ workbench.currentUser?.nickname ?? t('common.na') }}</h3>
            <p class="text-sm text-text-secondary mt-1">{{ workbench.currentUser?.username ?? t('common.na') }}</p>
          </div>
          <div class="flex flex-wrap gap-2">
            <UiBadge :label="enumLabel('viewStatus', 'healthy')" subtle />
            <UiBadge :label="workbench.currentUser?.status ?? t('common.na')" :tone="workbench.currentUser?.status === 'active' ? 'success' : 'warning'" />
            <UiBadge :label="snapshot.scopeSummary" subtle />
          </div>
        </div>
      </div>

      <div data-testid="user-center-profile-metrics" class="flex gap-3">
        <UiMetricCard
          v-for="metric in securityMetrics"
          :key="metric.id"
          :label="metric.label"
          :value="metric.value"
          tone="muted"
          class="min-w-[120px]"
        />
      </div>
    </section>

    <div class="grid gap-12 xl:grid-cols-2">
      <!-- Security & Info -->
      <section class="space-y-4">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-bold text-text-primary">{{ t('userCenter.profile.securityTitle') }}</h3>
          <UiButton variant="ghost" size="sm" @click="resetPassword">{{ t('userCenter.profile.resetPassword') }}</UiButton>
        </div>

        <div class="grid gap-x-8 gap-y-6 md:grid-cols-2 text-[13px]">
          <div>
            <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1">{{ t('userCenter.profile.passwordLabel') }}</strong>
            <span class="text-text-primary font-mono">{{ maskedPassword }}</span>
          </div>
          <div>
            <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1">{{ t('userCenter.profile.passwordUpdatedAtLabel') }}</strong>
            <span class="text-text-primary">{{ formatDateTime(workbench.currentUser?.passwordUpdatedAt) }}</span>
          </div>
          <div class="md:col-span-2">
            <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1">{{ t('userCenter.profile.roleLabel') }}</strong>
            <span class="text-text-primary">{{ snapshot.roleNames.join(' / ') }}</span>
          </div>
          <div>
            <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1">{{ t('userCenter.profile.phoneLabel') }}</strong>
            <span class="text-text-primary">{{ workbench.currentUser?.phone || t('common.na') }}</span>
          </div>
          <div>
            <strong class="block text-text-tertiary text-[10px] uppercase font-bold tracking-wider mb-1">{{ t('userCenter.profile.emailLabel') }}</strong>
            <span class="text-text-primary">{{ workbench.currentUser?.email ?? t('common.na') }}</span>
          </div>
        </div>
      </section>

      <!-- Permissions -->
      <section class="space-y-4">
        <div class="space-y-1">
          <h3 class="text-lg font-bold text-text-primary">{{ t('userCenter.profile.permissionTitle') }}</h3>
          <p class="text-[13px] text-text-secondary">{{ t('userCenter.profile.permissionSubtitle') }}</p>
        </div>

        <div v-if="snapshot.groups.length" class="space-y-4">
          <div
            v-for="group in snapshot.groups"
            :key="group.targetType"
            class="bg-subtle/30 rounded-md border border-border-subtle p-4"
          >
            <div class="mb-3 flex items-center justify-between">
              <strong class="text-[13px] font-bold text-text-primary">{{ enumLabel('permissionTargetType', group.targetType) || group.targetType }}</strong>
              <UiBadge :label="t('userCenter.profile.permissionCountLabel', { count: group.permissions.length })" subtle />
            </div>
            <ul class="space-y-3">
              <li v-for="permission in group.permissions" :key="permission.id" class="space-y-0.5">
                <span class="block text-[12px] font-semibold text-text-primary">{{ permission.name }}</span>
                <span class="block text-[11px] text-text-tertiary">{{ permission.targetLabels.join(' / ') || permission.code }}</span>
              </li>
            </ul>
          </div>
        </div>
        <UiEmptyState
          v-else
          :title="t('userCenter.profile.emptyPermissionTitle')"
          :description="t('userCenter.profile.emptyPermissionDescription')"
        />
      </section>
    </div>

    <!-- Timeline -->
    <section class="space-y-4 border-t border-border-subtle pt-8">
      <div class="space-y-1">
        <h3 class="text-lg font-bold text-text-primary">{{ t('userCenter.profile.timelineTitle') }}</h3>
        <p class="text-[13px] text-text-secondary">{{ t('userCenter.profile.timelineSubtitle') }}</p>
      </div>

      <div data-testid="user-center-profile-timeline" class="bg-subtle/20 border border-border-subtle rounded-lg p-4 max-w-3xl max-h-80 overflow-y-auto">
        <UiTimelineList v-if="timelineItems.length" :items="timelineItems" />
        <UiEmptyState
          v-else
          :title="t('userCenter.profile.emptyTimelineTitle')"
          :description="t('userCenter.profile.emptyTimelineDescription')"
        />
      </div>
    </section>
  </div>
</template>
