<script setup lang="ts">
import { UiPageHeader, UiPageShell, UiTabs } from '@octopus/ui'

import SettingsConnectionPanel from './SettingsConnectionPanel.vue'
import SettingsGeneralPanel from './SettingsGeneralPanel.vue'
import SettingsThemePanel from './SettingsThemePanel.vue'
import SettingsVersionPanel from './SettingsVersionPanel.vue'
import { useAppSettingsView } from './useAppSettingsView'

const {
  t,
  appUpdate,
  shell,
  activeTab,
  theme,
  locale,
  fontSize,
  leftSidebarCollapsed,
  rightSidebarCollapsed,
  tabs,
  hostBackendBadges,
  workspaceLabel,
  themeOptions,
  localeOptions,
  fontSizeOptions,
  updateChannelOptions,
  updateChannel,
  versionStatus,
  latestRelease,
  updateStatusTone,
  updateStatusLabel,
  updateStatusDescription,
  primaryUpdateActionLabel,
  primaryUpdateActionDisabled,
  hasReleaseNotesLink,
  resetToDefault,
  formatRelativeTimestamp,
  formatReleaseDate,
  handlePrimaryUpdateAction,
  openReleaseNotes,
} = useAppSettingsView()

function updateLocale(value: string) {
  locale.value = value as typeof locale.value
}

function updateTheme(value: string) {
  theme.value = value as typeof theme.value
}

function updateUpdateChannel(value: string) {
  updateChannel.value = value as typeof updateChannel.value
}
</script>

<template>
  <UiPageShell width="standard" test-id="settings-view">
    <UiPageHeader
      :eyebrow="t('settings.header.eyebrow')"
      :title="t('settings.header.title')"
      :description="t('settings.header.subtitle')"
    >
      <template #actions>
        <div data-testid="settings-tabs" class="w-full max-w-xl md:w-auto">
          <UiTabs v-model="activeTab" :tabs="tabs" variant="segmented" />
        </div>
      </template>
    </UiPageHeader>

    <main>
      <div class="flex flex-col gap-10">
        <SettingsGeneralPanel
          v-if="activeTab === 'general'"
          :locale="locale"
          :locale-options="localeOptions"
          :left-sidebar-collapsed="leftSidebarCollapsed"
          :right-sidebar-collapsed="rightSidebarCollapsed"
          @reset="resetToDefault"
          @update:locale="updateLocale"
          @update:left-sidebar-collapsed="leftSidebarCollapsed = $event"
          @update:right-sidebar-collapsed="rightSidebarCollapsed = $event"
        />

        <SettingsConnectionPanel
          v-else-if="activeTab === 'connection'"
          :workspace-connections="shell.workspaceConnections"
          :backend-connection="shell.backendConnection"
          :bootstrap-connections="shell.bootstrapConnections"
          :host-backend-badges="hostBackendBadges"
          :workspace-label="workspaceLabel"
        />

        <SettingsThemePanel
          v-else-if="activeTab === 'theme'"
          :theme="theme"
          :font-size="fontSize"
          :theme-options="themeOptions"
          :font-size-options="fontSizeOptions"
          @reset="resetToDefault"
          @update:theme="updateTheme"
          @update:font-size="fontSize = $event"
        />

        <SettingsVersionPanel
          v-else
          :app-update="appUpdate"
          :version-status="versionStatus"
          :latest-release="latestRelease"
          :update-channel="updateChannel"
          :update-channel-options="updateChannelOptions"
          :update-status-tone="updateStatusTone"
          :update-status-label="updateStatusLabel"
          :update-status-description="updateStatusDescription"
          :primary-update-action-label="primaryUpdateActionLabel"
          :primary-update-action-disabled="primaryUpdateActionDisabled"
          :has-release-notes-link="hasReleaseNotesLink"
          :format-relative-timestamp="formatRelativeTimestamp"
          :format-release-date="formatReleaseDate"
          @update:update-channel="updateUpdateChannel"
          @primary="handlePrimaryUpdateAction"
          @check-updates="appUpdate.checkForUpdates()"
          @open-release-notes="openReleaseNotes"
        />
      </div>
    </main>
  </UiPageShell>
</template>
