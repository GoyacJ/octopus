<script setup lang="ts">
import { cn } from '../lib/utils'

export interface UiTimelineListItem {
  id: string
  title: string
  description?: string
  timestamp?: string
  helper?: string
}

const props = withDefaults(defineProps<{
  items: UiTimelineListItem[]
  class?: string
  density?: 'default' | 'compact'
  testId?: string
}>(), {
  class: '',
  density: 'default',
  testId: '',
})

defineSlots<{
  item?: (props: { item: UiTimelineListItem, index: number, density: 'default' | 'compact' }) => unknown
}>()
</script>

<template>
  <ul
    :data-testid="props.testId || undefined"
    :class="cn('flex flex-col', props.density === 'compact' ? 'gap-2.5' : 'gap-3', props.class)"
  >
    <li
      v-for="(item, index) in props.items"
      :key="item.id"
      data-ui-performance-contained="true"
      :class="cn(
        'flex items-start gap-3 rounded-[var(--radius-l)] border border-border bg-surface shadow-xs [contain:layout_paint_style] [content-visibility:auto]',
        props.density === 'compact' ? 'px-3 py-2.5' : 'px-4 py-3',
        props.density === 'compact' ? '[contain-intrinsic-size:88px]' : '[contain-intrinsic-size:112px]',
      )"
    >
      <slot name="item" :item="item" :index="index" :density="props.density">
        <div :class="cn('shrink-0 rounded-full bg-primary', props.density === 'compact' ? 'mt-1.5 size-2' : 'mt-2 size-2.5')" />
        <div class="min-w-0 flex-1">
          <small v-if="item.helper" class="block pb-1 text-[0.68rem] font-semibold uppercase tracking-[0.08em] text-text-tertiary">{{ item.helper }}</small>
          <strong class="block text-sm font-semibold text-text-primary">{{ item.title }}</strong>
          <small v-if="item.description" class="block pt-1 text-sm leading-6 text-text-secondary">{{ item.description }}</small>
        </div>
        <span v-if="item.timestamp" class="shrink-0 text-xs leading-5 text-text-tertiary">{{ item.timestamp }}</span>
      </slot>
    </li>
  </ul>
</template>
