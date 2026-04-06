<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { MenuRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiField, UiInput, UiRecordCard, UiSelect } from '@octopus/ui'

import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const userCenterStore = useUserCenterStore()
const workspaceStore = useWorkspaceStore()

const selectedMenuId = ref('')
const form = reactive({
  label: '',
  source: 'user-center',
  routeName: '',
  parentId: '',
  status: 'active',
  order: '0',
})

const sourceOptions = [
  { value: 'main-sidebar', label: 'main-sidebar' },
  { value: 'user-center', label: 'user-center' },
]

const statusOptions = [
  { value: 'active', label: 'active' },
  { value: 'disabled', label: 'disabled' },
]

watch(
  () => userCenterStore.menus.map(menu => menu.id).join('|'),
  () => {
    if (!selectedMenuId.value || !userCenterStore.menus.some(menu => menu.id === selectedMenuId.value)) {
      applyMenu(userCenterStore.menus[0]?.id)
      return
    }
    applyMenu(selectedMenuId.value)
  },
  { immediate: true },
)

function applyMenu(menuId?: string) {
  const menu = userCenterStore.menus.find(item => item.id === menuId)
  selectedMenuId.value = menu?.id ?? ''
  form.label = menu?.label ?? ''
  form.source = menu?.source ?? 'user-center'
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
    await userCenterStore.updateMenu(selectedMenuId.value, record)
  } else {
    const created = await userCenterStore.createMenu(record)
    selectedMenuId.value = created.id
  }
}
</script>

<template>
  <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
    <section class="space-y-3">
      <UiRecordCard
        v-for="menu in userCenterStore.menus"
        :key="menu.id"
        :title="menu.label"
        :description="menu.routeName || menu.id"
        interactive
        class="cursor-pointer"
        :class="selectedMenuId === menu.id ? 'ring-1 ring-primary' : ''"
        @click="applyMenu(menu.id)"
      >
        <template #badges>
          <UiBadge :label="menu.source" subtle />
          <UiBadge :label="menu.status" subtle />
        </template>
      </UiRecordCard>
    </section>

    <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
      <UiField :label="t('userCenter.menus.fields.label')">
        <UiInput v-model="form.label" />
      </UiField>
      <UiField :label="t('userCenter.menus.fields.source')">
        <UiSelect v-model="form.source" :options="sourceOptions" />
      </UiField>
      <UiField :label="t('userCenter.menus.fields.routeName')">
        <UiInput v-model="form.routeName" />
      </UiField>
      <UiField :label="t('userCenter.menus.fields.parentId')">
        <UiInput v-model="form.parentId" />
      </UiField>
      <UiField :label="t('common.status')">
        <UiSelect v-model="form.status" :options="statusOptions" />
      </UiField>
      <UiField :label="t('userCenter.menus.fields.order')">
        <UiInput v-model="form.order" />
      </UiField>
      <UiButton @click="saveMenu">{{ t('common.save') }}</UiButton>
    </section>
  </div>
</template>
