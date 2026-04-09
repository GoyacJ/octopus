<script setup lang="ts">
import type { CatalogConfiguredModelRow, CatalogFilterOption } from '@/stores/catalog'
import { UiDataTable, UiInput, UiPagination, UiSelect, UiSurface } from '@octopus/ui'

defineProps<{
  pagedRows: CatalogConfiguredModelRow[]
  columns: unknown[]
  searchQuery: string
  providerFilter: string
  surfaceFilter: string
  capabilityFilter: string
  localFilterOptions: {
    providers: CatalogFilterOption[]
    surfaces: CatalogFilterOption[]
    capabilities: CatalogFilterOption[]
  }
  filteredRowsLength: number
  page: number
  pageCount: number
  t: (key: string, params?: Record<string, unknown>) => string
}>()

const emit = defineEmits<{
  'update:searchQuery': [value: string]
  'update:providerFilter': [value: string]
  'update:surfaceFilter': [value: string]
  'update:capabilityFilter': [value: string]
  'update:page': [value: number]
  selectRow: [row: CatalogConfiguredModelRow]
}>()
</script>

<template>
  <section>
    <UiSurface variant="raised" padding="md">
      <UiDataTable
        :data="pagedRows"
        :columns="columns as any"
        row-test-id="models-table-row"
        :empty-title="t('models.empty.title')"
        :empty-description="t('models.empty.description')"
        :on-row-click="(row) => emit('selectRow', row as CatalogConfiguredModelRow)"
      >
        <template #toolbar>
          <div
            data-testid="models-filters"
            class="grid min-w-0 w-full gap-3 pb-3 xl:grid-cols-[minmax(0,1fr)_auto] xl:items-center"
          >
            <div class="flex min-w-0 flex-wrap items-center gap-3 xl:flex-nowrap">
              <UiInput
                :model-value="searchQuery"
                data-testid="models-search-input"
                class="min-w-[260px] flex-[1.35_1_320px]"
                :placeholder="t('models.filters.searchPlaceholder')"
                @update:model-value="emit('update:searchQuery', String($event))"
              />
              <UiSelect
                :model-value="providerFilter"
                data-testid="models-provider-filter"
                class="min-w-[150px] flex-[0_0_180px]"
                :options="[{ value: '', label: t('models.filters.allProviders') }, ...localFilterOptions.providers]"
                @update:model-value="emit('update:providerFilter', String($event))"
              />
              <UiSelect
                :model-value="surfaceFilter"
                data-testid="models-surface-filter"
                class="min-w-[150px] flex-[0_0_180px]"
                :options="[{ value: '', label: t('models.filters.allSurfaces') }, ...localFilterOptions.surfaces]"
                @update:model-value="emit('update:surfaceFilter', String($event))"
              />
              <UiSelect
                :model-value="capabilityFilter"
                data-testid="models-capability-filter"
                class="min-w-[150px] flex-[0_0_180px]"
                :options="[{ value: '', label: t('models.filters.allCapabilities') }, ...localFilterOptions.capabilities]"
                @update:model-value="emit('update:capabilityFilter', String($event))"
              />
            </div>
            <div class="flex justify-end text-[12px] text-text-tertiary">
              {{ t('models.pagination.summary', { count: filteredRowsLength, page, total: pageCount }) }}
            </div>
          </div>
        </template>
      </UiDataTable>

      <div class="mt-4">
        <UiPagination
          :page="page"
          data-testid="models-pagination"
          root-test-id="models-pagination"
          :page-count="pageCount"
          :summary-label="t('models.pagination.summary', { count: filteredRowsLength, page, total: pageCount })"
          @update:page="emit('update:page', $event)"
        />
      </div>
    </UiSurface>
  </section>
</template>
