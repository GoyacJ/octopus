<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiButton, UiField, UiListRow, UiSectionHeading, UiSelect, UiSwitch, UiTabs } from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const activeTab = ref<'general' | 'theme' | 'i18n' | 'version'>('general')
const theme = ref(shell.preferences.theme)
const locale = ref(shell.preferences.locale)
const leftSidebarCollapsed = ref(shell.preferences.leftSidebarCollapsed)
const rightSidebarCollapsed = ref(shell.preferences.rightSidebarCollapsed)

const themeOptions = computed(() => [
  { value: 'system', label: t('settings.preferences.themeOptions.system') },
  { value: 'light', label: t('settings.preferences.themeOptions.light') },
  { value: 'dark', label: t('settings.preferences.themeOptions.dark') },
])

const localeOptions = computed(() => [
  { value: 'zh-CN', label: t('settings.preferences.localeOptions.zh-CN') },
  { value: 'en-US', label: t('settings.preferences.localeOptions.en-US') },
])

const fallbackSettingsPage = {
  tabs: [
    { value: 'general', label: 'settings.tabs.general' },
    { value: 'theme', label: 'settings.tabs.theme' },
    { value: 'i18n', label: 'settings.tabs.i18n' },
    { value: 'version', label: 'settings.tabs.version' },
  ],
  sections: [],
} as const

const settingsPage = computed(() => workbench.settingsPage ?? fallbackSettingsPage)

const tabs = computed(() =>
  settingsPage.value.tabs.map((tab) => ({
    value: tab.value,
    label: workbench.settingsTabDisplayLabel(tab),
  })),
)

const activeSections = computed(() =>
  settingsPage.value.sections.filter((section) => section.tab === activeTab.value),
)

const activeWorkspaceName = computed(() =>
  workbench.activeWorkspace
    ? workbench.workspaceDisplayName(workbench.activeWorkspace.id)
    : t('common.na'),
)

const versionRows = computed(() => [
  { id: 'shell', label: t('settings.version.fields.shell'), value: shell.hostState.shell },
  { id: 'appVersion', label: t('settings.version.fields.appVersion'), value: shell.hostState.appVersion },
  { id: 'workspace', label: t('settings.version.fields.workspace'), value: activeWorkspaceName.value },
  {
    id: 'backendState',
    label: t('settings.version.fields.backendState'),
    value: shell.backendConnection
      ? enumLabel('backendConnectionState', shell.backendConnection.state)
      : t('common.na'),
  },
  {
    id: 'backendTransport',
    label: t('settings.version.fields.backendTransport'),
    value: shell.backendConnection
      ? enumLabel('backendTransport', shell.backendConnection.transport)
      : t('common.na'),
  },
  {
    id: 'cargoWorkspace',
    label: t('settings.version.fields.cargoWorkspace'),
    value: shell.hostState.cargoWorkspace
      ? t('settings.version.values.enabled')
      : t('settings.version.values.disabled'),
  },
])

const canManageDesktopBackend = computed(() => !!shell.backendConnection)

watch(
  () => shell.preferences,
  (preferences) => {
    theme.value = preferences.theme
    locale.value = preferences.locale
    leftSidebarCollapsed.value = preferences.leftSidebarCollapsed
    rightSidebarCollapsed.value = preferences.rightSidebarCollapsed
  },
  { deep: true, immediate: true },
)

async function savePreferences() {
  await shell.updatePreferences({
    theme: theme.value,
    locale: locale.value,
    leftSidebarCollapsed: leftSidebarCollapsed.value,
    rightSidebarCollapsed: rightSidebarCollapsed.value,
  })
}

async function refreshBackendStatus() {
  await shell.refreshBackendStatus()
}

async function restartBackend() {
  await shell.restartBackend()
}
</script>

<template>
  <div class="w-full flex flex-col gap-8 pb-20">
    <header class="px-2 space-y-4">
      <UiSectionHeading
        :eyebrow="t('settings.header.eyebrow')"
        :title="t('settings.header.title')"
        :subtitle="t('settings.header.subtitle')"
      />
      <div data-testid="settings-tabs">
        <UiTabs v-model="activeTab" :tabs="tabs" />
      </div>
    </header>

    <main class="grid gap-12 lg:grid-cols-[1fr_360px] items-start px-2">
      <!-- Main Settings Form Area -->
      <div class="flex flex-col gap-10">
        
        <!-- General Tab -->
        <section v-if="activeTab === 'general'" class="space-y-8">
          <div class="space-y-1">
            <h3 class="text-xl font-bold text-text-primary">{{ t('settings.general.layoutTitle') }}</h3>
            <p class="text-[14px] text-text-secondary">{{ t('settings.header.subtitle') }}</p>
          </div>

          <div class="space-y-2 bg-subtle/10 rounded-lg border border-border-subtle/30 dark:border-white/[0.08] p-2">
            <div data-testid="settings-layout-row-leftSidebarCollapsed">
              <UiListRow
                :title="t('settings.preferences.leftSidebarCollapsed')"
                :subtitle="t('settings.general.leftSidebarHint')"
              >
                <template #actions>
                  <UiSwitch v-model="leftSidebarCollapsed" />
                </template>
              </UiListRow>
            </div>

            <div data-testid="settings-layout-row-rightSidebarCollapsed">
              <UiListRow
                :title="t('settings.preferences.rightSidebarCollapsed')"
                :subtitle="t('settings.general.rightSidebarHint')"
              >
                <template #actions>
                  <UiSwitch v-model="rightSidebarCollapsed" />
                </template>
              </UiListRow>
            </div>
          </div>

          <div class="pt-6 border-t border-border-subtle/30 dark:border-white/[0.08] flex justify-end">
            <UiButton variant="primary" @click="savePreferences">{{ t('common.savePreferences') }}</UiButton>
          </div>
        </section>

        <!-- Theme Tab -->
        <section v-else-if="activeTab === 'theme'" class="space-y-8">
          <div class="space-y-1">
            <h3 class="text-xl font-bold text-text-primary">{{ t('settings.preferences.title') }}</h3>
            <p class="text-[14px] text-text-secondary">{{ t('settings.header.subtitle') }}</p>
          </div>

          <div class="max-w-md">
            <UiField :label="t('settings.preferences.theme')">
              <UiSelect v-model="theme" :options="themeOptions" />
            </UiField>
          </div>

          <div class="pt-6 border-t border-border-subtle/30 dark:border-white/[0.08] flex justify-end">
            <UiButton variant="primary" @click="savePreferences">{{ t('common.savePreferences') }}</UiButton>
          </div>
        </section>

        <!-- i18n Tab -->
        <section v-else-if="activeTab === 'i18n'" class="space-y-8">
          <div class="space-y-1">
            <h3 class="text-xl font-bold text-text-primary">{{ t('settings.i18n.title') }}</h3>
            <p class="text-[14px] text-text-secondary">{{ t('settings.header.subtitle') }}</p>
          </div>

          <div class="max-w-md">
            <UiField :label="t('settings.preferences.locale')">
              <UiSelect v-model="locale" :options="localeOptions" />
            </UiField>
          </div>

          <div class="pt-6 border-t border-border-subtle/30 dark:border-white/[0.08] flex justify-end">
            <UiButton variant="primary" @click="savePreferences">{{ t('common.savePreferences') }}</UiButton>
          </div>
        </section>

        <!-- Version Tab -->
        <section v-else class="space-y-8">
          <div class="space-y-6">
            <h3 class="text-xl font-bold text-text-primary">{{ t('settings.version.runtimeTitle') }}</h3>
            <div class="flex flex-wrap gap-2.5 mb-8">
              <UiBadge :label="enumLabel('hostPlatform', shell.hostState.platform)" :tone="shell.hostState.platform === 'tauri' ? 'success' : 'info'" />
              <UiBadge :label="enumLabel('hostMode', shell.hostState.mode)" subtle />
            </div>

            <div class="bg-subtle/10 border border-border-subtle/30 dark:border-white/[0.08] rounded-md overflow-hidden">
              <div
                v-for="(row, i) in versionRows"
                :key="row.id"
                :data-testid="`settings-version-row-${row.id}`"
                class="flex items-center justify-between px-6 py-4"
                :class="i !== versionRows.length - 1 ? 'border-b border-border-subtle/20 dark:border-white/[0.05]' : ''"
              >
                <span class="text-[14px] text-text-secondary font-medium">{{ row.label }}</span>
                <span class="text-[14px] font-bold text-text-primary tracking-tight font-mono">{{ row.value }}</span>
              </div>
            </div>

            <div
              v-if="canManageDesktopBackend"
              data-testid="settings-backend-actions"
              class="flex flex-wrap gap-3 pt-2"
            >
              <UiButton
                data-testid="settings-backend-refresh"
                variant="ghost"
                :disabled="shell.syncingBackend"
                @click="refreshBackendStatus"
              >
                {{ t('settings.version.actions.refreshBackend') }}
              </UiButton>
              <UiButton
                data-testid="settings-backend-restart"
                variant="primary"
                :disabled="shell.restartingBackend"
                @click="restartBackend"
              >
                {{ t('settings.version.actions.restartBackend') }}
              </UiButton>
            </div>
          </div>
        </section>

      </div>

      <!-- Right Sidebar (Expanded for full width) -->
      <aside class="flex flex-col gap-6">
        <div
          v-for="section in activeSections"
          :key="section.id"
          class="bg-subtle/20 rounded-lg border border-border-subtle/30 dark:border-white/[0.08] p-6 space-y-4"
        >
          <strong class="block text-[14px] font-bold text-text-primary">{{ workbench.settingsSectionDisplayTitle(section) }}</strong>
          <p v-if="section.description" class="text-[13px] text-text-secondary leading-relaxed">{{ workbench.settingsSectionDisplayDescription(section) }}</p>
          <ul class="list-disc pl-5 space-y-2 text-[13px] text-text-secondary mt-2">
            <li v-for="item in workbench.settingsSectionDisplayItems(section)" :key="item">{{ item }}</li>
          </ul>
        </div>
      </aside>
    </main>
  </div>
</template>
