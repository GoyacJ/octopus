<script setup lang="ts">
import { ChevronDown, ChevronRight } from 'lucide-vue-next'

import { cn } from '../lib/utils'

interface UiHierarchyListItem {
  id: string
  label: string
  description?: string
  depth: number
  expandable?: boolean
  expanded?: boolean
  selectable?: boolean
  disabled?: boolean
  testId?: string
  contentTestId?: string
}

const props = withDefaults(defineProps<{
  items: UiHierarchyListItem[]
  selectedId?: string
  class?: string
  rowClass?: string
}>(), {
  selectedId: '',
  class: '',
  rowClass: '',
})

const emit = defineEmits<{
  select: [id: string]
  toggle: [id: string]
}>()

function rowTestId(item: UiHierarchyListItem) {
  return item.testId || `ui-hierarchy-item-${item.id}`
}

function contentTestId(item: UiHierarchyListItem) {
  return item.contentTestId
}

function handleSelect(item: UiHierarchyListItem) {
  if (item.disabled || item.selectable === false) {
    return
  }

  emit('select', item.id)
}

function handleToggle(item: UiHierarchyListItem) {
  if (!item.expandable) {
    return
  }

  emit('toggle', item.id)
}
</script>

<template>
  <ul
    data-testid="ui-hierarchy-list"
    :class="cn('flex min-w-0 flex-col gap-2', props.class)"
  >
    <li
      v-for="item in props.items"
      :key="item.id"
      :data-testid="rowTestId(item)"
      :data-depth="String(item.depth)"
      class="min-w-0"
      @click="handleSelect(item)"
    >
      <div
        :style="{ paddingInlineStart: `${item.depth * 16}px` }"
        class="min-w-0"
      >
        <div
          :class="cn(
            'flex min-w-0 items-start gap-2 rounded-[var(--radius-l)] border border-border bg-surface px-2.5 py-2 shadow-xs transition-colors',
            item.disabled && 'opacity-60',
            props.selectedId && props.selectedId === item.id
              ? 'border-border-strong bg-primary/10'
              : 'hover:border-border-strong hover:bg-subtle',
            props.rowClass,
          )"
        >
          <div class="flex shrink-0 items-center gap-1">
            <slot name="leading" :item="item" />
            <button
              v-if="item.expandable"
              :data-testid="`ui-hierarchy-toggle-${item.id}`"
              type="button"
              class="ui-focus-ring inline-flex size-6 items-center justify-center rounded-[var(--radius-s)] text-text-secondary transition-colors hover:bg-muted/60 hover:text-text-primary"
              @click.stop="handleToggle(item)"
            >
              <ChevronDown v-if="item.expanded" :size="14" />
              <ChevronRight v-else :size="14" />
            </button>
            <div v-else class="size-6 shrink-0" aria-hidden="true" />
          </div>

          <div
            :data-testid="contentTestId(item)"
            :data-depth="contentTestId(item) ? String(item.depth) : undefined"
            :tabindex="item.selectable === false || item.disabled ? undefined : 0"
            :role="item.selectable === false || item.disabled ? undefined : 'button'"
            :aria-selected="props.selectedId === item.id ? 'true' : 'false'"
            class="min-w-0 flex-1"
            @keydown.enter.prevent="handleSelect(item)"
            @keydown.space.prevent="handleSelect(item)"
          >
            <slot :item="item">
              <div class="min-w-0">
                <div class="truncate text-sm font-medium text-text-primary">
                  {{ item.label }}
                </div>
                <div
                  v-if="item.description"
                  class="truncate pt-0.5 text-xs text-text-secondary"
                >
                  {{ item.description }}
                </div>
              </div>
            </slot>

            <div
              v-if="$slots.meta"
              class="pt-1"
            >
              <slot name="meta" :item="item" />
            </div>
          </div>

          <div
            v-if="$slots.badges"
            class="flex shrink-0 flex-wrap items-center justify-end gap-1"
          >
            <slot name="badges" :item="item" />
          </div>
        </div>
      </div>
    </li>
  </ul>
</template>
