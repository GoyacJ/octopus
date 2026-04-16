<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { UiPageHeader, UiPageShell, UiPanelFrame, UiTabs } from '@octopus/ui'

import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

type AccessSectionTab = 'members' | 'access' | 'governance'

const SECTION_ROUTE_NAME: Record<AccessSectionTab, string> = {
  members: 'workspace-access-control-members',
  access: 'workspace-access-control-access',
  governance: 'workspace-access-control-governance',
}

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shellStore = useShellStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()
const workspaceStore = useWorkspaceStore()

const activeSection = ref<AccessSectionTab>('members')

const tabs = computed(() => ([
  {
    value: 'members',
    label: t('accessControl.sections.members.label'),
  },
  {
    value: 'access',
    label: t('accessControl.sections.access.label'),
  },
  {
    value: 'governance',
    label: t('accessControl.sections.governance.label'),
  },
]))

const headerDescription = computed(() => {
  switch (workspaceAccessControlStore.experienceSummary?.experienceLevel) {
    case 'personal':
      return t('accessControl.header.personalDescription')
    case 'enterprise':
      return t('accessControl.header.enterpriseDescription')
    default:
      return t('accessControl.header.teamDescription')
  }
})

watch(
  () => route.name,
  () => {
    switch (route.name) {
      case 'workspace-access-control-access':
        activeSection.value = 'access'
        break
      case 'workspace-access-control-governance':
        activeSection.value = 'governance'
        break
      default:
        activeSection.value = 'members'
        break
    }
  },
  { immediate: true },
)

watch(
  () => [route.name, shellStore.activeWorkspaceConnectionId] as const,
  async ([routeName, workspaceConnectionId]) => {
    if (!workspaceConnectionId || typeof routeName !== 'string' || !routeName.startsWith('workspace-access-control')) {
      return
    }

    if (routeName === 'workspace-access-control-governance') {
      await workspaceAccessControlStore.loadGovernanceData(workspaceConnectionId)
      return
    }

    if (routeName === 'workspace-access-control-members' || routeName === 'workspace-access-control-access') {
      await workspaceAccessControlStore.loadMembersData(workspaceConnectionId)
      return
    }

    await workspaceAccessControlStore.loadExperience(workspaceConnectionId)
  },
  { immediate: true },
)

function handleTabChange(section: string) {
  if (!(section in SECTION_ROUTE_NAME)) {
    return
  }

  void router.push({
    name: SECTION_ROUTE_NAME[section as AccessSectionTab],
    params: { workspaceId: workspaceStore.currentWorkspaceId },
  })
}
</script>

<template>
  <UiPageShell width="wide" test-id="access-control-view" class="h-full">
    <UiPageHeader
      :eyebrow="t('accessControl.header.eyebrow')"
      :title="t('accessControl.header.title')"
      :description="headerDescription"
    />

    <UiPanelFrame variant="subtle" padding="sm">
      <UiTabs
        v-model="activeSection"
        :tabs="tabs"
        test-id="access-control-sections"
        @update:model-value="handleTabChange"
      />
    </UiPanelFrame>

    <main class="min-h-0 flex-1 overflow-y-auto pb-8">
      <RouterView />
    </main>
  </UiPageShell>
</template>
