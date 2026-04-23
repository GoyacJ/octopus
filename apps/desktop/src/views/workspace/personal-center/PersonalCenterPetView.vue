<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { RuntimePermissionMode } from '@octopus/schema'
import { UiEmptyState } from '@octopus/ui'

import { useCatalogStore } from '@/stores/catalog'
import { usePetStore } from '@/stores/pet'
import { useUserProfileStore } from '@/stores/user-profile'

import PetPreferencesPanel from './PetPreferencesPanel.vue'
import PetStatsPanel from './PetStatsPanel.vue'

const DEFAULT_QUIET_HOURS_START = '22:00'
const DEFAULT_QUIET_HOURS_END = '07:30'
const DEFAULT_REMINDER_TTL_MINUTES = 180

const { t } = useI18n()
const userProfileStore = useUserProfileStore()
const catalogStore = useCatalogStore()
const petStore = usePetStore()

const configuredModelId = ref('')
const permissionMode = ref<RuntimePermissionMode>('read-only')
const displayName = ref('')
const greeting = ref('')
const summary = ref('')
const reminderTtlMinutes = ref(String(DEFAULT_REMINDER_TTL_MINUTES))
const quietHoursEnabled = ref(false)
const quietHoursStart = ref(DEFAULT_QUIET_HOURS_START)
const quietHoursEnd = ref(DEFAULT_QUIET_HOURS_END)

const runtimeSource = computed(() =>
  userProfileStore.runtimeConfig?.sources.filter(source => source.scope === 'user').at(-1),
)
const modelOptions = computed(() => catalogStore.runnableConfiguredModelOptions.map(option => ({
  value: option.value,
  label: `${option.label} · ${option.providerLabel}`,
})))
const permissionOptions = computed(() => [
  {
    value: 'read-only',
    label: t('personalCenter.pet.permissionModes.readOnly'),
  },
  {
    value: 'workspace-write',
    label: t('personalCenter.pet.permissionModes.workspaceWrite'),
  },
  {
    value: 'danger-full-access',
    label: t('personalCenter.pet.permissionModes.dangerFullAccess'),
  },
])

function normalizeTime(value: string, fallback: string) {
  return /^([01]\d|2[0-3]):[0-5]\d$/.test(value.trim()) ? value.trim() : fallback
}

const normalizedReminderTtlMinutes = computed(() => {
  const parsed = Number.parseInt(reminderTtlMinutes.value, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : petStore.preferredReminderTtlMinutes
})
const normalizedQuietHours = computed(() => ({
  enabled: quietHoursEnabled.value,
  start: normalizeTime(quietHoursStart.value, petStore.preferredQuietHours.start),
  end: normalizeTime(quietHoursEnd.value, petStore.preferredQuietHours.end),
}))
const runtimePreview = computed(() => JSON.stringify({
  pet: {
    configuredModelId: configuredModelId.value || null,
    permissionMode: permissionMode.value,
    displayName: displayName.value.trim() || petStore.profile.displayName,
    greeting: greeting.value.trim() || petStore.profile.greeting,
    summary: summary.value.trim() || petStore.profile.summary,
    reminderTtlMinutes: normalizedReminderTtlMinutes.value,
    quietHours: normalizedQuietHours.value,
  },
}, null, 2))

function resetForm() {
  configuredModelId.value = petStore.resolvedConfiguredModelId || modelOptions.value[0]?.value || ''
  permissionMode.value = petStore.preferredPermissionMode
  displayName.value = petStore.profile.displayName
  greeting.value = petStore.profile.greeting
  summary.value = petStore.profile.summary
  reminderTtlMinutes.value = String(petStore.preferredReminderTtlMinutes)
  quietHoursEnabled.value = petStore.preferredQuietHours.enabled
  quietHoursStart.value = petStore.preferredQuietHours.start
  quietHoursEnd.value = petStore.preferredQuietHours.end
}

function updateConfiguredModelId(value: string) {
  configuredModelId.value = value
}

function updatePermissionMode(value: RuntimePermissionMode) {
  permissionMode.value = value
}

function updateDisplayName(value: string) {
  displayName.value = value
}

function updateGreeting(value: string) {
  greeting.value = value
}

function updateSummary(value: string) {
  summary.value = value
}

function updateReminderTtlMinutes(value: string) {
  reminderTtlMinutes.value = value
}

function updateQuietHoursEnabled(value: boolean) {
  quietHoursEnabled.value = value
}

function updateQuietHoursStart(value: string) {
  quietHoursStart.value = value
}

function updateQuietHoursEnd(value: string) {
  quietHoursEnd.value = value
}

watch(
  () => [
    userProfileStore.currentUser?.id ?? '',
    userProfileStore.runtimeConfig,
    petStore.profile.id,
    petStore.preferredConfiguredModelId,
    petStore.resolvedConfiguredModelId,
    petStore.preferredPermissionMode,
    petStore.preferredReminderTtlMinutes,
    petStore.preferredQuietHours.enabled,
    petStore.preferredQuietHours.start,
    petStore.preferredQuietHours.end,
    modelOptions.value.map(option => option.value).join('|'),
  ],
  () => {
    if (!userProfileStore.currentUser) {
      return
    }
    resetForm()
  },
  { immediate: true },
)

watch(
  () => [userProfileStore.currentUser?.id ?? '', userProfileStore.workspaceId],
  async ([userId]) => {
    if (!userId) {
      return
    }

    await Promise.all([
      userProfileStore.loadCurrentUserRuntimeConfig(),
      catalogStore.load(),
      petStore.loadSnapshot(undefined, undefined, true),
      petStore.loadDashboard(undefined, true),
    ])
  },
  { immediate: true },
)

async function savePetPreferences() {
  if (!userProfileStore.currentUser) {
    return
  }

  userProfileStore.mergeCurrentUserRuntimeDraftPatch({
    pet: {
      configuredModelId: configuredModelId.value || null,
      permissionMode: permissionMode.value,
      displayName: displayName.value.trim() || petStore.profile.displayName,
      greeting: greeting.value.trim() || petStore.profile.greeting,
      summary: summary.value.trim() || petStore.profile.summary,
      reminderTtlMinutes: normalizedReminderTtlMinutes.value,
      quietHours: normalizedQuietHours.value,
    },
  })
  const saved = await userProfileStore.saveCurrentUserRuntimeConfig()
  if (!saved) {
    return
  }

  await Promise.all([
    petStore.loadSnapshot(undefined, undefined, true),
    petStore.loadDashboard(undefined, true),
  ])
}
</script>

<template>
  <div data-testid="personal-center-pet-view" class="space-y-6">
    <template v-if="userProfileStore.currentUser">
      <PetStatsPanel />
      <PetPreferencesPanel
        :configured-model-id="configuredModelId"
        :permission-mode="permissionMode"
        :display-name="displayName"
        :greeting="greeting"
        :summary="summary"
        :reminder-ttl-minutes="reminderTtlMinutes"
        :quiet-hours-enabled="quietHoursEnabled"
        :quiet-hours-start="quietHoursStart"
        :quiet-hours-end="quietHoursEnd"
        :model-options="modelOptions"
        :permission-options="permissionOptions"
        :runtime-source-path="runtimeSource?.displayPath ?? t('common.na')"
        :runtime-preview="runtimePreview"
        :saving="userProfileStore.runtimeSaving"
        :title="t('personalCenter.pet.title')"
        :description="t('personalCenter.pet.description')"
        :reminder-title="t('personalCenter.pet.preferences.reminderTitle')"
        :reminder-description="t('personalCenter.pet.preferences.reminderDescription')"
        :preview-title="t('personalCenter.pet.preferences.previewTitle')"
        @update:configured-model-id="updateConfiguredModelId"
        @update:permission-mode="updatePermissionMode"
        @update:display-name="updateDisplayName"
        @update:greeting="updateGreeting"
        @update:summary="updateSummary"
        @update:reminder-ttl-minutes="updateReminderTtlMinutes"
        @update:quiet-hours-enabled="updateQuietHoursEnabled"
        @update:quiet-hours-start="updateQuietHoursStart"
        @update:quiet-hours-end="updateQuietHoursEnd"
        @reset="resetForm"
        @save="savePetPreferences"
      />
    </template>

    <UiEmptyState
      v-else
      :title="t('personalCenter.pet.emptyTitle')"
      :description="t('personalCenter.pet.emptyDescription')"
    />
  </div>
</template>
