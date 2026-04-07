<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { UiSectionHeading, UiTabs } from '@octopus/ui'

import { getMenuDefinition, getRouteMenuId } from '@/navigation/menuRegistry'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const userCenterStore = useUserCenterStore()
const workspaceStore = useWorkspaceStore()

const activeTab = ref('')
const currentMenuId = computed(() => getRouteMenuId(typeof route.name === 'string' ? route.name : undefined))

watch(
  () => [route.name, workspaceStore.currentWorkspaceId, userCenterStore.availableUserCenterMenus.map(menu => menu.id).join('|')],
  () => {
    if (route.name === 'workspace-user-center') {
      const firstRouteName = userCenterStore.firstAccessibleUserCenterRouteName
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
  userCenterStore.availableUserCenterMenus
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
  const entry = userCenterStore.availableUserCenterMenus.find(menu => menu.id === menuId)
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
  <div class="flex h-full min-h-0 w-full flex-col gap-6 pb-20">
    <header class="shrink-0 px-2">
      <UiSectionHeading
        :eyebrow="t('userCenter.header.eyebrow')"
        :title="t('userCenter.header.title')"
        :subtitle="userCenterStore.currentUser?.displayName ?? t('common.na')"
      />
    </header>

    <div class="border-b border-border-subtle px-2 pb-4 dark:border-white/[0.05]">
      <UiTabs
        v-model="activeTab"
        :tabs="tabs"
        data-testid="user-center-tabs"
        @update:model-value="handleTabChange"
      />
    </div>

    <main class="flex-1 overflow-y-auto min-h-0 px-2 pb-8">
      <RouterView />
    </main>
  </div>
</template>
