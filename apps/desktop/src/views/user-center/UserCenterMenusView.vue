<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Power, Save } from 'lucide-vue-next'

import {
  UiBadge,
  UiButton,
  UiField,
  UiInput,
  UiMetricCard,
  UiRecordCard,
  UiSectionHeading,
  UiSelect,
  UiSurface,
} from '@octopus/ui'

import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const selectedMenuId = ref<string>('')

const form = reactive({
  label: '',
  order: 0,
  status: 'active' as 'active' | 'disabled',
})

const flattenedMenus = computed(() => workbench.workspaceMenuTreeItems)
const selectedMenu = computed(() =>
  workbench.workspaceMenus.find((item) => item.id === selectedMenuId.value),
)
const selectedMenuSummary = computed(() =>
  flattenedMenus.value.find((item) => item.id === selectedMenuId.value),
)

const statusOptions = computed(() => [
  { value: 'active', label: t('userCenter.common.active') },
  { value: 'disabled', label: t('userCenter.common.disabled') },
])

const summaryMetrics = computed(() => {
  const disabledCount = flattenedMenus.value.filter((menu) => menu.status === 'disabled').length
  const userCenterCount = flattenedMenus.value.filter((menu) => menu.source === 'user-center').length
  const unusedCount = flattenedMenus.value.filter((menu) => menu.roleUsageCount === 0).length
  return [
    {
      id: 'total',
      label: t('userCenter.menus.metrics.total'),
      value: String(flattenedMenus.value.length),
      helper: t('userCenter.menus.metrics.userCenterHelper', { count: userCenterCount }),
    },
    {
      id: 'disabled',
      label: t('userCenter.menus.metrics.disabled'),
      value: String(disabledCount),
      helper: t('userCenter.menus.metrics.disabledHelper'),
      tone: 'warning' as const,
    },
    {
      id: 'unused',
      label: t('userCenter.menus.metrics.unused'),
      value: String(unusedCount),
      helper: t('userCenter.menus.metrics.unusedHelper'),
      tone: 'accent' as const,
    },
  ]
})

function applyMenu(menuId?: string) {
  if (!menuId) {
    selectedMenuId.value = ''
    form.label = ''
    form.order = 0
    form.status = 'active'
    return
  }

  const menu = workbench.workspaceMenus.find((item) => item.id === menuId)
  if (!menu) {
    applyMenu()
    return
  }

  selectedMenuId.value = menu.id
  form.label = menu.label
  form.order = menu.order
  form.status = menu.status
}

watch(
  () => [workbench.currentWorkspaceId, flattenedMenus.value.map((menu) => menu.id).join('|')],
  () => {
    if (!selectedMenuId.value || !flattenedMenus.value.some((menu) => menu.id === selectedMenuId.value)) {
      applyMenu(flattenedMenus.value[0]?.id)
      return
    }

    applyMenu(selectedMenuId.value)
  },
  { immediate: true },
)

function saveMenu() {
  if (!selectedMenuId.value) {
    return
  }

  workbench.updateMenu(selectedMenuId.value, {
    label: form.label,
    order: form.order,
    status: form.status,
  })
}
</script>

<template>
  <section class="section-stack">
    <div class="grid gap-4 md:grid-cols-3">
      <UiMetricCard
        v-for="metric in summaryMetrics"
        :key="metric.id"
        :label="metric.label"
        :value="metric.value"
        :helper="metric.helper"
        :tone="metric.tone"
      />
    </div>

    <UiSectionHeading
      :eyebrow="t('userCenter.menus.title')"
      :title="t('userCenter.menus.editTitle')"
      :subtitle="t('userCenter.menus.subtitle')"
    />

    <div class="grid gap-4 xl:grid-cols-[minmax(22rem,28rem)_minmax(0,1fr)]">
      <UiSurface :title="t('userCenter.menus.title')" :subtitle="t('userCenter.menus.subtitle')">
        <div data-testid="user-center-menus-tree" class="space-y-3">
          <UiRecordCard
            v-for="menu in flattenedMenus"
            :key="menu.id"
            :test-id="`user-center-menu-record-${menu.id}`"
            :title="menu.label"
            :description="menu.routeName || t('userCenter.menus.nonNavigable')"
            :active="selectedMenuId === menu.id"
            interactive
            @click="applyMenu(menu.id)"
          >
            <template #eyebrow>{{ menu.source }}</template>
            <template #badges>
              <UiBadge :label="menu.status" :tone="menu.status === 'active' ? 'success' : 'warning'" />
            </template>
            <template #meta>
              <span :style="{ paddingLeft: `${menu.depth * 0.8}rem` }">{{ t('userCenter.menus.roleUsage', { count: menu.roleUsageCount }) }}</span>
              <UiBadge v-if="menu.parentLabel" :label="menu.parentLabel" subtle />
            </template>
          </UiRecordCard>
        </div>
      </UiSurface>

      <UiSurface
        data-testid="user-center-menus-editor"
        :title="t('userCenter.menus.editTitle')"
        :subtitle="t('userCenter.menus.formSubtitle')"
      >
        <div v-if="selectedMenuSummary" class="mb-4 flex flex-wrap items-center gap-2">
          <UiBadge :label="t('userCenter.menus.roleUsage', { count: selectedMenuSummary.roleUsageCount })" subtle />
          <UiBadge :label="selectedMenuSummary.parentLabel ?? t('userCenter.menus.rootNode')" subtle />
          <UiBadge :label="selectedMenuSummary.source" subtle />
        </div>

        <UiSurface variant="subtle" padding="sm" class="mb-4" :title="selectedMenu?.label ?? t('common.na')" :subtitle="selectedMenu?.routeName || t('userCenter.menus.nonNavigable')" />

        <div class="grid gap-4 md:grid-cols-2">
          <UiField class="md:col-span-2" :label="t('userCenter.menus.labelLabel')">
            <UiInput v-model="form.label" />
          </UiField>
          <UiField :label="t('userCenter.menus.orderLabel')">
            <UiInput v-model="form.order" type="number" />
          </UiField>
          <UiField :label="t('userCenter.common.status')">
            <UiSelect v-model="form.status" :options="statusOptions" />
          </UiField>
        </div>

        <p class="mt-4 text-sm leading-6 text-text-secondary">{{ t('userCenter.menus.parentHint') }}</p>

        <div class="mt-4 flex flex-wrap justify-end gap-3">
          <UiButton variant="ghost" @click="selectedMenuId && workbench.toggleMenuStatus(selectedMenuId)">
            <Power :size="14" />
            {{ t('userCenter.menus.toggleStatus') }}
          </UiButton>
          <UiButton @click="saveMenu">
            <Save :size="14" />
            {{ t('common.save') }}
          </UiButton>
        </div>
      </UiSurface>
    </div>
  </section>
</template>
