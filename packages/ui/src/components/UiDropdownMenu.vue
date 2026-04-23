<script setup lang="ts">
import { computed } from 'vue'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'

import UiKbd from './UiKbd.vue'

export interface UiMenuItem {
  key: string
  label: string
  shortcut?: string[]
  disabled?: boolean
  tone?: 'default' | 'danger'
}

const props = withDefaults(defineProps<{
  open?: boolean
  items: UiMenuItem[]
  align?: 'start' | 'end'
}>(), {
  align: 'end',
})

const emit = defineEmits<{
  'update:open': [value: boolean]
  select: [key: string]
}>()

const contentClasses = computed(() => [
  'z-50 min-w-[10rem] rounded-[var(--radius-l)] border border-[color-mix(in_srgb,var(--border)_84%,transparent)] bg-popover p-1.5 shadow-md outline-none',
  props.align === 'end' ? 'origin-top-right' : 'origin-top-left',
].join(' '))

function handleSelect(item: UiMenuItem) {
  if (item.disabled) {
    return
  }

  emit('select', item.key)
}
</script>

<template>
  <div class="relative inline-flex">
    <DropdownMenuRoot
      :open="props.open"
      :modal="false"
      @update:open="emit('update:open', $event)"
    >
      <DropdownMenuTrigger as-child>
        <slot name="trigger" />
      </DropdownMenuTrigger>

      <DropdownMenuPortal>
        <DropdownMenuContent
          data-testid="ui-dropdown-content"
          :align="props.align"
          :side-offset="4"
          :class="contentClasses"
        >
          <DropdownMenuItem
            v-for="item in props.items"
            :key="item.key"
            :data-testid="`ui-dropdown-item-${item.key}`"
            :disabled="item.disabled"
            class="flex cursor-default items-center justify-between gap-3 rounded-[var(--radius-xs)] px-2.5 py-1.5 text-label outline-none transition-colors data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[highlighted]:bg-subtle"
            :class="item.tone === 'danger' ? 'text-destructive data-[highlighted]:bg-destructive/10' : 'text-text-primary'"
            @select="handleSelect(item)"
          >
            <slot name="item" :item="item">
              <span class="min-w-0 truncate">{{ item.label }}</span>
              <UiKbd
                v-if="item.shortcut?.length"
                :keys="item.shortcut"
                size="sm"
                class="shrink-0 border-border bg-subtle text-text-secondary"
              />
            </slot>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenuPortal>
    </DropdownMenuRoot>
  </div>
</template>
