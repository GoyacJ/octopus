<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type {
  WorkspaceSkillFileDocument,
  WorkspaceToolCatalogEntry,
} from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCodeEditor,
  UiEmptyState,
  UiStatusCallout,
  UiSwitch,
} from '@octopus/ui'

import type { SkillTreeRow } from './useToolsView'

const props = defineProps<{
  entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>
  loadingDetail: boolean
  loadingSkillFile: boolean
  selectedSkillTreeRows: SkillTreeRow[]
  selectedSkillFilePath: string
  currentSkillFile: WorkspaceSkillFileDocument | null
  canSaveSkillFile: boolean
  canCopySkillToManaged: boolean
  skillFileDraft: string
  panelError: string
  submitting: boolean
  deleting: boolean
  toggling: boolean
  availabilityLabel: (availability: WorkspaceToolCatalogEntry['availability']) => string
  availabilityTone: (availability: WorkspaceToolCatalogEntry['availability']) => 'default' | 'success' | 'warning'
  skillStateLabel: (entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>) => string
  sourceOriginLabel: (entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>) => string
  fileTypeLabel: (file: WorkspaceSkillFileDocument | null) => string
}>()

const emit = defineEmits<{
  'update:skillFileDraft': [value: string]
  selectSkillFile: [path: string]
  toggleDisabled: [disabled: boolean]
  save: []
  delete: []
  copyToManaged: []
}>()

const { t } = useI18n()
</script>

<template>
  <div class="space-y-4">
    <div class="grid gap-3 border-b border-border/40 pb-4 sm:grid-cols-[minmax(0,1fr)_auto]">
      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.tabs.skill') }}
        </div>
        <div class="text-[12px] text-text-secondary">
          {{ entry.description }}
        </div>
      </div>

      <div class="flex min-h-10 min-w-[196px] flex-wrap content-start justify-end gap-1.5">
        <UiBadge :label="availabilityLabel(entry.availability)" :tone="availabilityTone(entry.availability)" />
        <UiBadge v-if="entry.disabled" :label="t('tools.states.disabled')" tone="warning" />
        <UiBadge :label="skillStateLabel(entry)" subtle />
        <UiBadge v-if="entry.workspaceOwned" :label="t('tools.states.managed')" subtle />
        <UiBadge v-else :label="t('tools.states.readonly')" subtle />
        <UiBadge v-if="!entry.workspaceOwned" :label="t('tools.states.external')" subtle />
      </div>
    </div>

    <div class="space-y-3">
      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.sourcePath') }}
        </div>
        <div class="break-all font-mono text-[12px] text-text-secondary">
          {{ entry.displayPath }}
        </div>
      </div>

      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.sourceKey') }}
        </div>
        <div class="break-all font-mono text-[12px] text-text-secondary">
          {{ entry.sourceKey }}
        </div>
      </div>

      <div v-if="entry.ownerLabel" class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('common.owner') }}
        </div>
        <div class="text-[13px] text-text-primary">
          {{ entry.ownerLabel }}
        </div>
      </div>

      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.disabled') }}
        </div>
        <UiSwitch
          :model-value="entry.disabled"
          :disabled="toggling || !entry.management.canDisable"
          :label="t('tools.actions.disable')"
          @update:model-value="emit('toggleDisabled', Boolean($event))"
        />
      </div>
    </div>

    <div class="space-y-3">
      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.activeState') }}
        </div>
        <div class="text-[13px] text-text-primary">
          {{ skillStateLabel(entry) }}
        </div>
      </div>

      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.sourceOrigin') }}
        </div>
        <div class="text-[13px] text-text-primary">
          {{ sourceOriginLabel(entry) }}
        </div>
      </div>

      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.workspaceOwned') }}
        </div>
        <div class="text-[13px] text-text-primary">
          {{ entry.workspaceOwned ? t('tools.detail.workspaceOwnedYes') : t('tools.detail.workspaceOwnedNo') }}
        </div>
      </div>

      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.relativePath') }}
        </div>
        <div class="break-all font-mono text-[12px] text-text-secondary">
          {{ entry.relativePath ?? t('common.na') }}
        </div>
      </div>

      <div v-if="entry.consumers?.length" class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          使用者
        </div>
        <div class="flex flex-wrap gap-1.5">
          <UiBadge
            v-for="consumer in entry.consumers"
            :key="`${entry.id}-${consumer.kind}-${consumer.id}`"
            :label="consumer.name"
            subtle
          />
        </div>
      </div>
    </div>

    <div v-if="entry.shadowedBy" class="space-y-1">
      <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
        {{ t('tools.detail.shadowedBy') }}
      </div>
      <div class="text-[13px] text-text-primary">
        {{ entry.shadowedBy }}
      </div>
    </div>

    <div v-if="loadingDetail" class="rounded-md border border-border/40 bg-subtle/20 px-3 py-3 text-[12px] text-text-secondary">
      {{ t('tools.editor.loading') }}
    </div>

    <div v-else class="space-y-4">
      <div class="rounded-xl border border-border/40 bg-surface/80 p-2">
        <div class="mb-2 px-2 text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.editor.skillTree') }}
        </div>

        <div v-if="selectedSkillTreeRows.length" class="space-y-1">
          <template
            v-for="row in selectedSkillTreeRows"
            :key="row.path"
          >
            <div
              v-if="row.kind === 'directory'"
              class="flex w-full flex-col items-start gap-1 rounded-lg px-2 py-2 text-left text-[12px] text-text-secondary"
              :style="{ paddingInlineStart: `${row.depth * 14 + 12}px` }"
            >
              <span class="font-mono">{{ row.name }}</span>
              <span class="break-all text-[11px] text-text-tertiary">
                {{ row.path }}
              </span>
            </div>
            <UiButton
              v-else
              variant="ghost"
              :class="`h-auto w-full items-start justify-start rounded-lg px-2 py-2 text-left text-[12px] font-normal text-text-primary transition hover:bg-subtle/60 ${selectedSkillFilePath === row.path ? 'bg-subtle/80' : ''}`"
              :style="{ paddingInlineStart: `${row.depth * 14 + 12}px` }"
              @click="emit('selectSkillFile', row.path)"
            >
              <span class="flex w-full flex-col items-start gap-1">
                <span class="font-mono">{{ row.name }}</span>
                <span class="break-all text-[11px] text-text-tertiary">
                  {{ row.path }}
                </span>
              </span>
            </UiButton>
          </template>
        </div>

        <UiEmptyState
          v-else
          :title="t('tools.empty.selectionTitle')"
          :description="t('tools.editor.noSkillFiles')"
        />
      </div>

      <div class="space-y-4">
        <div
          v-if="loadingSkillFile"
          class="rounded-md border border-border/40 bg-subtle/20 px-3 py-3 text-[12px] text-text-secondary"
        >
          {{ t('tools.editor.loadingFile') }}
        </div>

        <template v-else-if="currentSkillFile">
          <div class="space-y-2 rounded-xl border border-border/40 bg-surface/80 px-4 py-3">
            <div class="space-y-3">
              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.editor.selectedFile') }}
                </div>
                <div class="break-all font-mono text-[12px] text-text-primary">
                  {{ currentSkillFile.path }}
                </div>
              </div>

              <div class="space-y-1">
                <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                  {{ t('tools.detail.sourcePath') }}
                </div>
                <div class="break-all text-[12px] text-text-secondary">
                  {{ currentSkillFile.displayPath }}
                </div>
              </div>

              <div class="space-y-3">
                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.editor.fileKind') }}
                  </div>
                  <div class="text-[12px] text-text-secondary">
                    {{ fileTypeLabel(currentSkillFile) }}
                  </div>
                </div>

                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.editor.fileLanguage') }}
                  </div>
                  <div class="text-[12px] text-text-secondary">
                    {{ currentSkillFile.language || t('common.na') }}
                  </div>
                </div>

                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.editor.fileSize') }}
                  </div>
                  <div class="text-[12px] text-text-secondary">
                    {{ currentSkillFile.byteSize }} B
                  </div>
                </div>
              </div>

              <div
                v-if="currentSkillFile.readonly"
                class="text-[12px] text-text-tertiary"
              >
                {{ t('tools.states.readonly') }}
              </div>
            </div>
          </div>

          <UiCodeEditor
            v-if="currentSkillFile.isText"
            :language="currentSkillFile.language || 'markdown'"
            theme="octopus"
            :readonly="!canSaveSkillFile"
            :model-value="skillFileDraft"
            @update:model-value="emit('update:skillFileDraft', $event)"
          />

          <div
            v-else
            class="rounded-xl border border-border/40 bg-surface/80 px-4 py-4"
          >
            <div class="space-y-2 text-[13px] text-text-secondary">
              <div>{{ t('tools.editor.binaryReadonly') }}</div>
              <div>{{ currentSkillFile.contentType || 'application/octet-stream' }}</div>
            </div>
          </div>
        </template>

        <UiEmptyState
          v-else
          :title="t('tools.editor.noSkillFileSelectedTitle')"
          :description="t('tools.editor.noSkillFileSelectedDescription')"
        />
      </div>
    </div>

    <UiStatusCallout v-if="panelError" tone="error" :description="panelError" />

    <div class="flex flex-wrap gap-2">
      <UiButton
        v-if="canSaveSkillFile"
        :loading="submitting"
        @click="emit('save')"
      >
        {{ t('common.save') }}
      </UiButton>
      <UiButton
        v-if="entry.management.canDelete"
        variant="ghost"
        :loading="deleting"
        @click="emit('delete')"
      >
        {{ t('common.delete') }}
      </UiButton>
      <UiButton
        v-if="canCopySkillToManaged"
        variant="ghost"
        :loading="submitting"
        @click="emit('copyToManaged')"
      >
        {{ t('tools.actions.copyToManaged') }}
      </UiButton>
    </div>
  </div>
</template>
