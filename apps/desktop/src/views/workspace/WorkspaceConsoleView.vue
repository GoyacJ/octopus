<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { Bot, Cpu, FolderKanban, FolderOpen, LibraryBig, Wrench } from 'lucide-vue-next'

import { UiEmptyState, UiNavCardList, UiPageHeader, UiPageShell, UiPanelFrame } from '@octopus/ui'

import { getMenuDefinition } from '@/navigation/menuRegistry'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const workspaceAccessStore = useWorkspaceAccessStore()

const iconMap = {
  'menu-workspace-console-projects': FolderKanban,
  'menu-workspace-console-knowledge': LibraryBig,
  'menu-workspace-console-resources': FolderOpen,
  'menu-workspace-console-agents': Bot,
  'menu-workspace-console-models': Cpu,
  'menu-workspace-console-tools': Wrench,
} as const

const helperKeyMap = {
  'menu-workspace-console-projects': 'console.cards.projects',
  'menu-workspace-console-knowledge': 'console.cards.knowledge',
  'menu-workspace-console-resources': 'console.cards.resources',
  'menu-workspace-console-agents': 'console.cards.agents',
  'menu-workspace-console-models': 'console.cards.models',
  'menu-workspace-console-tools': 'console.cards.tools',
} as const

const menuIds = Object.keys(iconMap) as Array<keyof typeof iconMap>

const items = computed(() => {
  const availableIds = workspaceAccessStore.availableConsoleMenus.length
    ? new Set(workspaceAccessStore.availableConsoleMenus.map(menu => menu.id))
    : new Set(menuIds)

  return menuIds
    .filter(menuId => availableIds.has(menuId))
    .flatMap((menuId) => {
      const definition = getMenuDefinition(menuId)
      if (!definition?.routeName) {
        return []
      }

      return [{
        id: menuId,
        label: t(definition.labelKey),
        helper: t(helperKeyMap[menuId]),
        icon: iconMap[menuId],
        active: route.name === definition.routeName,
      }]
    })
})

function handleSelect(menuId: string) {
  const definition = getMenuDefinition(menuId)
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
  <UiPageShell width="standard" test-id="workspace-console-view">
    <UiPageHeader
      :eyebrow="t('console.header.eyebrow')"
      :title="t('console.header.title')"
      :description="t('console.header.description')"
    />

    <UiPanelFrame
      variant="panel"
      padding="md"
      :title="t('console.sections.workspace.title')"
      :subtitle="t('console.sections.workspace.subtitle')"
    >
      <UiNavCardList
        v-if="items.length"
        :items="items"
        test-id="workspace-console-nav"
        @select="handleSelect"
      />
      <UiEmptyState
        v-else
        :title="t('console.empty.title')"
        :description="t('console.empty.description')"
      />
    </UiPanelFrame>
  </UiPageShell>
</template>
