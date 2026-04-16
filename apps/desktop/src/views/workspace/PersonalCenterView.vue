<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { UiPageHeader, UiPageShell, UiPanelFrame, UiTabs } from '@octopus/ui'

import { useUserProfileStore } from '@/stores/user-profile'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const userProfileStore = useUserProfileStore()
const workspaceStore = useWorkspaceStore()

const activeTab = ref('')

const tabs = computed(() => [
  {
    value: 'workspace-personal-center-profile',
    label: t('personalCenter.nav.profile'),
  },
  {
    value: 'workspace-personal-center-pet',
    label: t('personalCenter.nav.pet'),
  },
])

watch(
  () => route.name,
  () => {
    activeTab.value = typeof route.name === 'string' ? route.name : 'workspace-personal-center-profile'
  },
  { immediate: true },
)

watch(
  () => workspaceStore.activeConnectionId,
  (connectionId) => {
    if (!connectionId) {
      return
    }
    void userProfileStore.load(connectionId)
  },
  { immediate: true },
)

function handleTabChange(routeName: string) {
  void router.push({
    name: routeName,
    params: { workspaceId: workspaceStore.currentWorkspaceId },
  })
}
</script>

<template>
  <UiPageShell width="wide" test-id="personal-center-view" class="h-full">
    <UiPageHeader
      :eyebrow="t('personalCenter.header.eyebrow')"
      :title="t('personalCenter.header.title')"
      :description="t('personalCenter.header.subtitle')"
    />

    <UiPanelFrame variant="subtle" padding="sm">
      <UiTabs
        v-model="activeTab"
        :tabs="tabs"
        data-testid="personal-center-tabs"
        @update:model-value="handleTabChange"
      />
    </UiPanelFrame>

    <main class="min-h-0 flex-1 overflow-y-auto pb-8">
      <RouterView />
    </main>
  </UiPageShell>
</template>
