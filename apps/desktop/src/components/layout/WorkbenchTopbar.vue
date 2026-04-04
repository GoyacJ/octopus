<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import {
  Check,
  Menu,
  Monitor,
  MoonStar,
  Search,
  Settings,
  SunMedium,
  UserRound,
} from 'lucide-vue-next'

import { UiButton } from '@octopus/ui'

import { createWorkspaceSwitchTarget } from '@/i18n/navigation'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const themeMenuOpen = ref(false)
const localeMenuOpen = ref(false)
const accountMenuOpen = ref(false)

const workspaceLabel = computed(() =>
  workbench.activeWorkspace
    ? workbench.workspaceDisplayName(workbench.activeWorkspace.id)
    : 'Octopus',
)

const currentPageName = computed(() => {
  const name = String(route.name || '')
  if (name.includes('conversations')) return t('sidebar.workspaceMenu.items.conversations')
  if (name.includes('project-agents')) return t('sidebar.workspaceMenu.items.agents')
  if (name.includes('agents')) return t('sidebar.workspaceMenu.items.agents')
  if (name.includes('knowledge')) return t('sidebar.workspaceMenu.items.knowledge')
  if (name.includes('resources')) return t('sidebar.workspaceMenu.items.resources')
  if (name.includes('trace')) return t('sidebar.workspaceMenu.items.trace')
  if (name.includes('models')) return t('sidebar.workspaceMenu.items.models')
  if (name.includes('tools')) return t('sidebar.workspaceMenu.items.tools')
  if (name.includes('automations')) return t('sidebar.workspaceMenu.items.automations')
  if (name.includes('settings')) return t('topbar.settings')
  if (name.includes('user-center')) return t('sidebar.workspaceMenu.items.userCenter')
  if (name.includes('workspace-overview')) return t('sidebar.workspaceMenu.items.dashboard')
  return ''
})

const currentRoleLabel = computed(() => workbench.currentUserRoles[0]?.name ?? t('topbar.profileRole'))
const isSettingsRoute = computed(() => String(route.name || '') === 'settings')

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

function toggleThemeMenu() {
  themeMenuOpen.value = !themeMenuOpen.value
  localeMenuOpen.value = false
  accountMenuOpen.value = false
}

function toggleLocaleMenu() {
  localeMenuOpen.value = !localeMenuOpen.value
  themeMenuOpen.value = false
  accountMenuOpen.value = false
}

function toggleAccountMenu() {
  accountMenuOpen.value = !accountMenuOpen.value
  themeMenuOpen.value = false
  localeMenuOpen.value = false
}

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
  await router.push({
    name: 'settings',
    params: { workspaceId: workbench.currentWorkspaceId },
  })
}

async function switchWorkspace(workspaceId: string) {
  if (!workspaceId || workspaceId === workbench.currentWorkspaceId) {
    accountMenuOpen.value = false
    return
  }

  workbench.selectWorkspace(workspaceId)
  accountMenuOpen.value = false
  await router.push(createWorkspaceSwitchTarget(workbench.workspaces, workspaceId))
}

async function addWorkspace() {
  const workspace = workbench.createWorkspace()
  accountMenuOpen.value = false
  await router.push(createWorkspaceSwitchTarget(workbench.workspaces, workspace.id))
}

async function removeWorkspace(workspaceId: string) {
  if (workbench.workspaces.length <= 1) {
    return
  }

  const confirmed = window.confirm(t('topbar.removeWorkspace'))
  if (!confirmed) {
    return
  }

  const nextWorkspaceId = workbench.removeWorkspace(workspaceId)

  if (nextWorkspaceId) {
    await router.push(createWorkspaceSwitchTarget(workbench.workspaces, nextWorkspaceId))
  }
}
</script>

<template>
  <header
    class="flex h-12 items-center justify-between border-b border-border-subtle dark:border-white/[0.05] bg-background px-4 sticky top-0 z-30"
    data-testid="workbench-topbar"
  >
    <div class="flex min-w-0 items-center gap-3">
      <div data-testid="topbar-brand-frame" class="flex items-center gap-2">
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

        <div class="brand-logo-image flex h-7 w-7 items-center justify-center rounded-md bg-primary/10 text-xs font-bold text-primary">
          O
        </div>
        <span data-testid="brand-title" class="text-sm font-semibold text-text-primary">Octopus</span>
      </div>

      <div class="hidden min-w-0 items-center gap-2 text-sm text-text-secondary md:flex">
        <span class="truncate">{{ workspaceLabel }}</span>
        <span v-if="currentPageName" class="text-text-tertiary">/</span>
        <span v-if="currentPageName" class="truncate text-text-primary">{{ currentPageName }}</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div data-testid="topbar-search-frame">
        <button
          type="button"
          data-testid="global-search-trigger"
          class="flex items-center gap-2 rounded-md border border-border-subtle dark:border-white/[0.08] px-2.5 py-1.5 text-xs text-text-secondary hover:bg-accent"
          @click="shell.openSearch"
        >
          <Search :size="14" />
          <span>{{ t('topbar.searchPlaceholder') }}</span>
        </button>
      </div>

      <div data-testid="topbar-menu-frame" class="relative">
        <div data-testid="topbar-actions" class="flex items-center gap-1.5">
          <div data-testid="topbar-menu" class="flex items-center gap-1.5">
            <div class="relative">
              <UiButton variant="ghost" size="icon" data-testid="topbar-theme-toggle" class="h-8 w-8" @click="toggleThemeMenu">
                <component :is="themeIcons[shell.preferences.theme]" :size="15" />
              </UiButton>
              <div v-if="themeMenuOpen" data-testid="topbar-theme-menu" class="absolute right-0 top-10 z-40 w-44 rounded-lg border border-border-subtle dark:border-white/[0.08] bg-background p-1 shadow-lg">
                <div data-testid="topbar-theme-menu-panel" class="flex flex-col gap-0.5">
                  <button
                    v-for="(icon, key) in themeIcons"
                    :key="key"
                    type="button"
                    :data-testid="`topbar-theme-option-${key}`"
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
            </div>

            <div class="relative">
              <UiButton variant="ghost" size="icon" data-testid="topbar-locale-toggle" class="h-8 w-8" @click="toggleLocaleMenu">
                <span class="text-[11px] font-bold uppercase">{{ shell.preferences.locale === 'zh-CN' ? '中' : 'EN' }}</span>
              </UiButton>
              <div v-if="localeMenuOpen" data-testid="topbar-locale-menu" class="absolute right-0 top-10 z-40 w-40 rounded-lg border border-border-subtle dark:border-white/[0.08] bg-background p-1 shadow-lg">
                <div data-testid="topbar-locale-menu-panel" class="flex flex-col gap-0.5">
                  <button
                    v-for="locale in localeOptions"
                    :key="locale"
                    type="button"
                    :data-testid="`topbar-locale-option-${locale}`"
                    class="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-sm hover:bg-accent"
                    @click="selectLocale(locale)"
                  >
                    <span>{{ t(`topbar.localeModes.${locale}`) }}</span>
                    <Check v-if="shell.preferences.locale === locale" :size="14" class="text-primary" />
                  </button>
                </div>
              </div>
            </div>

            <button
              type="button"
              data-testid="topbar-settings-button"
              class="rounded-md px-2.5 py-1.5 text-xs text-text-secondary hover:bg-accent"
              :class="{ active: isSettingsRoute, 'bg-accent text-text-primary': isSettingsRoute }"
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
                class="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-accent"
                @click="toggleAccountMenu"
              >
                <div class="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-[10px] font-bold text-white uppercase">
                  {{ workbench.currentUser?.nickname?.slice(0, 1) || 'U' }}
                </div>
                <UserRound :size="14" class="text-text-tertiary" />
              </button>

              <div v-if="accountMenuOpen" data-testid="topbar-account-menu" class="absolute right-0 top-10 z-40 w-64 rounded-lg border border-border-subtle dark:border-white/[0.08] bg-background p-3 shadow-lg">
                <div data-testid="topbar-account-menu-panel" class="space-y-3">
                  <div class="flex items-center gap-3">
                    <div class="flex h-10 w-10 items-center justify-center rounded-full bg-primary text-sm font-bold text-white uppercase">
                      {{ workbench.currentUser?.nickname?.slice(0, 1) || 'U' }}
                    </div>
                    <div class="min-w-0 flex-1">
                      <div class="truncate text-sm font-semibold text-text-primary">{{ workbench.currentUser?.nickname }}</div>
                      <div class="truncate text-xs text-text-secondary">{{ currentRoleLabel }}</div>
                    </div>
                  </div>

                  <div class="border-t border-border-subtle dark:border-white/[0.05] pt-2">
                    <button
                      type="button"
                      class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm text-text-secondary hover:bg-accent"
                      @click="openSettings"
                    >
                      <Settings :size="14" />
                      {{ t('topbar.settings') }}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </header>
</template>
