<script setup lang="ts">
import { computed } from 'vue'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuRoot,
  DropdownMenuTrigger,
} from 'reka-ui'

export interface UiMenuItem {
  key: string
  label: string
  disabled?: boolean
  tone?: 'default' | 'danger'
}

const props = withDefaults(defineProps<{
  open?: boolean
  items: UiMenuItem[]
  align?: 'start' | 'end'
}>(), {
  open: false,
  align: 'end',
})

const emit = defineEmits<{
  'update:open': [value: boolean]
  select: [key: string]
}>()

const contentClasses = computed(() => [
  'z-50 min-w-[10rem] rounded-md border border-border-strong bg-popover p-1 shadow-[0_4px_12px_rgba(15,15,15,0.1)] outline-none',
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

      <DropdownMenuContent
        :align="props.align"
        :side-offset="4"
        :class="contentClasses"
      >
        <DropdownMenuItem
          v-for="item in props.items"
          :key="item.key"
          :data-testid="`ui-dropdown-item-${item.key}`"
          :disabled="item.disabled"
          class="flex cursor-default items-center rounded px-2.5 py-1.5 text-[13px] outline-none transition-colors data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[highlighted]:bg-accent"
          :class="item.tone === 'danger' ? 'text-destructive data-[highlighted]:bg-destructive/10' : 'text-text-primary'"
          @select="handleSelect(item)"
        >
          <slot name="item" :item="item">
            {{ item.label }}
          </slot>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenuRoot>
  </div>
</template>
