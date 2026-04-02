<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Monitor, Palette, PanelLeft, PanelRight } from 'lucide-vue-next'

import { UiBadge, UiButton, UiField, UiSectionHeading, UiSelect, UiSurface } from '@octopus/ui'

import { enumLabel, resolveMockField, resolveMockList } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

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
</script>

<template>
  <section class="settings-page section-stack">
    <UiSectionHeading
      :eyebrow="t('settings.header.eyebrow')"
      :title="t('settings.header.title')"
      :subtitle="t('settings.header.subtitle')"
    />

    <div class="settings-overview-grid">
      <UiSurface :title="t('settings.preferences.title')" :subtitle="t('settings.preferences.subtitle')">
        <div class="preference-grid">
          <UiField :label="t('settings.preferences.theme')">
            <UiSelect v-model="theme" :options="themeOptions" />
          </UiField>
          <UiField :label="t('settings.preferences.locale')">
            <UiSelect v-model="locale" :options="localeOptions" />
          </UiField>
        </div>

        <div class="preference-switch-grid">
          <button type="button" class="preference-toggle-card" :class="{ active: leftSidebarCollapsed }" @click="leftSidebarCollapsed = !leftSidebarCollapsed">
            <span class="preference-toggle-icon"><PanelLeft :size="16" /></span>
            <span>
              <strong>{{ t('settings.preferences.leftSidebarCollapsed') }}</strong>
              <small>{{ leftSidebarCollapsed ? t('userCenter.common.active') : t('userCenter.common.disabled') }}</small>
            </span>
          </button>
          <button type="button" class="preference-toggle-card" :class="{ active: rightSidebarCollapsed }" @click="rightSidebarCollapsed = !rightSidebarCollapsed">
            <span class="preference-toggle-icon"><PanelRight :size="16" /></span>
            <span>
              <strong>{{ t('settings.preferences.rightSidebarCollapsed') }}</strong>
              <small>{{ rightSidebarCollapsed ? t('userCenter.common.active') : t('userCenter.common.disabled') }}</small>
            </span>
          </button>
        </div>

        <div class="settings-action-row">
          <UiButton @click="savePreferences">{{ t('common.savePreferences') }}</UiButton>
        </div>
      </UiSurface>

      <UiSurface :title="t('settings.host.title')" :subtitle="t('settings.host.subtitle')">
        <div class="host-header">
          <div class="host-icon">
            <Monitor :size="18" />
          </div>
          <div class="host-copy">
            <strong>{{ shell.hostState.shell }}</strong>
            <p>{{ shell.hostState.appVersion }}</p>
          </div>
        </div>
        <div class="host-meta-row">
          <UiBadge :label="enumLabel('hostPlatform', shell.hostState.platform)" :tone="shell.hostState.platform === 'tauri' ? 'success' : 'info'" />
          <UiBadge :label="enumLabel('hostMode', shell.hostState.mode)" subtle />
          <UiBadge :label="workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name) : t('common.na')" subtle />
        </div>
        <p class="host-copy-text">
          {{ t('common.currentShell', { shell: shell.hostState.shell }) }}
          {{ t('common.activeWorkspace', { workspace: workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name) : t('common.na') }) }}
        </p>
      </UiSurface>
    </div>

    <div class="settings-section-grid surface-grid two">
      <UiSurface
        v-for="section in workbench.settingsSections"
        :key="section.id"
        :title="resolveMockField('settingsSection', section.id, 'title', section.title)"
        :subtitle="resolveMockField('settingsSection', section.id, 'description', section.description)"
      >
        <div class="settings-section-header">
          <UiBadge :label="enumLabel('viewStatus', section.status)" :tone="section.status === 'attention' ? 'warning' : 'success'" />
          <span class="settings-section-icon"><Palette :size="14" /></span>
        </div>
        <ul class="settings-section-list">
          <li v-for="(item, index) in resolveMockList('settingsSection', section.id, 'items', section.items)" :key="`${section.id}-${index}`">{{ item }}</li>
        </ul>
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.settings-page,
.host-copy {
  display: flex;
  flex-direction: column;
}

.settings-overview-grid,
.preference-grid,
.preference-switch-grid {
  display: grid;
  gap: 1rem;
}

.settings-overview-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.preference-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.preference-switch-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin-top: 1rem;
}

.preference-toggle-card,
.host-header,
.host-meta-row,
.settings-section-header,
.settings-action-row {
  display: flex;
  align-items: center;
}

.preference-toggle-card,
.host-header {
  gap: 0.85rem;
}

.preference-toggle-card,
.settings-section-header {
  justify-content: space-between;
}

.preference-toggle-card {
  width: 100%;
  padding: 0.95rem 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-lg) + 2px);
  background: color-mix(in srgb, var(--bg-subtle) 68%, transparent);
  text-align: left;
  transition: border-color var(--duration-fast) var(--ease-apple), transform var(--duration-fast) var(--ease-apple), box-shadow var(--duration-fast) var(--ease-apple);
}

.preference-toggle-card:hover,
.preference-toggle-card.active {
  border-color: color-mix(in srgb, var(--brand-primary) 24%, var(--border-strong));
  transform: translateY(-1px);
  box-shadow: var(--shadow-sm);
}

.preference-toggle-card span:last-child {
  display: flex;
  flex-direction: column;
  gap: 0.18rem;
}

.preference-toggle-card small,
.host-copy p,
.host-copy-text,
.settings-section-list li {
  color: var(--text-secondary);
}

.preference-toggle-icon,
.host-icon,
.settings-section-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
  color: var(--brand-primary);
}

.preference-toggle-icon,
.settings-section-icon {
  width: 2.3rem;
  height: 2.3rem;
}

.host-icon {
  width: 2.7rem;
  height: 2.7rem;
}

.host-copy {
  gap: 0.18rem;
}

.host-meta-row {
  gap: 0.55rem;
  flex-wrap: wrap;
  margin: 1rem 0 0.75rem;
}

.host-copy-text {
  line-height: 1.65;
}

.settings-action-row {
  justify-content: flex-end;
  margin-top: 1rem;
}

.settings-section-header {
  margin-bottom: 0.8rem;
}

.settings-section-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding-left: 1rem;
}

.settings-section-list li {
  line-height: 1.6;
}

@media (max-width: 1040px) {
  .settings-overview-grid,
  .preference-grid,
  .preference-switch-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
