<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import type { InboxItemRecord, NotificationRecord } from '@octopus/schema'
import { AlertTriangle, Bell, Menu, Monitor, MoonStar, Search, Settings, SunMedium, UserRound } from 'lucide-vue-next'

import { UiButton, UiKbd, UiMessageCenter, UiNotificationBadge, UiOfflineBanner, UiPopover, UiSelectionMenu } from '@octopus/ui'

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

const themeOptions = ['system', 'light', 'dark'] as const
const themeIcons = {
  system: Monitor,
  light: SunMedium,
  dark: MoonStar,
} as const

type ThemeMode = (typeof themeOptions)[number]
const localeOptions = ['zh-CN', 'en-US'] as const
type LocaleMode = (typeof localeOptions)[number]

const themeMenuSections = computed(() => [
  {
    id: 'themes',
    items: themeOptions.map(mode => ({
      id: mode,
      label: t(`topbar.themeModes.${mode}`),
      icon: themeIcons[mode],
      active: shell.preferences.theme === mode,
      testId: `topbar-theme-option-${mode}`,
    })),
  },
])

const localeMenuSections = computed(() => [
  {
    id: 'locales',
    items: localeOptions.map(locale => ({
      id: locale,
      label: t(`topbar.localeModes.${locale}`),
      active: shell.preferences.locale === locale,
      testId: `topbar-locale-option-${locale}`,
    })),
  },
])

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

async function selectTheme(theme: 'light' | 'dark' | 'system') {
  await shell.updatePreferences({ theme })
  themeMenuOpen.value = false
}

async function selectLocale(locale: 'zh-CN' | 'en-US') {
  await shell.updatePreferences({ locale })
  localeMenuOpen.value = false
}

function isThemeMode(value: string): value is ThemeMode {
  return themeOptions.some(mode => mode === value)
}

function isLocaleMode(value: string): value is LocaleMode {
  return localeOptions.some(locale => locale === value)
}

function handleThemeMenuSelect(id: string) {
  if (!isThemeMode(id)) {
    return
  }

  void selectTheme(id)
}

function handleLocaleMenuSelect(id: string) {
  if (!isLocaleMode(id)) {
    return
  }

  void selectLocale(id)
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

const connectionBanner = computed(() => {
  const connection = shell.activeWorkspaceConnection
  if (!connection) {
    return null
  }

  if (connection.status === 'connected' || connection.status === 'expired') {
    return null
  }

  if (connection.status === 'unreachable') {
    return {
      tone: 'danger' as const,
      title: t('topbar.connection.unreachable.title', { workspace: workspaceLabel.value }),
      description: t('topbar.connection.unreachable.description'),
      showRetry: true,
    }
  }

  return {
    tone: 'warning' as const,
    title: t('topbar.connection.disconnected.title', { workspace: workspaceLabel.value }),
    description: t('topbar.connection.disconnected.description'),
    showRetry: false,
  }
})

function handleMessageCenterOpenChange(open: boolean) {
  if (open) {
    closeLegacyMenus()
    messageCenter.openCenter()
    return
  }

  messageCenter.closeCenter()
}

function handleThemeMenuOpenChange(open: boolean) {
  if (open) {
    closeMessageCenter()
    localeMenuOpen.value = false
    accountMenuOpen.value = false
  }

  themeMenuOpen.value = open
}

function handleLocaleMenuOpenChange(open: boolean) {
  if (open) {
    closeMessageCenter()
    themeMenuOpen.value = false
    accountMenuOpen.value = false
  }

  localeMenuOpen.value = open
}

function handleAccountMenuOpenChange(open: boolean) {
  if (open) {
    closeMessageCenter()
    themeMenuOpen.value = false
    localeMenuOpen.value = false
  }

  accountMenuOpen.value = open
}

function shellTriggerStateClasses(active: boolean) {
  return active
    ? 'border-border-strong bg-accent text-text-primary'
    : 'text-text-secondary hover:border-border hover:bg-subtle hover:text-text-primary'
}

function shellTriggerIconButtonClasses(active: boolean) {
  return `h-8 w-8 border border-transparent ${shellTriggerStateClasses(active)}`.trim()
}

function shellTriggerButtonClasses(active: boolean) {
  return `border border-transparent ${shellTriggerStateClasses(active)}`.trim()
}

function notificationTriggerClasses() {
  return shellTriggerIconButtonClasses(messageCenter.open)
}

function settingsButtonClasses() {
  return `ui-focus-ring rounded-[var(--radius-xs)] px-2.5 py-1.5 text-xs transition-colors ${shellTriggerButtonClasses(isSettingsRoute.value)}`.trim()
}

function searchTriggerClasses() {
  return [
    'ui-focus-ring flex items-center justify-between min-w-[240px] rounded-[var(--radius-lg)] border px-3 py-2 text-[13px] transition-all duration-normal ease-apple',
    shell.searchOpen
      ? 'border-primary bg-surface text-text-primary shadow-[var(--layer-glow-primary)] ring-1 ring-primary/20'
      : 'border-border/60 bg-subtle/50 text-text-secondary hover:border-border-strong hover:bg-subtle hover:text-text-primary hover:shadow-sm',
  ].join(' ')
}

function profileTriggerClasses() {
  return shellTriggerButtonClasses(accountMenuOpen.value)
}

function profileCaretClasses() {
  return accountMenuOpen.value ? 'text-text-primary' : 'text-text-tertiary'
}

function notificationIconClasses() {
  return messageCenter.open ? 'text-text-primary' : 'text-text-secondary'
}

function searchLabelClasses() {
  return 'max-w-[11rem] truncate'
}

function themeToggleButtonClasses() {
  return shellTriggerIconButtonClasses(themeMenuOpen.value)
}

function localeToggleButtonClasses() {
  return shellTriggerIconButtonClasses(localeMenuOpen.value)
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

async function retryConnection() {
  await shell.refreshBackendStatus()
}
</script>

<template>
  <div class="sticky top-0 z-30 pointer-events-none">
    <header
      class="flex h-16 items-center justify-between px-6 pt-2 pointer-events-auto"
      data-testid="workbench-topbar"
    >
      <div class="flex min-w-0 items-center gap-3">
        <div class="flex items-center gap-2 bg-white/5 backdrop-blur-md border border-white/5 px-3 py-1.5 rounded-2xl shadow-lg">
          <UiButton
            v-if="shell.leftSidebarCollapsed"
            variant="ghost"
            size="icon"
            data-testid="topbar-left-sidebar-toggle"
            class="h-7 w-7 rounded-lg hover:bg-white/10"
            @click="shell.toggleLeftSidebar()"
          >
            <Menu :size="16" />
          </UiButton>

          <nav class="hidden items-center gap-1 md:flex" data-testid="topbar-breadcrumbs" aria-label="Breadcrumb">
            <template v-for="(item, index) in breadcrumbItems" :key="`${index}:${item}`">
              <div 
                class="flex items-center px-2 py-0.5 rounded-lg transition-all duration-fast"
                :class="index === breadcrumbItems.length - 1 ? 'text-primary font-bold' : 'text-text-tertiary'"
              >
                <span class="max-w-[120px] truncate text-[11px] uppercase tracking-wider">
                  {{ item }}
                </span>
              </div>
              <span v-if="index < breadcrumbItems.length - 1" class="text-text-tertiary/20 font-light mx-0">/</span>
            </template>
          </nav>
        </div>
      </div>

      <div class="flex items-center gap-3 bg-white/5 backdrop-blur-md border border-white/5 p-1.5 rounded-2xl shadow-lg">
        <button
          type="button"
          data-testid="global-search-trigger"
          :class="searchTriggerClasses()"
          :aria-pressed="shell.searchOpen ? 'true' : 'false'"
          @click="shell.openSearch"
        >
          <Search :size="14" class="text-text-tertiary" />
          <span class="hidden lg:inline text-[12px] font-medium opacity-60">{{ t('topbar.searchPlaceholder') }}</span>
          <div class="flex items-center gap-0.5 ml-2">
            <UiKbd
              v-for="key in shell.searchShortcutKeys"
              :key="key"
              size="sm"
              class="bg-white/10 border-transparent text-[9px] font-black min-w-[16px] h-[16px]"
            >
              {{ key }}
            </UiKbd>
          </div>
        </button>

        <div class="h-4 w-px bg-white/10 mx-1" />

      <UiSelectionMenu
        :open="themeMenuOpen"
        align="end"
        side="bottom"
        class="w-56"
        :title="t('topbar.theme')"
        :description="t('topbar.themeMenuLabel')"
        :sections="themeMenuSections"
        test-id="topbar-theme-menu"
        @update:open="handleThemeMenuOpenChange"
        @select="handleThemeMenuSelect"
      >
        <template #trigger>
          <UiButton
            variant="ghost"
            size="icon"
            data-testid="topbar-theme-toggle"
            :class="themeToggleButtonClasses()"
            :aria-label="t('topbar.theme')"
          >
            <component :is="themeIcons[shell.preferences.theme]" :size="15" />
          </UiButton>
        </template>
      </UiSelectionMenu>

      <UiSelectionMenu
        :open="localeMenuOpen"
        align="end"
        side="bottom"
        class="w-56"
        :title="t('topbar.locale')"
        :description="t('topbar.localeMenuLabel')"
        :sections="localeMenuSections"
        test-id="topbar-locale-menu"
        @update:open="handleLocaleMenuOpenChange"
        @select="handleLocaleMenuSelect"
      >
        <template #trigger>
          <UiButton
            variant="ghost"
            size="icon"
            data-testid="topbar-locale-toggle"
            :class="localeToggleButtonClasses()"
            :aria-label="t('topbar.locale')"
          >
            <span class="text-[11px] font-bold uppercase">{{ shell.preferences.locale === 'zh-CN' ? '中' : 'EN' }}</span>
          </UiButton>
        </template>
      </UiSelectionMenu>

      <button
        type="button"
        data-testid="topbar-settings-button"
        :class="settingsButtonClasses()"
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
            class="ui-focus-ring relative flex items-center justify-center rounded-[var(--radius-xs)] transition-colors"
            :class="notificationTriggerClasses()"
            :aria-label="t('notifications.triggerAriaLabel')"
          >
            <Bell :size="15" :class="notificationIconClasses()" />
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

      <UiPopover
        :open="accountMenuOpen"
        align="end"
        side="bottom"
        class="w-64 overflow-hidden p-0"
        root-class="!inline-flex"
        @update:open="handleAccountMenuOpenChange"
      >
        <template #trigger>
          <button
            type="button"
            data-testid="topbar-profile-trigger"
            class="ui-focus-ring flex items-center gap-2 rounded-[var(--radius-xs)] px-2 py-1.5 transition-colors"
            :class="profileTriggerClasses()"
            :aria-label="t('topbar.accountSectionTitle')"
          >
            <div class="flex h-6 w-6 items-center justify-center overflow-hidden rounded-full bg-primary text-[10px] font-bold text-white uppercase">
              <img v-if="currentUser?.avatar" :src="currentUser.avatar" alt="" class="h-full w-full object-cover">
              <span v-else>{{ currentUser?.displayName?.slice(0, 1) || 'U' }}</span>
            </div>
            <UserRound :size="14" :class="profileCaretClasses()" />
          </button>
        </template>

        <div data-testid="topbar-account-menu" class="flex flex-col">
          <div
            data-testid="topbar-account-menu-intro"
            class="border-b border-border bg-subtle px-4 py-3"
          >
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
          </div>

          <div class="px-2 py-2">
            <div class="rounded-[var(--radius-m)] border border-transparent px-2 py-2">
              <div class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
                {{ t('topbar.accountSectionTitle') }}
              </div>
              <div class="mt-1 text-sm text-text-secondary">
                {{ workspaceLabel }}
              </div>
            </div>
          </div>

          <div
            data-testid="topbar-account-menu-actions"
            class="border-t border-border bg-subtle px-2 py-2"
          >
            <UiButton
              variant="ghost"
              class="w-full justify-start rounded-[var(--radius-m)] px-3 py-2 text-left text-sm text-text-secondary"
              @click="openPersonalCenter"
            >
              <UserRound :size="14" />
              {{ t('sidebar.navigation.personalCenter') }}
            </UiButton>
          </div>
        </div>
      </UiPopover>
      </div>
    </header>

    <UiOfflineBanner
      v-if="connectionBanner"
      test-id="topbar-connection-banner"
      actions-test-id="topbar-connection-banner-actions"
      :tone="connectionBanner.tone"
      :title="connectionBanner.title"
      :description="connectionBanner.description"
    >
      <template #icon>
        <AlertTriangle :size="16" />
      </template>

      <template v-if="connectionBanner.showRetry" #actions>
        <UiButton
          size="sm"
          variant="outline"
          data-testid="topbar-connection-retry"
          :disabled="shell.syncingBackend"
          @click="retryConnection"
        >
          {{ shell.syncingBackend ? t('topbar.connection.retrying') : t('topbar.connection.retry') }}
        </UiButton>
      </template>
    </UiOfflineBanner>
  </div>
</template>
