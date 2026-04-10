<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { MenuRecord } from '@octopus/schema'
import {
  UiButton,
  UiField,
  UiInspectorPanel,
  UiInput,
  UiListDetailShell,
  UiPanelFrame,
  UiSelect,
} from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { getMenuDefinition } from '@/navigation/menuRegistry'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { useWorkspaceStore } from '@/stores/workspace'
import PermissionCenterMenuTree from './PermissionCenterMenuTree.vue'
import { buildPermissionCenterMenuTreeSections } from './menu-tree'

const { t, locale } = useI18n()
const workspaceAccessStore = useWorkspaceAccessStore()
const workspaceStore = useWorkspaceStore()

const selectedMenuId = ref('')
const form = reactive({
  label: '',
  source: 'permission-center',
  routeName: '',
  parentId: '',
  status: 'active',
  order: '0',
})

const sourceOptions = computed(() => {
  locale.value
  return [
    { value: 'main-sidebar', label: enumLabel('menuSource', 'main-sidebar') },
    { value: 'console', label: enumLabel('menuSource', 'console') },
    { value: 'permission-center', label: enumLabel('menuSource', 'permission-center') },
  ]
})

const statusOptions = computed(() => {
  locale.value
  return [
    { value: 'active', label: enumLabel('recordStatus', 'active') },
    { value: 'disabled', label: enumLabel('recordStatus', 'disabled') },
  ]
})

const menuTreeSections = computed(() => buildPermissionCenterMenuTreeSections(
  workspaceAccessStore.menus,
  {
    app: t('permissionCenter.roles.menuGroups.app'),
    workspace: t('permissionCenter.roles.menuGroups.workspace'),
    console: t('permissionCenter.roles.menuGroups.console'),
    permissionCenter: t('permissionCenter.roles.menuGroups.permissionCenter'),
    project: t('permissionCenter.roles.menuGroups.project'),
  },
  menu => menuLabel(menu.id, menu.label),
))

watch(
  () => workspaceAccessStore.menus.map(menu => menu.id).join('|'),
  () => {
    if (!selectedMenuId.value || !workspaceAccessStore.menus.some(menu => menu.id === selectedMenuId.value)) {
      applyMenu(workspaceAccessStore.menus[0]?.id)
      return
    }
    applyMenu(selectedMenuId.value)
  },
  { immediate: true },
)

function applyMenu(menuId?: string) {
  const menu = workspaceAccessStore.menus.find(item => item.id === menuId)
  selectedMenuId.value = menu?.id ?? ''
  form.label = menu?.label ?? ''
  form.source = menu?.source ?? 'permission-center'
  form.routeName = menu?.routeName ?? ''
  form.parentId = menu?.parentId ?? ''
  form.status = menu?.status ?? 'active'
  form.order = String(menu?.order ?? 0)
}

async function saveMenu() {
  if (!workspaceStore.currentWorkspaceId || !form.label.trim()) {
    return
  }

  const record: MenuRecord = {
    id: selectedMenuId.value || `menu-${Date.now()}`,
    workspaceId: workspaceStore.currentWorkspaceId,
    label: form.label.trim(),
    source: form.source as MenuRecord['source'],
    routeName: form.routeName.trim() || undefined,
    parentId: form.parentId.trim() || undefined,
    status: form.status as MenuRecord['status'],
    order: Number.parseInt(form.order, 10) || 0,
  }

  if (selectedMenuId.value) {
    await workspaceAccessStore.updateMenu(selectedMenuId.value, record)
  } else {
    const created = await workspaceAccessStore.createMenu(record)
    selectedMenuId.value = created.id
  }
}

function menuLabel(menuId: string, fallback: string) {
  const definition = getMenuDefinition(menuId)
  return definition ? t(definition.labelKey) : fallback
}
</script>

<template>
  <div data-testid="permission-center-menus-shell">
    <UiListDetailShell>
      <template #list>
        <section class="space-y-3">
          <UiPanelFrame variant="subtle" padding="md">
            <div class="text-sm font-semibold text-text-primary">{{ menuLabel(selectedMenuId, form.label || t('common.na')) }}</div>
            <div class="mt-1 text-xs text-text-secondary">{{ t('common.status') }}</div>
          </UiPanelFrame>
          <PermissionCenterMenuTree
            selection-mode="single"
            test-id-prefix="menus-tree"
            :sections="menuTreeSections"
            :active-id="selectedMenuId"
            @select="applyMenu"
          />
        </section>
      </template>

      <UiInspectorPanel :title="menuLabel(selectedMenuId, form.label || t('common.na'))">
        <div class="space-y-4">
          <UiField :label="t('permissionCenter.menus.fields.label')">
            <UiInput v-model="form.label" data-testid="menus-label-input" />
          </UiField>
          <UiField :label="t('permissionCenter.menus.fields.source')">
            <UiSelect v-model="form.source" :options="sourceOptions" />
          </UiField>
          <UiField :label="t('permissionCenter.menus.fields.routeName')">
            <UiInput v-model="form.routeName" />
          </UiField>
          <UiField :label="t('permissionCenter.menus.fields.parentId')">
            <UiInput v-model="form.parentId" />
          </UiField>
          <UiField :label="t('common.status')">
            <UiSelect v-model="form.status" :options="statusOptions" />
          </UiField>
          <UiField :label="t('permissionCenter.menus.fields.order')">
            <UiInput v-model="form.order" />
          </UiField>
          <UiButton @click="saveMenu">{{ t('common.save') }}</UiButton>
        </div>
      </UiInspectorPanel>
    </UiListDetailShell>
  </div>
</template>
