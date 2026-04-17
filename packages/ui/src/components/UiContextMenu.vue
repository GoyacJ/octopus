<script setup lang="ts">
import {
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuRoot,
  ContextMenuTrigger,
} from 'reka-ui'

import type { UiMenuItem } from './UiDropdownMenu.vue'

const props = defineProps<{
  items: UiMenuItem[]
}>()

const emit = defineEmits<{
  select: [key: string]
}>()

function handleSelect(item: UiMenuItem) {
  if (item.disabled) {
    return
  }

  emit('select', item.key)
}
</script>

<template>
  <ContextMenuRoot :modal="false">
    <ContextMenuTrigger as-child>
      <slot />
    </ContextMenuTrigger>

    <ContextMenuContent
      data-testid="ui-context-content"
      class="z-50 min-w-44 rounded-[var(--radius-l)] border border-[color-mix(in_srgb,var(--border)_84%,transparent)] bg-popover p-1 shadow-md outline-none"
      :side-offset="8"
    >
      <ContextMenuItem
        v-for="item in props.items"
        :key="item.key"
        :data-testid="`ui-context-item-${item.key}`"
        :disabled="item.disabled"
        class="flex cursor-default items-center rounded-[var(--radius-s)] px-3 py-2 text-left text-sm outline-none transition data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[highlighted]:bg-subtle"
        :class="item.tone === 'danger' ? 'text-destructive' : 'text-popover-foreground'"
        @select="handleSelect(item)"
      >
        {{ item.label }}
      </ContextMenuItem>
    </ContextMenuContent>
  </ContextMenuRoot>
</template>
