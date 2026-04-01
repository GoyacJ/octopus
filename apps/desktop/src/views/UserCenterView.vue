<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'

import { UiBadge, UiSectionHeading } from '@octopus/ui'

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

const navigationItems = computed(() =>
  workbench.availableUserCenterMenus
    .map((menu) => {
      const definition = getMenuDefinition(menu.id)
      if (!definition?.routeName) {
        return undefined
      }

      return {
        id: menu.id,
        label: menu.label,
        routeName: definition.routeName,
        testId: definition.routeName.replace('user-center-', ''),
      }
    })
    .filter((item): item is { id: string, label: string, routeName: string, testId: string } => Boolean(item)),
)

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
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('userCenter.header.eyebrow')"
      :title="t('userCenter.header.title')"
      :subtitle="t('userCenter.header.subtitle')"
    />

    <div class="user-center-summary">
      <UiBadge :label="workspaceName" tone="info" />
      <UiBadge :label="workbench.currentUser?.nickname ?? t('common.na')" subtle />
      <UiBadge :label="currentRoleSummary" subtle />
    </div>

    <div class="user-center-shell">
      <aside class="user-center-nav">
        <div class="nav-header">
          <strong>{{ t('userCenter.nav.title') }}</strong>
          <small>{{ t('userCenter.nav.subtitle') }}</small>
        </div>

        <RouterLink
          v-for="item in navigationItems"
          :key="item.id"
          class="nav-link"
          :class="{ active: route.name === item.routeName }"
          :data-testid="`user-center-nav-${item.testId}`"
          :to="{ name: item.routeName, params: { workspaceId: workbench.currentWorkspaceId } }"
        >
          <span>{{ item.label }}</span>
        </RouterLink>
      </aside>

      <div class="user-center-panel">
        <RouterView />
      </div>
    </div>
  </section>
</template>

<style scoped>
.user-center-summary,
.nav-link {
  display: flex;
  align-items: center;
}

.user-center-summary {
  flex-wrap: wrap;
  gap: 0.65rem;
}

.user-center-shell {
  display: grid;
  grid-template-columns: minmax(220px, 260px) minmax(0, 1fr);
  gap: 1rem;
  min-height: 0;
}

.user-center-nav,
.user-center-panel {
  min-height: 0;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-xl);
  background: color-mix(in srgb, var(--bg-surface) 92%, transparent);
  box-shadow: var(--shadow-sm);
}

.user-center-nav {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 1rem;
}

.nav-header {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin-bottom: 0.5rem;
}

.nav-header small {
  color: var(--text-secondary);
}

.nav-link {
  justify-content: space-between;
  min-height: 2.8rem;
  padding: 0 0.9rem;
  border-radius: var(--radius-l);
  color: var(--text-secondary);
  transition: background-color 160ms ease, color 160ms ease, transform 160ms ease;
}

.nav-link:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--brand-primary) 8%, transparent);
}

.nav-link.active {
  color: var(--text-primary);
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--brand-primary) 16%, transparent), color-mix(in srgb, var(--brand-primary) 4%, transparent));
  transform: translateX(2px);
}

.user-center-panel {
  padding: 1rem;
}

@media (max-width: 960px) {
  .user-center-shell {
    grid-template-columns: 1fr;
  }
}
</style>
