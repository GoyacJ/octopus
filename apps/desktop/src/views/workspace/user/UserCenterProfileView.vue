<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiButton, UiCodeEditor, UiEmptyState, UiMetricCard, UiRecordCard } from '@octopus/ui'

import { useUserCenterStore } from '@/stores/user-center'

const { t } = useI18n()
const userCenterStore = useUserCenterStore()

const currentUser = computed(() => userCenterStore.currentUser)
const overview = computed(() => userCenterStore.overview)
const runtimeConfig = computed(() => userCenterStore.runtimeConfig)
const runtimeSource = computed(() => runtimeConfig.value?.sources.filter(source => source.scope === 'user').at(-1))
const runtimeEffectivePreview = computed(() => JSON.stringify(runtimeConfig.value?.effectiveConfig ?? {}, null, 2))
const metrics = computed(() => [
  { id: 'roles', label: t('userCenter.profile.metrics.roleCount'), value: String(userCenterStore.currentRoleNames.length) },
  { id: 'permissions', label: t('userCenter.profile.metrics.permissionCount'), value: String(userCenterStore.permissions.length) },
  { id: 'menus', label: t('userCenter.profile.metrics.menuCount'), value: String(userCenterStore.currentEffectiveMenuIds.length) },
])

watch(
  () => currentUser.value?.id ?? '',
  (userId) => {
    if (!userId) {
      return
    }
    void userCenterStore.loadCurrentUserRuntimeConfig()
  },
  { immediate: true },
)
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
      <template #leading>
        <div class="flex h-12 w-12 items-center justify-center overflow-hidden rounded-full border border-border/60 bg-accent text-sm font-semibold uppercase text-text-secondary">
          <img v-if="currentUser.avatar" :src="currentUser.avatar" alt="" class="h-full w-full object-cover">
          <span v-else>{{ currentUser.displayName.slice(0, 1) }}</span>
        </div>
      </template>
      <template #badges>
        <UiBadge :label="currentUser.status" subtle />
        <UiBadge :label="currentUser.passwordState" subtle />
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

    <UiRecordCard
      v-if="currentUser"
      :title="t('userCenter.profile.runtime.title')"
      :description="t('userCenter.profile.runtime.description')"
      test-id="user-runtime-editor"
    >
      <template #eyebrow>
        user
      </template>
      <template #badges>
        <UiBadge
          :label="userCenterStore.runtimeValidation?.valid ? t('settings.runtime.validation.valid') : t('settings.runtime.validation.idle')"
          :tone="userCenterStore.runtimeValidation?.valid ? 'success' : 'default'"
        />
        <UiBadge
          :label="runtimeSource?.loaded ? t('settings.runtime.sourceStatuses.loaded') : t('settings.runtime.sourceStatuses.missing')"
          :tone="runtimeSource?.loaded ? 'success' : 'warning'"
        />
      </template>

      <div class="space-y-3">
        <UiCodeEditor
          language="json"
          theme="octopus"
          :model-value="userCenterStore.runtimeDraft"
          @update:model-value="userCenterStore.setCurrentUserRuntimeDraft($event)"
        />

        <div
          v-if="userCenterStore.runtimeValidation?.errors.length"
          class="rounded-md border border-status-error/20 bg-status-error/5 px-3 py-2 text-[12px] text-status-error"
        >
          {{ userCenterStore.runtimeValidation.errors.join(' ') }}
        </div>
      </div>

      <template #meta>
        <span class="text-[11px] uppercase tracking-[0.24em] text-text-tertiary">
          {{ t('settings.runtime.sourcePath') }}
        </span>
        <span class="min-w-0 truncate font-mono text-[12px] text-text-secondary">
          {{ runtimeSource?.displayPath ?? t('common.na') }}
        </span>
      </template>
      <template #actions>
        <UiButton
          variant="ghost"
          size="sm"
          :disabled="userCenterStore.runtimeValidating || userCenterStore.runtimeSaving"
          @click="userCenterStore.validateCurrentUserRuntimeConfig()"
        >
          {{ t('settings.runtime.actions.validate') }}
        </UiButton>
        <UiButton
          size="sm"
          :disabled="userCenterStore.runtimeSaving"
          @click="userCenterStore.saveCurrentUserRuntimeConfig()"
        >
          {{ t('settings.runtime.actions.save') }}
        </UiButton>
      </template>
    </UiRecordCard>

    <UiRecordCard
      v-if="currentUser"
      :title="t('userCenter.profile.runtime.effectiveTitle')"
      :description="t('userCenter.profile.runtime.effectiveDescription')"
      test-id="user-runtime-effective-preview"
    >
      <UiCodeEditor
        language="json"
        theme="octopus"
        readonly
        :model-value="runtimeEffectivePreview"
      />
    </UiRecordCard>

    <UiEmptyState v-if="!currentUser" :title="t('userCenter.profile.emptyTitle')" :description="t('userCenter.profile.emptyDescription')" />
  </div>
</template>
