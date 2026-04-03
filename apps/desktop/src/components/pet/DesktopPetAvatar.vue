<script setup lang="ts">
import { computed } from 'vue'

import type { PetMotionState, PetProfile } from '@octopus/schema'

import { UiBadge, UiButton } from '@octopus/ui'

import duckAsset from '@/assets/pets/duck.svg'
import gooseAsset from '@/assets/pets/goose.svg'
import blobAsset from '@/assets/pets/blob.svg'
import catAsset from '@/assets/pets/cat.svg'
import dragonAsset from '@/assets/pets/dragon.svg'
import octopusAsset from '@/assets/pets/octopus.svg'
import owlAsset from '@/assets/pets/owl.svg'
import penguinAsset from '@/assets/pets/penguin.svg'
import turtleAsset from '@/assets/pets/turtle.svg'
import snailAsset from '@/assets/pets/snail.svg'
import ghostAsset from '@/assets/pets/ghost.svg'
import axolotlAsset from '@/assets/pets/axolotl.svg'
import capybaraAsset from '@/assets/pets/capybara.svg'
import cactusAsset from '@/assets/pets/cactus.svg'
import robotAsset from '@/assets/pets/robot.svg'
import rabbitAsset from '@/assets/pets/rabbit.svg'
import mushroomAsset from '@/assets/pets/mushroom.svg'
import chonkAsset from '@/assets/pets/chonk.svg'

const props = defineProps<{
  pet: PetProfile
  motionState: PetMotionState
  unreadCount: number
}>()

const emit = defineEmits<{
  open: []
  nudge: [deltaX: number, deltaY: number]
  hoverState: [state: PetMotionState]
}>()

const petAssetMap: Record<PetProfile['species'], string> = {
  duck: duckAsset,
  goose: gooseAsset,
  blob: blobAsset,
  cat: catAsset,
  dragon: dragonAsset,
  octopus: octopusAsset,
  owl: owlAsset,
  penguin: penguinAsset,
  turtle: turtleAsset,
  snail: snailAsset,
  ghost: ghostAsset,
  axolotl: axolotlAsset,
  capybara: capybaraAsset,
  cactus: cactusAsset,
  robot: robotAsset,
  rabbit: rabbitAsset,
  mushroom: mushroomAsset,
  chonk: chonkAsset,
}

const moodTone = computed(() => {
  if (props.motionState === 'chat') return 'info'
  if (props.motionState === 'sleep') return 'warning'
  return 'default'
})

</script>

<template>
  <div class="pet-avatar-shell" data-testid="desktop-pet-avatar">
    <UiBadge :label="pet.mood" :tone="moodTone" subtle class="pet-mood-badge" />
    <button
      type="button"
      class="pet-avatar-button"
      data-testid="desktop-pet-trigger"
      @click="emit('open')"
      @mouseenter="emit('hoverState', 'hover')"
      @mouseleave="emit('hoverState', 'idle')"
    >
      <div class="pet-avatar-stage" :class="`is-${motionState}`">
        <img
          :src="petAssetMap[pet.species] || pet.fallbackAsset"
          :alt="pet.displayName"
          class="pet-avatar-image"
          data-testid="desktop-pet-image"
        >
      </div>
      <span v-if="unreadCount" class="pet-unread-dot" data-testid="desktop-pet-unread">{{ unreadCount }}</span>
    </button>
    <div class="pet-avatar-actions">
      <UiButton size="icon" variant="secondary" data-testid="desktop-pet-nudge-left" @click="emit('nudge', -18, 0)">←</UiButton>
      <UiButton size="icon" variant="secondary" data-testid="desktop-pet-nudge-up" @click="emit('nudge', 0, -18)">↑</UiButton>
    </div>
  </div>
</template>

<style scoped>
.pet-avatar-shell {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: end;
  gap: 0.55rem;
}

.pet-mood-badge {
  text-transform: capitalize;
}

.pet-avatar-button {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 6rem;
  height: 6rem;
  border-radius: 1.75rem;
  border: 1px solid color-mix(in srgb, var(--brand-primary) 16%, var(--border-subtle));
  background: color-mix(in srgb, var(--bg-popover) 88%, white 12%);
  box-shadow: var(--shadow-lg);
  backdrop-filter: blur(18px);
  transition: transform var(--duration-fast) var(--ease-apple), box-shadow var(--duration-fast) var(--ease-apple);
}

.pet-avatar-button:hover {
  transform: translateY(-2px) scale(1.01);
  box-shadow: var(--shadow-xl);
}

.pet-avatar-stage {
  width: 4.75rem;
  height: 4.75rem;
  border-radius: 1.25rem;
  overflow: hidden;
}

.pet-avatar-stage.is-hover {
  transform: translateY(-2px);
}

.pet-avatar-stage.is-chat {
  transform: scale(1.04);
}

.pet-rive-canvas,
.pet-avatar-image {
  width: 100%;
  height: 100%;
}

.pet-avatar-image {
  object-fit: cover;
}

.pet-unread-dot {
  position: absolute;
  top: 0.2rem;
  right: 0.2rem;
  min-width: 1.3rem;
  height: 1.3rem;
  padding: 0 0.35rem;
  border-radius: 999px;
  background: var(--brand-primary);
  color: white;
  font-size: 0.72rem;
  font-weight: 700;
  line-height: 1.3rem;
}

.pet-avatar-actions {
  display: flex;
  gap: 0.45rem;
}
</style>
