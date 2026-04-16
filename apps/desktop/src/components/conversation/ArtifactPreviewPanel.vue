<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import type { DeliverableVersionContent, ResourcePreviewKind } from '@octopus/schema'

import {
  UiBadge,
  UiButton,
  UiCodeEditor,
  UiEmptyState,
  UiPanelFrame,
  UiStatusCallout,
  UiTextarea,
} from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'

type PreviewMode = 'markdown' | 'text' | 'code' | 'image' | 'file'

const props = withDefaults(defineProps<{
  content?: DeliverableVersionContent | null
  draft?: string
  editing?: boolean
  saving?: boolean
  error?: string
  saveStatus?: string
}>(), {
  content: null,
  draft: '',
  editing: false,
  saving: false,
  error: '',
  saveStatus: '',
})

const emit = defineEmits<{
  edit: []
  cancel: []
  save: []
  updateDraft: [value: string]
}>()

const { t } = useI18n()

const resolvedPreviewKind = computed<ResourcePreviewKind | null>(() => props.content?.previewKind ?? null)
const previewMode = computed<PreviewMode>(() => {
  if (!resolvedPreviewKind.value) {
    return 'file'
  }
  if (resolvedPreviewKind.value === 'image') {
    return 'image'
  }
  if (
    resolvedPreviewKind.value === 'code'
    || props.content?.contentType?.includes('json')
  ) {
    return 'code'
  }
  if (resolvedPreviewKind.value === 'markdown') {
    return 'markdown'
  }
  if (resolvedPreviewKind.value === 'text') {
    return 'text'
  }
  return 'file'
})
const previewLabel = computed(() =>
  resolvedPreviewKind.value ? enumLabel('resourcePreviewKind', resolvedPreviewKind.value) : t('common.na'),
)
const canInlineEdit = computed(() =>
  Boolean(props.content?.editable)
  && (previewMode.value === 'markdown' || previewMode.value === 'text' || previewMode.value === 'code'),
)
const editorLanguage = computed(() => {
  if (previewMode.value === 'markdown') {
    return 'markdown'
  }
  if (props.content?.contentType?.includes('json')) {
    return 'json'
  }
  if (previewMode.value === 'code') {
    return 'code'
  }
  return 'plaintext'
})
const previewSrc = computed(() => {
  if (previewMode.value !== 'image' || !props.content?.dataBase64) {
    return ''
  }
  return `data:${props.content.contentType || 'image/png'};base64,${props.content.dataBase64}`
})

function formatByteSize(byteSize?: number) {
  if (!byteSize || Number.isNaN(byteSize)) {
    return t('common.na')
  }
  if (byteSize < 1024) {
    return `${byteSize} B`
  }
  if (byteSize < 1024 * 1024) {
    return `${(byteSize / 1024).toFixed(1)} KB`
  }
  return `${(byteSize / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <UiPanelFrame
    data-testid="deliverable-preview-panel"
    variant="raised"
    padding="md"
    class="space-y-4"
  >
    <div class="flex items-center justify-between gap-3">
      <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">
        {{ t('conversation.detail.deliverables.previewTitle') }}
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <UiBadge :label="previewLabel" subtle />
        <span v-if="props.content?.fileName" class="text-xs text-text-tertiary">
          {{ props.content.fileName }}
        </span>
      </div>
    </div>

    <UiStatusCallout
      v-if="props.error"
      tone="error"
      :description="props.error"
    />

    <UiTextarea
      v-if="props.editing && canInlineEdit"
      data-testid="deliverable-editor"
      :model-value="props.draft"
      :rows="18"
      class="min-h-[20rem] border-border bg-background font-mono text-[13px] leading-6"
      @update:model-value="emit('updateDraft', $event)"
    />

    <div
      v-else-if="previewMode === 'markdown'"
      class="min-h-[20rem] whitespace-pre-wrap rounded-[var(--radius-l)] border border-border bg-background px-4 py-4 text-[14px] leading-7 text-text-primary"
    >
      {{ props.content?.textContent || '' }}
    </div>

    <div
      v-else-if="previewMode === 'text'"
      class="min-h-[20rem] whitespace-pre-wrap rounded-[var(--radius-l)] border border-border bg-background px-4 py-4 font-mono text-[13px] leading-6 text-text-primary"
    >
      {{ props.content?.textContent || '' }}
    </div>

    <UiCodeEditor
      v-else-if="previewMode === 'code'"
      readonly
      :language="editorLanguage"
      :model-value="props.content?.textContent || ''"
    />

    <div
      v-else-if="previewMode === 'image' && previewSrc"
      class="overflow-hidden rounded-[var(--radius-l)] border border-border bg-background p-2"
    >
      <img
        :src="previewSrc"
        :alt="props.content?.fileName || t('conversation.detail.deliverables.previewTitle')"
        class="max-h-[420px] w-full object-contain"
      >
    </div>

    <UiEmptyState
      v-else
      :title="t('conversation.detail.deliverables.previewUnavailableTitle')"
      :description="t('conversation.detail.deliverables.previewUnavailableDescription')"
    />

    <div class="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-3">
      <div class="min-w-0 text-xs text-text-tertiary">
        <span v-if="props.saveStatus">{{ props.saveStatus }}</span>
        <span v-else>
          {{ t('conversation.detail.deliverables.previewMeta', {
            size: formatByteSize(props.content?.byteSize),
            contentType: props.content?.contentType || t('common.na'),
          }) }}
        </span>
      </div>

      <div class="flex flex-wrap gap-2">
        <UiButton
          v-if="!props.editing && canInlineEdit"
          data-testid="deliverable-edit-button"
          variant="outline"
          size="sm"
          @click="emit('edit')"
        >
          {{ t('common.edit') }}
        </UiButton>

        <template v-if="props.editing">
          <UiButton
            data-testid="deliverable-save-version"
            size="sm"
            :disabled="props.saving"
            @click="emit('save')"
          >
            {{ t('conversation.detail.deliverables.saveVersion') }}
          </UiButton>
          <UiButton
            data-testid="deliverable-cancel-edit"
            variant="ghost"
            size="sm"
            :disabled="props.saving"
            @click="emit('cancel')"
          >
            {{ t('common.cancel') }}
          </UiButton>
        </template>
      </div>
    </div>
  </UiPanelFrame>
</template>
