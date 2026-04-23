<script setup lang="ts">
import type { Component } from 'vue'

import { Check } from 'lucide-vue-next'

import UiKbd from './UiKbd.vue'
import UiPopover from './UiPopover.vue'
import { cn } from '../lib/utils'

export interface UiSelectionMenuItem {
  id: string
  label: string
  shortcut?: string[]
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
      <div v-if="props.title || props.description" class="border-b border-border bg-subtle px-3 py-3">
        <strong v-if="props.title" class="block text-label font-semibold text-text-primary">{{ props.title }}</strong>
        <p v-if="props.description" class="pt-0.5 text-micro text-text-tertiary">{{ props.description }}</p>
      </div>

      <div
        :data-testid="props.listTestId || undefined"
        class="max-h-72 overflow-y-auto p-2"
      >
        <div
          v-for="section in props.sections"
          :key="section.id ?? section.label ?? section.items.map((item) => item.id).join(':')"
          class="mb-2 flex flex-col gap-0.5 last:mb-0"
        >
          <p v-if="section.label" class="px-2 py-1 text-micro font-bold uppercase tracking-wider text-text-tertiary/60">
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
                  'flex w-full items-center justify-between gap-3 rounded-[var(--radius-m)] border border-transparent px-2.5 py-2 text-left transition-colors',
                  item.active
                    ? 'border-border-strong bg-accent text-text-primary font-medium'
                    : 'text-text-secondary hover:border-border hover:bg-subtle hover:text-text-primary',
                  item.disabled ? 'cursor-not-allowed opacity-40' : '',
                )"
                @click="selectItem(item)"
              >
                <span class="flex min-w-0 items-start gap-2.5">
                  <component :is="item.icon" v-if="item.icon" :size="14" class="mt-0.5 shrink-0 opacity-70" />
                  <span class="min-w-0">
                    <strong class="block truncate text-label">{{ item.label }}</strong>
                    <small v-if="item.helper" class="block pt-0.5 text-micro text-text-tertiary">{{ item.helper }}</small>
                  </span>
                </span>

                <span class="flex shrink-0 items-center gap-2">
                  <UiKbd
                    v-if="item.shortcut?.length"
                    :keys="item.shortcut"
                    size="sm"
                    class="shrink-0 border-border bg-surface text-text-secondary"
                  />
                  <span
                    v-if="item.badge"
                    class="rounded border border-border/40 bg-subtle px-1.5 py-0.5 text-micro font-bold uppercase text-text-tertiary"
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
