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
  isolation: isolate;
  display: grid;
  grid-template-columns: auto minmax(280px, 540px) auto;
  align-items: center;
  gap: 1.25rem;
  min-height: 64px;
  padding: 0.7rem 1.25rem;
  border-bottom: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-sidebar) 92%, transparent);
  backdrop-filter: blur(18px);
}

.brand-block,
.topbar-menu,
.profile-chip,
.search-trigger {
  min-width: 0;
}

.brand-block {
  display: flex;
  align-items: center;
  gap: 0.85rem;
}

.brand-logo {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.75rem;
  height: 2.75rem;
  border-radius: 0.9rem;
  background:
    radial-gradient(circle at top, color-mix(in srgb, var(--brand-primary) 34%, transparent), transparent 58%),
    linear-gradient(180deg, color-mix(in srgb, var(--bg-subtle) 90%, white), var(--bg-surface));
  color: var(--brand-primary);
  box-shadow: var(--shadow-md);
}

.brand-copy {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.brand-copy small,
.search-trigger,
.profile-copy small,
.account-copy small,
.account-copy span,
.section-heading small,
.workspace-item-copy small,
.dropdown-heading small {
  color: var(--text-secondary);
}

.search-trigger {
  display: inline-flex;
  align-items: center;
  justify-self: center;
  gap: 0.7rem;
  width: 100%;
  min-height: 2.7rem;
  padding: 0 0.95rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--bg-input) 88%, transparent);
  text-align: left;
}

.search-trigger span {
  flex: 1 1 auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

kbd {
  padding: 0.18rem 0.45rem;
  border-radius: var(--radius-s);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
  font-size: 0.78rem;
}

.topbar-menu {
  position: relative;
  z-index: 33;
  display: flex;
  align-items: center;
  justify-self: end;
  gap: 0.6rem;
  min-height: 2.9rem;
  padding: 0.28rem 0.32rem 0.28rem 0.4rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 90%, transparent);
  border-radius: 999px;
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--bg-subtle) 86%, white), color-mix(in srgb, var(--bg-surface) 92%, transparent)),
    color-mix(in srgb, var(--bg-sidebar) 88%, transparent);
  box-shadow: var(--shadow-md);
}

.topbar-actions {
  display: flex;
  align-items: center;
  gap: 0.2rem;
}

.menu-shell,
.profile-menu-shell {
  position: relative;
}

.menu-divider {
  width: 1px;
  height: 1.7rem;
  background: color-mix(in srgb, var(--border-subtle) 82%, transparent);
}

.icon-button,
.profile-chip,
.workspace-item,
.add-workspace-button,
.dropdown-option,
.workspace-remove,
.search-trigger {
  transition:
    transform var(--duration-fast) var(--ease-apple),
    border-color var(--duration-fast) var(--ease-apple),
    background var(--duration-fast) var(--ease-apple),
    color var(--duration-fast) var(--ease-apple),
    box-shadow var(--duration-fast) var(--ease-apple),
    opacity var(--duration-fast) var(--ease-apple);
}

.icon-button,
.profile-chip,
.workspace-item,
.add-workspace-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
}

.icon-button {
  width: 2.2rem;
  height: 2.2rem;
  border-radius: 999px;
  color: var(--text-secondary);
}

.icon-button:hover,
.profile-chip:hover,
.workspace-item:hover,
.add-workspace-button:hover,
.search-trigger:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--brand-primary) 26%, var(--border-subtle));
  background: color-mix(in srgb, var(--bg-surface) 82%, transparent);
  color: var(--text-primary);
  box-shadow: var(--shadow-sm);
}

.icon-button:active,
.profile-chip:active,
.search-trigger:active {
  transform: scale(0.97);
}

.icon-button.active,
.profile-chip.active {
  border-color: color-mix(in srgb, var(--brand-primary) 40%, var(--border-subtle));
  background:
    radial-gradient(circle at top, color-mix(in srgb, var(--brand-primary) 16%, transparent), transparent 62%),
    color-mix(in srgb, var(--bg-surface) 92%, transparent);
  color: var(--text-primary);
  box-shadow: var(--shadow-sm);
}

.profile-chip {
  gap: 0.7rem;
  min-height: 2.4rem;
  padding: 0.12rem 0.72rem 0.12rem 0.28rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-surface) 76%, transparent);
}

.profile-avatar,
.account-avatar,
.workspace-avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  color: var(--text-primary);
  font-weight: 700;
}

.profile-avatar {
  width: 1.95rem;
  height: 1.95rem;
  background: color-mix(in srgb, var(--brand-primary) 24%, transparent);
}

.profile-copy,
.account-copy,
.workspace-item-copy,
.section-heading,
.dropdown-heading {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  min-width: 0;
}

.profile-chevron {
  color: var(--text-secondary);
  transition:
    transform var(--duration-fast) var(--ease-apple),
    color var(--duration-fast) var(--ease-apple);
}

.profile-chevron.open {
  transform: rotate(180deg);
  color: var(--text-primary);
}

.topbar-dropdown,
.account-menu {
  position: absolute;
  top: calc(100% + 0.55rem);
  right: 0;
  z-index: 36;
  transform-origin: top right;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 90%, transparent);
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 12%, transparent), transparent 42%),
    color-mix(in srgb, var(--bg-sidebar) 96%, black);
  box-shadow: var(--shadow-lg);
  backdrop-filter: blur(18px);
}

.topbar-dropdown {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  width: min(15.5rem, 78vw);
  padding: 0.7rem;
  border-radius: 1rem;
}

.locale-dropdown {
  width: min(13rem, 72vw);
}

.dropdown-heading {
  gap: 0.12rem;
  padding: 0.18rem 0.35rem 0.45rem;
}

.dropdown-option {
  display: inline-flex;
  align-items: center;
  gap: 0.7rem;
  width: 100%;
  min-height: 2.8rem;
  padding: 0.55rem 0.65rem;
  border: 1px solid transparent;
  border-radius: 0.85rem;
  background: transparent;
  color: var(--text-primary);
  text-align: left;
}

.dropdown-option:hover {
  transform: translateX(2px);
  border-color: color-mix(in srgb, var(--brand-primary) 24%, var(--border-subtle));
  background: color-mix(in srgb, var(--bg-surface) 84%, transparent);
}

.dropdown-option.selected {
  border-color: color-mix(in srgb, var(--brand-primary) 42%, var(--border-subtle));
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 14%, transparent), transparent 56%),
    color-mix(in srgb, var(--bg-surface) 88%, transparent);
}

.dropdown-option-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.9rem;
  height: 1.9rem;
  border-radius: 0.72rem;
  background: color-mix(in srgb, var(--brand-primary) 14%, transparent);
  color: var(--text-primary);
}

.dropdown-option-copy {
  flex: 1 1 auto;
  min-width: 0;
}

.dropdown-option-check {
  color: var(--brand-primary);
}

.account-menu {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: min(23rem, 88vw);
  padding: 1rem;
  border-radius: 1.1rem;
}

.account-summary {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  padding-bottom: 1rem;
  border-bottom: 1px solid var(--border-subtle);
}

.account-avatar {
  width: 2.8rem;
  height: 2.8rem;
  background: color-mix(in srgb, var(--brand-primary) 20%, transparent);
}

.account-copy {
  gap: 0.18rem;
}

.account-section {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.workspace-list {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.workspace-item {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  width: 100%;
  padding: 0.35rem;
  border-radius: 1rem;
  background: color-mix(in srgb, var(--bg-subtle) 68%, transparent);
}

.workspace-item.active {
  border-color: color-mix(in srgb, var(--brand-primary) 42%, var(--border-subtle));
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 14%, transparent), transparent 44%),
    color-mix(in srgb, var(--bg-surface) 86%, transparent);
}

.workspace-switch {
  display: inline-flex;
  align-items: center;
  gap: 0.75rem;
  flex: 1 1 auto;
  min-width: 0;
  padding: 0.35rem 0.4rem;
  border: 0;
  background: transparent;
  text-align: left;
}

.workspace-avatar {
  width: 2rem;
  height: 2rem;
  background: color-mix(in srgb, var(--brand-primary) 18%, transparent);
}

.workspace-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.9rem;
  height: 1.9rem;
  border: 1px solid transparent;
  border-radius: 999px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-subtle) 55%, transparent);
}

.workspace-remove:not(:disabled):hover {
  color: var(--text-primary);
  border-color: color-mix(in srgb, var(--brand-primary) 24%, var(--border-subtle));
  background: color-mix(in srgb, var(--bg-surface) 82%, transparent);
}

.workspace-remove:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.add-workspace-button {
  justify-content: center;
  gap: 0.55rem;
  min-height: 2.9rem;
  border-radius: 0.95rem;
  border-color: var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 68%, transparent);
  color: var(--text-primary);
}

.topbar-flyout-enter-active,
.topbar-flyout-leave-active {
  transition:
    opacity var(--duration-fast) var(--ease-apple),
    transform var(--duration-fast) var(--ease-apple);
}

.topbar-flyout-enter-from,
.topbar-flyout-leave-to {
  opacity: 0;
  transform: translateY(-0.35rem) scale(0.96);
}

@media (max-width: 980px) {
  .topbar-shell {
    grid-template-columns: 1fr;
  }

  .search-trigger,
  .topbar-menu {
    justify-self: stretch;
  }

  .topbar-menu {
    justify-content: space-between;
    border-radius: 1.1rem;
  }

  .menu-divider {
    display: none;
  }

  .topbar-dropdown,
  .account-menu {
    right: 0;
    left: 0;
    width: auto;
  }
}
</style>
