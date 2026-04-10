<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { UiPageHeader, UiPageShell, UiPanelFrame, UiTabs } from '@octopus/ui'

import { getMenuDefinition, getRouteMenuId } from '@/navigation/menuRegistry'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workspaceAccessStore = useWorkspaceAccessStore()
const workspaceStore = useWorkspaceStore()

const activeTab = ref('')
const currentMenuId = computed(() => getRouteMenuId(typeof route.name === 'string' ? route.name : undefined))

watch(
  () => [route.name, workspaceStore.currentWorkspaceId, workspaceAccessStore.availablePermissionCenterMenus.map(menu => menu.id).join('|')],
  () => {
    if (route.name === 'workspace-permission-center') {
      const firstRouteName = workspaceAccessStore.firstAccessiblePermissionCenterRouteName
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
  workspaceAccessStore.availablePermissionCenterMenus
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
  const entry = workspaceAccessStore.availablePermissionCenterMenus.find(menu => menu.id === menuId)
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
  <UiPageShell width="wide" test-id="permission-center-view" class="h-full">
    <UiPageHeader
      :eyebrow="t('permissionCenter.header.eyebrow')"
      :title="t('permissionCenter.header.title')"
      :description="workspaceAccessStore.currentUser?.displayName ?? t('common.na')"
    />

    <UiPanelFrame variant="subtle" padding="sm">
      <UiTabs
        v-model="activeTab"
        :tabs="tabs"
        data-testid="permission-center-tabs"
        @update:model-value="handleTabChange"
      />
    </UiPanelFrame>

    <main class="min-h-0 flex-1 overflow-y-auto pb-8">
      <RouterView />
    </main>
  </UiPageShell>
</template>
