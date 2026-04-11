<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import type { InboxItemRecord, NotificationRecord } from '@octopus/schema'
import { Bell, Check, Menu, Monitor, MoonStar, Search, Settings, SunMedium, UserRound } from 'lucide-vue-next'

import { UiButton, UiMessageCenter, UiNotificationBadge, UiPopover } from '@octopus/ui'

import { resolveWorkspaceLabel } from '@/composables/workspace-label'
import { getAncestorMenuIds, getMenuDefinition, getRouteMenuId } from '@/navigation/menuRegistry'
import { useInboxStore } from '@/stores/inbox'
import { useMessageCenterStore } from '@/stores/message-center'
import { useNotificationStore } from '@/stores/notifications'
import { useShellStore } from '@/stores/shell'
import { useUserProfileStore } from '@/stores/user-profile'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const inbox = useInboxStore()
const messageCenter = useMessageCenterStore()
const notifications = useNotificationStore()
const shell = useShellStore()
const userProfileStore = useUserProfileStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()
const workspaceStore = useWorkspaceStore()

const themeMenuOpen = ref(false)
const localeMenuOpen = ref(false)
const accountMenuOpen = ref(false)

const workspaceLabel = computed(() =>
  resolveWorkspaceLabel(
    shell.activeWorkspaceConnection,
    workspaceStore.activeWorkspace?.name,
    t,
  ),
)

const projectLabel = computed(() => workspaceStore.activeProject?.name)

const currentRouteName = computed(() => typeof route.name === 'string' ? route.name : '')
const currentRouteMenuId = computed(() => getRouteMenuId(currentRouteName.value))
const currentRouteMenuDefinition = computed(() => {
  if (!currentRouteMenuId.value) {
    return undefined
  }

  return getMenuDefinition(currentRouteMenuId.value)
})
const currentTopLevelMenuDefinition = computed(() => {
  if (!currentRouteMenuId.value) {
    return undefined
  }

  const topLevelMenuId = getAncestorMenuIds(currentRouteMenuId.value)[0] ?? currentRouteMenuId.value
  return getMenuDefinition(topLevelMenuId)
})
const currentPageName = computed(() =>
  currentTopLevelMenuDefinition.value?.labelKey
    ? t(currentTopLevelMenuDefinition.value.labelKey)
    : '',
)
const breadcrumbItems = computed(() => {
  const items: string[] = ['Octopus']
  const section = currentRouteMenuDefinition.value?.section

  if (section && section !== 'app') {
    items.push(workspaceLabel.value)
  }

  if (section === 'project' && projectLabel.value) {
    items.push(projectLabel.value)
  }

  if (currentPageName.value) {
    items.push(currentPageName.value)
  }

  return items
})

const currentUser = computed(() => userProfileStore.currentUser)
const currentRoleLabel = computed(() => workspaceAccessControlStore.currentRoleNames[0] ?? t('topbar.profileRole'))
const isSettingsRoute = computed(() => String(route.name ?? '') === 'app-settings')

const themeIcons = {
  system: Monitor,
  light: SunMedium,
  dark: MoonStar,
} as const

const localeOptions = ['zh-CN', 'en-US'] as const

function closeLegacyMenus() {
  themeMenuOpen.value = false
  localeMenuOpen.value = false
  accountMenuOpen.value = false
}

function closeMessageCenter() {
  messageCenter.closeCenter()
}

function closeMenus() {
  closeLegacyMenus()
  closeMessageCenter()
}

function handleClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement
  if (!target.closest('.dropdown-trigger') && !target.closest('.dropdown-menu')) {
    closeLegacyMenus()
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})

async function selectTheme(theme: 'light' | 'dark' | 'system') {
  await shell.updatePreferences({ theme })
  themeMenuOpen.value = false
}

async function selectLocale(locale: 'zh-CN' | 'en-US') {
  await shell.updatePreferences({ locale })
  localeMenuOpen.value = false
}

async function openSettings() {
  closeMenus()
  await router.push({ name: 'app-settings' })
}

async function openPersonalCenter() {
  closeMenus()
  if (!workspaceStore.currentWorkspaceId) {
    return
  }
  await router.push({
    name: 'workspace-personal-center-profile',
    params: { workspaceId: workspaceStore.currentWorkspaceId },
  })
}

const notificationFilterLabels = computed(() => ({
  all: t('notifications.filters.all'),
  app: t('notifications.filters.app'),
  workspace: t('notifications.filters.workspace'),
  user: t('notifications.filters.user'),
}))

const notificationScopeLabels = computed(() => ({
  app: t('notifications.scopes.app'),
  workspace: t('notifications.scopes.workspace'),
  user: t('notifications.scopes.user'),
}))

const notificationUnreadLabel = computed(() =>
  t('notifications.unreadCount', { count: notifications.unreadSummary.total }),
)

const inboxSubtitle = computed(() =>
  t('messageCenter.inbox.subtitle', { count: inbox.actionableCount }),
)

function handleMessageCenterOpenChange(open: boolean) {
  if (open) {
    closeLegacyMenus()
    messageCenter.openCenter()
    return
  }

  messageCenter.closeCenter()
}

function toggleThemeMenu() {
  closeMessageCenter()
  localeMenuOpen.value = false
  accountMenuOpen.value = false
  themeMenuOpen.value = !themeMenuOpen.value
}

function toggleLocaleMenu() {
  closeMessageCenter()
  themeMenuOpen.value = false
  accountMenuOpen.value = false
  localeMenuOpen.value = !localeMenuOpen.value
}

function toggleAccountMenu() {
  closeMessageCenter()
  themeMenuOpen.value = false
  localeMenuOpen.value = false
  accountMenuOpen.value = !accountMenuOpen.value
}

async function handleNotificationSelect(notification: NotificationRecord) {
  await notifications.markRead(notification.id)
  closeMessageCenter()
  if (notification.routeTo) {
    await router.push(notification.routeTo)
  }
}

async function handleInboxSelect(item: InboxItemRecord) {
  if (!item.routeTo) {
    return
  }

  closeMessageCenter()
  await router.push(item.routeTo)
}
</script>

<template>
  <header
    class="sticky top-0 z-30 flex h-12 items-center justify-between border-b border-border bg-background px-4"
    data-testid="workbench-topbar"
  >
    <div class="flex min-w-0 items-center gap-3">
      <UiButton
        v-if="shell.leftSidebarCollapsed"
        variant="ghost"
        size="icon"
        data-testid="topbar-left-sidebar-toggle"
        class="h-7 w-7"
        @click="shell.toggleLeftSidebar()"
      >
        <Menu :size="16" />
      </UiButton>

      <div class="flex items-center gap-2 text-sm">
        <div class="hidden items-center gap-2 md:flex" data-testid="topbar-breadcrumbs">
          <template v-for="(item, index) in breadcrumbItems" :key="`${index}:${item}`">
            <span
              :class="index === breadcrumbItems.length - 1 ? 'truncate font-medium text-text-primary' : index === 0 ? 'font-semibold text-text-primary' : 'truncate text-text-secondary'"
            >
              {{ item }}
            </span>
            <span v-if="index < breadcrumbItems.length - 1" class="text-text-tertiary">></span>
          </template>
        </div>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <button
        type="button"
        data-testid="global-search-trigger"
        class="flex items-center gap-2 rounded-[var(--radius-xs)] border border-border px-2.5 py-1.5 text-xs text-text-secondary hover:bg-accent"
        @click="shell.openSearch"
      >
        <Search :size="14" />
        <span>{{ t('topbar.searchPlaceholder') }}</span>
      </button>

      <div class="relative">
        <UiButton variant="ghost" size="icon" data-testid="topbar-theme-toggle" class="dropdown-trigger h-8 w-8" @click="toggleThemeMenu">
          <component :is="themeIcons[shell.preferences.theme]" :size="15" />
        </UiButton>
        <div v-if="themeMenuOpen" class="dropdown-menu absolute right-0 top-10 z-40 w-44 rounded-[var(--radius-l)] border border-border bg-popover p-1 shadow-md">
          <button
            v-for="(icon, key) in themeIcons"
            :key="key"
            type="button"
            class="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-sm hover:bg-accent"
            @click="selectTheme(key)"
          >
            <span class="flex items-center gap-2">
              <component :is="icon" :size="14" />
              {{ t(`topbar.themeModes.${key}`) }}
            </span>
            <Check v-if="shell.preferences.theme === key" :size="14" class="text-primary" />
          </button>
        </div>
      </div>

      <div class="relative">
        <UiButton variant="ghost" size="icon" data-testid="topbar-locale-toggle" class="dropdown-trigger h-8 w-8" @click="toggleLocaleMenu">
          <span class="text-[11px] font-bold uppercase">{{ shell.preferences.locale === 'zh-CN' ? '中' : 'EN' }}</span>
        </UiButton>
        <div v-if="localeMenuOpen" class="dropdown-menu absolute right-0 top-10 z-40 w-40 rounded-[var(--radius-l)] border border-border bg-popover p-1 shadow-md">
          <button
            v-for="locale in localeOptions"
            :key="locale"
            type="button"
            class="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-sm hover:bg-accent"
            @click="selectLocale(locale)"
          >
            <span>{{ t(`topbar.localeModes.${locale}`) }}</span>
            <Check v-if="shell.preferences.locale === locale" :size="14" class="text-primary" />
          </button>
        </div>
      </div>

      <button
        type="button"
        data-testid="topbar-settings-button"
        class="rounded-[var(--radius-xs)] px-2.5 py-1.5 text-xs text-text-secondary hover:bg-accent"
        :class="{ 'bg-accent text-text-primary': isSettingsRoute }"
        @click="openSettings"
      >
        <span class="flex items-center gap-1.5">
          <Settings :size="14" />
          {{ t('topbar.settings') }}
        </span>
      </button>

      <UiPopover
        :open="messageCenter.open"
        align="end"
        side="bottom"
        root-class="!inline-flex"
        class="border-none bg-transparent p-0 shadow-none"
        @update:open="handleMessageCenterOpenChange"
      >
        <template #trigger>
          <button
            type="button"
            data-testid="topbar-notification-trigger"
            class="dropdown-trigger relative flex h-8 w-8 items-center justify-center rounded-[var(--radius-xs)] hover:bg-accent"
            :aria-label="t('notifications.triggerAriaLabel')"
          >
            <Bell :size="15" class="text-text-secondary" />
            <span class="absolute -right-1 -top-1">
              <UiNotificationBadge :count="messageCenter.combinedCount" />
            </span>
          </button>
        </template>
        <UiMessageCenter
          :open="messageCenter.open"
          :active-tab="messageCenter.activeTab"
          :notification-tab-label="t('messageCenter.tabs.notifications')"
          :inbox-tab-label="t('messageCenter.tabs.inbox')"
          :notification-title="t('notifications.title')"
          :notification-unread-label="notificationUnreadLabel"
          :notifications="notifications.filteredNotifications"
          :unread-count="notifications.unreadSummary.total"
          :active-filter="notifications.filterScope"
          :filter-labels="notificationFilterLabels"
          :scope-labels="notificationScopeLabels"
          :notification-empty-title="t('notifications.empty.title')"
          :notification-empty-description="t('notifications.empty.description')"
          :notification-mark-all-label="t('notifications.markAllRead')"
          :inbox-title="t('messageCenter.inbox.title')"
          :inbox-subtitle="inboxSubtitle"
          :inbox-loading="inbox.loading"
          :inbox-error="inbox.error"
          :inbox-items="inbox.items"
          :inbox-empty-title="t('messageCenter.inbox.emptyTitle')"
          :inbox-empty-description="t('messageCenter.inbox.emptyDescription')"
          :inbox-open-label="t('messageCenter.inbox.openLabel')"
          :inbox-status-heading="t('messageCenter.inbox.statusHeading')"
          :inbox-type-heading="t('messageCenter.inbox.typeHeading')"
          :inbox-loading-label="t('messageCenter.inbox.loadingLabel')"
          :inbox-error-title="t('messageCenter.inbox.errorTitle')"
          :inbox-error-description="t('messageCenter.inbox.errorDescription')"
          @update:active-tab="messageCenter.setActiveTab"
          @update:filter="notifications.setFilter"
          @mark-read="notifications.markRead"
          @mark-all-read="notifications.markAllRead({ scope: notifications.filterScope })"
          @select-notification="handleNotificationSelect"
          @select-inbox="handleInboxSelect"
        />
      </UiPopover>

      <div class="relative">
        <button
          type="button"
          data-testid="topbar-profile-trigger"
          class="dropdown-trigger flex items-center gap-2 rounded-[var(--radius-xs)] px-2 py-1.5 hover:bg-accent"
          @click="toggleAccountMenu"
        >
          <div class="flex h-6 w-6 items-center justify-center overflow-hidden rounded-full bg-primary text-[10px] font-bold text-white uppercase">
            <img v-if="currentUser?.avatar" :src="currentUser.avatar" alt="" class="h-full w-full object-cover">
            <span v-else>{{ currentUser?.displayName?.slice(0, 1) || 'U' }}</span>
          </div>
          <UserRound :size="14" class="text-text-tertiary" />
        </button>

        <div v-if="accountMenuOpen" class="dropdown-menu absolute right-0 top-10 z-40 w-64 rounded-[var(--radius-l)] border border-border bg-popover p-3 shadow-md">
          <div class="space-y-3">
            <div class="flex items-center gap-3">
              <div class="flex h-10 w-10 items-center justify-center overflow-hidden rounded-full bg-primary text-sm font-bold text-white uppercase">
                <img v-if="currentUser?.avatar" :src="currentUser.avatar" alt="" class="h-full w-full object-cover">
                <span v-else>{{ currentUser?.displayName?.slice(0, 1) || 'U' }}</span>
              </div>
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm font-semibold text-text-primary">{{ currentUser?.displayName ?? t('common.na') }}</div>
                <div class="truncate text-xs text-text-secondary">{{ currentRoleLabel }}</div>
              </div>
            </div>

            <div class="border-t border-border pt-2">
              <button
                type="button"
                class="flex w-full items-center gap-2 rounded-[var(--radius-xs)] px-2 py-1.5 text-left text-sm text-text-secondary hover:bg-accent"
                @click="openPersonalCenter"
              >
                <UserRound :size="14" />
                {{ t('sidebar.navigation.personalCenter') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </header>
</template>
