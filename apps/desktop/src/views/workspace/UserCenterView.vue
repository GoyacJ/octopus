<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'

import {
  UiActionCard,
  UiBadge,
  UiMetricCard,
  UiNavCardList,
  UiSectionHeading,
} from '@octopus/ui'

import { getMenuDefinition, getRouteMenuId } from '@/navigation/menuRegistry'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workbench = useWorkbenchStore()

const workspaceName = computed(() =>
  workbench.activeWorkspace
    ? workbench.workspaceDisplayName(workbench.activeWorkspace.id)
    : t('common.na'),
)

const currentRoleSummary = computed(() => {
  if (!workbench.currentUserRoles.length) {
    return t('userCenter.common.noRoles')
  }

  return workbench.currentUserRoles.map((role) => role.name).join(' / ')
})

const overview = computed(() => workbench.userCenterOverview)

const navigationEntries = computed(() =>
  workbench.availableUserCenterMenus
    .flatMap((menu) => {
      const definition = getMenuDefinition(menu.id)
      if (!definition?.routeName) {
        return []
      }

      return [{
        id: menu.id,
        label: menu.label,
        helper: definition.routeName.replace('workspace-user-center-', ''),
        routeName: definition.routeName,
        testId: definition.routeName.replace('workspace-user-center-', ''),
        active: route.name === definition.routeName,
      }]
    }),
)

const navigationEntryById = computed(() =>
  new Map(navigationEntries.value.map((item) => [item.id, item])),
)

const navigationItems = computed(() =>
  navigationEntries.value.map(({ routeName: _routeName, testId: _testId, ...item }) => item),
)

function resolveNavigationEntry(menuId: string) {
  return navigationEntryById.value.get(menuId)
}

function navigationTestId(menuId: string) {
  return resolveNavigationEntry(menuId)?.testId ?? menuId
}

function navigationRoute(menuId: string) {
  const entry = resolveNavigationEntry(menuId)
  return {
    name: entry?.routeName ?? 'workspace-user-center-profile',
    params: {
      workspaceId: workbench.currentWorkspaceId,
    },
  }
}

function ensureAuthorizedChildRoute() {
  const currentMenuId = getRouteMenuId(typeof route.name === 'string' ? route.name : undefined)
  if (currentMenuId && navigationItems.value.some((item) => item.id === currentMenuId)) {
    return
  }

  const nextRouteName = workbench.firstAccessibleUserCenterRouteName
  if (!nextRouteName) {
    return
  }

  void router.replace({
    name: nextRouteName,
    params: {
      workspaceId: workbench.currentWorkspaceId,
    },
  })
}

function severityTone(severity: 'low' | 'medium' | 'high') {
  if (severity === 'high') {
    return 'error'
  }
  if (severity === 'medium') {
    return 'warning'
  }
  return 'info'
}

watch(
  () => [
    route.name,
    workbench.currentWorkspaceId,
    workbench.currentUserId,
    workbench.currentEffectiveMenuIds.join('|'),
  ],
  () => {
    if (route.name === 'workspace-user-center') {
      return
    }

    ensureAuthorizedChildRoute()
  },
  { immediate: true },
)
</script>

<template>
  <div class="w-full flex flex-col gap-10 pb-20 h-full min-h-0">
    <header class="px-2 shrink-0">
      <UiSectionHeading
        :eyebrow="t('userCenter.header.eyebrow')"
        :title="t('userCenter.header.title')"
      />
    </header>

    <!-- Overview Hero Area -->
    <div class="px-2">
      <div class="flex flex-col md:flex-row md:items-start justify-between gap-6 bg-subtle/30 rounded-lg border border-border-subtle dark:border-white/[0.08] p-6">
        <div class="space-y-4">
          <div class="flex flex-wrap gap-2">
            <UiBadge :label="workspaceName" tone="info" subtle />
            <UiBadge :label="workbench.currentUser?.nickname ?? t('common.na')" subtle />
            <UiBadge :label="currentRoleSummary" subtle />
          </div>
          <div class="space-y-2">
            <h3 class="text-xl font-bold text-text-primary">{{ t('userCenter.header.heroTitle') }}</h3>
            <p class="text-[13px] leading-relaxed text-text-secondary">{{ t('userCenter.header.heroSubtitle') }}</p>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3 min-w-[280px]">
          <UiMetricCard
            v-for="metric in overview.metrics"
            :key="metric.id"
            :label="metric.label"
            :value="metric.value"
            :helper="metric.helper"
            :data-testid="`user-center-metric-${metric.id}`"
            tone="muted"
          />
        </div>
      </div>

      <!-- Quick Links & Alerts -->
      <div v-if="overview.quickLinks.length || overview.alerts.length" class="mt-6 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <RouterLink
          v-for="link in overview.quickLinks"
          :key="link.id"
          class="block min-w-0 no-underline"
          :to="{ name: link.routeName, params: { workspaceId: workbench.currentWorkspaceId } }"
        >
          <UiActionCard :title="link.label" :description="link.helper" class="h-full bg-background" />
        </RouterLink>

        <RouterLink
          v-for="alert in overview.alerts"
          :key="alert.id"
          class="block min-w-0 no-underline"
          :to="alert.routeName ? { name: alert.routeName, params: { workspaceId: workbench.currentWorkspaceId } } : { name: 'workspace-user-center-profile', params: { workspaceId: workbench.currentWorkspaceId } }"
        >
          <UiActionCard :title="alert.title" :description="alert.description" class="h-full bg-warning/5 border-warning/20 hover:border-warning/40">
            <template #suffix>
              <UiBadge :label="t(`enum.riskLevel.${alert.severity}`)" :tone="severityTone(alert.severity)" />
            </template>
          </UiActionCard>
        </RouterLink>
      </div>
    </div>

    <!-- Main Content Split View -->
    <div class="flex flex-1 min-h-0 gap-8 px-2 border-t border-border-subtle dark:border-white/[0.05] pt-8">
      
      <!-- Left: Navigation Sidebar -->
      <aside class="w-64 shrink-0 border-r border-border-subtle dark:border-white/[0.05] pr-8">
        <h3 class="text-sm font-bold text-text-primary mb-4">{{ t('userCenter.nav.title') }}</h3>
        <nav class="flex flex-col gap-1">
          <RouterLink
            v-for="item in navigationItems"
            :key="item.id"
            :to="navigationRoute(item.id)"
            class="flex items-center justify-between px-3 py-2 rounded-md transition-colors text-[13px]"
            :class="item.active ? 'bg-accent text-text-primary font-medium' : 'text-text-secondary hover:bg-accent hover:text-text-primary'"
            :data-testid="`user-center-nav-${navigationTestId(item.id)}`"
          >
            <span>{{ item.label }}</span>
          </RouterLink>
        </nav>
      </aside>

      <!-- Right: Sub-routes Content -->
      <main class="flex-1 overflow-y-auto min-h-0 pb-8">
        <RouterView />
      </main>

    </div>
  </div>
</template>
