<script setup lang="ts">
import { RotateCcw } from 'lucide-vue-next'
import { createDefaultShellPreferences } from '@octopus/schema'
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import { UiBadge, UiButton, UiEmptyState, UiField, UiListRow, UiRecordCard, UiSectionHeading, UiSelect, UiSwitch, UiTabs } from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'

const { t } = useI18n()
const shell = useShellStore()
const route = useRoute()

const activeTab = ref<'general' | 'connection' | 'theme' | 'version'>('general')

onMounted(() => {
  const tab = route.query.tab as any
  if (['general', 'connection', 'theme', 'version'].includes(tab)) {
    activeTab.value = tab
  }
})
const theme = ref(shell.preferences.theme)
const locale = ref(shell.preferences.locale)
const fontSize = ref(String(shell.preferences.fontSize))
const fontFamily = ref(shell.preferences.fontFamily)
const fontStyle = ref(shell.preferences.fontStyle)
const leftSidebarCollapsed = ref(shell.preferences.leftSidebarCollapsed)
const rightSidebarCollapsed = ref(shell.preferences.rightSidebarCollapsed)
const tabs = computed(() => [
  { value: 'general', label: t('settings.tabs.general') },
  { value: 'connection', label: t('settings.tabs.connection') },
  { value: 'theme', label: t('settings.tabs.theme') },
  { value: 'version', label: t('settings.tabs.version') },
])

const hostBackendBadges = computed((): Array<{ id: string, label: string, tone: 'info' | 'success' | 'warning' }> => {
  if (!shell.backendConnection) {
    return []
  }

  return [
    {
      id: 'state',
      label: enumLabel('backendConnectionState', shell.backendConnection.state),
      tone: shell.backendConnection.state === 'ready' ? 'success' : 'warning' as const,
    },
    {
      id: 'transport',
      label: enumLabel('backendTransport', shell.backendConnection.transport),
      tone: 'info' as const,
    },
  ]
})

function workspaceLabel(workspaceId: string): string {
  return shell.workspaceConnections.find((item) => item.workspaceId === workspaceId)?.label ?? workspaceId
}

const themeOptions = computed(() => [
  { value: 'system', label: t('settings.preferences.themeOptions.system') },
  { value: 'light', label: t('settings.preferences.themeOptions.light') },
  { value: 'dark', label: t('settings.preferences.themeOptions.dark') },
])

const localeOptions = computed(() => [
  { value: 'zh-CN', label: t('settings.preferences.localeOptions.zh-CN') },
  { value: 'en-US', label: t('settings.preferences.localeOptions.en-US') },
])

const fontStyleOptions = computed(() => [
  { value: 'sans', label: t('settings.preferences.fontStyleOptions.sans') },
  { value: 'serif', label: t('settings.preferences.fontStyleOptions.serif') },
  { value: 'mono', label: t('settings.preferences.fontStyleOptions.mono') },
])

const activeWorkspaceName = computed(() =>
  shell.activeWorkspaceConnection
    ? shell.activeWorkspaceConnection.label
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

const canManageSettings = computed(() => true)
const canManageDesktopBackend = computed(() => canManageSettings.value && !!shell.backendConnection)

// Update local state when store changes
watch(
  () => shell.preferences,
  (preferences) => {
    if (theme.value !== preferences.theme) theme.value = preferences.theme
    if (locale.value !== preferences.locale) locale.value = preferences.locale
    if (fontSize.value !== String(preferences.fontSize)) fontSize.value = String(preferences.fontSize)
    if (fontFamily.value !== preferences.fontFamily) fontFamily.value = preferences.fontFamily
    if (fontStyle.value !== preferences.fontStyle) fontStyle.value = preferences.fontStyle
    if (leftSidebarCollapsed.value !== preferences.leftSidebarCollapsed) leftSidebarCollapsed.value = preferences.leftSidebarCollapsed
    if (rightSidebarCollapsed.value !== preferences.rightSidebarCollapsed) rightSidebarCollapsed.value = preferences.rightSidebarCollapsed
  },
  { deep: true, immediate: true },
)

// Automatically save changes to the store
watch(
  [theme, locale, fontSize, fontFamily, fontStyle, leftSidebarCollapsed, rightSidebarCollapsed],
  async ([nextTheme, nextLocale, nextFontSize, nextFontFamily, nextFontStyle, nextLeftSidebar, nextRightSidebar]) => {
    if (!canManageSettings.value) return

    const patch: any = {}
    if (nextTheme !== shell.preferences.theme) patch.theme = nextTheme
    if (nextLocale !== shell.preferences.locale) patch.locale = nextLocale
    const parsedFontSize = Number.parseInt(nextFontSize, 10)
    if (!Number.isNaN(parsedFontSize) && parsedFontSize !== shell.preferences.fontSize) {
      patch.fontSize = parsedFontSize
    }
    if (nextFontFamily !== shell.preferences.fontFamily) patch.fontFamily = nextFontFamily
    if (nextFontStyle !== shell.preferences.fontStyle) patch.fontStyle = nextFontStyle
    if (nextLeftSidebar !== shell.preferences.leftSidebarCollapsed) patch.leftSidebarCollapsed = nextLeftSidebar
    if (nextRightSidebar !== shell.preferences.rightSidebarCollapsed) patch.rightSidebarCollapsed = nextRightSidebar

    if (Object.keys(patch).length > 0) {
      await shell.updatePreferences(patch)
    }
  }
)

async function refreshBackendStatus() {
  await shell.refreshBackendStatus()
}

async function restartBackend() {
  await shell.restartBackend()
}

async function resetToDefault() {
  const defaults = createDefaultShellPreferences(shell.defaultWorkspaceId, shell.defaultProjectId)
  await shell.updatePreferences(defaults)
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
      <div data-testid="settings-tabs" class="max-w-xl">
        <UiTabs v-model="activeTab" :tabs="tabs" />
      </div>
    </header>

    <main class="px-2">
      <div class="grid gap-8 xl:grid-cols-[minmax(0,1fr)_320px] items-start">
        <div class="flex flex-col gap-10">
        <section v-if="activeTab === 'general'" class="space-y-8">
          <div class="flex items-center justify-between">
            <div class="space-y-1">
              <h3 class="text-xl font-bold text-text-primary">{{ t('settings.general.title') }}</h3>
              <p class="text-[14px] text-text-secondary">{{ t('settings.header.subtitle') }}</p>
            </div>
            <UiButton variant="ghost" size="sm" class="flex items-center gap-2 text-text-secondary hover:text-text-primary transition-colors" @click="resetToDefault">
              <RotateCcw :size="14" />
              <span>{{ t('common.resetToDefault') }}</span>
            </UiButton>
          </div>

          <div class="space-y-6">
            <div class="space-y-3">
              <h4 class="text-[14px] font-bold text-text-primary px-1">{{ t('settings.general.layoutTitle') }}</h4>
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
            </div>

            <div class="space-y-3">
              <h4 class="text-[14px] font-bold text-text-primary px-1">{{ t('settings.general.i18nTitle') }}</h4>
              <div class="max-w-md px-1">
                <UiField :label="t('settings.preferences.locale')">
                  <UiSelect v-model="locale" :options="localeOptions" />
                </UiField>
              </div>
            </div>
          </div>
        </section>

        <section v-if="activeTab === 'connection'" class="space-y-10">
          <div class="space-y-1">
            <h3 class="text-xl font-bold text-text-primary">{{ t('connections.header.title') }}</h3>
            <p class="text-[14px] text-text-secondary">{{ t('connections.header.subtitle') }}</p>
          </div>

          <div class="grid gap-10 xl:grid-cols-2">
            <!-- Product Connections -->
            <section class="space-y-6">
              <div class="space-y-1 border-b border-border-subtle dark:border-white/[0.05] pb-4">
                <h3 class="text-lg font-bold text-text-primary">{{ t('connections.product.title') }}</h3>
                <p class="text-[13px] text-text-secondary">{{ t('connections.product.subtitle') }}</p>
              </div>

              <div data-testid="connections-product-list" class="space-y-4">
                <UiRecordCard
                  v-for="connection in shell.workspaceConnections"
                  :key="connection.workspaceConnectionId"
                  :test-id="`connection-record-${connection.workspaceConnectionId}`"
                  :title="connection.label"
                  :description="t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) })"
                >
                  <template #badges>
                    <UiBadge :label="enumLabel('transportSecurityLevel', connection.transportSecurity)" :tone="connection.transportSecurity === 'loopback' ? 'info' : 'success' as const" />
                    <UiBadge :label="enumLabel('workspaceConnectionStatus', connection.status)" subtle />
                  </template>
                  <template #meta>
                    <span class="truncate text-[12px] text-text-tertiary font-mono">{{ connection.baseUrl ?? t('common.noRemoteBaseUrl') }}</span>
                  </template>
                </UiRecordCard>

                <UiEmptyState v-if="!shell.workspaceConnections.length" :title="t('connections.empty.title')" :description="t('connections.empty.description')" />
              </div>
            </section>

            <!-- Host Connections -->
            <section class="space-y-6">
              <div class="space-y-1 border-b border-border-subtle dark:border-white/[0.05] pb-4">
                <h3 class="text-lg font-bold text-text-primary">{{ t('connections.host.title') }}</h3>
                <p class="text-[13px] text-text-secondary">{{ t('connections.host.subtitle') }}</p>
              </div>

              <div data-testid="connections-host-list" class="space-y-4">
                <UiRecordCard
                  v-if="shell.backendConnection"
                  test-id="host-backend-connection"
                  :title="t('connections.host.backendTitle')"
                  :description="t('connections.host.backendSubtitle')"
                >
                  <template #badges>
                    <UiBadge
                      v-for="badge in hostBackendBadges"
                      :key="badge.id"
                      :label="badge.label"
                      :tone="badge.tone"
                    />
                  </template>
                  <template #meta>
                    <span class="truncate text-[12px] text-text-tertiary font-mono">{{ shell.backendConnection.baseUrl ?? t('common.noBaseUrl') }}</span>
                  </template>
                </UiRecordCard>

                <UiRecordCard
                  v-for="connection in shell.bootstrapConnections"
                  :key="connection.id"
                  :test-id="`connection-record-${connection.id}`"
                  :title="connection.label"
                  :description="t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) })"
                >
                  <template #badges>
                    <UiBadge :label="enumLabel('connectionMode', connection.mode)" :tone="connection.mode === 'local' ? 'info' : 'success' as const" />
                    <UiBadge :label="enumLabel('connectionState', connection.state)" subtle />
                  </template>
                  <template #meta>
                    <span class="truncate text-[12px] text-text-tertiary font-mono">{{ connection.baseUrl ?? t('common.noBaseUrl') }}</span>
                  </template>
                </UiRecordCard>

                <UiEmptyState
                  v-if="!shell.backendConnection && !shell.bootstrapConnections.length"
                  :title="t('connections.host.emptyTitle')"
                  :description="t('connections.host.emptyDescription')"
                />
              </div>
            </section>
          </div>
        </section>

        <!-- Theme Tab -->
        <section v-else-if="activeTab === 'theme'" class="space-y-8">
          <div class="flex items-center justify-between">
            <div class="space-y-1">
              <h3 class="text-xl font-bold text-text-primary">{{ t('settings.preferences.title') }}</h3>
              <p class="text-[14px] text-text-secondary">{{ t('settings.header.subtitle') }}</p>
            </div>
            <UiButton variant="ghost" size="sm" class="flex items-center gap-2 text-text-secondary hover:text-text-primary transition-colors" @click="resetToDefault">
              <RotateCcw :size="14" />
              <span>{{ t('common.resetToDefault') }}</span>
            </UiButton>
          </div>

          <div class="space-y-6">
            <div class="max-w-md">
              <UiField :label="t('settings.preferences.theme')">
                <UiSelect v-model="theme" :options="themeOptions" />
              </UiField>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-2xl">
              <UiField :label="t('settings.preferences.fontFamily')">
                <UiSelect v-model="fontFamily" :options="[
                  { value: 'Inter, sans-serif', label: 'Inter' },
                  { value: 'system-ui, sans-serif', label: 'System UI' },
                  { value: 'monospace', label: 'Monospace' }
                ]" />
              </UiField>

              <UiField :label="t('settings.preferences.fontSize')">
                <UiSelect v-model="fontSize" :options="[
                  { value: '12', label: '12px' },
                  { value: '13', label: '13px' },
                  { value: '14', label: '14px' },
                  { value: '15', label: '15px' },
                  { value: '16', label: '16px' }
                ]" />
              </UiField>

              <UiField :label="t('settings.preferences.fontStyle')">
                <UiSelect v-model="fontStyle" :options="fontStyleOptions" />
              </UiField>
            </div>
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

        <aside class="space-y-4">
          <div class="rounded-lg border border-border-subtle/30 bg-subtle/20 p-5">
            <h4 class="text-sm font-bold text-text-primary">{{ t('settings.version.fields.workspace') }}</h4>
            <p class="mt-2 text-sm text-text-secondary">{{ activeWorkspaceName }}</p>
          </div>
          <div class="rounded-lg border border-border-subtle/30 bg-subtle/20 p-5">
            <h4 class="text-sm font-bold text-text-primary">{{ t('settings.version.fields.backendState') }}</h4>
            <p class="mt-2 text-sm text-text-secondary">
              {{ shell.backendConnection ? enumLabel('backendConnectionState', shell.backendConnection.state) : t('common.na') }}
            </p>
          </div>
        </aside>
      </div>
    </main>
  </div>
</template>
