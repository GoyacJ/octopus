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
  align?: 'start' | 'end'
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
    :class="cn('w-[min(22rem,calc(100vw-2rem))] p-0', props.class)"
    @update:open="emit('update:open', $event)"
  >
    <template #trigger>
      <slot name="trigger" />
    </template>

    <div :data-testid="props.testId" class="flex flex-col">
      <div v-if="props.title || props.description" class="border-b border-border/60 px-3.5 py-3">
        <strong v-if="props.title" class="block text-sm font-semibold text-text-primary">{{ props.title }}</strong>
        <p v-if="props.description" class="pt-1 text-xs leading-5 text-text-secondary">{{ props.description }}</p>
      </div>

      <div
        :data-testid="props.listTestId || undefined"
        class="max-h-80 overflow-y-auto px-2 py-2"
      >
        <div
          v-for="section in props.sections"
          :key="section.id ?? section.label ?? section.items.map((item) => item.id).join(':')"
          class="flex flex-col gap-1.5"
        >
          <p v-if="section.label" class="px-1.5 pt-1 text-[0.68rem] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
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
                  'flex w-full items-center justify-between gap-3 rounded-[calc(var(--radius-lg)+1px)] px-3 py-2.5 text-left transition-all duration-fast ease-apple',
                  item.active
                    ? 'bg-primary/[0.08] text-text-primary'
                    : 'text-text-secondary hover:bg-accent/70 hover:text-text-primary',
                  item.disabled ? 'cursor-not-allowed opacity-50' : '',
                )"
                @click="selectItem(item)"
              >
                <span class="flex min-w-0 items-start gap-3">
                  <component :is="item.icon" v-if="item.icon" :size="16" class="mt-0.5 shrink-0" />
                  <span class="min-w-0">
                    <strong class="block truncate text-sm font-medium text-current">{{ item.label }}</strong>
                    <small v-if="item.helper" class="block pt-1 text-xs leading-5 text-text-secondary">{{ item.helper }}</small>
                  </span>
                </span>

                <span class="flex shrink-0 items-center gap-2">
                  <span
                    v-if="item.badge"
                    class="rounded-full bg-muted px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.08em] text-text-secondary"
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
