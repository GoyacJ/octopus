<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiField, UiSectionHeading, UiSurface } from '@octopus/ui'

import { enumLabel, resolveMockField, resolveMockList } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const theme = ref(shell.preferences.theme)
const locale = ref(shell.preferences.locale)
const compactSidebar = ref(shell.preferences.compactSidebar)

watch(
  () => shell.preferences,
  (preferences) => {
    theme.value = preferences.theme
    locale.value = preferences.locale
    compactSidebar.value = preferences.compactSidebar
  },
  { deep: true, immediate: true },
)

async function savePreferences() {
  await shell.updatePreferences({
    theme: theme.value,
    locale: locale.value,
    compactSidebar: compactSidebar.value,
  })
}
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('settings.header.eyebrow')"
      :title="t('settings.header.title')"
      :subtitle="t('settings.header.subtitle')"
    />

    <UiSurface :title="t('settings.preferences.title')" :subtitle="t('settings.preferences.subtitle')">
      <div class="field-grid">
        <UiField :label="t('settings.preferences.theme')">
          <select v-model="theme">
            <option value="system">{{ t('settings.preferences.themeOptions.system') }}</option>
            <option value="light">{{ t('settings.preferences.themeOptions.light') }}</option>
            <option value="dark">{{ t('settings.preferences.themeOptions.dark') }}</option>
          </select>
        </UiField>
        <UiField :label="t('settings.preferences.locale')">
          <select v-model="locale">
            <option value="zh-CN">{{ t('settings.preferences.localeOptions.zh-CN') }}</option>
            <option value="en-US">{{ t('settings.preferences.localeOptions.en-US') }}</option>
          </select>
        </UiField>
      </div>
      <label class="checkbox-row">
        <input v-model="compactSidebar" type="checkbox" />
        <span>{{ t('settings.preferences.compactSidebar') }}</span>
      </label>
      <div class="action-row">
        <button type="button" class="primary-button" @click="savePreferences">{{ t('common.savePreferences') }}</button>
      </div>
    </UiSurface>

    <UiSurface :title="t('settings.host.title')" :subtitle="t('settings.host.subtitle')">
      <div class="meta-row">
        <UiBadge :label="enumLabel('hostPlatform', shell.hostState.platform)" :tone="shell.hostState.platform === 'tauri' ? 'success' : 'info'" />
        <UiBadge :label="enumLabel('hostMode', shell.hostState.mode)" subtle />
        <UiBadge :label="shell.hostState.appVersion" subtle />
      </div>
      <p class="settings-copy">
        {{ t('common.currentShell', { shell: shell.hostState.shell }) }}
        {{ t('common.activeWorkspace', { workspace: workbench.activeWorkspace ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name) : t('common.na') }) }}
      </p>
    </UiSurface>

    <div class="surface-grid two">
      <UiSurface
        v-for="section in workbench.settingsSections"
        :key="section.id"
        :title="resolveMockField('settingsSection', section.id, 'title', section.title)"
        :subtitle="resolveMockField('settingsSection', section.id, 'description', section.description)"
      >
        <div class="meta-row">
          <UiBadge :label="enumLabel('viewStatus', section.status)" :tone="section.status === 'attention' ? 'warning' : 'success'" />
        </div>
        <ul>
          <li v-for="(item, index) in resolveMockList('settingsSection', section.id, 'items', section.items)" :key="`${section.id}-${index}`">{{ item }}</li>
        </ul>
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.checkbox-row,
.settings-copy,
li {
  color: var(--text-secondary);
  line-height: 1.6;
}

.checkbox-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.75rem;
}

ul {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding-left: 1rem;
}
</style>
