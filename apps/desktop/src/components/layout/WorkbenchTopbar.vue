<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { Check, Menu, Monitor, MoonStar, Search, Settings, SunMedium, UserRound } from 'lucide-vue-next'

import { UiButton } from '@octopus/ui'

import { useShellStore } from '@/stores/shell'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const userCenterStore = useUserCenterStore()

const themeMenuOpen = ref(false)
const localeMenuOpen = ref(false)
const accountMenuOpen = ref(false)

const workspaceLabel = computed(() =>
  shell.activeWorkspaceConnection?.label
  ?? workspaceStore.activeWorkspace?.name
  ?? 'Workspace',
)

const currentPageName = computed(() => {
  switch (String(route.name ?? '')) {
    case 'workspace-overview':
      return t('sidebar.navigation.overview')
    case 'project-dashboard':
      return t('sidebar.navigation.dashboard')
    case 'project-conversations':
    case 'project-conversation':
      return t('sidebar.projectModules.conversations')
    case 'workspace-resources':
    case 'project-resources':
      return t('sidebar.navigation.resources')
    case 'workspace-knowledge':
    case 'project-knowledge':
      return t('sidebar.navigation.knowledge')
    case 'workspace-agents':
    case 'project-agents':
      return t('sidebar.navigation.agents')
    case 'workspace-teams':
      return t('sidebar.navigation.teams')
    case 'workspace-models':
      return t('sidebar.navigation.models')
    case 'workspace-tools':
      return t('sidebar.navigation.tools')
    case 'workspace-automations':
      return t('sidebar.navigation.automations')
    case 'project-runtime':
      return t('sidebar.navigation.runtime')
    case 'workspace-user-center':
    case 'workspace-user-center-profile':
    case 'workspace-user-center-users':
    case 'workspace-user-center-roles':
    case 'workspace-user-center-permissions':
    case 'workspace-user-center-menus':
      return t('sidebar.navigation.userCenter')
    case 'app-settings':
      return t('topbar.settings')
    case 'app-connections':
      return t('connections.header.title')
    default:
      return ''
  }
})

const currentUser = computed(() => userCenterStore.currentUser)
const currentRoleLabel = computed(() => userCenterStore.currentRoleNames[0] ?? t('topbar.profileRole'))
const isSettingsRoute = computed(() => String(route.name ?? '') === 'app-settings')

const themeIcons = {
  system: Monitor,
  light: SunMedium,
  dark: MoonStar,
} as const

const localeOptions = ['zh-CN', 'en-US'] as const

function closeMenus() {
  themeMenuOpen.value = false
  localeMenuOpen.value = false
  accountMenuOpen.value = false
}

function handleClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement
  if (!target.closest('.dropdown-trigger') && !target.closest('.dropdown-menu')) {
    closeMenus()
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

async function openUserCenter() {
  closeMenus()
  if (!workspaceStore.currentWorkspaceId) {
    return
  }
  await router.push({
    name: userCenterStore.firstAccessibleUserCenterRouteName ?? 'workspace-user-center',
    params: { workspaceId: workspaceStore.currentWorkspaceId },
  })
}
</script>

<template>
  <header
    class="sticky top-0 z-30 flex h-12 items-center justify-between border-b border-border-subtle bg-background px-4 dark:border-white/[0.05]"
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

      <div class="flex items-center gap-2">
        <span class="text-sm font-semibold text-text-primary">网易Lobster</span>
      </div>

      <div class="hidden min-w-0 items-center gap-2 text-sm text-text-secondary md:flex">
        <span class="truncate">{{ workspaceLabel }}</span>
        <span v-if="currentPageName" class="text-text-tertiary">/</span>
        <span v-if="currentPageName" class="truncate text-text-primary">{{ currentPageName }}</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <button
        type="button"
        data-testid="global-search-trigger"
        class="flex items-center gap-2 rounded-md border border-border-subtle px-2.5 py-1.5 text-xs text-text-secondary hover:bg-accent dark:border-white/[0.08]"
        @click="shell.openSearch"
      >
        <Search :size="14" />
        <span>{{ t('topbar.searchPlaceholder') }}</span>
      </button>

      <div class="relative">
        <UiButton variant="ghost" size="icon" data-testid="topbar-theme-toggle" class="dropdown-trigger h-8 w-8" @click="themeMenuOpen = !themeMenuOpen">
          <component :is="themeIcons[shell.preferences.theme]" :size="15" />
        </UiButton>
        <div v-if="themeMenuOpen" class="dropdown-menu absolute right-0 top-10 z-40 w-44 rounded-lg border border-border-subtle bg-background p-1 shadow-lg dark:border-white/[0.08]">
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
        <UiButton variant="ghost" size="icon" data-testid="topbar-locale-toggle" class="dropdown-trigger h-8 w-8" @click="localeMenuOpen = !localeMenuOpen">
          <span class="text-[11px] font-bold uppercase">{{ shell.preferences.locale === 'zh-CN' ? '中' : 'EN' }}</span>
        </UiButton>
        <div v-if="localeMenuOpen" class="dropdown-menu absolute right-0 top-10 z-40 w-40 rounded-lg border border-border-subtle bg-background p-1 shadow-lg dark:border-white/[0.08]">
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
        class="rounded-md px-2.5 py-1.5 text-xs text-text-secondary hover:bg-accent"
        :class="{ 'bg-accent text-text-primary': isSettingsRoute }"
        @click="openSettings"
      >
        <span class="flex items-center gap-1.5">
          <Settings :size="14" />
          {{ t('topbar.settings') }}
        </span>
      </button>

      <div class="relative">
        <button
          type="button"
          data-testid="topbar-profile-trigger"
          class="dropdown-trigger flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-accent"
          @click="accountMenuOpen = !accountMenuOpen"
        >
          <div class="flex h-6 w-6 items-center justify-center overflow-hidden rounded-full bg-primary text-[10px] font-bold text-white uppercase">
            <img v-if="currentUser?.avatar" :src="currentUser.avatar" alt="" class="h-full w-full object-cover">
            <span v-else>{{ currentUser?.displayName?.slice(0, 1) || 'U' }}</span>
          </div>
          <UserRound :size="14" class="text-text-tertiary" />
        </button>

        <div v-if="accountMenuOpen" class="dropdown-menu absolute right-0 top-10 z-40 w-64 rounded-lg border border-border-subtle bg-background p-3 shadow-lg dark:border-white/[0.08]">
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

            <div class="border-t border-border-subtle pt-2 dark:border-white/[0.05]">
              <button
                type="button"
                class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm text-text-secondary hover:bg-accent"
                @click="openUserCenter"
              >
                <UserRound :size="14" />
                {{ t('sidebar.navigation.userCenter') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </header>
</template>
