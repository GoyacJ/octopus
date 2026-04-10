<script setup lang="ts">
import { UiBadge, UiButton, UiField, UiRecordCard, UiStatusCallout, UiTextarea, UiInput } from '@octopus/ui'

defineProps<{
  basicsForm: { name: string, description: string }
  statusLabel: string
  badgeTone: 'warning' | 'success'
  basicsError: string
  savingBasics: boolean
}>()

const emit = defineEmits<{
  reset: []
  save: []
}>()
</script>

<template>
  <UiRecordCard
    :title="$t('projectSettings.basics.title')"
    :description="$t('projectSettings.basics.description')"
  >
    <template #eyebrow>
      {{ $t('projectSettings.tabs.basics') }}
    </template>
    <template #badges>
      <UiBadge :label="statusLabel" :tone="badgeTone" />
    </template>

    <div class="space-y-4">
      <UiField :label="$t('projects.fields.name')">
        <UiInput
          v-model="basicsForm.name"
          data-testid="project-settings-name-input"
          :placeholder="$t('sidebar.projectTree.inputPlaceholder')"
        />
      </UiField>

      <UiField :label="$t('projects.fields.description')">
        <UiTextarea
          v-model="basicsForm.description"
          data-testid="project-settings-description-input"
          :rows="8"
        />
      </UiField>

      <UiStatusCallout v-if="basicsError" tone="error" :description="basicsError" />
    </div>

    <template #actions>
      <UiButton variant="ghost" :disabled="savingBasics" @click="emit('reset')">
        {{ $t('common.reset') }}
      </UiButton>
      <UiButton
        data-testid="project-settings-save-button"
        :disabled="savingBasics || !basicsForm.name.trim()"
        @click="emit('save')"
      >
        {{ $t('common.save') }}
      </UiButton>
    </template>
  </UiRecordCard>
</template>
