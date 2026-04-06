<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { RouterView } from 'vue-router'
import { useI18n } from 'vue-i18n'

import { UiButton } from '@octopus/ui'

import i18n from '@/plugins/i18n'
import { useWorkbenchRouteSync } from '@/composables/useWorkbenchRouteSync'
import WorkbenchLayout from '@/layouts/WorkbenchLayout.vue'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'

const shell = useShellStore()
const runtime = useRuntimeStore()
const { t } = useI18n()

useWorkbenchRouteSync()

function resolveTheme(theme: 'light' | 'dark' | 'system'): 'light' | 'dark' {
  if (theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  return theme
}

async function bootstrapShell() {
  await shell.bootstrap(shell.defaultWorkspaceId, shell.defaultProjectId)
  runtime.syncWorkspaceScopeFromShell()
}

const showHostUnavailable = computed(() => {
  if (shell.loading) {
    return false
  }

  if (shell.error) {
    return true
  }

  return shell.backendConnection?.state === 'unavailable'
})

const hostUnavailableDescription = computed(() =>
  shell.error || t('app.hostUnavailable.description'),
)

onMounted(async () => {
  await bootstrapShell()
})

watch(
  () => shell.preferences.locale,
  (locale) => {
    i18n.global.locale.value = locale
  },
  { immediate: true },
)

watch(
  () => shell.preferences.theme,
  (theme) => {
    document.documentElement.dataset.theme = resolveTheme(theme)
  },
  { immediate: true },
)

watch(
  [() => shell.preferences.fontSize, () => shell.preferences.fontFamily, () => shell.preferences.fontStyle],
  ([fontSize, fontFamily, fontStyle]) => {
    const root = document.documentElement
    
    // Apply font size
    root.style.setProperty('--font-size-base', `${fontSize}px`)
    root.style.fontSize = `${fontSize}px`
    
    // Reset specific families to handle style switching
    let primaryStack = fontFamily
    if (fontStyle === 'serif') {
      primaryStack = 'Georgia, "Times New Roman", serif'
    } else if (fontStyle === 'mono') {
      primaryStack = 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace'
    } else if (fontStyle === 'sans' && fontFamily === 'Inter, sans-serif') {
      primaryStack = '"Inter", system-ui, -apple-system, sans-serif'
    }
    
    // Update the variables that Tailwind and UI tokens use
    root.style.setProperty('--font-sans', primaryStack)
    root.style.setProperty('--font-serif', fontStyle === 'serif' ? primaryStack : 'Georgia, serif')
    root.style.setProperty('--font-mono', fontStyle === 'mono' ? primaryStack : 'ui-monospace, monospace')
    
    // Force immediate body update for insurance
    document.body.style.fontFamily = primaryStack
  },
  { immediate: true },
)

watch(
  () => shell.activeWorkspaceConnectionId,
  () => {
    runtime.syncWorkspaceScopeFromShell()
  },
)

</script>

<template>
  <div
    v-if="showHostUnavailable"
    data-testid="desktop-backend-guard"
    class="flex min-h-screen items-center justify-center bg-background px-6"
  >
    <div class="w-full max-w-xl rounded-2xl border border-border-subtle bg-card p-8 shadow-[0_24px_64px_rgba(15,23,42,0.08)] dark:border-white/[0.08]">
      <div class="space-y-2">
        <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-text-tertiary">
          {{ t('app.hostUnavailable.eyebrow') }}
        </p>
        <h1 class="text-2xl font-semibold tracking-tight text-text-primary">
          {{ t('app.hostUnavailable.title') }}
        </h1>
        <p class="text-sm leading-6 text-text-secondary">
          {{ hostUnavailableDescription }}
        </p>
      </div>

      <div class="mt-6 flex flex-wrap gap-3">
        <UiButton data-testid="desktop-backend-retry" @click="bootstrapShell">
          {{ t('app.hostUnavailable.retry') }}
        </UiButton>
        <UiButton
          v-if="shell.canRestartBackend"
          data-testid="desktop-backend-restart"
          variant="ghost"
          @click="shell.restartBackend"
        >
          {{ t('app.hostUnavailable.restart') }}
        </UiButton>
      </div>
    </div>
  </div>
  <WorkbenchLayout v-else>
    <RouterView />
  </WorkbenchLayout>
</template>
