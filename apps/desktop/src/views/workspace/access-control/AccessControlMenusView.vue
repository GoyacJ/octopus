<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiField,
  UiInput,
  UiPanelFrame,
  UiSelect,
  UiStatTile,
  UiStatusCallout,
} from '@octopus/ui'

import type { CreateMenuPolicyRequest, MenuDefinition, MenuPolicyRecord, MenuPolicyUpsertRequest } from '@octopus/schema'

import { getMenuDefinition } from '@/navigation/menuRegistry'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { menuVisibilityOptions, normalizeOrderInput } from './helpers'

const accessControlStore = useWorkspaceAccessControlStore()

const gateMap = computed(() => new Map(accessControlStore.menuGates.map(gate => [gate.menuId, gate])))
const policyMap = computed(() => new Map(accessControlStore.menuPolicies.map(policy => [policy.menuId, policy])))

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
    <section class="grid gap-4 md:grid-cols-3">
      <UiStatTile label="菜单定义" :value="String(accessControlStore.menuDefinitions.length)" />
      <UiStatTile label="可见菜单" :value="String(accessControlStore.currentEffectiveMenuIds.length)" tone="success" />
      <UiStatTile label="菜单策略" :value="String(accessControlStore.menuPolicies.length)" />
    </section>

    <UiStatusCallout
      v-if="submitError"
      tone="error"
      :description="submitError"
    />
    <UiStatusCallout
      v-if="successMessage"
      tone="success"
      :description="successMessage"
    />

    <div class="grid gap-4 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
      <UiPanelFrame variant="panel" padding="md" title="菜单与功能 Gate" subtitle="菜单可见性来自 feature gate 与菜单策略的组合结果。">
        <div v-if="accessControlStore.menuDefinitions.length" class="space-y-3">
          <article
            v-for="menu in accessControlStore.menuDefinitions"
            :key="menu.id"
            class="rounded-[12px] border border-border bg-card p-4"
          >
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h3 class="text-sm font-semibold text-foreground">{{ getMenuDefinition(menu.id)?.defaultLabel ?? menu.label }}</h3>
                <p class="text-xs text-muted-foreground">{{ menu.routeName ?? menu.id }}</p>
              </div>
              <div class="flex flex-wrap gap-2">
                <UiBadge :label="gateMap.get(menu.id)?.allowed ? 'allow' : 'deny'" :tone="gateMap.get(menu.id)?.allowed ? 'success' : 'warning'" subtle />
                <UiBadge :label="menu.source" subtle />
                <UiBadge v-if="policyMap.get(menu.id)" label="已配置策略" subtle />
              </div>
            </div>
            <div class="mt-3 flex justify-end">
              <UiButton
                size="sm"
                variant="ghost"
                data-testid="access-control-menu-select"
                @click="selectMenu(menu)"
              >
                配置策略
              </UiButton>
            </div>
          </article>
        </div>
        <UiEmptyState v-else title="暂无菜单定义" description="菜单目录还未下发。" />
      </UiPanelFrame>

      <UiPanelFrame variant="panel" padding="md" :title="selectedMenu ? '菜单策略' : '选择菜单'" subtitle="系统预置 menu code，管理员只维护策略，不新增菜单目录。">
        <div v-if="selectedMenu" class="space-y-4">
          <div class="rounded-[12px] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ getMenuDefinition(selectedMenu.id)?.defaultLabel ?? selectedMenu.label }}</div>
              <UiBadge :label="selectedMenu.source" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedMenu.featureCode }}</div>
          </div>

          <UiField label="是否启用">
            <UiCheckbox v-model="form.enabled" data-testid="access-control-menu-enabled">启用菜单策略</UiCheckbox>
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

          <div class="flex justify-end gap-2">
            <UiButton variant="ghost" @click="resetForm(selectedMenu)">重置</UiButton>
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
        <UiEmptyState v-else title="请选择菜单" description="从左侧菜单定义中选择一项后即可编辑策略。" />
      </UiPanelFrame>
    </div>
  </div>
</template>
