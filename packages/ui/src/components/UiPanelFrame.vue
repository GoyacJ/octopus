<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'
import UiSurface from './UiSurface.vue'

type PanelFrameVariant = 'hero' | 'panel' | 'raised' | 'subtle' | 'interactive'
type PanelFramePadding = 'none' | 'sm' | 'md' | 'lg'

const props = withDefaults(defineProps<{
  variant?: PanelFrameVariant
  padding?: PanelFramePadding
  eyebrow?: string
  title?: string
  subtitle?: string
  class?: string
  innerClass?: string
}>(), {
  variant: 'panel',
  padding: 'md',
  eyebrow: '',
  title: '',
  subtitle: '',
  class: '',
  innerClass: '',
})

const frameClasses = computed(() => cn(
  'w-full min-w-0 transition-all duration-normal',
  props.class,
))

const surfaceVariant = computed(() => {
  if (props.variant === 'hero') {
    return 'overlay'
  }
  if (props.variant === 'panel' || props.variant === 'raised' || props.variant === 'interactive') {
    return 'raised'
  }
  return 'subtle'
})

</script>

<template>
  <div :class="frameClasses">
    <UiSurface
      :variant="surfaceVariant"
      :padding="padding"
      :eyebrow="eyebrow"
      :title="title"
      :subtitle="subtitle"
      :class="innerClass"
    >
      <template v-if="$slots.actions" #actions>
        <slot name="actions" />
      </template>

      <div class="relative min-w-0">
        <slot />
      </div>
    </UiSurface>
  </div>
</template>
