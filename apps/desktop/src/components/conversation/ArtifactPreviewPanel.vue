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
  <UiSurface
    data-testid="deliverable-preview-panel"
    variant="glass"
    padding="md"
    class="space-y-5 border-primary/20 highlight-border shadow-2xl"
  >
    <div class="flex items-center justify-between gap-4">
      <div class="flex items-center gap-2.5">
         <div class="size-2 rounded-full bg-primary shadow-[0_0_8px_var(--color-primary)] animate-pulse" />
         <div class="text-[11px] font-extrabold uppercase tracking-[0.2em] text-text-primary">
           {{ t('conversation.detail.deliverables.previewTitle') }}
         </div>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <UiBadge :label="previewLabel" class="bg-primary/10 text-primary border-primary/20 text-[9px] font-bold" />
        <span v-if="props.content?.fileName" class="text-[11px] font-mono text-text-tertiary bg-black/20 px-1.5 py-0.5 rounded">
          {{ props.content.fileName }}
        </span>
      </div>
    </div>

    <UiStatusCallout
      v-if="props.error"
      tone="error"
      class="bg-status-error/5 border-status-error/20"
      :description="props.error"
    />

    <div class="relative group/preview">
      <UiTextarea
        v-if="props.editing && canInlineEdit"
        data-testid="deliverable-editor"
        :model-value="props.draft"
        :rows="18"
        class="min-h-[20rem] border-primary/30 bg-black/40 font-mono text-[13px] leading-relaxed text-primary shadow-inner rounded-xl focus:ring-1 focus:ring-primary/50"
        @update:model-value="emit('updateDraft', $event)"
      />

      <div
        v-else-if="previewMode === 'markdown'"
        class="min-h-[22rem] whitespace-pre-wrap rounded-xl border border-border/30 bg-black/20 px-6 py-6 text-[14.5px] leading-relaxed text-text-primary/90 shadow-inner overflow-y-auto scroll-y"
      >
        {{ props.content?.textContent || '' }}
      </div>

      <div
        v-else-if="previewMode === 'text'"
        class="min-h-[22rem] whitespace-pre-wrap rounded-xl border border-border/30 bg-black/30 px-6 py-6 font-mono text-[13px] leading-relaxed text-text-secondary shadow-inner overflow-y-auto scroll-y"
      >
        {{ props.content?.textContent || '' }}
      </div>

      <UiCodeEditor
        v-else-if="previewMode === 'code'"
        readonly
        :language="editorLanguage"
        :model-value="props.content?.textContent || ''"
        class="min-h-[22rem] rounded-xl border border-border/30 overflow-hidden shadow-2xl"
      />

      <div
        v-else-if="previewMode === 'image' && previewSrc"
        class="overflow-hidden rounded-xl border border-border/30 bg-black/40 p-3 shadow-2xl"
      >
        <img
          :src="previewSrc"
          :alt="props.content?.fileName || t('conversation.detail.deliverables.previewTitle')"
          class="max-h-[480px] w-full object-contain transition-transform group-hover/preview:scale-[1.01]"
        >
      </div>

      <UiEmptyState
        v-else
        :title="t('conversation.detail.deliverables.previewUnavailableTitle')"
        :description="t('conversation.detail.deliverables.previewUnavailableDescription')"
        class="bg-black/10 py-16 rounded-xl"
      />
    </div>

    <div class="flex flex-wrap items-center justify-between gap-4 border-t border-border/30 pt-4">
      <div class="min-w-0 text-[10px] font-bold uppercase tracking-widest text-text-tertiary/60">
        <span v-if="props.saveStatus" class="text-primary animate-pulse">{{ props.saveStatus }}</span>
        <span v-else class="flex items-center gap-2">
           <span class="size-1 rounded-full bg-text-tertiary/40" />
           {{ formatByteSize(props.content?.byteSize) }}
           <span class="mx-1 opacity-50">|</span>
           {{ props.content?.contentType || 'DOCUMENT' }}
        </span>
      </div>

      <div class="flex flex-wrap gap-2">
        <UiButton
          v-if="!props.editing && canInlineEdit"
          data-testid="deliverable-edit-button"
          variant="outline"
          size="sm"
          class="bg-surface/50 border-border/50 text-[11px] font-bold uppercase tracking-wider"
          @click="emit('edit')"
        >
          {{ t('common.edit') }}
        </UiButton>

        <template v-if="props.editing">
          <UiButton
            data-testid="deliverable-save-version"
            size="sm"
            class="shadow-lg shadow-primary/20 text-[11px] font-bold uppercase tracking-wider px-4"
            :disabled="props.saving"
            @click="emit('save')"
          >
            {{ t('conversation.detail.deliverables.saveVersion') }}
          </UiButton>
          <UiButton
            data-testid="deliverable-cancel-edit"
            variant="ghost"
            size="sm"
            class="text-[11px] font-bold uppercase tracking-wider hover:bg-black/20"
            :disabled="props.saving"
            @click="emit('cancel')"
          >
            {{ t('common.cancel') }}
          </UiButton>
        </template>
      </div>
    </div>
  </UiSurface>
</template>
