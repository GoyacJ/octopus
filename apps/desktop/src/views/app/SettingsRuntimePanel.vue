<script setup lang="ts">
import type { RuntimeConfigSource } from '@octopus/schema'
import { UiBadge, UiButton, UiCodeEditor, UiEmptyState, UiRecordCard, UiStatusCallout } from '@octopus/ui'

defineProps<{
  runtime: any
  workspaceRuntimeSource?: RuntimeConfigSource
  workspaceRuntimeDraft: string
  runtimeEffectivePreview: string
  runtimeSecretStatuses: Array<{ scope: string, path: string, status: string }>
  resolveValidationTone: (validation?: any) => 'default' | 'success' | 'warning' | 'error' | 'info'
  resolveValidationLabel: (validation?: any) => string
  resolveSourceStatusLabel: (source?: RuntimeConfigSource) => string
  resolveSourceStatusTone: (source?: RuntimeConfigSource) => 'default' | 'success' | 'warning' | 'error' | 'info'
}>()

const emit = defineEmits<{
  reload: []
  validate: []
  save: []
}>()
</script>

<template>
  <section class="space-y-8">
    <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
      <div class="space-y-1">
        <h3 class="text-xl font-bold text-text-primary">{{ $t('settings.runtime.title') }}</h3>
        <p class="max-w-3xl text-[14px] text-text-secondary">
          {{ $t('settings.runtime.subtitle') }}
        </p>
      </div>
      <UiButton
        variant="ghost"
        size="sm"
        class="text-text-secondary hover:text-text-primary transition-colors"
        @click="emit('reload')"
      >
        {{ $t('settings.runtime.actions.reload') }}
      </UiButton>
    </div>

    <UiStatusCallout
      v-if="runtime.configLoading && !runtime.config"
      :description="$t('settings.runtime.loading')"
    />

    <UiEmptyState
      v-else-if="!runtime.config"
      :title="$t('settings.runtime.emptyTitle')"
      :description="runtime.configError || $t('settings.runtime.emptyDescription')"
    />

    <div v-else class="grid gap-6 xl:grid-cols-[minmax(0,1.3fr)_minmax(22rem,0.9fr)]">
      <div class="space-y-4">
        <UiRecordCard
          :title="$t('settings.runtime.workspace.title')"
          :description="$t('settings.runtime.workspace.description')"
          test-id="settings-runtime-editor-workspace"
        >
          <template #eyebrow>
            workspace
          </template>
          <template #badges>
            <UiBadge
              :label="resolveSourceStatusLabel(workspaceRuntimeSource)"
              :tone="resolveSourceStatusTone(workspaceRuntimeSource)"
            />
            <UiBadge
              :label="resolveValidationLabel(runtime.configValidation.workspace)"
              :tone="resolveValidationTone(runtime.configValidation.workspace)"
            />
          </template>

          <div class="space-y-3">
            <UiCodeEditor
              language="json"
              theme="octopus"
              :model-value="workspaceRuntimeDraft"
              @update:model-value="runtime.setConfigDraft('workspace', $event)"
            />

            <UiStatusCallout
              v-if="runtime.configValidation.workspace?.errors.length"
              tone="error"
              :description="runtime.configValidation.workspace.errors.join(' ')"
            />

            <UiStatusCallout
              v-if="runtime.configValidation.workspace?.warnings.length"
              tone="warning"
              :description="runtime.configValidation.workspace.warnings.join(' ')"
            />
          </div>

          <template #meta>
            <span class="text-[11px] uppercase tracking-[0.24em] text-text-tertiary">
              {{ $t('settings.runtime.sourcePath') }}
            </span>
            <span class="min-w-0 truncate font-mono text-[12px] text-text-secondary">
              {{ workspaceRuntimeSource?.displayPath ?? $t('common.na') }}
            </span>
          </template>
          <template #actions>
            <UiButton
              variant="ghost"
              size="sm"
              :disabled="runtime.configValidating || runtime.configSaving"
              @click="emit('validate')"
            >
              {{ $t('settings.runtime.actions.validate') }}
            </UiButton>
            <UiButton
              size="sm"
              :disabled="runtime.configSaving"
              @click="emit('save')"
            >
              {{ $t('settings.runtime.actions.save') }}
            </UiButton>
          </template>
        </UiRecordCard>
      </div>

      <div class="space-y-4">
        <UiRecordCard
          :title="$t('settings.runtime.effective.title')"
          :description="$t('settings.runtime.effective.description')"
          test-id="settings-runtime-effective-preview"
        >
          <template #badges>
            <UiBadge :label="runtime.config.effectiveConfigHash" tone="info" />
            <UiBadge
              :label="runtime.config.validation.valid ? $t('settings.runtime.validation.valid') : $t('settings.runtime.validation.invalid')"
              :tone="runtime.config.validation.valid ? 'success' : 'error'"
            />
          </template>

          <div class="space-y-3">
            <UiCodeEditor
              language="json"
              theme="octopus"
              readonly
              :model-value="runtimeEffectivePreview"
            />

            <div class="rounded-[var(--radius-l)] border border-border bg-subtle px-3 py-3 text-[12px] text-text-secondary">
              <p class="text-[11px] font-bold uppercase tracking-[0.24em] text-text-tertiary">
                {{ $t('settings.runtime.secretReferencesTitle') }}
              </p>
              <div v-if="runtimeSecretStatuses.length" class="mt-3 flex flex-wrap gap-2">
                <UiBadge
                  v-for="secret in runtimeSecretStatuses"
                  :key="`${secret.scope}-${secret.path}`"
                  :label="`${secret.scope}: ${secret.status}`"
                  :tone="secret.status === 'reference-missing' ? 'warning' : 'info'"
                />
              </div>
              <p v-else class="mt-2">
                {{ $t('settings.runtime.noSecretReferences') }}
              </p>
            </div>
          </div>
        </UiRecordCard>
      </div>
    </div>
  </section>
</template>
