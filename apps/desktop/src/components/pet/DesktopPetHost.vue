<script setup lang="ts">
import { computed } from 'vue'

import { UiPopover } from '@octopus/ui'

import DesktopPetAvatar from '@/components/pet/DesktopPetAvatar.vue'
import DesktopPetChat from '@/components/pet/DesktopPetChat.vue'
import { useWorkbenchStore } from '@/stores/workbench'

const workbench = useWorkbenchStore()

const pet = computed(() => workbench.currentUserPet)
const presence = computed(() => workbench.currentUserPetPresence)
const messages = computed(() => workbench.currentUserPetMessages)
const hostStyle = computed(() => ({
  right: `${presence.value?.position.x ?? 24}px`,
  bottom: `${presence.value?.position.y ?? 24}px`,
}))

function setChatOpen(value: boolean) {
  workbench.togglePetChat(value)
}
</script>

<template>
  <div
    v-if="pet && presence?.isVisible"
    class="desktop-pet-host"
    :style="hostStyle"
    data-testid="desktop-pet-host"
  >
    <UiPopover
      :open="presence.chatOpen"
      align="end"
      class="desktop-pet-popover"
      @update:open="setChatOpen"
    >
      <template #trigger>
        <DesktopPetAvatar
          :pet="pet"
          :motion-state="presence.motionState"
          :unread-count="presence.unreadCount"
          @open="workbench.togglePetChat(true)"
          @nudge="workbench.nudgePetPosition"
          @hover-state="workbench.setPetMotionState"
        />
      </template>
      <DesktopPetChat :pet="pet" :messages="messages" @send="workbench.sendPetMessage" />
    </UiPopover>
  </div>
</template>

<style scoped>
.desktop-pet-host {
  position: fixed;
  z-index: 45;
}

.desktop-pet-popover :deep(.relative.inline-flex) {
  flex-direction: column;
  align-items: end;
}

.desktop-pet-popover :deep(.absolute) {
  margin-top: 0;
  margin-bottom: 0.8rem;
  right: 0;
  left: auto;
  top: auto;
  bottom: 100%;
}
</style>
