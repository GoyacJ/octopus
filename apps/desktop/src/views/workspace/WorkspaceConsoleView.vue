<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { UiPageHeader, UiPageShell, UiPanelFrame, UiTabs } from '@octopus/ui'

import { getMenuDefinition, getRouteMenuId } from '@/navigation/menuRegistry'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()

const activeTab = ref('')
const currentMenuId = computed(() => getRouteMenuId(typeof route.name === 'string' ? route.name : undefined))

watch(
  () => [route.name, workspaceStore.currentWorkspaceId, workspaceAccessControlStore.availableConsoleMenus.map(menu => menu.id).join('|')],
  () => {
    if (route.name === 'workspace-console') {
      const firstRouteName = workspaceAccessControlStore.firstAccessibleConsoleRouteName
      if (firstRouteName) {
        const menuId = getRouteMenuId(firstRouteName)
        if (menuId) {
          activeTab.value = menuId
          void router.replace({
            name: firstRouteName,
            params: { workspaceId: workspaceStore.currentWorkspaceId },
          })
        }
      }
      return
    }

    activeTab.value = currentMenuId.value ?? ''
  },
  { immediate: true },
)

const tabs = computed(() =>
  workspaceAccessControlStore.availableConsoleMenus
    .flatMap((menu) => {
      const definition = getMenuDefinition(menu.id)
      if (!definition?.routeName) {
        return []
      }

      return [{
        value: menu.id,
        label: t(definition.labelKey),
      }]
    }),
)

function handleTabChange(menuId: string) {
  const entry = workspaceAccessControlStore.availableConsoleMenus.find(menu => menu.id === menuId)
  const definition = entry ? getMenuDefinition(entry.id) : undefined
  if (!definition?.routeName) {
    return
  }

  void router.push({
    name: definition.routeName,
    params: { workspaceId: workspaceStore.currentWorkspaceId },
  })
}
</script>

<template>
  <UiPageShell width="wide" test-id="workspace-console-view" class="h-full">
    <UiPageHeader
      :eyebrow="t('console.header.eyebrow')"
      :title="t('console.header.title')"
      :description="t('console.header.description')"
    />

    <UiPanelFrame variant="subtle" padding="sm">
      <UiTabs
        v-model="activeTab"
        :tabs="tabs"
        data-testid="workspace-console-tabs"
        @update:model-value="handleTabChange"
      />
    </UiPanelFrame>

    <main class="min-h-0 flex-1 overflow-y-auto pb-8">
      <RouterView />
    </main>
  </UiPageShell>
</template>
