<script setup lang="ts">
import type { Component } from 'vue'

import { Check } from 'lucide-vue-next'

import UiPopover from './UiPopover.vue'
import { cn } from '../lib/utils'

export interface UiSelectionMenuItem {
  id: string
  label: string
  helper?: string
  badge?: string
  active?: boolean
  disabled?: boolean
  icon?: Component
  testId?: string
}

export interface UiSelectionMenuSection {
  id?: string
  label?: string
  items: UiSelectionMenuItem[]
}

const props = withDefaults(defineProps<{
  open?: boolean
  align?: 'start' | 'center' | 'end'
  side?: 'top' | 'right' | 'bottom' | 'left'
  title?: string
  description?: string
  sections: UiSelectionMenuSection[]
  class?: string
  testId?: string
  listTestId?: string
  closeOnSelect?: boolean
}>(), {
  open: false,
  align: 'start',
  title: '',
  description: '',
  class: '',
  testId: 'ui-selection-menu',
  listTestId: '',
  closeOnSelect: true,
})

const emit = defineEmits<{
  'update:open': [value: boolean]
  select: [id: string]
}>()

defineSlots<{
  trigger?: () => unknown
  item?: (props: { item: UiSelectionMenuItem, select: () => void }) => unknown
}>()

function selectItem(item: UiSelectionMenuItem) {
  if (item.disabled) {
    return
  }

  emit('select', item.id)

  if (props.closeOnSelect) {
    emit('update:open', false)
  }
}
</script>

<template>
  <UiPopover
    :open="props.open"
    :align="props.align"
    :side="props.side"
    :class="cn('w-[min(20rem,calc(100vw-2rem))] p-0', props.class)"
    @update:open="emit('update:open', $event)"
  >
    <template #trigger>
      <slot name="trigger" />
    </template>

    <div :data-testid="props.testId" class="flex flex-col">
      <div v-if="props.title || props.description" class="border-b border-border/40 dark:border-white/[0.08] px-3 py-2.5">
        <strong v-if="props.title" class="block text-[13px] font-bold text-text-primary">{{ props.title }}</strong>
        <p v-if="props.description" class="pt-0.5 text-[11px] leading-relaxed text-text-tertiary">{{ props.description }}</p>
      </div>

      <div
        :data-testid="props.listTestId || undefined"
        class="max-h-72 overflow-y-auto p-1"
      >
        <div
          v-for="section in props.sections"
          :key="section.id ?? section.label ?? section.items.map((item) => item.id).join(':')"
          class="flex flex-col gap-0.5 mb-2 last:mb-0"
        >
          <p v-if="section.label" class="px-2 py-1 text-[10px] font-bold uppercase tracking-wider text-text-tertiary/60">
            {{ section.label }}
          </p>

          <template v-for="item in section.items" :key="item.id">
            <slot
              name="item"
              :item="item"
              :select="() => selectItem(item)"
            >
              <button
                type="button"
                :data-testid="item.testId ?? `ui-selection-item-${item.id}`"
                :disabled="item.disabled"
                :class="cn(
                  'flex w-full items-center justify-between gap-3 rounded px-2 py-1.5 text-left transition-colors',
                  item.active
                    ? 'bg-accent text-text-primary font-medium'
                    : 'text-text-secondary hover:bg-accent hover:text-text-primary',
                  item.disabled ? 'cursor-not-allowed opacity-40' : '',
                )"
                @click="selectItem(item)"
              >
                <span class="flex min-w-0 items-start gap-2.5">
                  <component :is="item.icon" v-if="item.icon" :size="14" class="mt-0.5 shrink-0 opacity-70" />
                  <span class="min-w-0">
                    <strong class="block truncate text-[13px] leading-tight">{{ item.label }}</strong>
                    <small v-if="item.helper" class="block pt-0.5 text-[11px] text-text-tertiary leading-normal">{{ item.helper }}</small>
                  </span>
                </span>

                <span class="flex shrink-0 items-center gap-2">
                  <span
                    v-if="item.badge"
                    class="rounded px-1.5 py-0.5 bg-subtle text-[9px] font-bold uppercase text-text-tertiary border border-border/40 dark:border-white/[0.08]"
                  >
                    {{ item.badge }}
                  </span>
                  <Check v-if="item.active" :size="14" class="text-primary" />
                </span>
              </button>
            </slot>
          </template>
        </div>
      </div>
    </div>
  </UiPopover>
</template>
