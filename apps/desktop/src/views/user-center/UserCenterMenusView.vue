<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { FolderTree, Power, Route, Save } from 'lucide-vue-next'

import { UiBadge, UiButton, UiField, UiInput, UiSectionHeading, UiSelect, UiSurface } from '@octopus/ui'

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

const selectedMenu = computed(() =>
  workbench.workspaceMenus.find((item) => item.id === selectedMenuId.value),
)

const statusOptions = computed(() => [
  { value: 'active', label: t('userCenter.common.active') },
  { value: 'disabled', label: t('userCenter.common.disabled') },
])

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
  <section class="menu-page section-stack">
    <UiSectionHeading
      :eyebrow="t('userCenter.menus.title')"
      :title="t('userCenter.menus.editTitle')"
      :subtitle="t('userCenter.menus.subtitle')"
    />

    <div class="menu-layout">
      <UiSurface class="menu-tree-surface" :title="t('userCenter.menus.title')" :subtitle="t('userCenter.menus.subtitle')">
        <div class="menu-tree">
          <article
            v-for="menu in flattenedMenus"
            :key="menu.id"
            class="menu-card"
            :class="{ active: selectedMenuId === menu.id }"
            :style="{ '--depth': String(menu.depth) }"
            @click="applyMenu(menu.id)"
          >
            <div class="menu-card-main">
              <span class="menu-indent" aria-hidden="true" />
              <div class="menu-icon-wrap">
                <FolderTree :size="16" />
              </div>
              <div class="menu-copy">
                <strong>{{ menu.label }}</strong>
                <small>{{ menu.routeName || t('userCenter.menus.nonNavigable') }}</small>
              </div>
            </div>
            <div class="menu-meta">
              <UiBadge :label="menu.status" :tone="menu.status === 'active' ? 'success' : 'warning'" />
              <UiBadge :label="t('userCenter.menus.roleUsage', { count: roleUsage(menu.id) })" subtle />
            </div>
          </article>
        </div>
      </UiSurface>

      <UiSurface class="menu-editor-surface" :title="t('userCenter.menus.editTitle')" :subtitle="t('userCenter.menus.formSubtitle')">
        <div class="menu-editor-shell">
          <div class="menu-preview-card">
            <div class="menu-preview-header">
              <div class="menu-preview-icon">
                <Route :size="18" />
              </div>
              <div>
                <strong>{{ selectedMenu?.label ?? t('common.na') }}</strong>
                <p>{{ selectedMenu?.routeName || t('userCenter.menus.nonNavigable') }}</p>
              </div>
            </div>
            <div class="menu-preview-badges">
              <UiBadge v-if="selectedMenu" :label="selectedMenu.source" subtle />
              <UiBadge v-if="selectedMenu" :label="t('userCenter.menus.roleUsage', { count: roleUsage(selectedMenu.id) })" subtle />
            </div>
          </div>

          <div class="menu-form-grid">
            <UiField class="menu-form-full" :label="t('userCenter.menus.labelLabel')">
              <UiInput v-model="form.label" />
            </UiField>
            <UiField :label="t('userCenter.menus.orderLabel')">
              <UiInput v-model="form.order" type="number" />
            </UiField>
            <UiField :label="t('userCenter.common.status')">
              <UiSelect v-model="form.status" :options="statusOptions" />
            </UiField>
          </div>

          <div class="menu-editor-actions">
            <UiButton variant="ghost" @click="selectedMenuId && workbench.toggleMenuStatus(selectedMenuId)">
              <Power :size="14" />
              {{ t('userCenter.menus.toggleStatus') }}
            </UiButton>
            <UiButton @click="saveMenu">
              <Save :size="14" />
              {{ t('common.save') }}
            </UiButton>
          </div>

          <p class="menu-helper-text">{{ t('userCenter.menus.parentHint') }}</p>
        </div>
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.menu-page,
.menu-editor-shell,
.menu-copy {
  display: flex;
  flex-direction: column;
}

.menu-layout {
  display: grid;
  grid-template-columns: minmax(22rem, 28rem) minmax(0, 1fr);
  gap: 1.1rem;
}

.menu-tree {
  display: grid;
  gap: 0.75rem;
}

.menu-card,
.menu-card-main,
.menu-meta,
.menu-preview-header,
.menu-preview-badges,
.menu-editor-actions {
  display: flex;
  align-items: center;
}

.menu-card,
.menu-preview-header,
.menu-editor-actions {
  justify-content: space-between;
}

.menu-card {
  gap: 1rem;
  min-width: 0;
  padding: 0.95rem 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-lg) + 2px);
  background: color-mix(in srgb, var(--bg-subtle) 68%, transparent);
  cursor: pointer;
  transition: border-color var(--duration-fast) var(--ease-apple), transform var(--duration-fast) var(--ease-apple), box-shadow var(--duration-fast) var(--ease-apple);
}

.menu-card:hover,
.menu-card.active {
  border-color: color-mix(in srgb, var(--brand-primary) 26%, var(--border-strong));
  transform: translateY(-1px);
  box-shadow: var(--shadow-sm);
}

.menu-card-main {
  min-width: 0;
  gap: 0.75rem;
}

.menu-indent {
  width: calc(var(--depth) * 0.9rem);
  flex-shrink: 0;
}

.menu-icon-wrap,
.menu-preview-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.85rem;
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
  color: var(--brand-primary);
  flex-shrink: 0;
}

.menu-icon-wrap {
  width: 2.2rem;
  height: 2.2rem;
}

.menu-preview-icon {
  width: 2.5rem;
  height: 2.5rem;
}

.menu-copy {
  min-width: 0;
  gap: 0.18rem;
}

.menu-copy small,
.menu-preview-header p,
.menu-helper-text {
  color: var(--text-secondary);
}

.menu-meta,
.menu-preview-badges,
.menu-editor-actions {
  gap: 0.55rem;
  flex-wrap: wrap;
}

.menu-preview-card {
  display: grid;
  gap: 0.9rem;
  padding: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 90%, transparent);
  border-radius: calc(var(--radius-lg) + 1px);
  background: color-mix(in srgb, var(--bg-subtle) 62%, transparent);
}

.menu-preview-header {
  justify-content: flex-start;
  gap: 0.85rem;
}

.menu-form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
}

.menu-form-full {
  grid-column: 1 / -1;
}

.menu-helper-text {
  font-size: 0.82rem;
  line-height: 1.55;
}

@media (max-width: 1180px) {
  .menu-layout {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (max-width: 760px) {
  .menu-form-grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .menu-card {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
