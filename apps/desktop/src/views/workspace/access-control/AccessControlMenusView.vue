<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiField,
  UiHierarchyList,
  UiInput,
  UiListDetailWorkspace,
  UiPagination,
  UiPanelFrame,
  UiSelect,
  UiStatusCallout,
  UiToolbarRow,
} from '@octopus/ui'

import type { CreateMenuPolicyRequest, MenuDefinition, MenuPolicyUpsertRequest } from '@octopus/schema'

import { usePagination } from '@/composables/usePagination'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { getAccessMenuLabel } from './display-i18n'
import { createMenuVisibilityOptions, getMenuSourceLabel, normalizeOrderInput } from './helpers'
import { useAccessControlNotifications } from './useAccessControlNotifications'

const { t } = useI18n()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifySuccess } = useAccessControlNotifications('access-control.menus')

const query = ref('')
const configuredFilter = ref('')
const selectedMenuId = ref('')
const saving = ref(false)
const deleting = ref(false)
const submitError = ref('')
const expandedMenuIds = ref<string[]>([])

const form = reactive({
  enabled: true,
  orderText: '0',
  group: '',
  visibility: 'inherit',
})

const gateMap = computed(() => new Map(accessControlStore.menuGates.map(gate => [gate.menuId, gate])))
const policyMap = computed(() => new Map(accessControlStore.menuPolicies.map(policy => [policy.menuId, policy])))

const menuVisibilityOptions = computed(() => createMenuVisibilityOptions(t))
const configuredFilterOptions = computed(() => [
  { label: t('accessControl.common.filters.allMenus'), value: '' },
  { label: t('accessControl.common.filters.configuredOnly'), value: 'configured' },
  { label: t('accessControl.common.filters.unconfiguredOnly'), value: 'unconfigured' },
])

const filteredMenus = computed(() => {
  const normalizedQuery = query.value.trim().toLowerCase()
  const configuredMenus = [...accessControlStore.menuDefinitions]
    .sort((left, right) => left.order - right.order)
    .filter((menu) => {
      const matchesConfigured = configuredFilter.value === ''
        || (configuredFilter.value === 'configured' && policyMap.value.has(menu.id))
        || (configuredFilter.value === 'unconfigured' && !policyMap.value.has(menu.id))

      if (!matchesConfigured) {
        return false
      }

      if (!normalizedQuery) {
        return true
      }

      return true
    })

  if (!normalizedQuery) {
    return configuredMenus
  }

  const configuredMenuMap = new Map(configuredMenus.map(menu => [menu.id, menu]))
  const matchedIds = new Set(
    configuredMenus
      .filter((menu) => {
        return [
          getAccessMenuLabel(menu),
          menu.routeName ?? '',
          menu.id,
          menu.featureCode,
          menu.source,
        ].join(' ').toLowerCase().includes(normalizedQuery)
      })
      .map(menu => menu.id),
  )

  const visibleIds = new Set<string>()
  for (const menuId of matchedIds) {
    let current = configuredMenuMap.get(menuId)
    while (current) {
      visibleIds.add(current.id)
      current = current.parentId ? configuredMenuMap.get(current.parentId) : undefined
    }
  }

  return configuredMenus.filter(menu => visibleIds.has(menu.id))
})

interface MenuHierarchyItem {
  id: string
  label: string
  description?: string
  depth: number
  expandable?: boolean
  expanded?: boolean
  selectable?: boolean
  testId: string
  contentTestId?: string
  menu: MenuDefinition
}

const filteredMenuIdSet = computed(() => new Set(filteredMenus.value.map(menu => menu.id)))
const filteredMenuMap = computed(() => new Map(filteredMenus.value.map(menu => [menu.id, menu])))
const filteredMenuChildrenMap = computed(() => {
  const grouped = new Map<string, MenuDefinition[]>()
  for (const menu of filteredMenus.value) {
    const parentId = menu.parentId && filteredMenuIdSet.value.has(menu.parentId) ? menu.parentId : ''
    const items = grouped.get(parentId) ?? []
    items.push(menu)
    grouped.set(parentId, items)
  }
  for (const [key, items] of grouped) {
    grouped.set(key, [...items].sort((left, right) => left.order - right.order))
  }
  return grouped
})
const menuRootIds = computed(() =>
  (filteredMenuChildrenMap.value.get('') ?? []).map(menu => menu.id),
)
const visibleMenuTreeItems = computed<MenuHierarchyItem[]>(() => {
  const items: MenuHierarchyItem[] = []
  const normalizedQuery = query.value.trim().toLowerCase()

  for (const rootId of pagination.pagedItems.value) {
    appendMenuItems(items, rootId, 0, normalizedQuery)
  }

  return items
})

const selectedMenu = computed(() =>
  accessControlStore.menuDefinitions.find(menu => menu.id === selectedMenuId.value) ?? null,
)

const pagination = usePagination(menuRootIds, {
  pageSize: 8,
  resetOn: [query, configuredFilter],
})

watch(visibleMenuTreeItems, (items) => {
  if (selectedMenuId.value && !items.some(item => item.id === selectedMenuId.value)) {
    selectedMenuId.value = ''
  }
}, { immediate: true })

watch(filteredMenuChildrenMap, (childrenMap) => {
  const parentIds = Array.from(childrenMap.entries())
    .filter(([, items]) => items.length > 0)
    .map(([parentId]) => parentId)
    .filter(Boolean)
  const knownIds = new Set(filteredMenus.value.map(menu => menu.id))
  const next = expandedMenuIds.value.filter(id => knownIds.has(id))
  if (!expandedMenuIds.value.length) {
    expandedMenuIds.value = [...parentIds]
    return
  }
  expandedMenuIds.value = next
}, { immediate: true })

function resetForm(menu?: MenuDefinition | null) {
  const record = menu ? policyMap.value.get(menu.id) : undefined
  Object.assign(form, {
    enabled: record?.enabled ?? true,
    orderText: String(record?.order ?? menu?.order ?? 0),
    group: record?.group ?? '',
    visibility: record?.visibility ?? 'inherit',
  })
  submitError.value = ''
}

function selectMenu(menu: MenuDefinition) {
  selectedMenuId.value = menu.id
  resetForm(menu)
}

function selectMenuById(menuId: string) {
  const menu = filteredMenuMap.value.get(menuId)
  if (!menu) {
    return
  }
  selectMenu(menu)
}

function toggleMenuExpanded(menuId: string) {
  const next = new Set(expandedMenuIds.value)
  if (next.has(menuId)) {
    next.delete(menuId)
  } else {
    next.add(menuId)
  }
  expandedMenuIds.value = Array.from(next)
}

function menuLabel(menu: MenuDefinition) {
  return getAccessMenuLabel(menu)
}

function menuDescription(menu: MenuDefinition) {
  return menu.routeName ?? menu.id
}

function menuSourceLabel(menuId: string) {
  const menu = filteredMenuMap.value.get(menuId)
  return menu ? getMenuSourceLabel(t, menu.source) : ''
}

function appendMenuItems(
  items: MenuHierarchyItem[],
  menuId: string,
  depth: number,
  normalizedQuery: string,
) {
  const menu = filteredMenuMap.value.get(menuId)
  if (!menu) {
    return
  }

  const children = filteredMenuChildrenMap.value.get(menu.id) ?? []
  const expanded = normalizedQuery
    ? true
    : expandedMenuIds.value.includes(menu.id)
  items.push({
    id: menu.id,
    label: menuLabel(menu),
    description: menuDescription(menu),
    depth,
    expandable: children.length > 0,
    expanded,
    selectable: true,
    testId: 'access-control-menu-select',
    contentTestId: `access-control-menu-node-${menu.id}`,
    menu,
  })

  if (!children.length || !expanded) {
    return
  }

  for (const child of children) {
    appendMenuItems(items, child.id, depth + 1, normalizedQuery)
  }
}

async function handleSave() {
  submitError.value = ''
  if (!selectedMenu.value) {
    submitError.value = t('accessControl.menus.feedback.selectRequired')
    return
  }

  saving.value = true
  try {
    const existing = policyMap.value.get(selectedMenu.value.id)
    const payload: MenuPolicyUpsertRequest = {
      enabled: form.enabled,
      order: normalizeOrderInput(form.orderText, selectedMenu.value.order),
      group: form.group.trim() || undefined,
      visibility: form.visibility,
    }
    if (existing) {
      await accessControlStore.updateMenuPolicy(selectedMenu.value.id, payload)
    } else {
      const createPayload: CreateMenuPolicyRequest = {
        menuId: selectedMenu.value.id,
        ...payload,
      }
      await accessControlStore.createMenuPolicy(createPayload)
    }
    resetForm(selectedMenu.value)
    await notifySuccess(
      t('accessControl.menus.feedback.toastSaved'),
      getAccessMenuLabel(selectedMenu.value),
    )
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.menus.feedback.saveFailed')
  } finally {
    saving.value = false
  }
}

async function handleDelete() {
  submitError.value = ''
  if (!selectedMenu.value) {
    submitError.value = t('accessControl.menus.feedback.selectRequired')
    return
  }

  deleting.value = true
  try {
    const label = getAccessMenuLabel(selectedMenu.value)
    await accessControlStore.deleteMenuPolicy(selectedMenu.value.id)
    resetForm(selectedMenu.value)
    await notifySuccess(t('accessControl.menus.feedback.toastDeleted'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.menus.feedback.deleteFailed')
  } finally {
    deleting.value = false
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-menus-shell">
    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedMenu)"
      :detail-title="selectedMenu ? t('accessControl.menus.detail.title') : ''"
      :detail-subtitle="t('accessControl.menus.detail.subtitle')"
      :empty-detail-title="t('accessControl.menus.detail.emptyTitle')"
      :empty-detail-description="t('accessControl.menus.detail.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-menus-toolbar">
          <template #search>
            <UiInput v-model="query" :placeholder="t('accessControl.menus.toolbar.search')" />
          </template>
          <template #filters>
            <UiField :label="t('accessControl.menus.toolbar.configured')" class="w-full md:w-[180px]">
              <UiSelect v-model="configuredFilter" :options="configuredFilterOptions" />
            </UiField>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.menus.list.title')"
          :subtitle="t('accessControl.common.list.totalMenus', { count: pagination.totalItems.value })"
        >
          <UiHierarchyList
            v-if="visibleMenuTreeItems.length"
            :items="visibleMenuTreeItems"
            :selected-id="selectedMenuId"
            class="space-y-2"
            @select="selectMenuById"
            @toggle="toggleMenuExpanded"
          >
            <template #default="{ item }">
              <div class="min-w-0">
                <div class="truncate text-sm font-medium text-text-primary">
                  {{ item.label }}
                </div>
                <div class="truncate pt-0.5 text-xs text-text-secondary">
                  {{ item.description }}
                </div>
              </div>
            </template>

            <template #badges="{ item }">
              <UiBadge
                :label="gateMap.get(item.id)?.allowed ? t('accessControl.menus.badges.gateAllow') : t('accessControl.menus.badges.gateDeny')"
                :tone="gateMap.get(item.id)?.allowed ? 'success' : 'warning'"
                subtle
              />
              <UiBadge :label="menuSourceLabel(item.id)" subtle />
              <UiBadge v-if="policyMap.get(item.id)" :label="t('accessControl.common.list.configured')" subtle />
            </template>
          </UiHierarchyList>
          <UiEmptyState
            v-else
            :title="t('accessControl.menus.list.emptyTitle')"
            :description="t('accessControl.menus.list.emptyDescription')"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="pagination.currentPage.value"
              :page-count="pagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: pagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedMenu" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ getAccessMenuLabel(selectedMenu) }}</div>
              <UiBadge :label="getMenuSourceLabel(t, selectedMenu.source)" subtle />
              <UiBadge :label="gateMap.get(selectedMenu.id)?.allowed ? t('accessControl.menus.badges.gateAllow') : t('accessControl.menus.badges.gateDeny')" :tone="gateMap.get(selectedMenu.id)?.allowed ? 'success' : 'warning'" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedMenu.featureCode }}</div>
          </div>

          <UiField :label="t('accessControl.menus.fields.enabled')">
            <UiCheckbox v-model="form.enabled">
              {{ t('accessControl.menus.fields.enabledCheckbox') }}
            </UiCheckbox>
          </UiField>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.menus.fields.order')">
              <UiInput v-model="form.orderText" data-testid="access-control-menu-order" />
            </UiField>
            <UiField :label="t('accessControl.menus.fields.group')">
              <UiInput v-model="form.group" data-testid="access-control-menu-group" />
            </UiField>
          </div>

          <UiField :label="t('accessControl.menus.fields.visibility')">
            <UiSelect v-model="form.visibility" :options="menuVisibilityOptions" data-testid="access-control-menu-visibility" />
          </UiField>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton variant="ghost" @click="resetForm(selectedMenu)">
              {{ t('common.reset') }}
            </UiButton>
            <div class="flex flex-wrap gap-2">
              <UiButton
                v-if="policyMap.get(selectedMenu.id)"
                variant="ghost"
                class="text-destructive"
                :loading="deleting"
                data-testid="access-control-menu-delete-policy"
                @click="handleDelete"
              >
                {{ t('accessControl.menus.actions.delete') }}
              </UiButton>
              <UiButton
                :loading="saving"
                data-testid="access-control-menu-save-policy"
                @click="handleSave"
              >
                {{ policyMap.get(selectedMenu.id) ? t('accessControl.menus.actions.save') : t('accessControl.menus.actions.create') }}
              </UiButton>
            </div>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>
  </div>
</template>
