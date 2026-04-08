<script setup lang="ts">
import type { NotificationRecord, NotificationScopeKind } from '@octopus/schema'

import UiToastItem from './UiToastItem.vue'

const props = defineProps<{
  notifications: NotificationRecord[]
  scopeLabels: Record<NotificationScopeKind, string>
}>()

const emit = defineEmits<{
  close: [id: string]
  select: [notification: NotificationRecord]
}>()
</script>

<template>
  <div
    class="pointer-events-none fixed right-4 top-4 z-[70] flex w-[22rem] flex-col gap-3"
    data-testid="ui-toast-viewport"
  >
    <div
      v-for="notification in props.notifications"
      :key="notification.id"
      class="pointer-events-auto"
    >
      <UiToastItem
        :notification="notification"
        :scope-label="props.scopeLabels[notification.scopeKind]"
        @close="emit('close', $event)"
        @select="emit('select', $event)"
      />
    </div>
  </div>
</template>
