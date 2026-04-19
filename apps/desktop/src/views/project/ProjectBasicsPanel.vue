<script setup lang="ts">
import { UiBadge, UiButton, UiField, UiRecordCard, UiSelect, UiStatusCallout, UiTextarea, UiInput } from '@octopus/ui'

import ProjectResourceDirectoryField from '@/components/projects/ProjectResourceDirectoryField.vue'

defineProps<{
  basicsForm: {
    name: string
    description: string
    resourceDirectory: string
    managerUserId: string
    presetCode: string
  }
  managerOptions: Array<{ value: string, label: string }>
  presetOptions: Array<{ value: string, label: string }>
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
      <div class="grid gap-4 lg:grid-cols-2">
        <UiField :label="$t('projects.fields.name')">
          <UiInput
            v-model="basicsForm.name"
            data-testid="project-settings-name-input"
            :placeholder="$t('sidebar.projectTree.inputPlaceholder')"
          />
        </UiField>

        <ProjectResourceDirectoryField
          v-model="basicsForm.resourceDirectory"
          path-test-id="project-settings-resource-directory-path"
          pick-test-id="project-settings-resource-directory-pick"
        />
      </div>

      <UiField :label="$t('projects.fields.description')">
        <UiTextarea
          v-model="basicsForm.description"
          data-testid="project-settings-description-input"
          :rows="6"
        />
      </UiField>

      <div class="grid gap-4 lg:grid-cols-2">
        <UiField :label="$t('projects.manager.label')">
          <UiSelect
            v-model="basicsForm.managerUserId"
            data-testid="project-settings-manager-select"
            :options="managerOptions"
          />
        </UiField>

        <UiField
          :label="$t('projects.fields.preset')"
          :hint="$t('projectSettings.basics.presetHint')"
        >
          <UiSelect
            v-model="basicsForm.presetCode"
            data-testid="project-settings-preset-select"
            :options="presetOptions"
          />
        </UiField>
      </div>

      <UiStatusCallout v-if="basicsError" tone="error" :description="basicsError" />
    </div>

    <template #actions>
      <UiButton variant="ghost" :disabled="savingBasics" @click="emit('reset')">
        {{ $t('common.reset') }}
      </UiButton>
      <UiButton
        data-testid="project-settings-save-button"
        :disabled="savingBasics || !basicsForm.name.trim() || !basicsForm.resourceDirectory.trim()"
        @click="emit('save')"
      >
        {{ $t('common.save') }}
      </UiButton>
    </template>
  </UiRecordCard>
</template>
