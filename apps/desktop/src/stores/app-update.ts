import { defineStore } from 'pinia'

import {
  createDefaultHostUpdateStatus,
  normalizeHostUpdateStatus,
  type HostUpdateChannel,
  type HostUpdateStatus,
} from '@octopus/schema'

import * as tauriClient from '@/tauri/client'
import { useShellStore } from '@/stores/shell'

function toErrorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback
}

export const useAppUpdateStore = defineStore('app-update', {
  state: () => ({
    statusState: null as HostUpdateStatus | null,
    initialized: false,
    initializePromise: null as Promise<void> | null,
  }),
  getters: {
    status(state): HostUpdateStatus | null {
      return state.statusState
    },
    currentChannel(state): HostUpdateChannel {
      if (state.statusState?.currentChannel) {
        return state.statusState.currentChannel
      }

      return useShellStore().preferences.updateChannel
    },
  },
  actions: {
    applyStatus(status: HostUpdateStatus | null | undefined) {
      const shell = useShellStore()
      this.statusState = normalizeHostUpdateStatus({
        currentVersion: shell.hostState.appVersion,
        currentChannel: shell.preferences.updateChannel,
        ...status,
      })
    },
    applyError(error: unknown, fallback: string) {
      const shell = useShellStore()
      this.applyStatus(createDefaultHostUpdateStatus({
        currentVersion: this.statusState?.currentVersion ?? shell.hostState.appVersion,
        currentChannel: this.statusState?.currentChannel ?? shell.preferences.updateChannel,
        latestRelease: this.statusState?.latestRelease ?? null,
        lastCheckedAt: this.statusState?.lastCheckedAt ?? null,
        progress: this.statusState?.progress ?? null,
        capabilities: this.statusState?.capabilities ?? createDefaultHostUpdateStatus().capabilities,
        state: 'error',
        errorCode: 'UPDATE_ACTION_FAILED',
        errorMessage: toErrorMessage(error, fallback),
      }))
    },
    async refreshStatus() {
      this.applyStatus(await tauriClient.getHostUpdateStatus())
    },
    async checkForUpdates(channel = this.currentChannel) {
      try {
        this.applyStatus({
          ...this.statusState,
          currentChannel: channel,
          state: 'checking',
          errorCode: null,
          errorMessage: null,
        })
        this.applyStatus(await tauriClient.checkHostUpdate(channel))
      }
      catch (error) {
        this.applyError(error, 'Failed to check for updates')
      }
    },
    async initialize() {
      if (this.initialized) {
        return
      }

      if (this.initializePromise) {
        await this.initializePromise
        return
      }

      this.initializePromise = (async () => {
        await this.refreshStatus()
        if (this.statusState?.capabilities.canCheck) {
          await this.checkForUpdates(useShellStore().preferences.updateChannel)
        }
        this.initialized = true
      })()

      try {
        await this.initializePromise
      }
      finally {
        this.initializePromise = null
      }
    },
    async setUpdateChannel(channel: HostUpdateChannel) {
      const shell = useShellStore()
      await shell.updatePreferences({ updateChannel: channel })
      this.applyStatus({
        ...this.statusState,
        currentChannel: channel,
      })

      await this.checkForUpdates(channel)
    },
    async downloadUpdate() {
      try {
        this.applyStatus({
          ...this.statusState,
          state: 'downloading',
          errorCode: null,
          errorMessage: null,
        })
        this.applyStatus(await tauriClient.downloadHostUpdate())
      }
      catch (error) {
        this.applyError(error, 'Failed to download update')
      }
    },
    async installUpdate() {
      try {
        this.applyStatus({
          ...this.statusState,
          state: 'installing',
          errorCode: null,
          errorMessage: null,
        })
        this.applyStatus(await tauriClient.installHostUpdate())
      }
      catch (error) {
        this.applyError(error, 'Failed to install update')
      }
    },
  },
})
