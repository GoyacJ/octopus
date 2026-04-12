<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import type { CapabilityManagementEntry } from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCodeEditor,
  UiField,
  UiInput,
  UiStatusCallout,
  UiSwitch,
} from '@octopus/ui'

const props = defineProps<{
  entry: Extract<CapabilityManagementEntry, { kind: 'mcp' }>
  loadingDetail: boolean
  mcpServerNameDraft: string
  mcpConfigDraft: string
  panelError: string
  submitting: boolean
  deleting: boolean
  toggling: boolean
  canCopyMcpToManaged: boolean
  availabilityLabel: (availability: CapabilityManagementEntry['availability']) => string
  availabilityTone: (availability: CapabilityManagementEntry['availability']) => 'default' | 'success' | 'warning'
  ownerScopeLabel: (ownerScope: CapabilityManagementEntry['ownerScope']) => string
}>()

const emit = defineEmits<{
  'update:mcpConfigDraft': [value: string]
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
          {{ t('tools.tabs.mcp') }}
        </div>
        <div class="text-[12px] text-text-secondary">
          {{ entry.description }}
        </div>
      </div>

      <div class="flex min-h-10 min-w-[196px] flex-wrap content-start justify-end gap-1.5">
        <UiBadge :label="availabilityLabel(entry.availability)" :tone="availabilityTone(entry.availability)" />
        <UiBadge v-if="entry.disabled" :label="t('tools.states.disabled')" tone="warning" />
        <UiBadge v-if="entry.toolNames.length" :label="`${entry.toolNames.length} tools`" subtle />
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

      <div v-if="entry.ownerScope || entry.ownerLabel" class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.source') }}
        </div>
        <div class="flex flex-wrap gap-1.5">
          <UiBadge
            v-if="entry.ownerScope"
            :label="ownerScopeLabel(entry.ownerScope)"
            subtle
          />
          <UiBadge
            v-if="entry.ownerLabel"
            :label="entry.ownerLabel"
            subtle
          />
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
          {{ t('tools.detail.serverName') }}
        </div>
        <div class="text-[13px] text-text-primary">
          {{ entry.serverName }}
        </div>
      </div>

      <div class="space-y-1">
        <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
          {{ t('tools.detail.scope') }}
        </div>
        <div class="text-[13px] text-text-primary">
          {{ entry.scope }}
        </div>
      </div>
    </div>

    <div class="space-y-1">
      <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
        {{ t('tools.detail.endpoint') }}
      </div>
      <div class="break-all font-mono text-[12px] text-text-secondary">
        {{ entry.endpoint }}
      </div>
    </div>

    <div class="space-y-1">
      <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
        {{ t('tools.detail.toolNames') }}
      </div>
      <div
        v-if="entry.toolNames.length"
        class="flex flex-wrap gap-1.5"
      >
        <UiBadge
          v-for="toolName in entry.toolNames"
          :key="toolName"
          :label="toolName"
          subtle
        />
      </div>
      <div v-else class="text-[13px] text-text-secondary">
        {{ t('common.na') }}
      </div>
    </div>

    <div v-if="entry.consumers?.length" class="space-y-1">
      <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
        {{ t('tools.detail.consumers') }}
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

    <div v-if="entry.statusDetail" class="space-y-1">
      <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
        {{ t('tools.detail.statusDetail') }}
      </div>
      <div class="text-[13px] text-status-warning">
        {{ entry.statusDetail }}
      </div>
    </div>

    <div v-if="loadingDetail" class="rounded-md border border-border/40 bg-subtle/20 px-3 py-3 text-[12px] text-text-secondary">
      {{ t('tools.editor.loading') }}
    </div>

    <template v-else>
      <UiField :label="t('tools.editor.mcpServerName')">
        <UiInput :model-value="mcpServerNameDraft" disabled />
      </UiField>

      <UiField :label="t('tools.editor.mcpConfig')">
        <UiCodeEditor
          language="json"
          theme="octopus"
          :readonly="!entry.management.canEdit"
          :model-value="mcpConfigDraft"
          @update:model-value="emit('update:mcpConfigDraft', $event)"
        />
      </UiField>
    </template>

    <UiStatusCallout v-if="panelError" tone="error" :description="panelError" />

    <div v-if="canCopyMcpToManaged || entry.management.canEdit || entry.management.canDelete" class="flex gap-2">
      <UiButton v-if="canCopyMcpToManaged" :loading="submitting" @click="emit('copyToManaged')">
        {{ t('tools.actions.copyToManaged') }}
      </UiButton>
      <UiButton v-if="entry.management.canEdit" :loading="submitting" @click="emit('save')">
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
    </div>
  </div>
</template>
