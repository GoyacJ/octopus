<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPanelFrame,
  UiSelect,
  UiStatusCallout,
  UiToolbarRow,
} from '@octopus/ui'

import type { CreateMenuPolicyRequest, MenuDefinition, MenuPolicyUpsertRequest } from '@octopus/schema'

import { getMenuDefinition } from '@/navigation/menuRegistry'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { menuVisibilityOptions, normalizeOrderInput } from './helpers'

const accessControlStore = useWorkspaceAccessControlStore()

const query = ref('')
const configuredFilter = ref('')
const selectedMenuId = ref('')
const saving = ref(false)
const deleting = ref(false)
const submitError = ref('')
const successMessage = ref('')

const form = reactive({
  enabled: true,
  orderText: '0',
  group: '',
  visibility: 'inherit',
})

const gateMap = computed(() => new Map(accessControlStore.menuGates.map(gate => [gate.menuId, gate])))
const policyMap = computed(() => new Map(accessControlStore.menuPolicies.map(policy => [policy.menuId, policy])))

const configuredFilterOptions = [
  { label: '全部菜单', value: '' },
  { label: '仅已配置', value: 'configured' },
  { label: '仅未配置', value: 'unconfigured' },
]

const filteredMenus = computed(() => {
  const normalizedQuery = query.value.trim().toLowerCase()
  return [...accessControlStore.menuDefinitions]
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

      const definition = getMenuDefinition(menu.id)
      return [
        definition?.defaultLabel ?? menu.label,
        menu.routeName ?? '',
        menu.id,
        menu.featureCode,
        menu.source,
      ].join(' ').toLowerCase().includes(normalizedQuery)
    })
})

const selectedMenu = computed(() =>
  accessControlStore.menuDefinitions.find(menu => menu.id === selectedMenuId.value) ?? null,
)

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
  successMessage.value = ''
  resetForm(menu)
}

async function handleSave() {
  submitError.value = ''
  if (!selectedMenu.value) {
    submitError.value = '请先选择一个菜单。'
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
    successMessage.value = '已配置策略'
    resetForm(selectedMenu.value)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存菜单策略失败。'
  } finally {
    saving.value = false
  }
}

async function handleDelete() {
  submitError.value = ''
  if (!selectedMenu.value) {
    submitError.value = '请先选择一个菜单。'
    return
  }

  deleting.value = true
  try {
    await accessControlStore.deleteMenuPolicy(selectedMenu.value.id)
    successMessage.value = '已删除菜单策略'
    resetForm(selectedMenu.value)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除菜单策略失败。'
  } finally {
    deleting.value = false
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-menus-shell">
    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />
    <UiStatusCallout v-if="successMessage" tone="success" :description="successMessage" />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedMenu)"
      :detail-title="selectedMenu ? '菜单策略' : ''"
      detail-subtitle="系统预置 menu code，管理员只维护策略，不新增菜单目录。"
      empty-detail-title="请选择菜单"
      empty-detail-description="从左侧菜单列表中选择一项后即可查看或编辑策略。"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-menus-toolbar">
          <template #search>
            <UiInput v-model="query" placeholder="搜索菜单名称、路由或 feature code" />
          </template>
          <template #filters>
            <UiField label="配置状态" class="w-full md:w-[180px]">
              <UiSelect v-model="configuredFilter" :options="configuredFilterOptions" />
            </UiField>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="菜单列表" :subtitle="`共 ${filteredMenus.length} 项菜单`">
          <div v-if="filteredMenus.length" class="space-y-2">
            <button
              v-for="menu in filteredMenus"
              :key="menu.id"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedMenuId === menu.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              data-testid="access-control-menu-select"
              @click="selectMenu(menu)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 space-y-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm font-semibold text-foreground">{{ getMenuDefinition(menu.id)?.defaultLabel ?? menu.label }}</span>
                    <UiBadge :label="gateMap.get(menu.id)?.allowed ? 'allow' : 'deny'" :tone="gateMap.get(menu.id)?.allowed ? 'success' : 'warning'" subtle />
                  </div>
                  <p class="text-xs text-muted-foreground">{{ menu.routeName ?? menu.id }}</p>
                </div>
                <div class="flex flex-wrap gap-2">
                  <UiBadge :label="menu.source" subtle />
                  <UiBadge v-if="policyMap.get(menu.id)" label="已配置策略" subtle />
                </div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无菜单定义" description="当前筛选条件下没有可显示的菜单。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedMenu" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ getMenuDefinition(selectedMenu.id)?.defaultLabel ?? selectedMenu.label }}</div>
              <UiBadge :label="selectedMenu.source" subtle />
              <UiBadge :label="gateMap.get(selectedMenu.id)?.allowed ? 'Feature gate allow' : 'Feature gate deny'" :tone="gateMap.get(selectedMenu.id)?.allowed ? 'success' : 'warning'" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedMenu.featureCode }}</div>
          </div>

          <UiField label="是否启用">
            <UiCheckbox v-model="form.enabled">启用菜单策略</UiCheckbox>
          </UiField>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="排序">
              <UiInput v-model="form.orderText" data-testid="access-control-menu-order" />
            </UiField>
            <UiField label="分组">
              <UiInput v-model="form.group" data-testid="access-control-menu-group" />
            </UiField>
          </div>

          <UiField label="可见性">
            <UiSelect v-model="form.visibility" :options="menuVisibilityOptions" data-testid="access-control-menu-visibility" />
          </UiField>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton variant="ghost" @click="resetForm(selectedMenu)">
              重置
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
                删除策略
              </UiButton>
              <UiButton
                :loading="saving"
                data-testid="access-control-menu-save-policy"
                @click="handleSave"
              >
                {{ policyMap.get(selectedMenu.id) ? '保存策略' : '创建策略' }}
              </UiButton>
            </div>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>
  </div>
</template>
