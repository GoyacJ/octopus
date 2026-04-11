<script setup lang="ts">
import { computed } from 'vue'
import type {
  InboxItemRecord,
  NotificationFilterScope,
  NotificationRecord,
  NotificationScopeKind,
} from '@octopus/schema'

import { formatDateTime } from '../lib/formatDateTime'
import UiButton from './UiButton.vue'
import UiEmptyState from './UiEmptyState.vue'
import UiInboxBlock from './UiInboxBlock.vue'
import UiNotificationRow from './UiNotificationRow.vue'
import UiStatusCallout from './UiStatusCallout.vue'
import UiSurface from './UiSurface.vue'
import UiTabs from './UiTabs.vue'

type UiMessageCenterTab = 'notifications' | 'inbox'

const props = defineProps<{
  open: boolean
  activeTab: UiMessageCenterTab
  notificationTabLabel: string
  inboxTabLabel: string
  notificationTitle: string
  notificationUnreadLabel: string
  notifications: NotificationRecord[]
  unreadCount: number
  activeFilter: NotificationFilterScope
  filterLabels: Record<NotificationFilterScope, string>
  scopeLabels: Record<NotificationScopeKind, string>
  notificationEmptyTitle: string
  notificationEmptyDescription: string
  notificationMarkAllLabel: string
  inboxTitle: string
  inboxSubtitle: string
  inboxLoading: boolean
  inboxError: string
  inboxItems: InboxItemRecord[]
  inboxEmptyTitle: string
  inboxEmptyDescription: string
  inboxOpenLabel: string
  inboxStatusHeading: string
  inboxTypeHeading: string
  inboxLoadingLabel: string
  inboxErrorTitle: string
  inboxErrorDescription: string
}>()

const emit = defineEmits<{
  'update:activeTab': [value: UiMessageCenterTab]
  'update:filter': [value: NotificationFilterScope]
  'mark-read': [id: string]
  'mark-all-read': []
  'select-notification': [notification: NotificationRecord]
  'select-inbox': [item: InboxItemRecord]
}>()

const filters: NotificationFilterScope[] = ['all', 'app', 'workspace', 'user']

const tabs = computed(() => [
  { value: 'notifications', label: props.notificationTabLabel },
  { value: 'inbox', label: props.inboxTabLabel },
])

function formatLabel(value?: string): string {
  if (!value) {
    return ''
  }

  return value
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, letter => letter.toUpperCase())
}
</script>

<template>
  <div
    v-if="props.open"
    class="w-[22rem]"
    data-testid="ui-message-center"
  >
    <UiSurface variant="overlay" padding="sm" class="border-border bg-popover shadow-md">
      <div class="px-1 pb-3">
        <UiTabs
          :model-value="props.activeTab"
          :tabs="tabs"
          variant="pill"
          test-id="ui-message-center-tabs"
          @update:model-value="emit('update:activeTab', $event as UiMessageCenterTab)"
        />
      </div>

      <div v-if="props.activeTab === 'notifications'" class="space-y-3 px-1">
        <div class="flex items-start justify-between gap-3">
          <div class="space-y-1">
            <p class="text-[11px] font-semibold uppercase tracking-[0.2em] text-text-tertiary">
              {{ props.notificationTitle }}
            </p>
            <p class="text-sm text-text-secondary">
              {{ props.notificationUnreadLabel }}
            </p>
          </div>
          <UiButton
            variant="ghost"
            size="sm"
            data-testid="ui-message-center-mark-all"
            :disabled="props.unreadCount === 0"
            @click="emit('mark-all-read')"
          >
            {{ props.notificationMarkAllLabel }}
          </UiButton>
        </div>

        <div class="flex flex-wrap gap-2">
          <button
            v-for="filter in filters"
            :key="filter"
            type="button"
            class="rounded-full border px-2.5 py-1 text-xs font-medium transition-colors duration-fast"
            :class="filter === props.activeFilter
              ? 'border-border-strong bg-accent text-text-primary shadow-xs'
              : 'border-border bg-surface text-text-secondary hover:bg-accent'"
            :data-testid="`ui-notification-filter-${filter}`"
            @click="emit('update:filter', filter)"
          >
            {{ props.filterLabels[filter] }}
          </button>
        </div>

        <div v-if="props.notifications.length" class="max-h-[26rem] space-y-2 overflow-y-auto pr-1">
          <UiNotificationRow
            v-for="notification in props.notifications"
            :key="notification.id"
            :notification="notification"
            :scope-label="props.scopeLabels[notification.scopeKind]"
            @mark-read="emit('mark-read', $event)"
            @select="emit('select-notification', $event)"
          />
        </div>

        <UiEmptyState
          v-else
          :title="props.notificationEmptyTitle"
          :description="props.notificationEmptyDescription"
          class="rounded-[var(--radius-xl)] border border-dashed border-border px-2 py-8"
        />
      </div>

      <div v-else class="space-y-3 px-1">
        <div class="space-y-1">
          <p class="text-[11px] font-semibold uppercase tracking-[0.2em] text-text-tertiary">
            {{ props.inboxTitle }}
          </p>
          <p class="text-sm text-text-secondary">
            {{ props.inboxSubtitle }}
          </p>
        </div>

        <div
          v-if="props.inboxLoading"
          class="rounded-[var(--radius-xl)] border border-dashed border-border px-4 py-8 text-center text-sm text-text-secondary"
        >
          {{ props.inboxLoadingLabel }}
        </div>

        <UiStatusCallout
          v-else-if="props.inboxError"
          tone="error"
          :title="props.inboxErrorTitle"
          :description="props.inboxErrorDescription"
        >
          <p class="text-xs leading-5 text-current/80">
            {{ props.inboxError }}
          </p>
        </UiStatusCallout>

        <div v-else-if="props.inboxItems.length" class="max-h-[26rem] space-y-2 overflow-y-auto pr-1">
          <UiInboxBlock
            v-for="item in props.inboxItems"
            :key="item.id"
            :title="item.title"
            :description="item.description"
            :priority-label="formatLabel(item.priority)"
            :timestamp-label="formatDateTime(item.createdAt)"
            :status-label="formatLabel(item.status)"
            :impact="formatLabel(item.itemType)"
            :status-heading="props.inboxStatusHeading"
            :impact-heading="props.inboxTypeHeading"
          >
            <template v-if="item.routeTo" #actions>
              <UiButton
                size="sm"
                :data-testid="`ui-message-center-inbox-action-${item.id}`"
                @click="emit('select-inbox', item)"
              >
                {{ item.actionLabel || props.inboxOpenLabel }}
              </UiButton>
            </template>
          </UiInboxBlock>
        </div>

        <UiEmptyState
          v-else
          :title="props.inboxEmptyTitle"
          :description="props.inboxEmptyDescription"
          class="rounded-[var(--radius-xl)] border border-dashed border-border px-2 py-8"
        />
      </div>
    </UiSurface>
  </div>
</template>
