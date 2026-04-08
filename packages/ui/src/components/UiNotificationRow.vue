<script setup lang="ts">
import type { NotificationRecord } from '@octopus/schema'
import { CheckCheck } from 'lucide-vue-next'

const props = defineProps<{
  notification: NotificationRecord
  scopeLabel: string
}>()

const emit = defineEmits<{
  'mark-read': [id: string]
  select: [notification: NotificationRecord]
}>()

function handleSelect() {
  emit('select', props.notification)
}

function handleMarkRead() {
  emit('mark-read', props.notification.id)
}
</script>

<template>
  <article
    :data-testid="`ui-notification-row-${props.notification.id}`"
    class="group flex items-start gap-3 rounded-2xl border border-border/40 bg-gradient-to-br from-background via-background to-accent/30 px-3 py-3 text-left transition-all duration-normal ease-apple hover:border-border/60 hover:bg-accent/30"
    :class="{ 'opacity-70': props.notification.readAt }"
    @click="handleSelect"
  >
    <div class="mt-1 h-2.5 w-2.5 rounded-full bg-foreground/70" :class="{
      'bg-emerald-500': props.notification.level === 'success',
      'bg-amber-500': props.notification.level === 'warning',
      'bg-rose-500': props.notification.level === 'error',
      'bg-sky-500': props.notification.level === 'info',
    }" />
    <div class="min-w-0 flex-1 space-y-1">
      <div class="flex items-center gap-2">
        <span class="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">
          {{ props.scopeLabel }}
        </span>
        <span v-if="!props.notification.readAt" class="rounded-full bg-foreground/8 px-2 py-0.5 text-[10px] font-medium text-text-secondary">
          New
        </span>
      </div>
      <p class="truncate text-sm font-semibold text-text-primary">
        {{ props.notification.title }}
      </p>
      <p class="text-xs leading-5 text-text-secondary">
        {{ props.notification.body }}
      </p>
    </div>
    <button
      type="button"
      class="rounded-full p-1 text-text-tertiary transition-colors hover:bg-accent hover:text-text-primary"
      :data-testid="`ui-notification-row-mark-read-${props.notification.id}`"
      @click.stop="handleMarkRead"
    >
      <CheckCheck :size="14" />
    </button>
  </article>
</template>
