<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { AlertTriangle } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { UiButton, UiPageShell, UiPanelFrame, UiStatusCallout } from '@octopus/ui'

import {
  clearRuntimeAppError,
  formatRuntimeAppErrorDetails,
  retryRuntimeAppSurface,
  runtimeAppErrorState,
} from '@/runtime/app-error-boundary'
import { createProjectConversationTarget, createWorkspaceOverviewTarget } from '@/i18n/navigation'
import { useShellStore } from '@/stores/shell'

const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const { t } = useI18n()
const copyStatus = ref<'idle' | 'success' | 'error'>('idle')

const errorRecord = computed(() => runtimeAppErrorState.current)
const workspaceId = computed(() =>
  typeof route.params.workspaceId === 'string'
    ? route.params.workspaceId
    : shell.activeWorkspaceConnection?.workspaceId || shell.preferences.defaultWorkspaceId || '',
)
const projectId = computed(() =>
  typeof route.params.projectId === 'string'
    ? route.params.projectId
    : shell.defaultProjectId || '',
)
const canReturnToProject = computed(() => Boolean(workspaceId.value && projectId.value))
const detailText = computed(() => errorRecord.value ? formatRuntimeAppErrorDetails(errorRecord.value) : '')

watch(errorRecord, () => {
  copyStatus.value = 'idle'
})

function fallbackCopyText(value: string): boolean {
  const textarea = document.createElement('textarea')
  textarea.value = value
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.opacity = '0'
  document.body.appendChild(textarea)
  textarea.select()
  const copied = document.execCommand('copy')
  document.body.removeChild(textarea)
  return copied
}

async function copyErrorDetails() {
  if (!detailText.value) {
    return
  }

  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(detailText.value)
    } else if (!fallbackCopyText(detailText.value)) {
      throw new Error('Clipboard copy failed')
    }

    copyStatus.value = 'success'
  } catch {
    copyStatus.value = 'error'
  }
}

function retryCurrentPage() {
  retryRuntimeAppSurface()
}

async function returnToProject() {
  if (!canReturnToProject.value) {
    return
  }

  clearRuntimeAppError()
  await router.replace(createProjectConversationTarget(workspaceId.value, projectId.value))
}

async function returnToOverview() {
  if (!workspaceId.value) {
    return
  }

  clearRuntimeAppError()
  await router.replace(createWorkspaceOverviewTarget(
    workspaceId.value,
    projectId.value || undefined,
  ))
}
</script>

<template>
  <UiPageShell width="wide" class="h-full" content-class="min-h-full">
    <div class="flex min-h-full items-center py-6">
      <UiPanelFrame
        data-testid="app-runtime-error-boundary"
        variant="panel"
        padding="lg"
        class="mx-auto w-full max-w-[880px]"
        inner-class="space-y-6"
      >
        <div class="flex items-start gap-4">
          <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-[var(--radius-m)] border border-border bg-surface-muted text-status-error">
            <AlertTriangle :size="18" />
          </div>
          <div class="min-w-0 space-y-2">
            <p class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
              {{ t('app.runtimeError.eyebrow') }}
            </p>
            <h1 class="text-2xl font-semibold tracking-tight text-text-primary">
              {{ t('app.runtimeError.title') }}
            </h1>
            <p class="text-sm leading-6 text-text-secondary">
              {{ t('app.runtimeError.description') }}
            </p>
          </div>
        </div>

        <UiStatusCallout
          tone="error"
          :title="errorRecord?.name"
          :description="errorRecord?.message || t('app.runtimeError.fallbackMessage')"
        />

        <div class="flex flex-wrap gap-3">
          <UiButton data-testid="app-runtime-error-retry" @click="retryCurrentPage">
            {{ t('app.runtimeError.actions.retry') }}
          </UiButton>
          <UiButton
            v-if="canReturnToProject"
            data-testid="app-runtime-error-project"
            variant="outline"
            @click="returnToProject"
          >
            {{ t('app.runtimeError.actions.project') }}
          </UiButton>
          <UiButton
            data-testid="app-runtime-error-overview"
            variant="ghost"
            @click="returnToOverview"
          >
            {{ t('app.runtimeError.actions.overview') }}
          </UiButton>
          <UiButton
            data-testid="app-runtime-error-copy"
            variant="ghost"
            @click="copyErrorDetails"
          >
            {{ t('app.runtimeError.actions.copy') }}
          </UiButton>
        </div>

        <div class="space-y-2">
          <div class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
            {{ t('app.runtimeError.detailsTitle') }}
          </div>
          <pre
            data-testid="app-runtime-error-details"
            class="max-h-[240px] overflow-auto rounded-[var(--radius-m)] border border-border bg-surface-muted px-4 py-3 text-xs leading-6 text-text-secondary"
          >{{ detailText }}</pre>
          <p v-if="copyStatus === 'success'" class="text-xs text-text-secondary">
            {{ t('app.runtimeError.copySuccess') }}
          </p>
          <p v-else-if="copyStatus === 'error'" class="text-xs text-status-error">
            {{ t('app.runtimeError.copyFailure') }}
          </p>
        </div>
      </UiPanelFrame>
    </div>
  </UiPageShell>
</template>
