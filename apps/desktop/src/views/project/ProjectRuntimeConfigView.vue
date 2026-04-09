<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiCodeEditor,
  UiEmptyState,
  UiPageHeader,
  UiPageShell,
  UiPanelFrame,
  UiStatusCallout,
} from '@octopus/ui'

import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const workspaceStore = useWorkspaceStore()

const projectId = computed(() =>
  typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId,
)
const runtimeConfig = computed(() => workspaceStore.activeProjectRuntimeConfig)
const runtimeSource = computed(() => runtimeConfig.value?.sources.filter(source => source.scope === 'project').at(-1))
const runtimeEffectivePreview = computed(() => JSON.stringify(runtimeConfig.value?.effectiveConfig ?? {}, null, 2))

async function loadProjectRuntimeConfig(force = false) {
  if (!projectId.value) {
    return
  }
  await workspaceStore.loadProjectRuntimeConfig(projectId.value, force)
}

watch(
  () => [projectId.value, workspaceStore.activeConnectionId],
  ([nextProjectId, activeConnectionId]) => {
    if (!nextProjectId || !activeConnectionId) {
      return
    }
    void loadProjectRuntimeConfig()
  },
  { immediate: true },
)
</script>

<template>
  <UiPageShell width="wide" test-id="project-runtime-view">
    <UiPageHeader
      :eyebrow="t('projectRuntime.header.eyebrow')"
      :title="workspaceStore.activeProject?.name ?? t('projectRuntime.header.title')"
      :description="t('projectRuntime.header.subtitle')"
    />

    <UiStatusCallout
      v-if="workspaceStore.activeProjectRuntimeLoading && !runtimeConfig"
      tone="info"
      :description="t('settings.runtime.loading')"
    />

    <UiEmptyState
      v-else-if="!runtimeConfig"
      :title="t('projectRuntime.emptyTitle')"
      :description="workspaceStore.error || t('projectRuntime.emptyDescription')"
    />

    <section v-else class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(22rem,0.9fr)]">
      <div data-testid="project-runtime-editor">
        <UiPanelFrame
          variant="panel"
          padding="md"
          :eyebrow="'project'"
          :title="t('projectRuntime.editor.title')"
          :subtitle="t('projectRuntime.editor.description')"
        >
          <template #actions>
            <UiBadge
              :label="runtimeSource?.loaded ? t('settings.runtime.sourceStatuses.loaded') : t('settings.runtime.sourceStatuses.missing')"
              :tone="runtimeSource?.loaded ? 'success' : 'warning'"
            />
            <UiBadge
              :label="workspaceStore.activeProjectRuntimeValidation?.valid ? t('settings.runtime.validation.valid') : t('settings.runtime.validation.idle')"
              :tone="workspaceStore.activeProjectRuntimeValidation?.valid ? 'success' : 'default'"
            />
          </template>

          <div class="space-y-3">
            <UiCodeEditor
              language="json"
              theme="octopus"
              :model-value="workspaceStore.activeProjectRuntimeDraft"
              @update:model-value="workspaceStore.setProjectRuntimeDraft(projectId, $event)"
            />

            <UiStatusCallout
              v-if="workspaceStore.activeProjectRuntimeValidation?.errors.length"
              tone="error"
              :description="workspaceStore.activeProjectRuntimeValidation.errors.join(' ')"
            />
          </div>

          <div class="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-border/70 pt-3">
            <div class="min-w-0">
              <div class="text-[11px] uppercase tracking-[0.08em] text-text-tertiary">
                {{ t('settings.runtime.sourcePath') }}
              </div>
              <div class="min-w-0 truncate font-mono text-[12px] text-text-secondary">
                {{ runtimeSource?.displayPath ?? t('common.na') }}
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-2">
              <UiButton
                variant="ghost"
                size="sm"
                :disabled="workspaceStore.activeProjectRuntimeValidating || workspaceStore.activeProjectRuntimeSaving"
                @click="workspaceStore.validateProjectRuntimeConfig(projectId)"
              >
                {{ t('settings.runtime.actions.validate') }}
              </UiButton>
              <UiButton
                size="sm"
                :disabled="workspaceStore.activeProjectRuntimeSaving"
                @click="workspaceStore.saveProjectRuntimeConfig(projectId)"
              >
                {{ t('settings.runtime.actions.save') }}
              </UiButton>
            </div>
          </div>
        </UiPanelFrame>
      </div>

      <div data-testid="project-runtime-effective-preview">
        <UiPanelFrame
          variant="subtle"
          padding="md"
          :title="t('projectRuntime.effective.title')"
          :subtitle="t('projectRuntime.effective.description')"
        >
          <UiCodeEditor
            language="json"
            theme="octopus"
            readonly
            :model-value="runtimeEffectivePreview"
          />
        </UiPanelFrame>
      </div>
    </section>
  </UiPageShell>
</template>
