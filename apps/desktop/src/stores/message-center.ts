import { defineStore } from 'pinia'

import { useInboxStore } from './inbox'
import { useNotificationStore } from './notifications'

export type MessageCenterTab = 'notifications' | 'inbox'

export const useMessageCenterStore = defineStore('messageCenter', {
  state: () => ({
    open: false,
    activeTab: 'notifications' as MessageCenterTab,
  }),
  getters: {
    combinedCount(): number {
      const notifications = useNotificationStore()
      const inbox = useInboxStore()
      return notifications.unreadSummary.total + inbox.actionableCount
    },
  },
  actions: {
    openCenter() {
      this.open = true
    },
    closeCenter() {
      this.open = false
    },
    setActiveTab(tab: MessageCenterTab) {
      this.activeTab = tab
    },
  },
})
