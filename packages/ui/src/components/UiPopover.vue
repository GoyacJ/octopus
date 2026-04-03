<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  PopoverContent,
  PopoverRoot,
  PopoverTrigger,
} from 'reka-ui'

import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  open?: boolean
  align?: 'start' | 'end'
  side?: 'top' | 'right' | 'bottom' | 'left'
  class?: string
}>(), {
  open: false,
  align: 'start',
  class: '',
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const root = ref<HTMLElement | null>(null)

const contentClasses = computed(() => cn(
  'absolute z-40 min-w-[14rem] rounded-md border border-border-strong bg-popover p-1.5 shadow-[0_4px_12px_rgba(15,15,15,0.1)] outline-none',
  props.align === 'end' ? 'right-0' : 'left-0',
  props.class,
))

function handlePointerDown(event: Event) {
  if (!props.open) {
    return
  }

  if (root.value?.contains(event.target as Node)) {
    return
  }

  emit('update:open', false)
}

onMounted(() => {
  window.addEventListener('pointerdown', handlePointerDown)
  window.addEventListener('mousedown', handlePointerDown)
})

onBeforeUnmount(() => {
  window.removeEventListener('pointerdown', handlePointerDown)
  window.removeEventListener('mousedown', handlePointerDown)
})
</script>

<template>
  <div ref="root" class="relative inline-flex">
    <PopoverRoot
      :open="props.open"
      :modal="false"
      @update:open="emit('update:open', $event)"
    >
      <PopoverTrigger as-child>
        <slot name="trigger" />
      </PopoverTrigger>
      <PopoverContent
        :align="props.align"
        :side="props.side"
        :side-offset="4"
        :class="contentClasses"
      >
        <slot />
      </PopoverContent>
    </PopoverRoot>
  </div>
</template>
