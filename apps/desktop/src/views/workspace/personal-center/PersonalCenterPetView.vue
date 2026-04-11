<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { RuntimePermissionMode } from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiInput,
  UiRecordCard,
  UiSelect,
  UiTextarea,
} from '@octopus/ui'

import { useCatalogStore } from '@/stores/catalog'
import { usePetStore } from '@/stores/pet'
import { useUserProfileStore } from '@/stores/user-profile'

const { t } = useI18n()
const userProfileStore = useUserProfileStore()
const catalogStore = useCatalogStore()
const petStore = usePetStore()

const form = reactive({
  configuredModelId: '',
  permissionMode: 'read-only' as RuntimePermissionMode,
  displayName: '',
  greeting: '',
  summary: '',
})

const runtimeSource = computed(() =>
  userProfileStore.runtimeConfig?.sources.filter(source => source.scope === 'user').at(-1),
)
const modelOptions = computed(() => catalogStore.configuredModelOptions.map(option => ({
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
const selectedModelLabel = computed(() =>
  modelOptions.value.find(option => option.value === form.configuredModelId)?.label ?? t('common.na'),
)
const runtimePreview = computed(() => JSON.stringify({
  pet: {
    configuredModelId: form.configuredModelId || null,
    permissionMode: form.permissionMode,
    displayName: form.displayName.trim() || petStore.profile.displayName,
    greeting: form.greeting.trim() || petStore.profile.greeting,
    summary: form.summary.trim() || petStore.profile.summary,
  },
}, null, 2))

watch(
  () => [userProfileStore.runtimeConfig, petStore.profile, petStore.preferredConfiguredModelId, petStore.preferredPermissionMode, modelOptions.value.map(option => option.value).join('|')],
  () => {
    form.configuredModelId = petStore.preferredConfiguredModelId || modelOptions.value[0]?.value || ''
    form.permissionMode = petStore.preferredPermissionMode
    form.displayName = petStore.profile.displayName
    form.greeting = petStore.profile.greeting
    form.summary = petStore.profile.summary
  },
  { immediate: true },
)

watch(
  () => userProfileStore.currentUser?.id ?? '',
  (userId) => {
    if (!userId) {
      return
    }
    void userProfileStore.loadCurrentUserRuntimeConfig()
    void catalogStore.load()
    void petStore.loadSnapshot()
  },
  { immediate: true },
)

function resetForm() {
  form.configuredModelId = petStore.preferredConfiguredModelId || modelOptions.value[0]?.value || ''
  form.permissionMode = petStore.preferredPermissionMode
  form.displayName = petStore.profile.displayName
  form.greeting = petStore.profile.greeting
  form.summary = petStore.profile.summary
}

async function savePetPreferences() {
  const patch = {
    pet: {
      configuredModelId: form.configuredModelId || null,
      permissionMode: form.permissionMode,
      displayName: form.displayName.trim() || petStore.profile.displayName,
      greeting: form.greeting.trim() || petStore.profile.greeting,
      summary: form.summary.trim() || petStore.profile.summary,
    },
  }
  userProfileStore.setCurrentUserRuntimeDraft(JSON.stringify(patch, null, 2))
  await userProfileStore.saveCurrentUserRuntimeConfig()
  await petStore.loadSnapshot(undefined, undefined, true)
}
</script>

<template>
  <div data-testid="personal-center-pet-view" class="space-y-6">
    <UiRecordCard
      v-if="userProfileStore.currentUser"
      :title="t('personalCenter.pet.title')"
      :description="t('personalCenter.pet.description')"
      test-id="personal-center-pet-card"
    >
      <template #badges>
        <UiBadge :label="petStore.profile.displayName" subtle />
        <UiBadge :label="petStore.presence.isVisible ? t('personalCenter.pet.visibility.visible') : t('personalCenter.pet.visibility.hidden')" subtle />
      </template>

      <div class="grid gap-4 md:grid-cols-2">
        <UiField :label="t('personalCenter.pet.fields.name')">
          <UiInput v-model="form.displayName" data-testid="personal-center-pet-display-name" />
        </UiField>
        <UiField :label="t('personalCenter.pet.fields.model')" :hint="t('personalCenter.pet.hints.model')">
          <UiSelect v-model="form.configuredModelId" :options="modelOptions" data-testid="personal-center-pet-model-select" />
        </UiField>
        <UiField class="md:col-span-2" :label="t('personalCenter.pet.fields.greeting')">
          <UiTextarea v-model="form.greeting" :rows="3" data-testid="personal-center-pet-greeting-input" />
        </UiField>
        <UiField class="md:col-span-2" :label="t('personalCenter.pet.fields.summary')">
          <UiTextarea v-model="form.summary" :rows="4" data-testid="personal-center-pet-summary-input" />
        </UiField>
        <UiField :label="t('personalCenter.pet.fields.permissionMode')" :hint="t('personalCenter.pet.hints.permissionMode')">
          <UiSelect v-model="form.permissionMode" :options="permissionOptions" data-testid="personal-center-pet-permission-select" />
        </UiField>
        <UiField :label="t('personalCenter.pet.fields.source')">
          <UiInput :model-value="runtimeSource?.displayPath ?? t('common.na')" disabled data-testid="personal-center-pet-source-path" />
        </UiField>
      </div>

      <template #meta>
        <div class="grid gap-1 text-xs text-text-tertiary">
          <span>{{ t('personalCenter.pet.preview.selectedModel') }}：{{ selectedModelLabel }}</span>
          <span>{{ t('personalCenter.pet.preview.greeting') }}：{{ form.greeting || t('common.na') }}</span>
          <span>{{ t('personalCenter.pet.preview.summary') }}：{{ form.summary || t('common.na') }}</span>
        </div>
      </template>
      <template #actions>
        <UiButton
          variant="ghost"
          size="sm"
          :disabled="userProfileStore.runtimeSaving"
          data-testid="personal-center-pet-reset"
          @click="resetForm"
        >
          {{ t('personalCenter.pet.actions.reset') }}
        </UiButton>
        <UiButton
          size="sm"
          :disabled="userProfileStore.runtimeSaving || !form.configuredModelId"
          :loading="userProfileStore.runtimeSaving"
          data-testid="personal-center-pet-save"
          @click="savePetPreferences"
        >
          {{ t('personalCenter.pet.actions.save') }}
        </UiButton>
      </template>
    </UiRecordCard>

    <UiRecordCard
      v-if="userProfileStore.currentUser"
      :title="t('personalCenter.pet.runtime.title')"
      :description="t('personalCenter.pet.runtime.description')"
      test-id="personal-center-pet-runtime-preview"
    >
      <pre class="overflow-x-auto rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3 text-xs leading-6 text-text-secondary">{{ runtimePreview }}</pre>
    </UiRecordCard>

    <UiEmptyState
      v-if="!userProfileStore.currentUser"
      :title="t('personalCenter.pet.emptyTitle')"
      :description="t('personalCenter.pet.emptyDescription')"
    />
  </div>
</template>
