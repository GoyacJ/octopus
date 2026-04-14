import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { createDefaultHostUpdateStatus, createDefaultShellPreferences } from '@octopus/schema'
import type { HostUpdateChannel } from '@octopus/schema'

import { enumLabel } from '@/i18n/copy'
import { useAppUpdateStore } from '@/stores/app-update'
import { useShellStore } from '@/stores/shell'

export type SettingsTab = 'general' | 'connection' | 'theme' | 'version'

function resolveSettingsTab(value: unknown): SettingsTab {
  return ['general', 'connection', 'theme', 'version'].includes(String(value))
    ? value as SettingsTab
    : 'general'
}

export function useAppSettingsView() {
  const { t } = useI18n()
  const appUpdate = useAppUpdateStore()
  const shell = useShellStore()
  const route = useRoute()

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
    return shell.workspaceConnections.find(item => item.workspaceId === workspaceId)?.label ?? workspaceId
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

  const fontSizeOptions = [
    { value: '13', label: '13px' },
    { value: '14', label: '14px' },
    { value: '15', label: '15px' },
    { value: '16', label: '16px' },
  ]

  const updateChannelOptions = computed(() => [
    { value: 'formal', label: t('settings.version.channels.formal') },
    { value: 'preview', label: t('settings.version.channels.preview') },
  ])

  const versionStatus = computed(() =>
    appUpdate.status ?? createDefaultHostUpdateStatus({
      currentVersion: shell.hostState.appVersion,
      currentChannel: shell.preferences.updateChannel,
    }),
  )

  const updateChannel = computed({
    get: () => versionStatus.value.currentChannel,
    set: (value: string) => {
      if (value && value !== versionStatus.value.currentChannel) {
        void appUpdate.setUpdateChannel(value as HostUpdateChannel)
      }
    },
  })

  const latestRelease = computed(() => versionStatus.value.latestRelease)

  const updateStatusTone = computed<'info' | 'success' | 'warning' | 'error'>(() => {
    switch (versionStatus.value.state) {
      case 'up_to_date':
      case 'downloaded':
        return 'success'
      case 'update_available':
      case 'downloading':
      case 'installing':
      case 'checking':
        return 'warning'
      case 'error':
        return 'error'
      default:
        return 'info'
    }
  })

  const updateStatusLabel = computed(() => t(`settings.version.states.${versionStatus.value.state}`))

  const updateStatusDescription = computed(() => {
    if (versionStatus.value.state === 'error' && versionStatus.value.errorMessage) {
      return versionStatus.value.errorMessage
    }

    if (!versionStatus.value.capabilities.canDownload || !versionStatus.value.capabilities.canInstall) {
      return t('settings.version.environment.unsupported')
    }

    if (versionStatus.value.state === 'update_available' && latestRelease.value?.version) {
      return t('settings.version.statusDescriptions.updateAvailable', {
        version: latestRelease.value.version,
      })
    }

    if (versionStatus.value.state === 'downloaded') {
      return t('settings.version.statusDescriptions.downloaded')
    }

    if (versionStatus.value.state === 'downloading' && typeof versionStatus.value.progress?.percent === 'number') {
      return t('settings.version.statusDescriptions.downloading', {
        percent: versionStatus.value.progress.percent,
      })
    }

    return t(`settings.version.statusDescriptions.${versionStatus.value.state}`)
  })

  const primaryUpdateActionLabel = computed(() => {
    switch (versionStatus.value.state) {
      case 'checking':
        return t('settings.version.actions.checking')
      case 'update_available':
        return t('settings.version.actions.download')
      case 'downloading':
        return t('settings.version.actions.downloading')
      case 'downloaded':
        return t('settings.version.actions.install')
      case 'installing':
        return t('settings.version.actions.installing')
      default:
        return t('settings.version.actions.check')
    }
  })

  const primaryUpdateActionDisabled = computed(() => {
    if (versionStatus.value.state === 'checking' || versionStatus.value.state === 'downloading' || versionStatus.value.state === 'installing') {
      return true
    }

    if (versionStatus.value.state === 'update_available') {
      return !versionStatus.value.capabilities.canDownload
    }

    if (versionStatus.value.state === 'downloaded') {
      return !versionStatus.value.capabilities.canInstall
    }

    return !versionStatus.value.capabilities.canCheck
  })

  const hasReleaseNotesLink = computed(() => Boolean(latestRelease.value?.notesUrl))
  const canManageSettings = computed(() => true)

  watch(
    () => route.query.tab,
    (tab) => {
      activeTab.value = resolveSettingsTab(tab)
    },
  )

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

  watch(
    [theme, locale, fontSize, leftSidebarCollapsed, rightSidebarCollapsed],
    async ([nextTheme, nextLocale, nextFontSize, nextLeftSidebar, nextRightSidebar]) => {
      if (!canManageSettings.value) return

      const patch: Record<string, unknown> = {}
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

  async function resetToDefault() {
    const defaults = createDefaultShellPreferences(shell.defaultWorkspaceId, shell.defaultProjectId)
    await shell.updatePreferences(defaults)
  }

  function formatRelativeTimestamp(value: number | null): string {
    if (!value) {
      return t('settings.version.values.notChecked')
    }

    return new Intl.DateTimeFormat(shell.preferences.locale, {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(value)
  }

  function formatReleaseDate(value?: string | null): string {
    if (!value) {
      return t('common.na')
    }

    return new Intl.DateTimeFormat(shell.preferences.locale, {
      dateStyle: 'medium',
    }).format(new Date(value))
  }

  async function handlePrimaryUpdateAction() {
    switch (versionStatus.value.state) {
      case 'update_available':
        await appUpdate.downloadUpdate()
        return
      case 'downloaded':
        await appUpdate.installUpdate()
        return
      default:
        await appUpdate.checkForUpdates()
    }
  }

  function openReleaseNotes() {
    if (!latestRelease.value?.notesUrl) {
      return
    }

    window.open(latestRelease.value.notesUrl, '_blank', 'noopener,noreferrer')
  }

  return {
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
  }
}
