<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { PanelLeftOpen, Plus, Power, Shield, Trash2, Users } from 'lucide-vue-next'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiField,
  UiInput,
  UiMetricCard,
  UiRecordCard,
  UiSelect,
  UiTextarea,
} from '@octopus/ui'

import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const selectedRoleId = ref<string>('')

const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active' as 'active' | 'disabled',
  permissionIds: [] as string[],
  menuIds: [] as string[],
})

const statusOptions = computed(() => [
  { value: 'active', label: t('userCenter.common.active') },
  { value: 'disabled', label: t('userCenter.common.disabled') },
])

const canManageRoles = computed(() =>
  workbench.hasPermission('role:manage:update', 'update'),
)

const roleItems = computed(() => workbench.workspaceRoleListItems)
const selectedRoleSummary = computed(() =>
  roleItems.value.find((item) => item.id === selectedRoleId.value),
)

const availableMenus = computed(() =>
  workbench.workspaceMenus.filter((menu) => menu.source === 'user-center' || !menu.parentId),
)

const summaryMetrics = computed(() => {
  const disabledCount = roleItems.value.filter((role) => role.status === 'disabled').length
  const orphanCount = roleItems.value.filter((role) => role.memberCount === 0).length
  const broadAccessCount = roleItems.value.filter((role) => role.riskFlags.includes(t('userCenter.risk.broadMenuAccess'))).length
  return [
    {
      id: 'total',
      label: t('userCenter.roles.metrics.total'),
      value: String(roleItems.value.length),
      helper: t('userCenter.roles.metrics.disabledHelper', { count: disabledCount }),
    },
    {
      id: 'orphan',
      label: t('userCenter.roles.metrics.orphan'),
      value: String(orphanCount),
      helper: t('userCenter.roles.metrics.orphanHelper'),
      tone: 'warning' as const,
    },
    {
      id: 'broad',
      label: t('userCenter.roles.metrics.broadAccess'),
      value: String(broadAccessCount),
      helper: t('userCenter.roles.metrics.broadAccessHelper'),
      tone: 'accent' as const,
    },
  ]
})

function applyRole(roleId?: string) {
  if (!roleId) {
    selectedRoleId.value = ''
    form.name = ''
    form.code = ''
    form.description = ''
    form.status = 'active'
    form.permissionIds = []
    form.menuIds = []
    return
  }

  const role = workbench.workspaceRoles.find((item) => item.id === roleId)
  if (!role) {
    applyRole()
    return
  }

  selectedRoleId.value = role.id
  form.name = role.name
  form.code = role.code
  form.description = role.description
  form.status = role.status
  form.permissionIds = [...role.permissionIds]
  form.menuIds = [...role.menuIds]
}

watch(
  () => [workbench.currentWorkspaceId, workbench.workspaceRoles.map((role) => role.id).join('|')],
  () => {
    if (!selectedRoleId.value || !workbench.workspaceRoles.some((role) => role.id === selectedRoleId.value)) {
      applyRole(workbench.workspaceRoles[0]?.id)
      return
    }

    applyRole(selectedRoleId.value)
  },
  { immediate: true },
)

function saveRole() {
  if (selectedRoleId.value) {
    workbench.updateRole(selectedRoleId.value, {
      name: form.name,
      code: form.code,
      description: form.description,
      status: form.status,
      permissionIds: form.permissionIds,
      menuIds: form.menuIds,
    })
    return
  }

  const role = workbench.createRole({
    name: form.name,
    code: form.code,
    description: form.description,
    status: form.status,
    permissionIds: form.permissionIds,
    menuIds: form.menuIds,
  })
  applyRole(role.id)
}

function removeRole(roleId: string) {
  const removed = workbench.deleteRole(roleId)
  if (!removed) {
    return
  }

  applyRole(workbench.workspaceRoles[0]?.id)
}
</script>

<template>
  <div class="space-y-10">
    <div class="grid gap-3 sm:grid-cols-3">
      <UiMetricCard
        v-for="metric in summaryMetrics"
        :key="metric.id"
        :label="metric.label"
        :value="metric.value"
        :helper="metric.helper"
        :tone="metric.tone"
      />
    </div>

    <div class="flex gap-8 border-t border-border-subtle pt-8">
      
      <!-- Left: Roles List -->
      <aside class="w-80 shrink-0 border-r border-border-subtle pr-8 flex flex-col gap-4">
        <div data-testid="user-center-roles-toolbar" class="flex items-center justify-between">
          <div class="space-y-1">
            <h3 class="text-sm font-bold text-text-primary">{{ t('userCenter.roles.listTitle') }}</h3>
            <p class="text-[11px] text-text-tertiary">{{ t('userCenter.roles.listSubtitle') }}</p>
          </div>
          <UiButton v-if="canManageRoles" variant="primary" size="icon" class="h-6 w-6 rounded" @click="applyRole()">
            <Plus :size="14" />
          </UiButton>
        </div>

        <div v-if="roleItems.length" class="flex-1 overflow-y-auto space-y-2 pr-1">
          <UiRecordCard
            v-for="role in roleItems"
            :key="role.id"
            :test-id="`user-center-role-record-${role.id}`"
            :title="role.name"
            :active="selectedRoleId === role.id"
            interactive
            @click="applyRole(role.id)"
          >
            <template #eyebrow>{{ role.code }}</template>
            <template #badges>
              <UiBadge :label="role.status" :tone="role.status === 'active' ? 'success' : 'warning'" subtle />
            </template>
            <template #meta>
              <span class="inline-flex items-center gap-1 text-[10px]"><Shield :size="12" />{{ role.permissionCount }}</span>
              <span class="inline-flex items-center gap-1 text-[10px]"><PanelLeftOpen :size="12" />{{ role.menuCount }}</span>
              <span class="inline-flex items-center gap-1 text-[10px]"><Users :size="12" />{{ role.memberCount }}</span>
            </template>
            <template #actions>
              <UiButton v-if="canManageRoles" variant="ghost" size="icon" class="h-6 w-6" @click.stop="workbench.toggleRoleStatus(role.id)">
                <Power :size="12" />
              </UiButton>
              <UiButton v-if="canManageRoles" variant="ghost" size="icon" class="h-6 w-6 text-destructive hover:bg-destructive/10" @click.stop="removeRole(role.id)">
                <Trash2 :size="12" />
              </UiButton>
            </template>
          </UiRecordCard>
        </div>
        <UiEmptyState v-else :title="t('userCenter.roles.listTitle')" :description="t('userCenter.roles.listSubtitle')" />
      </aside>

      <!-- Right: Role Editor Form -->
      <main data-testid="user-center-roles-editor" class="flex-1 overflow-y-auto pb-8">
        <header class="space-y-1 mb-8">
          <h2 class="text-xl font-bold text-text-primary">{{ t(selectedRoleId ? 'userCenter.roles.editTitle' : 'userCenter.roles.createTitle') }}</h2>
          <p class="text-[13px] text-text-secondary">{{ t('userCenter.roles.formSubtitle') }}</p>
        </header>

        <div class="grid gap-x-8 gap-y-6 md:grid-cols-2 max-w-2xl">
          <UiField :label="t('userCenter.roles.nameLabel')">
            <UiInput v-model="form.name" :disabled="!canManageRoles" />
          </UiField>
          <UiField :label="t('userCenter.roles.codeLabel')">
            <UiInput v-model="form.code" :disabled="!canManageRoles" />
          </UiField>
          <UiField :label="t('userCenter.common.status')">
            <UiSelect v-model="form.status" :options="statusOptions" :disabled="!canManageRoles" />
          </UiField>
          <UiField class="md:col-span-2" :label="t('userCenter.roles.descriptionLabel')">
            <UiTextarea v-model="form.description" :rows="3" :disabled="!canManageRoles" />
          </UiField>
        </div>

        <div class="mt-8 grid gap-8 xl:grid-cols-2 max-w-4xl pt-8 border-t border-border-subtle">
          <section class="space-y-3">
            <div class="flex items-center justify-between">
              <h4 class="text-[13px] font-bold text-text-primary">{{ t('userCenter.roles.permissionBindingTitle') }}</h4>
              <UiBadge :label="String(form.permissionIds.length)" subtle />
            </div>
            <div class="bg-subtle/30 rounded-md border border-border-subtle p-4 space-y-2 max-h-64 overflow-y-auto">
              <UiCheckbox
                v-for="permission in workbench.workspacePermissions"
                :key="permission.id"
                v-model="form.permissionIds"
                :value="permission.id"
                :label="permission.name"
                :disabled="!canManageRoles"
              />
            </div>
          </section>

          <section class="space-y-3">
            <div class="flex items-center justify-between">
              <div class="space-y-0.5">
                <h4 class="text-[13px] font-bold text-text-primary">{{ t('userCenter.roles.menuBindingTitle') }}</h4>
                <p class="text-[10px] text-text-tertiary">{{ t('userCenter.roles.menuHint') }}</p>
              </div>
              <UiBadge :label="String(form.menuIds.length)" subtle />
            </div>
            <div class="bg-subtle/30 rounded-md border border-border-subtle p-4 space-y-2 max-h-64 overflow-y-auto">
              <UiCheckbox
                v-for="menu in availableMenus"
                :key="menu.id"
                v-model="form.menuIds"
                :value="menu.id"
                :label="menu.label"
                :disabled="!canManageRoles"
              />
            </div>
          </section>
        </div>

        <div class="mt-8 flex justify-end gap-3 max-w-4xl">
          <UiButton variant="ghost" @click="applyRole(workbench.workspaceRoles[0]?.id)">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton v-if="canManageRoles" variant="primary" @click="saveRole">
            {{ t('common.save') }}
          </UiButton>
        </div>
      </main>
    </div>
  </div>
</template>
