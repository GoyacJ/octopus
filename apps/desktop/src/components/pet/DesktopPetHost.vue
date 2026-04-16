<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

import type { NotificationRecord, PetMotionState } from '@octopus/schema'

import { UiStatusCallout } from '@octopus/ui'

import DesktopPetAvatar from '@/components/pet/DesktopPetAvatar.vue'
import DesktopPetBubble from '@/components/pet/DesktopPetBubble.vue'
import DesktopPetChat from '@/components/pet/DesktopPetChat.vue'
import { useNotificationStore } from '@/stores/notifications'
import { usePetStore } from '@/stores/pet'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const pet = usePetStore()
const notifications = useNotificationStore()
const shell = useShellStore()
const workspace = useWorkspaceStore()
const router = useRouter()
const { t } = useI18n()

const host = ref<HTMLElement | null>(null)
const panel = ref<HTMLElement | null>(null)
const panelReady = ref(false)
const panelStyle = ref<Record<string, string>>({})

const open = computed({
  get: () => pet.presence.chatOpen,
  set: (value: boolean) => {
    if (value) {
      void pet.openChat()
      return
    }
    void pet.closeChat()
  },
})

const canRender = computed(() => !!shell.activeWorkspaceConnectionId)
const hasProjectScope = computed(() => !!workspace.currentProjectId)
const currentWorkspaceId = computed(() => shell.activeWorkspaceConnection?.workspaceId ?? '')
const currentUserId = computed(() => shell.activeWorkspaceSession?.session.userId ?? '')
const notificationScopeLabels = computed(() => ({
  app: t('notifications.scopes.app'),
  workspace: t('notifications.scopes.workspace'),
  user: t('notifications.scopes.user'),
}))
const helperText = computed(() => {
  if (!hasProjectScope.value) {
    return t('petHost.status.noProject')
  }
  if (pet.loading) {
    return t('petHost.status.loading')
  }
  return ''
})

const runtimeError = computed(() => pet.error)
const reminderNotification = computed<NotificationRecord | null>(() => {
  if (!canRender.value || open.value) {
    return null
  }

  return notifications.notifications.find((notification) => {
    if (!notification.routeTo) {
      return false
    }
    if (
      typeof notification.toastVisibleUntil !== 'number'
      || notification.toastVisibleUntil <= notifications.toastNow
    ) {
      return false
    }
    if (notification.scopeKind === 'workspace') {
      return !!currentWorkspaceId.value && notification.scopeOwnerId === currentWorkspaceId.value
    }
    if (notification.scopeKind === 'user') {
      return !!currentUserId.value && notification.scopeOwnerId === currentUserId.value
    }
    return notification.scopeKind === 'app'
  }) ?? null
})
const reminderScopeLabel = computed(() => (
  reminderNotification.value
    ? notificationScopeLabels.value[reminderNotification.value.scopeKind]
    : ''
))
const reminderActionLabel = computed(() => (
  reminderNotification.value?.actionLabel?.trim() || t('common.open')
))

watch(
  () => [shell.activeWorkspaceConnectionId, workspace.currentProjectId],
  () => {
    if (shell.activeWorkspaceConnectionId) {
      void pet.loadSnapshot(workspace.currentProjectId, shell.activeWorkspaceConnectionId)
    }
  },
  { immediate: true },
)

watch(
  () => reminderNotification.value?.id ?? null,
  (notificationId) => {
    notifications.setPetBubbleNotification(notificationId)
  },
  { immediate: true, flush: 'sync' },
)

watch(open, async (value) => {
  panelReady.value = false
  if (!value) {
    return
  }
  await nextTick()
  updatePanelPosition()
})

function handleHoverState(state: PetMotionState) {
  if (pet.presence.chatOpen || pet.loading || !hasProjectScope.value) {
    return
  }
  void pet.savePresence({ motionState: state })
}

async function handleSend(content: string) {
  return await pet.sendMessage(content)
}

function toggleChat() {
  open.value = !open.value
}

async function handleReminderSelect(notification: NotificationRecord) {
  if (!notification.routeTo) {
    return
  }

  await router.push(notification.routeTo)
  await notifications.dismissToast(notification.id)
}

function updatePanelPosition() {
  const hostElement = host.value
  const panelElement = panel.value
  const triggerElement = hostElement?.querySelector<HTMLElement>('[data-testid="desktop-pet-trigger"]')
  if (!hostElement || !panelElement || !triggerElement) {
    return
  }

  const triggerRect = triggerElement.getBoundingClientRect()
  const panelRect = panelElement.getBoundingClientRect()
  const viewportPadding = 16
  const viewportWidth = window.innerWidth
  const viewportHeight = window.innerHeight
  const desiredLeft = triggerRect.right - panelRect.width
  const maxLeft = Math.max(viewportPadding, viewportWidth - panelRect.width - viewportPadding)
  const clampedLeft = Math.min(Math.max(desiredLeft, viewportPadding), maxLeft)
  const desiredBottom = viewportHeight - triggerRect.top + 8
  const maxBottom = Math.max(viewportPadding, viewportHeight - viewportPadding)

  panelStyle.value = {
    left: `${clampedLeft}px`,
    bottom: `${Math.min(desiredBottom, maxBottom)}px`,
    visibility: 'visible',
  }
  panelReady.value = true
}

function handleViewportChange() {
  if (!open.value) {
    return
  }
  updatePanelPosition()
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (!open.value) {
    return
  }
  const target = event.target
  if (!(target instanceof Node) || host.value?.contains(target)) {
    return
  }
  open.value = false
}

window.addEventListener('resize', handleViewportChange)
window.addEventListener('scroll', handleViewportChange, true)
document.addEventListener('pointerdown', handleDocumentPointerDown)

onBeforeUnmount(() => {
  notifications.setPetBubbleNotification(null)
  window.removeEventListener('resize', handleViewportChange)
  window.removeEventListener('scroll', handleViewportChange, true)
  document.removeEventListener('pointerdown', handleDocumentPointerDown)
})
</script>

<template>
  <div v-if="canRender" ref="host" class="relative flex justify-end" data-testid="desktop-pet-host">
    <div
      v-if="reminderNotification"
      class="absolute bottom-full right-0 z-40 mb-3"
    >
      <DesktopPetBubble
        :notification="reminderNotification"
        :scope-label="reminderScopeLabel"
        :action-label="reminderActionLabel"
        @select="handleReminderSelect"
      />
    </div>

    <DesktopPetAvatar
      :pet="pet.profile"
      :motion-state="pet.motionState"
      :unread-count="pet.unreadCount"
      size="sidebar"
      @hover-state="handleHoverState"
      @click="toggleChat"
    />

    <div
      v-if="open"
      ref="panel"
      class="fixed bottom-0 left-0 z-50"
      :style="panelReady ? panelStyle : { visibility: 'hidden' }"
      data-testid="desktop-pet-panel"
    >
      <UiStatusCallout
        v-if="helperText"
        tone="info"
        class="w-[22rem] max-w-[calc(100vw-2rem)] text-sm text-text-secondary"
        :description="helperText"
        data-testid="desktop-pet-status"
      />
      <DesktopPetChat
        v-else
        :pet="pet.profile"
        :messages="pet.messages"
        :on-send="handleSend"
      />
      <UiStatusCallout
        v-if="runtimeError && !helperText"
        tone="error"
        class="mt-2 w-[22rem] max-w-[calc(100vw-2rem)] text-sm"
        :description="runtimeError"
        data-testid="desktop-pet-error"
        role="alert"
      />
    </div>
  </div>
</template>
