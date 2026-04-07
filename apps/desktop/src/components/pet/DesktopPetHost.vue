<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'

import type { PetMotionState } from '@octopus/schema'

import { UiSurface } from '@octopus/ui'

import DesktopPetAvatar from '@/components/pet/DesktopPetAvatar.vue'
import DesktopPetChat from '@/components/pet/DesktopPetChat.vue'
import { usePetStore } from '@/stores/pet'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const pet = usePetStore()
const shell = useShellStore()
const workspace = useWorkspaceStore()

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
const helperText = computed(() => {
  if (!hasProjectScope.value) {
    return '进入项目后即可和小章聊天。'
  }
  if (pet.loading) {
    return '小章正在准备中…'
  }
  return ''
})

const runtimeError = computed(() => pet.error)

watch(
  () => [shell.activeWorkspaceConnectionId, workspace.currentProjectId],
  () => {
    if (shell.activeWorkspaceConnectionId) {
      void pet.loadSnapshot(workspace.currentProjectId, shell.activeWorkspaceConnectionId)
    }
  },
  { immediate: true },
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
  window.removeEventListener('resize', handleViewportChange)
  window.removeEventListener('scroll', handleViewportChange, true)
  document.removeEventListener('pointerdown', handleDocumentPointerDown)
})
</script>

<template>
  <div v-if="canRender" ref="host" class="relative flex justify-end" data-testid="desktop-pet-host">
    <DesktopPetAvatar
      :pet="pet.profile"
      :motion-state="pet.motionState"
      :unread-count="pet.unreadCount"
      class="pet-avatar-mini"
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
      <UiSurface
        v-if="helperText"
        variant="overlay"
        padding="sm"
        class="w-[22rem] max-w-[calc(100vw-2rem)] text-sm text-text-secondary"
        data-testid="desktop-pet-status"
      >
        {{ helperText }}
      </UiSurface>
      <DesktopPetChat
        v-else
        :pet="pet.profile"
        :messages="pet.messages"
        :on-send="handleSend"
      />
      <UiSurface
        v-if="runtimeError && !helperText"
        variant="overlay"
        padding="sm"
        class="mt-2 w-[22rem] max-w-[calc(100vw-2rem)] border border-status-error/20 bg-status-error/5 text-sm text-status-error"
        data-testid="desktop-pet-error"
        role="alert"
      >
        {{ runtimeError }}
      </UiSurface>
    </div>
  </div>
</template>
