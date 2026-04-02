<script setup lang="ts">
import type { Locale, ThemeMode } from '@octopus/schema'
import type { Component } from 'vue'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { Bell, Check, ChevronDown, Globe2, LayoutPanelTop, Monitor, MoonStar, Plus, Search, Settings, SunMedium, X } from 'lucide-vue-next'

import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget, createWorkspaceOverviewTarget, createWorkspaceSwitchTarget } from '@/i18n/navigation'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const topbarMenuRef = ref<HTMLElement | null>(null)
const themeMenuOpen = ref(false)
const localeMenuOpen = ref(false)
const accountMenuOpen = ref(false)

const themeIcons: Record<ThemeMode, Component> = {
  system: Monitor,
  light: SunMedium,
  dark: MoonStar,
}

const currentUser = computed(() => ({
  name: workbench.currentUser?.nickname ?? 'Unknown User',
  email: workbench.currentUser?.email ?? 'unknown@octopus.local',
  role: workbench.currentUserRoles.length
    ? workbench.currentUserRoles.map((role) => role.name).join(' / ')
    : t('userCenter.common.noRoles'),
  avatar: workbench.currentUser?.avatar ?? workspaceLabel.value.slice(0, 1).toUpperCase(),
}))

const currentThemeIcon = computed(() => themeIcons[shell.preferences.theme])

const themeOptions = computed(() => [
  {
    value: 'system' as ThemeMode,
    label: t('topbar.themeModes.system'),
    icon: themeIcons.system,
  },
  {
    value: 'light' as ThemeMode,
    label: t('topbar.themeModes.light'),
    icon: themeIcons.light,
  },
  {
    value: 'dark' as ThemeMode,
    label: t('topbar.themeModes.dark'),
    icon: themeIcons.dark,
  },
])

const localeOptions = computed(() => [
  {
    value: 'zh-CN' as Locale,
    label: t('topbar.localeModes.zh-CN'),
  },
  {
    value: 'en-US' as Locale,
    label: t('topbar.localeModes.en-US'),
  },
])

const workspaceLabel = computed(() =>
  workbench.activeWorkspace
    ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name)
    : 'Octopus',
)

const workspaceMetaLabel = computed(() =>
  workbench.activeWorkspace?.isLocal
    ? t('topbar.localWorkspace')
    : t('topbar.sharedWorkspace'),
)

const settingsButtonActive = computed(() => route.name === 'settings')

function openSearch() {
  shell.openSearch()
}

function closeAllMenus() {
  themeMenuOpen.value = false
  localeMenuOpen.value = false
  accountMenuOpen.value = false
}

function toggleThemeMenu() {
  const nextState = !themeMenuOpen.value
  closeAllMenus()
  themeMenuOpen.value = nextState
}

async function selectTheme(theme: ThemeMode) {
  if (theme !== shell.preferences.theme) {
    await shell.updatePreferences({ theme })
  }

  closeAllMenus()
}

function toggleLocaleMenu() {
  const nextState = !localeMenuOpen.value
  closeAllMenus()
  localeMenuOpen.value = nextState
}

async function selectLocale(locale: Locale) {
  if (locale !== shell.preferences.locale) {
    await shell.updatePreferences({ locale })
  }

  closeAllMenus()
}

function toggleAccountMenu() {
  const nextState = !accountMenuOpen.value
  closeAllMenus()
  accountMenuOpen.value = nextState
}

async function openInbox() {
  closeAllMenus()
  shell.setDetailFocus('timeline')
  shell.setRightSidebarCollapsed(false)
  await router.push({
    ...createProjectConversationTarget(
      workbench.currentWorkspaceId,
      workbench.currentProjectId,
      workbench.currentConversationId,
    ),
    query: workbench.currentConversationId
      ? {
          detail: 'timeline',
        }
      : undefined,
  })
}

async function openSettings() {
  closeAllMenus()
  await router.push({
    name: 'settings',
    params: {
      workspaceId: workbench.currentWorkspaceId,
    },
  })
}

async function switchWorkspace(workspaceId: string) {
  if (!workspaceId || workspaceId === workbench.currentWorkspaceId) {
    closeAllMenus()
    return
  }

  workbench.selectWorkspace(workspaceId)
  closeAllMenus()
  await router.push(createWorkspaceSwitchTarget(workbench.workspaces, workspaceId))
}

async function addWorkspace() {
  const workspace = workbench.createWorkspace()
  closeAllMenus()
  await router.push(createWorkspaceOverviewTarget(workspace.id, workspace.projectIds[0]))
}

async function removeWorkspace(workspaceId: string) {
  const targetWorkspaceId = workbench.removeWorkspace(workspaceId)
  if (!targetWorkspaceId) {
    return
  }

  closeAllMenus()
  await router.push(createWorkspaceSwitchTarget(workbench.workspaces, targetWorkspaceId))
}

function handleClickOutside(event: MouseEvent) {
  if (topbarMenuRef.value?.contains(event.target as Node)) {
    return
  }

  closeAllMenus()
}

onMounted(() => {
  window.addEventListener('mousedown', handleClickOutside)
})

onBeforeUnmount(() => {
  window.removeEventListener('mousedown', handleClickOutside)
})
</script>

<template>
  <header class="topbar-shell" data-testid="workbench-topbar">
    <div class="brand-block">
      <div class="brand-logo" aria-hidden="true">
        <LayoutPanelTop :size="18" />
      </div>
      <div class="brand-copy">
        <strong data-testid="brand-title">Octopus</strong>
        <small>{{ workspaceLabel }}</small>
      </div>
    </div>

    <button
      type="button"
      class="search-trigger"
      data-testid="global-search-trigger"
      @click="openSearch"
    >
      <Search :size="16" />
      <span>{{ t('topbar.searchPlaceholder') }}</span>
      <kbd>⌘K</kbd>
    </button>

    <div ref="topbarMenuRef" class="topbar-menu" data-testid="topbar-menu">
      <div class="topbar-actions" data-testid="topbar-actions">
        <div class="menu-shell">
          <button
            type="button"
            class="icon-button"
            :class="{ active: themeMenuOpen }"
            data-testid="topbar-theme-toggle"
            :title="t('topbar.theme')"
            aria-haspopup="menu"
            :aria-expanded="themeMenuOpen"
            @click="toggleThemeMenu"
          >
            <component :is="currentThemeIcon" :size="16" />
          </button>

          <Transition name="topbar-flyout">
            <div
              v-if="themeMenuOpen"
              class="topbar-dropdown"
              data-testid="topbar-theme-menu"
            >
              <div class="dropdown-heading">
                <strong>{{ t('topbar.theme') }}</strong>
                <small>{{ t('topbar.themeMenuLabel') }}</small>
              </div>
              <button
                v-for="option in themeOptions"
                :key="option.value"
                type="button"
                class="dropdown-option"
                :class="{ selected: option.value === shell.preferences.theme }"
                :data-testid="`topbar-theme-option-${option.value}`"
                @click="selectTheme(option.value)"
              >
                <span class="dropdown-option-icon">
                  <component :is="option.icon" :size="16" />
                </span>
                <span class="dropdown-option-copy">{{ option.label }}</span>
                <Check
                  v-if="option.value === shell.preferences.theme"
                  :size="16"
                  class="dropdown-option-check"
                />
              </button>
            </div>
          </Transition>
        </div>

        <div class="menu-shell">
          <button
            type="button"
            class="icon-button"
            :class="{ active: localeMenuOpen }"
            data-testid="topbar-locale-toggle"
            :title="t('topbar.locale')"
            aria-haspopup="menu"
            :aria-expanded="localeMenuOpen"
            @click="toggleLocaleMenu"
          >
            <Globe2 :size="16" />
          </button>

          <Transition name="topbar-flyout">
            <div
              v-if="localeMenuOpen"
              class="topbar-dropdown locale-dropdown"
              data-testid="topbar-locale-menu"
            >
              <div class="dropdown-heading">
                <strong>{{ t('topbar.locale') }}</strong>
                <small>{{ t('topbar.localeMenuLabel') }}</small>
              </div>
              <button
                v-for="option in localeOptions"
                :key="option.value"
                type="button"
                class="dropdown-option"
                :class="{ selected: option.value === shell.preferences.locale }"
                :data-testid="`topbar-locale-option-${option.value}`"
                @click="selectLocale(option.value)"
              >
                <span class="dropdown-option-copy">{{ option.label }}</span>
                <Check
                  v-if="option.value === shell.preferences.locale"
                  :size="16"
                  class="dropdown-option-check"
                />
              </button>
            </div>
          </Transition>
        </div>
        <button
          type="button"
          class="icon-button"
          :title="t('topbar.inbox')"
          @click="openInbox"
        >
          <Bell :size="16" />
        </button>
        <button
          type="button"
          class="icon-button"
          :class="{ active: settingsButtonActive }"
          data-testid="topbar-settings-button"
          :title="t('topbar.settings')"
          @click="openSettings"
        >
          <Settings :size="16" />
        </button>
      </div>

      <div class="menu-divider" aria-hidden="true" />

      <div class="profile-menu-shell">
        <button
          type="button"
          class="profile-chip"
          :class="{ active: accountMenuOpen }"
          data-testid="topbar-profile-trigger"
          :title="t('topbar.workspaceMenu')"
          aria-haspopup="menu"
          :aria-expanded="accountMenuOpen"
          @click="toggleAccountMenu"
        >
          <span class="profile-avatar">{{ currentUser.avatar }}</span>
          <span class="profile-copy">
            <strong>{{ currentUser.name }}</strong>
            <small>{{ workspaceLabel }} · {{ workspaceMetaLabel }}</small>
          </span>
          <ChevronDown :size="16" class="profile-chevron" :class="{ open: accountMenuOpen }" />
        </button>

        <Transition name="topbar-flyout">
          <div
            v-if="accountMenuOpen"
            class="account-menu"
            data-testid="topbar-account-menu"
          >
            <div class="account-summary">
              <span class="account-avatar">{{ currentUser.avatar }}</span>
              <div class="account-copy">
                <strong>{{ currentUser.name }}</strong>
                <small>{{ currentUser.email }}</small>
                <span>{{ currentUser.role }}</span>
              </div>
            </div>

            <div class="account-section">
              <div class="section-heading">
                <strong>{{ t('topbar.workspaceSectionTitle') }}</strong>
                <small>{{ t('topbar.workspaceSectionSubtitle') }}</small>
              </div>

              <div class="workspace-list">
                <div
                  v-for="workspace in workbench.workspaces"
                  :key="workspace.id"
                  class="workspace-item"
                  :class="{ active: workspace.id === workbench.currentWorkspaceId }"
                >
                  <button
                    type="button"
                    class="workspace-switch"
                    :data-testid="`workspace-switch-${workspace.id}`"
                    @click="switchWorkspace(workspace.id)"
                  >
                    <span class="workspace-avatar">{{ workspace.avatar ?? workspace.name.slice(0, 1).toUpperCase() }}</span>
                    <span class="workspace-item-copy">
                      <strong>{{ resolveMockField('workspace', workspace.id, 'name', workspace.name) }}</strong>
                      <small>{{ workspace.isLocal ? t('topbar.localWorkspace') : t('topbar.sharedWorkspace') }}</small>
                    </span>
                  </button>
                  <button
                    type="button"
                    class="workspace-remove"
                    :data-testid="`remove-workspace-${workspace.id}`"
                    :title="t('topbar.removeWorkspace')"
                    :disabled="workbench.workspaces.length === 1"
                    @click="removeWorkspace(workspace.id)"
                  >
                    <X :size="14" />
                  </button>
                </div>
              </div>

              <button
                type="button"
                class="add-workspace-button"
                data-testid="add-workspace-button"
                @click="addWorkspace"
              >
                <Plus :size="16" />
                <span>{{ t('topbar.addWorkspace') }}</span>
              </button>
            </div>
          </div>
        </Transition>
      </div>
    </div>
  </header>
</template>

<style scoped>
.topbar-shell {
  position: relative;
  z-index: 32;
  display: grid;
  grid-template-columns: auto minmax(320px, 560px) auto;
  align-items: center;
  gap: 1.25rem;
  min-height: 64px;
  padding: 0 1.25rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
  backdrop-filter: blur(16px);
}

.brand-block {
  display: flex;
  align-items: center;
  gap: 0.9rem;
}

.brand-logo {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.35rem;
  height: 2.35rem;
  border-radius: var(--radius-l);
  background: linear-gradient(180deg, var(--brand-primary), var(--brand-primary-hover));
  color: var(--text-on-brand);
  box-shadow: var(--shadow-sm);
}

.brand-copy {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.brand-copy strong {
  font-size: 0.96rem;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.02em;
}

.brand-copy small {
  font-size: 0.72rem;
  color: var(--text-tertiary);
  font-weight: 500;
}

.search-trigger {
  display: inline-flex;
  align-items: center;
  justify-self: center;
  gap: 0.75rem;
  width: 100%;
  max-width: 560px;
  min-height: 2.6rem;
  padding: 0 1rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--bg-subtle) 85%, var(--bg-surface));
  color: var(--text-secondary);
  font-size: 0.84rem;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.24);
  transition: all var(--duration-fast) var(--ease-apple);
}

.search-trigger:hover {
  background: var(--bg-surface);
  border-color: color-mix(in srgb, var(--brand-primary) 22%, var(--border-subtle));
  color: var(--text-primary);
  box-shadow: var(--shadow-sm);
}

kbd {
  margin-left: auto;
  padding: 0.15rem 0.4rem;
  border-radius: var(--radius-s);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--text-tertiary);
}

.topbar-menu {
  display: flex;
  align-items: center;
  justify-self: end;
  gap: 0.5rem;
}

.topbar-actions {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding-right: 0.85rem;
  border-right: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
}

.menu-shell,
.profile-menu-shell {
  position: relative;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.2rem;
  height: 2.2rem;
  border-radius: var(--radius-m);
  color: var(--text-secondary);
  transition: all var(--duration-fast) var(--ease-apple);
}

.icon-button:hover,
.icon-button.active {
  background: color-mix(in srgb, var(--bg-subtle) 92%, transparent);
  color: var(--text-primary);
}

.profile-chip {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.3rem 0.8rem 0.3rem 0.3rem;
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--bg-subtle) 88%, var(--bg-surface));
  border: 1px solid transparent;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.2);
  transition: all var(--duration-fast) var(--ease-apple);
}

.profile-chip:hover,
.profile-chip.active {
  background: var(--bg-surface);
  border-color: var(--border-strong);
  box-shadow: var(--shadow-sm);
}

.profile-avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.85rem;
  height: 1.85rem;
  border-radius: var(--radius-full);
  background: var(--brand-primary);
  color: var(--text-on-brand);
  font-size: 0.75rem;
  font-weight: 700;
}

.profile-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.profile-copy strong {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-primary);
}

.profile-copy small {
  font-size: 0.7rem;
  color: var(--text-tertiary);
}

.topbar-dropdown,
.account-menu {
  position: absolute;
  top: calc(100% + 0.55rem);
  right: 0;
  z-index: 50;
  display: flex;
  flex-direction: column;
  padding: 0.5rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-l) + 2px);
  background: var(--bg-popover);
  box-shadow: var(--shadow-lg);
  backdrop-filter: blur(18px);
}

.topbar-dropdown {
  width: 240px;
}

.dropdown-heading {
  padding: 0.5rem 0.75rem;
}

.dropdown-heading strong {
  font-size: 0.8rem;
  font-weight: 700;
  color: var(--text-primary);
}

.dropdown-heading small {
  font-size: 0.7rem;
  color: var(--text-tertiary);
}

.dropdown-option {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.65rem 0.75rem;
  border-radius: var(--radius-m);
  color: var(--text-secondary);
  font-size: 0.85rem;
  transition: all var(--duration-fast) var(--ease-apple);
}

.dropdown-option:hover {
  background: var(--bg-subtle);
  color: var(--text-primary);
}

.dropdown-option.selected {
  background: color-mix(in srgb, var(--brand-primary) 10%, var(--bg-subtle));
  color: var(--brand-primary);
  font-weight: 600;
}

.account-menu {
  width: 320px;
}

.account-summary {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  border-bottom: 1px solid var(--border-subtle);
}

.account-avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 3rem;
  height: 3rem;
  border-radius: var(--radius-full);
  background: var(--brand-primary);
  color: var(--text-on-brand);
  font-size: 1.25rem;
  font-weight: 700;
}

.account-copy {
  display: flex;
  flex-direction: column;
}

.account-copy strong {
  font-size: 1rem;
  font-weight: 700;
  color: var(--text-primary);
}

.account-copy small {
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.account-section {
  padding: 0.5rem;
}

.section-heading {
  padding: 0.5rem 0.5rem 0.75rem;
}

.section-heading strong {
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--text-tertiary);
  text-transform: uppercase;
}

.workspace-list {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.workspace-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem;
  border-radius: var(--radius-m);
  transition: all var(--duration-fast) var(--ease-apple);
}

.workspace-item:hover {
  background: var(--bg-subtle);
}

.workspace-item.active {
  background: color-mix(in srgb, var(--brand-primary) 5%, var(--bg-subtle));
  border: 1px solid color-mix(in srgb, var(--brand-primary) 20%, transparent);
}

.workspace-switch {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex: 1;
}

.workspace-avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  border-radius: var(--radius-full);
  background: var(--bg-sidebar);
  border: 1px solid var(--border-subtle);
  font-size: 0.75rem;
  font-weight: 700;
}

.workspace-item-copy {
  display: flex;
  flex-direction: column;
}

.workspace-item-copy strong {
  font-size: 0.85rem;
  font-weight: 600;
}

.workspace-item-copy small {
  font-size: 0.7rem;
}

.add-workspace-button {
  margin-top: 0.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.75rem;
  border-radius: var(--radius-m);
  border: 1px dashed var(--border-strong);
  color: var(--text-secondary);
  font-size: 0.85rem;
  font-weight: 600;
  transition: all var(--duration-fast) var(--ease-apple);
}

.add-workspace-button:hover {
  border-color: var(--brand-primary);
  color: var(--brand-primary);
  background: color-mix(in srgb, var(--brand-primary) 5%, transparent);
}

.topbar-flyout-enter-active,
.topbar-flyout-leave-active {
  transition: all var(--duration-fast) var(--ease-apple);
}

.topbar-flyout-enter-from,
.topbar-flyout-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}

@media (max-width: 980px) {
  .topbar-shell {
    grid-template-columns: 1fr;
    padding: 1rem;
    gap: 1rem;
  }
}
</style>
