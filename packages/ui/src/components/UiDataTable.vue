<script setup lang="ts" generic="TData extends object">
import {
  FlexRender,
  getCoreRowModel,
  useVueTable,
  type ColumnDef,
} from '@tanstack/vue-table'
import { computed } from 'vue'
import { cn } from '../lib/utils'

import UiEmptyState from './UiEmptyState.vue'

const props = defineProps<{
  data: TData[]
  columns: ColumnDef<TData, unknown>[]
  rowTestId?: string
  emptyTitle?: string
  emptyDescription?: string
  onRowClick?: (row: TData) => void
  class?: string
}>()

const data = computed(() => props.data)
const columns = computed(() => props.columns)

const table = useVueTable({
  get data() {
    return data.value
  },
  get columns() {
    return columns.value
  },
  getCoreRowModel: getCoreRowModel(),
})

function resolveCellRender(cell: {
  column: {
    columnDef: ColumnDef<TData, unknown>
  }
  getValue: () => unknown
}) {
  const definition = cell.column.columnDef as ColumnDef<TData, unknown> & {
    cell?: unknown
  }

  return definition.cell ?? String(cell.getValue() ?? '')
}
</script>

<template>
  <div :class="cn('flex flex-col gap-3 min-w-0', props.class)">
    <div v-if="$slots.toolbar" class="flex flex-wrap items-center justify-between gap-3 min-w-0">
      <slot name="toolbar" />
    </div>

    <div v-if="table.getRowModel().rows.length" class="overflow-x-auto">
      <table class="w-full min-w-0 border-collapse text-left">
        <thead class="bg-subtle/12 text-text-tertiary">
        <tr
          v-for="headerGroup in table.getHeaderGroups()"
          :key="headerGroup.id"
          class="border-b border-[color-mix(in_srgb,var(--border)_42%,transparent)]"
        >
          <th
            v-for="header in headerGroup.headers"
            :key="header.id"
            class="px-3 py-2 text-[12px] font-medium whitespace-nowrap"
          >
            <FlexRender
              v-if="!header.isPlaceholder"
              :render="header.column.columnDef.header"
              :props="header.getContext()"
            />
          </th>
        </tr>
        </thead>
        <tbody class="text-text-primary text-[13px]">
        <tr
          v-for="row in table.getRowModel().rows"
          :key="row.id"
          :class="cn(
            'group border-b border-[color-mix(in_srgb,var(--border)_28%,transparent)] last:border-0 hover:bg-accent/40 transition-colors',
            props.onRowClick && 'cursor-pointer',
          )"
          :data-testid="props.rowTestId ? `${props.rowTestId}-${row.id}` : undefined"
          @click="props.onRowClick?.(row.original)"
        >
          <td
            v-for="cell in row.getVisibleCells()"
            :key="cell.id"
            class="px-3 py-2.5 vertical-top"
          >
            <FlexRender
              :render="resolveCellRender(cell)"
              :props="cell.getContext()"
            />
          </td>
        </tr>
        </tbody>
      </table>
    </div>

    <UiEmptyState
      v-else-if="props.emptyTitle || props.emptyDescription"
      :title="props.emptyTitle ?? ''"
      :description="props.emptyDescription ?? ''"
    />
  </div>
</template>
