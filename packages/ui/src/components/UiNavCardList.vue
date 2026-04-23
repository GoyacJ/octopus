<script setup lang="ts">
import type { Component } from 'vue'

import { cn } from '../lib/utils'

export interface UiNavCardListItem {
  id: string
  label: string
  helper?: string
  badge?: string
  active?: boolean
  icon?: Component
}

const props = withDefaults(defineProps<{
  items: UiNavCardListItem[]
  class?: string
  density?: 'default' | 'compact' | 'rail'
  testId?: string
}>(), {
  class: '',
  density: 'default',
  testId: '',
})

const emit = defineEmits<{
  select: [id: string]
}>()

defineSlots<{
  item?: (props: { item: UiNavCardListItem, active: boolean, select: () => void, density: 'default' | 'compact' | 'rail' }) => unknown
}>()
</script>

<template>
  <ul
    :data-testid="props.testId || undefined"
    :class="cn(
      'flex flex-col',
      props.density === 'rail' ? 'items-center gap-2' : 'gap-3',
      props.class,
    )"
  >
    <li
      v-for="item in props.items"
      :key="item.id"
      :data-testid="`ui-nav-card-${item.id}`"
      :class="cn(
        'rounded-[var(--radius-l)] border transition-colors duration-fast',
        props.density === 'rail' ? 'w-full max-w-[3.25rem]' : '',
        item.active
          ? 'is-active border-border-strong bg-accent shadow-xs'
          : 'border-border bg-surface hover:border-border-strong hover:bg-subtle',
      )"
    >
      <slot name="item" :item="item" :active="Boolean(item.active)" :select="() => emit('select', item.id)" :density="props.density">
        <button
          type="button"
          :data-testid="`ui-nav-card-action-${item.id}`"
          :class="cn(
            'flex w-full text-left',
            props.density === 'rail'
              ? 'min-h-12 flex-col items-center justify-center gap-1.5 px-2 py-2 text-center'
              : props.density === 'compact'
                ? 'items-center justify-between gap-3 px-3 py-2.5'
                : 'items-center justify-between gap-3 px-4 py-3',
          )"
          @click="emit('select', item.id)"
        >
          <span :class="cn('min-w-0', props.density === 'rail' ? 'flex flex-col items-center text-center' : '')">
            <component :is="item.icon" v-if="item.icon" :size="props.density === 'rail' ? 16 : 15" class="mb-1 shrink-0 text-text-secondary" />
            <strong
              :class="cn(
                'block font-semibold text-text-primary',
                props.density === 'compact' ? 'truncate text-[0.82rem]' : 'truncate text-sm',
                props.density === 'rail' ? 'text-[0.68rem] leading-4' : '',
              )"
            >
              {{ item.label }}
            </strong>
            <small
              v-if="item.helper && props.density !== 'rail'"
              class="block pt-1 text-xs leading-5 text-text-secondary"
            >
              {{ item.helper }}
            </small>
          </span>
          <span
            v-if="item.badge && props.density !== 'rail'"
            class="shrink-0 rounded-full bg-muted px-2 py-1 text-micro font-semibold uppercase tracking-[0.08em] text-text-secondary"
          >
            {{ item.badge }}
          </span>
        </button>
      </slot>
    </li>
  </ul>
</template>
