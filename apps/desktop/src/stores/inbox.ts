import { defineStore } from 'pinia'

import type { InboxItemRecord } from '@octopus/schema'

import {
  createWorkspaceRequestToken,
  resolveWorkspaceClientForConnection,
} from './workspace-scope'

export const useInboxStore = defineStore('inbox', {
  state: () => ({
    itemsState: [] as InboxItemRecord[],
    loadingState: false,
    errorState: '',
    bootstrapped: false,
    activeConnectionId: '',
    requestToken: 0,
  }),
  getters: {
    items(state): InboxItemRecord[] {
      return state.itemsState
    },
    actionableCount(state): number {
      return state.itemsState.filter(item => item.actionable).length
    },
    loading(state): boolean {
      return state.loadingState
    },
    error(state): string {
      return state.errorState
    },
  },
  actions: {
    reset() {
      this.itemsState = []
      this.loadingState = false
      this.errorState = ''
      this.bootstrapped = false
      this.activeConnectionId = ''
      this.requestToken = 0
    },
    async bootstrap(workspaceConnectionId?: string, force = false) {
      const resolvedClient = resolveWorkspaceClientForConnection(workspaceConnectionId)
      if (!resolvedClient) {
        this.reset()
        return
      }

      const { client, connectionId } = resolvedClient
      if (!force && this.bootstrapped && this.activeConnectionId === connectionId) {
        return
      }

      const token = createWorkspaceRequestToken(this.requestToken)
      this.requestToken = token
      this.activeConnectionId = connectionId
      this.loadingState = true
      this.errorState = ''

      try {
        const records = await client.inbox.list()
        if (this.requestToken !== token || this.activeConnectionId !== connectionId) {
          return
        }
        this.itemsState = records
        this.bootstrapped = true
      } catch (cause) {
        if (this.requestToken !== token || this.activeConnectionId !== connectionId) {
          return
        }
        this.itemsState = []
        this.errorState = cause instanceof Error ? cause.message : 'Failed to load inbox items'
      } finally {
        if (this.requestToken === token && this.activeConnectionId === connectionId) {
          this.loadingState = false
        }
      }
    },
  },
})
