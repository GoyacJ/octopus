<script setup lang="ts">
import { RotateCcw } from 'lucide-vue-next'
import { createDefaultShellPreferences } from '@octopus/schema'
import type { RuntimeConfigSource } from '@octopus/schema'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiCodeEditor,
  UiEmptyState,
  UiField,
  UiListRow,
  UiPageHeader,
  UiPageShell,
  UiRecordCard,
  UiSelect,
  UiStatusCallout,
  UiSwitch,
  UiTabs,
} from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'

type SettingsTab = 'general' | 'connection' | 'theme' | 'runtime' | 'version'

const { t } = useI18n()
const shell = useShellStore()
const runtime = useRuntimeStore()
const route = useRoute()

function resolveSettingsTab(value: unknown): SettingsTab {
  return ['general', 'connection', 'theme', 'runtime', 'version'].includes(String(value))
    ? value as SettingsTab
    : 'general'
}

function resolveValidationTone(validation = runtime.configValidation.workspace): 'default' | 'success' | 'warning' | 'error' | 'info' {
  if (!validation) {
    return 'default'
  }

  return validation.valid ? 'success' : 'error'
}

function resolveValidationLabel(validation = runtime.configValidation.workspace): string {
  if (!validation) {
    return t('settings.runtime.validation.idle')
  }

  return validation.valid
    ? t('settings.runtime.validation.valid')
    : t('settings.runtime.validation.invalid')
}

function resolveSourceStatusLabel(source?: RuntimeConfigSource): string {
  if (!source?.exists) {
    return t('settings.runtime.sourceStatuses.missing')
  }

  return source.loaded
    ? t('settings.runtime.sourceStatuses.loaded')
    : t('settings.runtime.sourceStatuses.detected')
}

function resolveSourceStatusTone(source?: RuntimeConfigSource): 'default' | 'success' | 'warning' | 'error' | 'info' {
  if (!source?.exists) {
    return 'warning'
  }

  return source.loaded ? 'success' : 'info'
}

const activeTab = ref<SettingsTab>(resolveSettingsTab(route.query.tab))
const theme = ref(shell.preferences.theme)
const locale = ref(shell.preferences.locale)
const fontSize = ref(String(shell.preferences.fontSize))
const leftSidebarCollapsed = ref(shell.preferences.leftSidebarCollapsed)
const rightSidebarCollapsed = ref(shell.preferences.rightSidebarCollapsed)
const tabs = computed(() => [
  { value: 'general', label: t('settings.tabs.general') },
  { value: 'connection', label: t('settings.tabs.connection') },
  { value: 'theme', label: t('settings.tabs.theme') },
  { value: 'runtime', label: t('settings.tabs.runtime') },
  { value: 'version', label: t('settings.tabs.version') },
])

const workspaceRuntimeSource = computed<RuntimeConfigSource | undefined>(() =>
  runtime.config?.sources.filter(item => item.scope === 'workspace').at(-1),
)
const workspaceRuntimeDraft = computed(() => runtime.configDrafts.workspace)

const runtimeEffectivePreview = computed(() =>
  JSON.stringify(runtime.config?.effectiveConfig ?? {}, null, 2),
)

const runtimeSecretStatuses = computed(() => runtime.config?.secretReferences ?? [])

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

const activeWorkspaceName = computed(() =>
  shell.activeWorkspaceConnection
    ? shell.activeWorkspaceConnection.label
    : t('common.na'),
)

const versionRows = computed(() => [
  { id: 'appVersion', label: t('settings.version.fields.appVersion'), value: shell.hostState.appVersion },
  { id: 'shell', label: t('settings.version.fields.shell'), value: shell.hostState.shell },
  { id: 'workspace', label: t('settings.version.fields.workspace'), value: activeWorkspaceName.value },
  {
    id: 'cargoWorkspace',
    label: t('settings.version.fields.cargoWorkspace'),
    value: shell.hostState.cargoWorkspace
      ? t('settings.version.values.enabled')
      : t('settings.version.values.disabled'),
  },
])

const canManageSettings = computed(() => true)

watch(
  () => route.query.tab,
  (tab) => {
    activeTab.value = resolveSettingsTab(tab)
  },
)

// Update local state when store changes
watch(
  () => shell.preferences,
  (preferences) => {
    if (theme.value !== preferences.theme) theme.value = preferences.theme
    if (locale.value !== preferences.locale) locale.value = preferences.locale
    if (fontSize.value !== String(preferences.fontSize)) fontSize.value = String(preferences.fontSize)
    if (leftSidebarCollapsed.value !== preferences.leftSidebarCollapsed) leftSidebarCollapsed.value = preferences.leftSidebarCollapsed
    if (rightSidebarCollapsed.value !== preferences.rightSidebarCollapsed) rightSidebarCollapsed.value = preferences.rightSidebarCollapsed
  },
  { deep: true, immediate: true },
)

// Automatically save changes to the store
watch(
  [theme, locale, fontSize, leftSidebarCollapsed, rightSidebarCollapsed],
  async ([nextTheme, nextLocale, nextFontSize, nextLeftSidebar, nextRightSidebar]) => {
    if (!canManageSettings.value) return

    const patch: any = {}
    if (nextTheme !== shell.preferences.theme) patch.theme = nextTheme
    if (nextLocale !== shell.preferences.locale) patch.locale = nextLocale
    const parsedFontSize = Number.parseInt(nextFontSize, 10)
    if (!Number.isNaN(parsedFontSize) && parsedFontSize !== shell.preferences.fontSize) {
      patch.fontSize = parsedFontSize
    }
    if (nextLeftSidebar !== shell.preferences.leftSidebarCollapsed) patch.leftSidebarCollapsed = nextLeftSidebar
    if (nextRightSidebar !== shell.preferences.rightSidebarCollapsed) patch.rightSidebarCollapsed = nextRightSidebar

    if (Object.keys(patch).length > 0) {
      await shell.updatePreferences(patch)
    }
  },
)

watch(
  () => ({
    tab: activeTab.value,
    workspaceConnectionId: shell.activeWorkspaceConnection?.workspaceConnectionId ?? '',
  }),
  ({ tab, workspaceConnectionId }, previous) => {
    if (tab !== 'runtime' || !workspaceConnectionId) {
      return
    }

    if (!runtime.config || previous?.workspaceConnectionId !== workspaceConnectionId) {
      void runtime.loadConfig(previous?.workspaceConnectionId !== workspaceConnectionId)
    }
  },
  { immediate: true },
)

async function resetToDefault() {
  const defaults = createDefaultShellPreferences(shell.defaultWorkspaceId, shell.defaultProjectId)
  await shell.updatePreferences(defaults)
}

async function validateWorkspaceRuntime() {
  await runtime.validateConfig('workspace')
}

async function saveWorkspaceRuntime() {
  await runtime.saveConfig('workspace')
}

async function reloadRuntimeConfig() {
  await runtime.loadConfig(true)
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
              <div class="space-y-2 rounded-[var(--radius-l)] border border-border bg-surface p-2">
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

        <section v-else-if="activeTab === 'connection'" class="space-y-10">
          <div class="space-y-1">
            <h3 class="text-xl font-bold text-text-primary">{{ t('connections.header.title') }}</h3>
            <p class="text-[14px] text-text-secondary">{{ t('connections.header.subtitle') }}</p>
          </div>

          <div class="grid gap-10 xl:grid-cols-2">
            <!-- Product Connections -->
            <section class="space-y-6">
              <div class="space-y-1 border-b border-border-subtle pb-4">
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
              <div class="space-y-1 border-b border-border-subtle pb-4">
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
            <div class="grid max-w-2xl gap-6 md:grid-cols-2">
              <UiField :label="t('settings.preferences.theme')">
                <UiSelect v-model="theme" :options="themeOptions" />
              </UiField>

              <UiField :label="t('settings.preferences.fontSize')">
                <UiSelect v-model="fontSize" :options="[
                  { value: '13', label: '13px' },
                  { value: '14', label: '14px' },
                  { value: '15', label: '15px' },
                  { value: '16', label: '16px' }
                ]" />
              </UiField>
            </div>
          </div>
        </section>

        <section v-else-if="activeTab === 'runtime'" class="space-y-8">
          <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div class="space-y-1">
              <h3 class="text-xl font-bold text-text-primary">{{ t('settings.runtime.title') }}</h3>
              <p class="max-w-3xl text-[14px] text-text-secondary">
                {{ t('settings.runtime.subtitle') }}
              </p>
            </div>
            <UiButton
              variant="ghost"
              size="sm"
              class="text-text-secondary hover:text-text-primary transition-colors"
              @click="reloadRuntimeConfig"
            >
              {{ t('settings.runtime.actions.reload') }}
            </UiButton>
          </div>

          <UiStatusCallout
            v-if="runtime.configLoading && !runtime.config"
            :description="t('settings.runtime.loading')"
          />

          <UiEmptyState
            v-else-if="!runtime.config"
            :title="t('settings.runtime.emptyTitle')"
            :description="runtime.configError || t('settings.runtime.emptyDescription')"
          />

          <div v-else class="grid gap-6 xl:grid-cols-[minmax(0,1.3fr)_minmax(22rem,0.9fr)]">
            <div class="space-y-4">
              <UiRecordCard
                :title="t('settings.runtime.workspace.title')"
                :description="t('settings.runtime.workspace.description')"
                test-id="settings-runtime-editor-workspace"
              >
                <template #eyebrow>
                  workspace
                </template>
                <template #badges>
                  <UiBadge
                    :label="resolveSourceStatusLabel(workspaceRuntimeSource)"
                    :tone="resolveSourceStatusTone(workspaceRuntimeSource)"
                  />
                  <UiBadge
                    :label="resolveValidationLabel(runtime.configValidation.workspace)"
                    :tone="resolveValidationTone(runtime.configValidation.workspace)"
                  />
                </template>

                <div class="space-y-3">
                  <UiCodeEditor
                    language="json"
                    theme="octopus"
                    :model-value="workspaceRuntimeDraft"
                    @update:model-value="runtime.setConfigDraft('workspace', $event)"
                  />

                  <UiStatusCallout
                    v-if="runtime.configValidation.workspace?.errors.length"
                    tone="error"
                    :description="runtime.configValidation.workspace.errors.join(' ')"
                  />

                  <UiStatusCallout
                    v-if="runtime.configValidation.workspace?.warnings.length"
                    tone="warning"
                    :description="runtime.configValidation.workspace.warnings.join(' ')"
                  />
                </div>

                <template #meta>
                  <span class="text-[11px] uppercase tracking-[0.24em] text-text-tertiary">
                    {{ t('settings.runtime.sourcePath') }}
                  </span>
                  <span class="min-w-0 truncate font-mono text-[12px] text-text-secondary">
                    {{ workspaceRuntimeSource?.displayPath ?? t('common.na') }}
                  </span>
                </template>
                <template #actions>
                  <UiButton
                    variant="ghost"
                    size="sm"
                    :disabled="runtime.configValidating || runtime.configSaving"
                    @click="validateWorkspaceRuntime"
                  >
                    {{ t('settings.runtime.actions.validate') }}
                  </UiButton>
                  <UiButton
                    size="sm"
                    :disabled="runtime.configSaving"
                    @click="saveWorkspaceRuntime"
                  >
                    {{ t('settings.runtime.actions.save') }}
                  </UiButton>
                </template>
              </UiRecordCard>
            </div>

            <div class="space-y-4">
              <UiRecordCard
                :title="t('settings.runtime.effective.title')"
                :description="t('settings.runtime.effective.description')"
                test-id="settings-runtime-effective-preview"
              >
                <template #badges>
                  <UiBadge :label="runtime.config.effectiveConfigHash" tone="info" />
                  <UiBadge
                    :label="runtime.config.validation.valid ? t('settings.runtime.validation.valid') : t('settings.runtime.validation.invalid')"
                    :tone="runtime.config.validation.valid ? 'success' : 'error'"
                  />
                </template>

                <div class="space-y-3">
                  <UiCodeEditor
                    language="json"
                    theme="octopus"
                    readonly
                    :model-value="runtimeEffectivePreview"
                  />

                  <div class="rounded-[var(--radius-l)] border border-border bg-subtle px-3 py-3 text-[12px] text-text-secondary">
                    <p class="text-[11px] font-bold uppercase tracking-[0.24em] text-text-tertiary">
                      {{ t('settings.runtime.secretReferencesTitle') }}
                    </p>
                    <div v-if="runtimeSecretStatuses.length" class="mt-3 flex flex-wrap gap-2">
                      <UiBadge
                        v-for="secret in runtimeSecretStatuses"
                        :key="`${secret.scope}-${secret.path}`"
                        :label="`${secret.scope}: ${secret.status}`"
                        :tone="secret.status === 'reference-missing' ? 'warning' : 'info'"
                      />
                    </div>
                    <p v-else class="mt-2">
                      {{ t('settings.runtime.noSecretReferences') }}
                    </p>
                  </div>
                </div>
              </UiRecordCard>
            </div>
          </div>
        </section>

        <!-- Version Tab -->
        <section v-else class="space-y-8">
          <div class="space-y-6">
            <div class="overflow-hidden rounded-[var(--radius-l)] border border-border bg-surface">
              <div
                v-for="(row, i) in versionRows"
                :key="row.id"
                :data-testid="`settings-version-row-${row.id}`"
                class="flex items-center justify-between px-6 py-4"
                :class="i !== versionRows.length - 1 ? 'border-b border-border/60' : ''"
              >
                <span class="text-[14px] text-text-secondary font-medium">{{ row.label }}</span>
                <span class="text-[14px] font-bold text-text-primary tracking-tight font-mono">{{ row.value }}</span>
              </div>
            </div>
          </div>
        </section>
      </div>
    </main>
  </UiPageShell>
</template>
