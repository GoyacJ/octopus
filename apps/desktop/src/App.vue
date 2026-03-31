<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { RouterView } from 'vue-router'

import i18n from '@/plugins/i18n'
import { useWorkbenchRouteSync } from '@/composables/useWorkbenchRouteSync'
import WorkbenchLayout from '@/layouts/WorkbenchLayout.vue'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const workbench = useWorkbenchStore()
const shell = useShellStore()

useWorkbenchRouteSync()

function resolveTheme(theme: 'light' | 'dark' | 'system'): 'light' | 'dark' {
  if (theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  return theme
}

onMounted(async () => {
  await shell.bootstrap(workbench.currentWorkspaceId, workbench.currentProjectId, workbench.connections)
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
</script>

<template>
  <WorkbenchLayout>
    <RouterView />
  </WorkbenchLayout>
</template>
