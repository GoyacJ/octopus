<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  PopoverContent,
  PopoverPortal,
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
  'z-50 min-w-[14rem] rounded-md border border-border-subtle dark:border-white/[0.08] bg-background p-1.5 shadow-lg outline-none',
  props.class,
))
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
      <PopoverPortal>
        <PopoverContent
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
