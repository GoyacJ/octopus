<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiSurface } from '@octopus/ui'

import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const selectedMenuId = ref<string>('')

const form = reactive({
  label: '',
  order: 0,
  status: 'active' as 'active' | 'disabled',
})

const flattenedMenus = computed(() => {
  const childrenByParent = new Map<string | undefined, typeof workbench.workspaceMenus>()
  for (const menu of workbench.workspaceMenus) {
    const key = menu.parentId
    const list = childrenByParent.get(key) ?? []
    list.push(menu)
    list.sort((left, right) => left.order - right.order)
    childrenByParent.set(key, list)
  }

  const result: Array<(typeof workbench.workspaceMenus[number]) & { depth: number }> = []
  const walk = (parentId: string | undefined, depth: number) => {
    for (const menu of childrenByParent.get(parentId) ?? []) {
      result.push({ ...menu, depth })
      walk(menu.id, depth + 1)
    }
  }

  walk(undefined, 0)
  return result
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

function roleUsage(menuId: string) {
  return workbench.workspaceRoles.filter((role) => role.menuIds.includes(menuId)).length
}
</script>

<template>
  <div class="page-layout">
    <UiSurface :title="t('userCenter.menus.title')" :subtitle="t('userCenter.menus.subtitle')">
      <div class="menu-tree">
        <article
          v-for="menu in flattenedMenus"
          :key="menu.id"
          class="menu-item"
          :class="{ active: selectedMenuId === menu.id }"
          :style="{ '--depth': String(menu.depth) }"
          @click="applyMenu(menu.id)"
        >
          <div class="menu-copy">
            <strong>{{ menu.label }}</strong>
            <small>{{ menu.routeName || t('userCenter.menus.nonNavigable') }}</small>
          </div>
          <div class="menu-meta">
            <UiBadge :label="menu.status" :tone="menu.status === 'active' ? 'success' : 'warning'" />
            <UiBadge :label="t('userCenter.menus.roleUsage', { count: roleUsage(menu.id) })" subtle />
          </div>
        </article>
      </div>
    </UiSurface>

    <UiSurface :title="t('userCenter.menus.editTitle')" :subtitle="t('userCenter.menus.formSubtitle')">
      <div class="form-grid">
        <label class="full-width">
          <span>{{ t('userCenter.menus.labelLabel') }}</span>
          <input v-model="form.label" />
        </label>
        <label>
          <span>{{ t('userCenter.menus.orderLabel') }}</span>
          <input v-model.number="form.order" type="number" />
        </label>
        <label>
          <span>{{ t('userCenter.common.status') }}</span>
          <select v-model="form.status">
            <option value="active">{{ t('userCenter.common.active') }}</option>
            <option value="disabled">{{ t('userCenter.common.disabled') }}</option>
          </select>
        </label>
      </div>

      <div class="form-actions">
        <button type="button" class="ghost-button" @click="selectedMenuId && workbench.toggleMenuStatus(selectedMenuId)">
          {{ t('userCenter.menus.toggleStatus') }}
        </button>
        <button type="button" class="primary-button" @click="saveMenu">
          {{ t('common.save') }}
        </button>
      </div>

      <p class="helper-text">{{ t('userCenter.menus.parentHint') }}</p>
    </UiSurface>
  </div>
</template>

<style scoped>
.page-layout,
.menu-item,
.menu-meta,
.form-actions {
  display: flex;
}

.page-layout {
  flex-direction: column;
  gap: 1rem;
}

.menu-tree {
  display: grid;
  gap: 0.75rem;
}

.menu-item {
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem 0.9rem 0.85rem calc(0.9rem + var(--depth) * 1rem);
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-l);
  cursor: pointer;
}

.menu-item.active {
  border-color: color-mix(in srgb, var(--brand-primary) 34%, var(--border-subtle));
  background: color-mix(in srgb, var(--brand-primary) 6%, transparent);
}

.menu-copy {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.menu-copy small,
.helper-text,
.form-grid label span {
  color: var(--text-secondary);
}

.menu-meta,
.form-actions {
  gap: 0.5rem;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 0.85rem 1rem;
  margin-bottom: 1rem;
}

.form-grid label {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.full-width {
  grid-column: 1 / -1;
}

.form-grid input,
.form-grid select {
  min-height: 2.6rem;
  padding: 0 0.75rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-m);
  background: var(--bg-input);
}

.form-actions {
  justify-content: flex-end;
}

.helper-text {
  margin: 0.75rem 0 0;
}
</style>
