<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { UiButton, UiEmptyState, UiMetricCard, UiPanelFrame, UiTabs } from '@octopus/ui'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

import AccessControlMenusView from './AccessControlMenusView.vue'
import AccessControlOrgView from './AccessControlOrgView.vue'
import AccessControlPoliciesView from './AccessControlPoliciesView.vue'
import AccessControlResourcesView from './AccessControlResourcesView.vue'
import AccessControlRolesView from './AccessControlRolesView.vue'
import AccessControlSessionsView from './AccessControlSessionsView.vue'

type GovernanceTab = 'organization' | 'roles' | 'policies' | 'menus' | 'resources' | 'sessions'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const accessControlStore = useWorkspaceAccessControlStore()
const workspaceStore = useWorkspaceStore()

const activeTab = ref<GovernanceTab>('organization')

const tabs = computed(() => ([
  { value: 'organization', label: t('accessControl.governance.sections.organization') },
  { value: 'roles', label: t('accessControl.governance.sections.roles') },
  { value: 'policies', label: t('accessControl.governance.sections.policies') },
  { value: 'menus', label: t('accessControl.governance.sections.menus') },
  { value: 'resources', label: t('accessControl.governance.sections.resources') },
  { value: 'sessions', label: t('accessControl.governance.sections.sessions') },
]))

const metrics = computed(() => [
  {
    id: 'org',
    label: t('accessControl.governance.metrics.orgUnits'),
    value: accessControlStore.experience?.counts.orgUnitCount ?? 0,
  },
  {
    id: 'policies',
    label: t('accessControl.governance.metrics.policies'),
    value: (accessControlStore.experience?.counts.dataPolicyCount ?? 0) + (accessControlStore.experience?.counts.resourcePolicyCount ?? 0),
  },
  {
    id: 'sessions',
    label: t('accessControl.governance.metrics.sessions'),
    value: accessControlStore.experience?.counts.sessionCount ?? 0,
  },
])

watch(() => route.query.tab, (value) => {
  if (
    value === 'organization'
    || value === 'roles'
    || value === 'policies'
    || value === 'menus'
    || value === 'resources'
    || value === 'sessions'
  ) {
    activeTab.value = value
  }
}, { immediate: true })

function openSection(section: 'members' | 'access') {
  void router.push({
    name: `workspace-access-control-${section}`,
    params: {
      workspaceId: workspaceStore.currentWorkspaceId,
    },
  })
}
</script>

<template>
  <div class="space-y-4" data-testid="access-governance-view">
    <div class="grid gap-4 lg:grid-cols-3">
      <UiMetricCard
        v-for="metric in metrics"
        :key="metric.id"
        :label="metric.label"
        :value="metric.value"
      />
    </div>

    <UiPanelFrame
      variant="raised"
      padding="md"
      :title="t('accessControl.governance.title')"
      :subtitle="t('accessControl.governance.description')"
    >
      <UiEmptyState
        v-if="accessControlStore.isGovernanceEmpty"
        :title="t('accessControl.governance.empty.title')"
        :description="t('accessControl.governance.empty.description')"
      >
        <template #actions>
          <UiButton size="sm" @click="openSection('access')">
            {{ t('accessControl.governance.empty.primaryAction') }}
          </UiButton>
          <UiButton size="sm" variant="ghost" @click="openSection('members')">
            {{ t('accessControl.governance.empty.secondaryAction') }}
          </UiButton>
        </template>
      </UiEmptyState>

      <div v-else class="space-y-4">
        <UiTabs
          v-model="activeTab"
          :tabs="tabs"
          test-id="access-governance-sections"
          variant="segmented"
        />

        <AccessControlOrgView v-if="activeTab === 'organization'" />
        <AccessControlRolesView v-else-if="activeTab === 'roles'" />
        <AccessControlPoliciesView v-else-if="activeTab === 'policies'" />
        <AccessControlMenusView v-else-if="activeTab === 'menus'" />
        <AccessControlResourcesView v-else-if="activeTab === 'resources'" />
        <AccessControlSessionsView v-else />
      </div>
    </UiPanelFrame>
  </div>
</template>
