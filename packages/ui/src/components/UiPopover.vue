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
  align?: 'start' | 'center' | 'end'
  side?: 'top' | 'right' | 'bottom' | 'left'
  class?: string
  rootClass?: string
}>(), {
  open: false,
  align: 'start',
  class: '',
  rootClass: '',
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const root = ref<HTMLElement | null>(null)

const contentClasses = computed(() => cn(
  'z-50 min-w-[14rem] rounded-[var(--radius-l)] border border-border bg-popover p-1.5 shadow-md outline-none',
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
