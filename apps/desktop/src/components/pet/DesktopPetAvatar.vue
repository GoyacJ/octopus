<script setup lang="ts">
import { computed } from 'vue'

import type { PetMotionState, PetProfile } from '@octopus/schema'

import { petAssetMap } from '@octopus/assets/pets'

// 必须禁用默认属性继承，手动透传给 button 元素，否则 Popover 无法识别触发器
defineOptions({
  inheritAttrs: false
})

const props = defineProps<{
  pet: PetProfile
  motionState: PetMotionState
  unreadCount: number
  size?: 'default' | 'sidebar'
}>()

const emit = defineEmits<{
  hoverState: [state: PetMotionState]
}>()

function resolveFallbackAsset(fallbackAsset: string) {
  if (Object.prototype.hasOwnProperty.call(petAssetMap, fallbackAsset)) {
    return petAssetMap[fallbackAsset as keyof typeof petAssetMap]
  }

  return fallbackAsset
}

const resolvedPetAsset = computed(() =>
  petAssetMap[props.pet.species] ?? resolveFallbackAsset(props.pet.fallbackAsset),
)
</script>

<template>
  <button
    v-bind="$attrs"
    type="button"
    class="pet-avatar-button"
    :class="{ 'pet-avatar-button--sidebar': size === 'sidebar' }"
    data-testid="desktop-pet-trigger"
    @mouseenter="emit('hoverState', 'hover')"
    @mouseleave="emit('hoverState', 'idle')"
  >
    <div class="pet-avatar-stage" :class="`is-${motionState}`">
      <img
        :src="resolvedPetAsset"
        :alt="pet.displayName"
        class="pet-avatar-image"
        data-testid="desktop-pet-image"
      >
    </div>
    <span v-if="unreadCount" class="pet-unread-dot" data-testid="desktop-pet-unread">{{ unreadCount }}</span>
  </button>
</template>

<style scoped>
@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-8px); }
}

@keyframes wiggle {
  0%, 100% { transform: rotate(0deg); }
  25% { transform: rotate(-6deg); }
  75% { transform: rotate(6deg); }
}

.pet-avatar-button {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 6.5rem;
  height: 6.5rem;
  padding: 0;
  background: transparent;
  border: none;
  cursor: pointer;
  transition: transform var(--duration-fast) var(--ease-apple);
  outline: none;
  user-select: none;
  -webkit-tap-highlight-color: transparent;
  flex-shrink: 0;
  overflow: visible;
}

.pet-avatar-button--sidebar {
  width: 36px;
  height: 36px;
}

.pet-avatar-button:hover {
  transform: translateY(-2px) scale(1.05);
}

.pet-avatar-button--sidebar:hover {
  transform: scale(1.04);
}

.pet-avatar-button:active {
  transform: translateY(1px) scale(0.98);
}

.pet-avatar-stage {
  width: 5.5rem;
  height: 5.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: visible;
  /* 确保内部元素不拦截事件，让点击能穿透到 button */
  pointer-events: none;
}

.pet-avatar-button--sidebar .pet-avatar-stage {
  width: 32px;
  height: 32px;
}

.pet-avatar-stage.is-idle {
  animation: float 3s ease-in-out infinite;
}

.pet-avatar-stage.is-hover {
  animation: wiggle 0.4s ease-in-out infinite;
}

.pet-avatar-stage.is-chat {
  animation: float 1s ease-in-out infinite;
  transform: scale(1.1);
}

.pet-avatar-image {
  width: 100%;
  height: 100%;
  object-fit: contain;
  filter: drop-shadow(0 4px 8px rgba(0,0,0,0.15));
}

.pet-avatar-button--sidebar .pet-avatar-image {
  width: 32px;
  height: 32px;
}

.pet-unread-dot {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  min-width: 1.4rem;
  height: 1.4rem;
  padding: 0 0.4rem;
  border-radius: 999px;
  background: var(--brand-primary);
  color: white;
  font-size: 0.75rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 6px rgba(0,0,0,0.25);
  pointer-events: none;
  z-index: 10;
}

.pet-avatar-button--sidebar .pet-unread-dot {
  top: 0;
  right: 0;
  min-width: 11px;
  width: 11px;
  height: 11px;
  padding: 0;
  font-size: 7px;
}
</style>
