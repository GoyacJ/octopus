<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'

import {
  UiActionCard,
  UiBadge,
  UiMetricCard,
  UiNavCardList,
  UiPageHero,
  UiPanelFrame,
  UiSectionHeading,
} from '@octopus/ui'

import { resolveMockField } from '@/i18n/copy'
import { getMenuDefinition, getRouteMenuId } from '@/navigation/menuRegistry'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workbench = useWorkbenchStore()

const workspaceName = computed(() =>
  workbench.activeWorkspace
    ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name)
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
        helper: definition.routeName.replace('user-center-', ''),
        routeName: definition.routeName,
        testId: definition.routeName.replace('user-center-', ''),
        active: route.name === definition.routeName,
        badge: route.name === definition.routeName ? t('common.selected') : undefined,
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
    name: entry?.routeName ?? 'user-center-profile',
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
    if (route.name === 'user-center') {
      return
    }

    ensureAuthorizedChildRoute()
  },
  { immediate: true },
)
</script>

<template>
  <section class="section-stack user-center-page">
    <UiSectionHeading
      :eyebrow="t('userCenter.header.eyebrow')"
      :title="t('userCenter.header.title')"
      :subtitle="t('userCenter.header.subtitle')"
    />

    <UiPanelFrame
      variant="hero"
      inner-class="p-0 border-0 bg-transparent shadow-none before:hidden after:hidden"
    >
      <UiPageHero>
        <template #meta>
          <UiBadge :label="workspaceName" tone="info" />
          <UiBadge :label="workbench.currentUser?.nickname ?? t('common.na')" subtle />
          <UiBadge :label="currentRoleSummary" subtle />
        </template>

        <div class="overview-intro">
          <h3>{{ t('userCenter.header.heroTitle') }}</h3>
          <p>{{ t('userCenter.header.heroSubtitle') }}</p>
        </div>

        <template #actions>
          <RouterLink
            v-for="link in overview.quickLinks"
            :key="link.id"
            class="user-center-action-link"
            :to="{ name: link.routeName, params: { workspaceId: workbench.currentWorkspaceId } }"
          >
            <UiActionCard
              :title="link.label"
              :description="link.helper"
              class="h-full"
            />
          </RouterLink>
        </template>

        <template #aside>
          <div class="overview-metrics">
            <UiMetricCard
              v-for="metric in overview.metrics"
              :key="metric.id"
              :label="metric.label"
              :value="metric.value"
              :helper="metric.helper"
              :data-testid="`user-center-metric-${metric.id}`"
            />
          </div>
        </template>
      </UiPageHero>
    </UiPanelFrame>

    <div v-if="overview.alerts.length" class="alert-grid">
      <RouterLink
        v-for="alert in overview.alerts"
        :key="alert.id"
        class="user-center-action-link"
        :to="alert.routeName ? { name: alert.routeName, params: { workspaceId: workbench.currentWorkspaceId } } : { name: 'user-center-profile', params: { workspaceId: workbench.currentWorkspaceId } }"
      >
        <UiActionCard
          :title="alert.title"
          :description="alert.description"
          class="h-full"
        >
          <template #suffix>
            <UiBadge :label="t(`enum.riskLevel.${alert.severity}`)" :tone="severityTone(alert.severity)" />
          </template>
        </UiActionCard>
      </RouterLink>
    </div>
    <UiPanelFrame
      v-else
      variant="subtle"
      padding="sm"
      :title="t('userCenter.alerts.emptyTitle')"
      :subtitle="t('userCenter.alerts.emptyDescription')"
    />

    <div class="user-center-layout">
      <UiPanelFrame
        variant="panel"
        padding="lg"
        :title="t('userCenter.nav.title')"
        :subtitle="t('userCenter.nav.subtitle')"
      >
        <UiNavCardList class="user-center-nav" :items="navigationItems">
          <template #item="{ item, active }">
            <RouterLink
              class="user-center-nav-link"
              :class="{ active }"
              :data-testid="`user-center-nav-${navigationTestId(item.id)}`"
              :to="navigationRoute(item.id)"
            >
              <span class="user-center-nav-copy">
                <strong>{{ item.label }}</strong>
                <small>{{ item.helper }}</small>
              </span>
              <UiBadge v-if="item.badge" :label="item.badge" subtle />
            </RouterLink>
          </template>
        </UiNavCardList>
      </UiPanelFrame>

      <UiPanelFrame variant="panel" padding="lg">
        <RouterView />
      </UiPanelFrame>
    </div>
  </section>
</template>

<style scoped>
.user-center-page {
  gap: 1rem;
}

.overview-intro {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.overview-intro h3,
.overview-intro p {
  margin: 0;
}

.overview-intro h3 {
  font-size: 1.45rem;
  line-height: 1.2;
  letter-spacing: -0.03em;
}

.overview-intro p {
  color: var(--text-secondary);
  line-height: 1.6;
}

.user-center-action-link {
  display: block;
  min-width: 0;
  color: inherit;
  text-decoration: none;
}

.overview-metrics,
.alert-grid {
  display: grid;
  gap: 0.85rem;
}

.overview-metrics {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.alert-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.user-center-layout {
  display: grid;
  grid-template-columns: minmax(15rem, 18rem) minmax(0, 1fr);
  gap: 1rem;
  min-height: 0;
}

.user-center-nav {
  margin-top: 0.25rem;
}

.user-center-nav-link {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  width: 100%;
  padding: 0.25rem 0.1rem;
  color: inherit;
  text-decoration: none;
}

.user-center-nav-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 0.2rem;
}

.user-center-nav-copy strong {
  color: var(--text-primary);
  font-size: 0.92rem;
  line-height: 1.35;
}

.user-center-nav-copy small {
  color: var(--text-secondary);
  font-size: 0.78rem;
  line-height: 1.5;
  text-transform: capitalize;
}

@media (max-width: 1180px) {
  .user-center-layout,
  .alert-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (max-width: 760px) {
  .overview-metrics {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
