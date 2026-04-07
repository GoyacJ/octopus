<script setup lang="ts">
import type {
  NotificationFilterScope,
  NotificationRecord,
  NotificationScopeKind,
} from '@octopus/schema'

import UiButton from './UiButton.vue'
import UiNotificationRow from './UiNotificationRow.vue'
import UiSurface from './UiSurface.vue'

const props = defineProps<{
  open: boolean
  notifications: NotificationRecord[]
  unreadCount: number
  activeFilter: NotificationFilterScope
  filterLabels: Record<NotificationFilterScope, string>
  scopeLabels: Record<NotificationScopeKind, string>
  title: string
  emptyTitle: string
  emptyDescription: string
  markAllLabel: string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  'update:filter': [value: NotificationFilterScope]
  'mark-read': [id: string]
  'mark-all-read': []
  select: [notification: NotificationRecord]
}>()

const filters: NotificationFilterScope[] = ['all', 'app', 'workspace', 'user']
</script>

<template>
  <div
    v-if="props.open"
    class="w-[22rem]"
    data-testid="ui-notification-center"
  >
    <UiSurface
      variant="overlay"
      padding="sm"
      class="border-border/40 bg-gradient-to-br from-background via-background to-accent/20 shadow-[0_18px_48px_rgba(15,23,42,0.12)] dark:border-white/[0.08]"
    >
      <div class="flex items-start justify-between gap-3 px-1 pb-3">
        <div class="space-y-1">
          <p class="text-[11px] font-semibold uppercase tracking-[0.2em] text-text-tertiary">
            {{ props.title }}
          </p>
          <p class="text-sm text-text-secondary">
            {{ props.unreadCount }} unread
          </p>
        </div>
        <UiButton
          variant="ghost"
          size="sm"
          data-testid="ui-notification-center-mark-all"
          @click="emit('mark-all-read')"
        >
          {{ props.markAllLabel }}
        </UiButton>
      </div>

      <div class="mb-3 flex flex-wrap gap-2 px-1">
        <button
          v-for="filter in filters"
          :key="filter"
          type="button"
          class="rounded-full border px-2.5 py-1 text-xs font-medium transition-all duration-normal ease-apple"
          :class="filter === props.activeFilter
            ? 'border-foreground/10 bg-foreground text-background'
            : 'border-border/40 bg-background text-text-secondary hover:bg-accent'"
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
          @select="emit('select', $event)"
        />
      </div>
      <div v-else class="rounded-2xl border border-dashed border-border/40 px-4 py-8 text-center">
        <p class="text-sm font-semibold text-text-primary">
          {{ props.emptyTitle }}
        </p>
        <p class="mt-1 text-xs leading-5 text-text-secondary">
          {{ props.emptyDescription }}
        </p>
      </div>
    </UiSurface>
  </div>
</template>
