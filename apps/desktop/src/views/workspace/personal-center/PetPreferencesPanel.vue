<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type { RuntimePermissionMode } from '@octopus/schema'
import {
  UiButton,
  UiField,
  UiInput,
  UiPanelFrame,
  UiSelect,
  UiSwitch,
  UiTextarea,
} from '@octopus/ui'

interface SelectOption {
  value: string
  label: string
}

const props = defineProps<{
  configuredModelId: string
  permissionMode: RuntimePermissionMode
  displayName: string
  greeting: string
  summary: string
  reminderTtlMinutes: string
  quietHoursEnabled: boolean
  quietHoursStart: string
  quietHoursEnd: string
  modelOptions: SelectOption[]
  permissionOptions: SelectOption[]
  runtimeSourcePath: string
  runtimePreview: string
  saving: boolean
  title: string
  description: string
  reminderTitle: string
  reminderDescription: string
  previewTitle: string
}>()

const { t } = useI18n()

const emit = defineEmits<{
  'update:configuredModelId': [value: string]
  'update:permissionMode': [value: RuntimePermissionMode]
  'update:displayName': [value: string]
  'update:greeting': [value: string]
  'update:summary': [value: string]
  'update:reminderTtlMinutes': [value: string]
  'update:quietHoursEnabled': [value: boolean]
  'update:quietHoursStart': [value: string]
  'update:quietHoursEnd': [value: string]
  reset: []
  save: []
}>()
</script>

<template>
  <section data-testid="personal-center-pet-preferences-panel">
    <UiPanelFrame
      variant="panel"
      padding="md"
      :title="props.title"
      :subtitle="props.description"
    >
      <template #actions>
        <div class="flex items-center gap-2">
          <UiButton
            variant="ghost"
            size="sm"
            :disabled="props.saving"
            data-testid="personal-center-pet-reset"
            @click="emit('reset')"
          >
            {{ t('common.reset') }}
          </UiButton>
          <UiButton
            size="sm"
            :disabled="props.saving || !props.configuredModelId"
            :loading="props.saving"
            data-testid="personal-center-pet-save"
            @click="emit('save')"
          >
            {{ t('personalCenter.pet.actions.save') }}
          </UiButton>
        </div>
      </template>

      <div class="grid gap-4 md:grid-cols-2">
        <UiField :label="t('personalCenter.pet.fields.name')">
          <UiInput
            :model-value="props.displayName"
            data-testid="personal-center-pet-display-name"
            @update:model-value="emit('update:displayName', $event)"
          />
        </UiField>

        <UiField
          :label="t('personalCenter.pet.fields.model')"
          :hint="t('personalCenter.pet.hints.model')"
        >
          <UiSelect
            :model-value="props.configuredModelId"
            :options="props.modelOptions"
            data-testid="personal-center-pet-model-select"
            @update:model-value="emit('update:configuredModelId', $event)"
          />
        </UiField>

        <UiField class="md:col-span-2" :label="t('personalCenter.pet.fields.greeting')">
          <UiTextarea
            :model-value="props.greeting"
            :rows="3"
            data-testid="personal-center-pet-greeting-input"
            @update:model-value="emit('update:greeting', $event)"
          />
        </UiField>

        <UiField class="md:col-span-2" :label="t('personalCenter.pet.fields.summary')">
          <UiTextarea
            :model-value="props.summary"
            :rows="4"
            data-testid="personal-center-pet-summary-input"
            @update:model-value="emit('update:summary', $event)"
          />
        </UiField>

        <UiField
          :label="t('personalCenter.pet.fields.permissionMode')"
          :hint="t('personalCenter.pet.hints.permissionMode')"
        >
          <UiSelect
            :model-value="props.permissionMode"
            :options="props.permissionOptions"
            data-testid="personal-center-pet-permission-select"
            @update:model-value="emit('update:permissionMode', $event as RuntimePermissionMode)"
          />
        </UiField>

        <UiField :label="t('personalCenter.pet.fields.source')">
          <UiInput
            :model-value="props.runtimeSourcePath"
            disabled
            data-testid="personal-center-pet-source-path"
          />
        </UiField>
      </div>

      <div class="mt-6 space-y-4 border-t border-border pt-4">
        <div class="space-y-1">
          <h3 class="text-sm font-semibold text-text-primary">
            {{ props.reminderTitle }}
          </h3>
          <p class="text-sm text-text-secondary">
            {{ props.reminderDescription }}
          </p>
        </div>

        <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          <UiField :label="t('personalCenter.pet.preferences.fields.reminderTtlMinutes')">
            <UiInput
              :model-value="props.reminderTtlMinutes"
              type="number"
              min="1"
              step="1"
              data-testid="personal-center-pet-reminder-ttl"
              @update:model-value="emit('update:reminderTtlMinutes', $event)"
            />
          </UiField>

          <UiField :label="t('personalCenter.pet.preferences.fields.quietHoursEnabled')">
            <UiSwitch
              :model-value="props.quietHoursEnabled"
              data-testid="personal-center-pet-quiet-hours-enabled"
              @update:model-value="emit('update:quietHoursEnabled', $event)"
            />
          </UiField>

          <UiField :label="t('personalCenter.pet.preferences.fields.quietHoursStart')">
            <UiInput
              :model-value="props.quietHoursStart"
              type="time"
              data-testid="personal-center-pet-quiet-hours-start"
              @update:model-value="emit('update:quietHoursStart', $event)"
            />
          </UiField>

          <UiField :label="t('personalCenter.pet.preferences.fields.quietHoursEnd')">
            <UiInput
              :model-value="props.quietHoursEnd"
              type="time"
              data-testid="personal-center-pet-quiet-hours-end"
              @update:model-value="emit('update:quietHoursEnd', $event)"
            />
          </UiField>
        </div>
      </div>

      <div class="mt-6 space-y-2 border-t border-border pt-4">
        <h3 class="text-sm font-semibold text-text-primary">
          {{ props.previewTitle }}
        </h3>
        <pre class="overflow-x-auto rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3 text-xs leading-6 text-text-secondary">{{ props.runtimePreview }}</pre>
      </div>
    </UiPanelFrame>
  </section>
</template>
