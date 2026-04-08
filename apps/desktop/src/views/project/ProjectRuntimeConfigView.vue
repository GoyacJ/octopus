<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import { UiBadge, UiButton, UiCodeEditor, UiEmptyState, UiRecordCard, UiSectionHeading } from '@octopus/ui'

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
  <div class="flex w-full flex-col gap-8 pb-20">
    <header class="px-2">
      <UiSectionHeading
        :eyebrow="t('projectRuntime.header.eyebrow')"
        :title="workspaceStore.activeProject?.name ?? t('projectRuntime.header.title')"
        :subtitle="t('projectRuntime.header.subtitle')"
      />
    </header>

    <section v-if="workspaceStore.activeProjectRuntimeLoading && !runtimeConfig" class="px-2">
      <div class="rounded-md border border-border/40 bg-subtle/10 px-4 py-6 text-[13px] text-text-secondary">
        {{ t('settings.runtime.loading') }}
      </div>
    </section>

    <UiEmptyState
      v-else-if="!runtimeConfig"
      class="px-2"
      :title="t('projectRuntime.emptyTitle')"
      :description="workspaceStore.error || t('projectRuntime.emptyDescription')"
    />

    <div v-else class="grid gap-6 px-2 xl:grid-cols-[minmax(0,1.3fr)_minmax(22rem,0.9fr)]">
      <UiRecordCard
        :title="t('projectRuntime.editor.title')"
        :description="t('projectRuntime.editor.description')"
        test-id="project-runtime-editor"
      >
        <template #eyebrow>
          project
        </template>
        <template #badges>
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

          <div
            v-if="workspaceStore.activeProjectRuntimeValidation?.errors.length"
            class="rounded-md border border-status-error/20 bg-status-error/5 px-3 py-2 text-[12px] text-status-error"
          >
            {{ workspaceStore.activeProjectRuntimeValidation.errors.join(' ') }}
          </div>
        </div>

        <template #meta>
          <span class="text-[11px] uppercase tracking-[0.24em] text-text-tertiary">
            {{ t('settings.runtime.sourcePath') }}
          </span>
          <span class="min-w-0 truncate font-mono text-[12px] text-text-secondary">
            {{ runtimeSource?.displayPath ?? t('common.na') }}
          </span>
        </template>
        <template #actions>
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
        </template>
      </UiRecordCard>

      <UiRecordCard
        :title="t('projectRuntime.effective.title')"
        :description="t('projectRuntime.effective.description')"
        test-id="project-runtime-effective-preview"
      >
        <UiCodeEditor
          language="json"
          theme="octopus"
          readonly
          :model-value="runtimeEffectivePreview"
        />
      </UiRecordCard>
    </div>
  </div>
</template>
