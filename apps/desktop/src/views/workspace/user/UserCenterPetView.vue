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
import { useUserCenterStore } from '@/stores/user-center'

const { t } = useI18n()
const userCenterStore = useUserCenterStore()
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
  userCenterStore.runtimeConfig?.sources.filter(source => source.scope === 'user').at(-1),
)
const modelOptions = computed(() => catalogStore.configuredModelOptions.map(option => ({
  value: option.value,
  label: `${option.label} · ${option.providerLabel}`,
})))
const permissionOptions = computed(() => [
  {
    value: 'read-only',
    label: t('userCenter.pet.permissionModes.readOnly'),
  },
  {
    value: 'workspace-write',
    label: t('userCenter.pet.permissionModes.workspaceWrite'),
  },
  {
    value: 'danger-full-access',
    label: t('userCenter.pet.permissionModes.dangerFullAccess'),
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
  () => [userCenterStore.runtimeConfig, petStore.profile, petStore.preferredConfiguredModelId, petStore.preferredPermissionMode, modelOptions.value.map(option => option.value).join('|')],
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
  () => userCenterStore.currentUser?.id ?? '',
  (userId) => {
    if (!userId) {
      return
    }
    void userCenterStore.loadCurrentUserRuntimeConfig()
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
  userCenterStore.setCurrentUserRuntimeDraft(JSON.stringify(patch, null, 2))
  await userCenterStore.saveCurrentUserRuntimeConfig()
  await petStore.loadSnapshot(undefined, undefined, true)
}
</script>

<template>
  <div data-testid="user-center-pet-view" class="space-y-6">
    <UiRecordCard
      v-if="userCenterStore.currentUser"
      :title="t('userCenter.pet.title')"
      :description="t('userCenter.pet.description')"
      test-id="user-center-pet-card"
    >
      <template #badges>
        <UiBadge :label="petStore.profile.displayName" subtle />
        <UiBadge :label="petStore.presence.isVisible ? t('userCenter.pet.visibility.visible') : t('userCenter.pet.visibility.hidden')" subtle />
      </template>

      <div class="grid gap-4 md:grid-cols-2">
        <UiField :label="t('userCenter.pet.fields.name')">
          <UiInput v-model="form.displayName" data-testid="user-center-pet-display-name" />
        </UiField>
        <UiField :label="t('userCenter.pet.fields.model')" :hint="t('userCenter.pet.hints.model')">
          <UiSelect v-model="form.configuredModelId" :options="modelOptions" data-testid="user-center-pet-model-select" />
        </UiField>
        <UiField class="md:col-span-2" :label="t('userCenter.pet.fields.greeting')">
          <UiTextarea v-model="form.greeting" :rows="3" data-testid="user-center-pet-greeting-input" />
        </UiField>
        <UiField class="md:col-span-2" :label="t('userCenter.pet.fields.summary')">
          <UiTextarea v-model="form.summary" :rows="4" data-testid="user-center-pet-summary-input" />
        </UiField>
        <UiField :label="t('userCenter.pet.fields.permissionMode')" :hint="t('userCenter.pet.hints.permissionMode')">
          <UiSelect v-model="form.permissionMode" :options="permissionOptions" data-testid="user-center-pet-permission-select" />
        </UiField>
        <UiField :label="t('userCenter.pet.fields.source')">
          <UiInput :model-value="runtimeSource?.displayPath ?? t('common.na')" disabled data-testid="user-center-pet-source-path" />
        </UiField>
      </div>

      <template #meta>
        <div class="grid gap-1 text-xs text-text-tertiary">
          <span>{{ t('userCenter.pet.preview.selectedModel') }}：{{ selectedModelLabel }}</span>
          <span>{{ t('userCenter.pet.preview.greeting') }}：{{ form.greeting || t('common.na') }}</span>
          <span>{{ t('userCenter.pet.preview.summary') }}：{{ form.summary || t('common.na') }}</span>
        </div>
      </template>
      <template #actions>
        <UiButton
          variant="ghost"
          size="sm"
          :disabled="userCenterStore.runtimeSaving"
          data-testid="user-center-pet-reset"
          @click="resetForm"
        >
          {{ t('userCenter.pet.actions.reset') }}
        </UiButton>
        <UiButton
          size="sm"
          :disabled="userCenterStore.runtimeSaving || !form.configuredModelId"
          :loading="userCenterStore.runtimeSaving"
          data-testid="user-center-pet-save"
          @click="savePetPreferences"
        >
          {{ t('userCenter.pet.actions.save') }}
        </UiButton>
      </template>
    </UiRecordCard>

    <UiRecordCard
      v-if="userCenterStore.currentUser"
      :title="t('userCenter.pet.runtime.title')"
      :description="t('userCenter.pet.runtime.description')"
      test-id="user-center-pet-runtime-preview"
    >
      <pre class="overflow-x-auto rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3 text-xs leading-6 text-text-secondary">{{ runtimePreview }}</pre>
    </UiRecordCard>

    <UiEmptyState
      v-if="!userCenterStore.currentUser"
      :title="t('userCenter.pet.emptyTitle')"
      :description="t('userCenter.pet.emptyDescription')"
    />
  </div>
</template>
