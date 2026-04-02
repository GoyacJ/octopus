<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterView } from 'vue-router'

import { UiButton, UiSurface } from '@octopus/ui'

import i18n from '@/plugins/i18n'
import { useWorkbenchRouteSync } from '@/composables/useWorkbenchRouteSync'
import WorkbenchLayout from '@/layouts/WorkbenchLayout.vue'
import { restartDesktopBackend } from '@/tauri/client'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const workbench = useWorkbenchStore()
const shell = useShellStore()

useWorkbenchRouteSync()

const retryingBackend = ref(false)
const restartingBackend = ref(false)
const backendRetryTimer = ref<number | null>(null)

const isDesktopBackendUnavailable = computed(() =>
  shell.hostState.platform === 'tauri' && shell.bootstrapPayload?.backend?.ready === false,
)

function resolveTheme(theme: 'light' | 'dark' | 'system'): 'light' | 'dark' {
  if (theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  return theme
}

async function bootstrapShell() {
  await shell.bootstrap(workbench.currentWorkspaceId, workbench.currentProjectId, workbench.connections)
}

function clearBackendRetryTimer() {
  if (backendRetryTimer.value) {
    clearTimeout(backendRetryTimer.value)
    backendRetryTimer.value = null
  }
}

function scheduleBackendRetry() {
  clearBackendRetryTimer()
  if (!isDesktopBackendUnavailable.value) {
    return
  }

  backendRetryTimer.value = window.setTimeout(() => {
    void retryBackendBootstrap()
  }, 1500)
}

async function retryBackendBootstrap() {
  if (retryingBackend.value) {
    return
  }

  retryingBackend.value = true
  try {
    await bootstrapShell()
  } finally {
    retryingBackend.value = false
  }
}

async function restartBackendAndRetry() {
  if (restartingBackend.value) {
    return
  }

  restartingBackend.value = true
  try {
    await restartDesktopBackend()
    await retryBackendBootstrap()
  } finally {
    restartingBackend.value = false
  }
}

onMounted(async () => {
  await bootstrapShell()
  scheduleBackendRetry()
})

onBeforeUnmount(() => {
  clearBackendRetryTimer()
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

watch(isDesktopBackendUnavailable, () => {
  scheduleBackendRetry()
}, { immediate: true })
</script>

<template>
  <WorkbenchLayout>
    <UiSurface
      v-if="isDesktopBackendUnavailable"
      class="desktop-backend-guard"
      data-testid="desktop-backend-guard"
      title="Desktop backend unavailable"
      subtitle="The Tauri shell is up, but the local desktop backend is not ready yet."
    >
      <div class="desktop-backend-guard-copy">
        <p>
          Retry the bootstrap connection or restart the desktop backend process before continuing into runtime-backed pages.
        </p>
      </div>
      <div class="desktop-backend-guard-actions">
        <UiButton
          variant="ghost"
          data-testid="desktop-backend-retry"
          :loading="retryingBackend"
          @click="retryBackendBootstrap"
        >
          Retry connection
        </UiButton>
        <UiButton
          data-testid="desktop-backend-restart"
          :loading="restartingBackend"
          @click="restartBackendAndRetry"
        >
          Restart backend
        </UiButton>
      </div>
    </UiSurface>
    <RouterView v-else />
  </WorkbenchLayout>
</template>

<style scoped>
.desktop-backend-guard {
  width: min(100%, 42rem);
  margin: 8vh auto 0;
}

.desktop-backend-guard-copy {
  color: var(--text-secondary);
  line-height: 1.65;
}

.desktop-backend-guard-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 1.25rem;
}
</style>
