<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { UiButton, UiDialog, UiField, UiInput, UiStatusCallout } from '@octopus/ui'

import type {
  PendingSkillAction,
  PendingSkillCopyItem,
  PendingSkillImportSource,
} from './useToolsView'

defineProps<{
  open: boolean
  title: string
  description: string
  pendingSkillAction: PendingSkillAction
  pendingSkillImportSource: PendingSkillImportSource
  pendingSkillSelectionLabel: string
  pendingSkillImportTargets: Array<{ id: string, label: string, slug: string }>
  pendingSkillCopies: PendingSkillCopyItem[]
  panelError: string
  submitting: boolean
  pendingSkillActionReady: boolean
  skillDisplayPath: (skillId: string) => string
  suggestSlug: (value: string) => string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  importArchiveSkill: []
  importFolderSkill: []
  submit: []
}>()

const { t } = useI18n()
</script>

<template>
  <UiDialog
    :open="open"
    :title="title"
    :description="description"
    content-test-id="tools-skill-action-dialog"
    @update:open="emit('update:open', Boolean($event))"
  >
    <div class="space-y-4">
      <div
        v-if="pendingSkillAction === 'import'"
        class="space-y-3"
      >
        <div class="flex flex-wrap gap-2">
          <UiButton
            variant="ghost"
            :loading="submitting && pendingSkillImportSource === 'archive'"
            @click="emit('importArchiveSkill')"
          >
            {{ t('tools.actions.importArchive') }}
          </UiButton>
          <UiButton
            variant="ghost"
            :loading="submitting && pendingSkillImportSource === 'folder'"
            @click="emit('importFolderSkill')"
          >
            {{ t('tools.actions.importFolder') }}
          </UiButton>
        </div>

        <div v-if="pendingSkillSelectionLabel" class="space-y-1">
          <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
            {{ t('tools.editor.selectedImportSource') }}
          </div>
          <div class="space-y-1">
            <div
              v-for="item in pendingSkillImportTargets"
              :key="item.id"
              class="rounded-lg border border-border/40 bg-surface/70 px-3 py-2 text-[13px] text-text-secondary"
            >
              <div class="break-all text-text-primary">
                {{ item.label }}
              </div>
              <div class="break-all text-[12px] text-text-tertiary">
                {{ item.slug }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else-if="pendingSkillAction === 'copy'"
        class="space-y-3"
      >
        <div
          v-for="item in pendingSkillCopies"
          :key="item.skillId"
          :data-testid="`tools-skill-action-copy-item-${item.skillId}`"
          class="space-y-3 rounded-lg border border-border/40 bg-surface/70 px-3 py-3"
        >
          <div class="space-y-1">
            <div class="text-[13px] text-text-primary">
              {{ item.sourceName }}
            </div>
            <div class="break-all text-[12px] text-text-tertiary">
              {{ skillDisplayPath(item.skillId) }}
            </div>
          </div>

          <UiField :label="t('tools.editor.skillName')">
            <UiInput v-model="item.targetName" />
          </UiField>

          <div class="text-[12px] text-text-tertiary">
            {{ suggestSlug(item.targetName) }}
          </div>
        </div>
      </div>

      <UiStatusCallout v-if="panelError" tone="error" :description="panelError" />
    </div>

    <template #footer>
      <UiButton variant="ghost" @click="emit('update:open', false)">
        {{ t('common.cancel') }}
      </UiButton>
      <UiButton :loading="submitting" :disabled="!pendingSkillActionReady" @click="emit('submit')">
        {{ t('common.confirm') }}
      </UiButton>
    </template>
  </UiDialog>
</template>
