<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  PopoverContent,
  PopoverPortal,
  PopoverRoot,
  PopoverTrigger,
} from 'reka-ui'

import { prefersReducedMotion } from '../lib/motion'
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  open?: boolean
  align?: 'start' | 'center' | 'end'
  side?: 'top' | 'right' | 'bottom' | 'left'
  class?: string
  rootClass?: string
  respectReducedMotion?: boolean
  reducedMotion?: boolean
}>(), {
  open: false,
  align: 'start',
  side: 'bottom',
  class: '',
  rootClass: '',
  respectReducedMotion: true,
  reducedMotion: undefined,
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const root = ref<HTMLElement | null>(null)
const reducedMotionActive = computed(() =>
  props.respectReducedMotion !== false && (props.reducedMotion ?? prefersReducedMotion()),
)
const reducedMotionState = computed(() => (reducedMotionActive.value ? 'true' : 'false'))

const contentClasses = computed(() => cn(
  'z-50 min-w-[14rem] rounded-[var(--radius-l)] border border-[color-mix(in_srgb,var(--border)_84%,transparent)] bg-popover p-1.5 shadow-md outline-none',
  props.class,
))
</script>

<template>
  <div ref="root" :class="cn('relative inline-flex', props.rootClass)">
    <PopoverRoot
      :open="props.open"
      :modal="false"
      @update:open="emit('update:open', $event)"
    >
      <PopoverTrigger as-child>
        <slot name="trigger" />
      </PopoverTrigger>
      <PopoverPortal>
        <PopoverContent
          data-testid="ui-popover-content"
          :data-reduced-motion="reducedMotionState"
          :align="props.align"
          :side="props.side"
          :side-offset="4"
          :class="contentClasses"
        >
          <slot />
        </PopoverContent>
      </PopoverPortal>
    </PopoverRoot>
  </div>
</template>
