<script setup lang="ts">
import { cn } from '../lib/utils'

export interface UiRankingListItem {
  id: string
  label: string
  value: string | number
  helper?: string
  ratio?: number | null
}

const props = withDefaults(defineProps<{
  items: UiRankingListItem[]
  ordered?: boolean
  class?: string
}>(), {
  ordered: true,
  class: '',
})

defineSlots<{
  item?: (props: { item: UiRankingListItem, index: number }) => unknown
}>()
</script>

<template>
  <component
    :is="props.ordered ? 'ol' : 'ul'"
    :class="cn('flex flex-col gap-3', props.class)"
  >
    <li
      v-for="(item, index) in props.items"
      :key="item.id"
      class="flex items-start gap-3 rounded-[calc(var(--radius-lg)+2px)] border border-border/80 bg-[color-mix(in_srgb,var(--bg-surface)_92%,var(--bg-subtle))] px-4 py-3 shadow-xs"
    >
      <slot name="item" :item="item" :index="index">
        <div class="mt-1 flex size-6 shrink-0 items-center justify-center rounded-full bg-primary/[0.12] text-xs font-semibold text-primary">
          {{ index + 1 }}
        </div>

        <div class="min-w-0 flex-1">
          <strong class="block truncate text-sm font-semibold text-text-primary">{{ item.label }}</strong>
          <small v-if="item.helper" class="block pt-1 text-xs leading-5 text-text-secondary">{{ item.helper }}</small>
          <div
            v-if="typeof item.ratio === 'number'"
            class="mt-2 h-2 rounded-full bg-muted/80"
          >
            <div
              :data-testid="`ui-ranking-bar-${item.id}`"
              class="h-full rounded-full bg-primary transition-[width] duration-normal ease-apple"
              :style="{ width: `${Math.max(0, Math.min(100, item.ratio * 100))}%` }"
            />
          </div>
        </div>

        <span class="shrink-0 text-sm font-semibold text-text-primary">{{ item.value }}</span>
      </slot>
    </li>
  </component>
</template>
