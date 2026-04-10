<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { RoleRecord } from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiField,
  UiInspectorPanel,
  UiInput,
  UiListDetailShell,
  UiMetricCard,
  UiPanelFrame,
  UiPagination,
  UiRecordCard,
  UiSelect,
  UiStatusCallout,
  UiTextarea,
} from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { getMenuDefinition } from '@/navigation/menuRegistry'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { useWorkspaceStore } from '@/stores/workspace'
import PermissionCenterMenuTree from './PermissionCenterMenuTree.vue'
import { buildPermissionCenterMenuTreeSections } from './menu-tree'

const PAGE_SIZE = 6

const { t, locale } = useI18n()
const workspaceAccessStore = useWorkspaceAccessStore()
const workspaceStore = useWorkspaceStore()

const selectedRoleId = ref('')
const currentPage = ref(1)
const saveMessage = ref('')
const deleteDialogOpen = ref(false)
const pendingDeleteRoleId = ref('')
const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active',
  permissionIds: [] as string[],
  menuIds: [] as string[],
})

const statusOptions = computed(() => {
  locale.value
  return [
    { value: 'active', label: enumLabel('recordStatus', 'active') },
    { value: 'disabled', label: enumLabel('recordStatus', 'disabled') },
  ]
})

const metrics = computed(() => [
  { id: 'total', label: t('permissionCenter.roles.metrics.total'), value: String(workspaceAccessStore.roles.length) },
  { id: 'disabled', label: t('permissionCenter.roles.metrics.disabled'), value: String(workspaceAccessStore.roles.filter(role => role.status === 'disabled').length) },
])

const permissionOptions = computed(() => workspaceAccessStore.permissions)
const menuOptions = computed(() => workspaceAccessStore.menus.filter(menu => menu.status === 'active'))
const menuTreeSections = computed(() => buildPermissionCenterMenuTreeSections(
  menuOptions.value,
  {
    app: t('permissionCenter.roles.menuGroups.app'),
    workspace: t('permissionCenter.roles.menuGroups.workspace'),
    console: t('permissionCenter.roles.menuGroups.console'),
    permissionCenter: t('permissionCenter.roles.menuGroups.permissionCenter'),
    project: t('permissionCenter.roles.menuGroups.project'),
  },
  menu => menuLabel(menu.id, menu.label),
))
const pageCount = computed(() => Math.max(1, Math.ceil(workspaceAccessStore.roles.length / PAGE_SIZE)))
const pagedRoles = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE
  return workspaceAccessStore.roles.slice(start, start + PAGE_SIZE)
})

watch(
  () => workspaceAccessStore.roles.map(role => role.id).join('|'),
  () => {
    if (currentPage.value > pageCount.value) {
      currentPage.value = pageCount.value
    }
    if (!selectedRoleId.value || !workspaceAccessStore.roles.some(role => role.id === selectedRoleId.value)) {
      applyRole(workspaceAccessStore.roles[0]?.id)
      return
    }
    applyRole(selectedRoleId.value)
  },
  { immediate: true },
)

function resetFormState() {
  form.name = ''
  form.code = ''
  form.description = ''
  form.status = 'active'
  form.permissionIds = []
  form.menuIds = []
}

function applyRole(roleId?: string) {
  const role = workspaceAccessStore.roles.find(item => item.id === roleId)
  if (!role) {
    selectedRoleId.value = ''
    resetFormState()
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

function createRoleDraft() {
  selectedRoleId.value = ''
  resetFormState()
}

async function saveRole() {
  if (!workspaceStore.currentWorkspaceId || !form.name.trim() || !form.code.trim()) {
    return
  }

  const record: RoleRecord = {
    id: selectedRoleId.value || `role-${Date.now()}`,
    workspaceId: workspaceStore.currentWorkspaceId,
    name: form.name.trim(),
    code: form.code.trim(),
    description: form.description.trim(),
    status: form.status as RoleRecord['status'],
    permissionIds: [...form.permissionIds],
    menuIds: [...form.menuIds],
  }

  if (selectedRoleId.value) {
    const updated = await workspaceAccessStore.updateRole(selectedRoleId.value, record)
    applyRole(updated.id)
    saveMessage.value = t('permissionCenter.roles.feedback.saved')
    return
  }

  const created = await workspaceAccessStore.createRole(record)
  selectedRoleId.value = created.id
  applyRole(created.id)
  saveMessage.value = t('permissionCenter.roles.feedback.saved')
}

function promptDeleteRole(roleId: string) {
  pendingDeleteRoleId.value = roleId
  deleteDialogOpen.value = true
}

async function confirmDeleteRole() {
  if (!pendingDeleteRoleId.value) {
    return
  }
  await workspaceAccessStore.deleteRole(pendingDeleteRoleId.value)
  deleteDialogOpen.value = false
  pendingDeleteRoleId.value = ''
  applyRole(workspaceAccessStore.roles[0]?.id)
  saveMessage.value = t('permissionCenter.roles.feedback.deleted')
}

function menuLabel(menuId: string, fallback: string) {
  const definition = getMenuDefinition(menuId)
  return definition ? t(definition.labelKey) : fallback
}
</script>

<template>
  <div data-testid="permission-center-roles-shell">
    <UiListDetailShell>
      <template #list>
        <section class="space-y-3">
          <div class="grid gap-3 md:grid-cols-2">
            <UiMetricCard v-for="metric in metrics" :key="metric.id" :label="metric.label" :value="metric.value" />
          </div>

          <UiPanelFrame
            variant="subtle"
            padding="md"
            :title="t('permissionCenter.roles.title')"
            :subtitle="t('permissionCenter.roles.listSubtitle')"
          >
            <template #actions>
              <UiButton data-testid="roles-create-button" @click="createRoleDraft">
                {{ t('permissionCenter.roles.actions.create') }}
              </UiButton>
            </template>
          </UiPanelFrame>

          <UiRecordCard
            v-for="role in pagedRoles"
            :key="role.id"
            :title="role.name"
            :description="role.description"
            interactive
            :active="selectedRoleId === role.id"
            @click="applyRole(role.id)"
          >
            <template #badges>
              <UiBadge :label="enumLabel('recordStatus', role.status)" subtle />
              <UiBadge :label="role.code" subtle />
              <UiButton
                variant="destructive"
                size="sm"
                :data-testid="`roles-delete-button-${role.code}`"
                @click.stop="promptDeleteRole(role.id)"
              >
                {{ t('permissionCenter.roles.actions.delete') }}
              </UiButton>
            </template>
          </UiRecordCard>

          <UiPagination
            v-model:page="currentPage"
            :page-count="pageCount"
            :summary-label="`${workspaceAccessStore.roles.length}`"
            root-test-id="roles-list-pagination"
          />
        </section>
      </template>

      <div data-testid="permission-center-roles-inspector">
        <UiInspectorPanel
          :title="selectedRoleId ? t('permissionCenter.roles.title') : t('permissionCenter.roles.actions.create')"
          :subtitle="selectedRoleId ? form.code : t('permissionCenter.roles.listSubtitle')"
        >
          <div class="space-y-4">
            <UiStatusCallout v-if="saveMessage" tone="success" :description="saveMessage" />

            <UiField :label="t('permissionCenter.roles.fields.name')">
              <UiInput v-model="form.name" data-testid="roles-name-input" />
            </UiField>
            <UiField :label="t('permissionCenter.roles.fields.code')">
              <UiInput v-model="form.code" data-testid="roles-code-input" />
            </UiField>
            <UiField :label="t('common.status')">
              <UiSelect v-model="form.status" :options="statusOptions" />
            </UiField>
            <UiField :label="t('permissionCenter.roles.fields.permissions')">
              <div class="space-y-2">
                <label
                  v-for="permission in permissionOptions"
                  :key="permission.id"
                  class="block rounded-[var(--radius-m)] border border-border bg-surface p-3"
                >
                  <UiCheckbox
                    v-model="form.permissionIds"
                    :value="permission.id"
                    :data-testid="`roles-permission-${permission.id}`"
                  >
                    <span class="font-medium text-text-primary">{{ permission.name }}</span>
                    <span class="ml-2 text-xs text-text-tertiary">{{ permission.code }}</span>
                  </UiCheckbox>
                  <div class="mt-1 text-xs text-text-secondary">
                    {{ permission.description }}
                  </div>
                </label>
              </div>
            </UiField>
            <UiField :label="t('permissionCenter.roles.fields.menus')" :hint="t('permissionCenter.roles.menuHint')">
              <PermissionCenterMenuTree
                v-model="form.menuIds"
                selection-mode="multiple"
                test-id-prefix="roles-menu"
                :sections="menuTreeSections"
              />
            </UiField>
            <UiField :label="t('permissionCenter.roles.fields.description')">
              <UiTextarea v-model="form.description" :rows="5" />
            </UiField>
            <div class="flex gap-3">
              <UiButton data-testid="roles-save-button" @click="saveRole">{{ t('permissionCenter.roles.actions.save') }}</UiButton>
              <UiButton variant="ghost" @click="selectedRoleId ? applyRole(selectedRoleId) : createRoleDraft()">{{ t('permissionCenter.roles.actions.reset') }}</UiButton>
            </div>
          </div>
        </UiInspectorPanel>
      </div>
    </UiListDetailShell>
  </div>

  <UiDialog
    v-model:open="deleteDialogOpen"
    :title="t('permissionCenter.roles.deleteTitle')"
    :description="t('permissionCenter.roles.deleteDescription')"
  >
    <template #footer>
      <UiButton variant="ghost" @click="deleteDialogOpen = false">
        {{ t('common.cancel') }}
      </UiButton>
      <UiButton data-testid="roles-delete-confirm-button" @click="confirmDeleteRole">
        {{ t('common.confirm') }}
      </UiButton>
    </template>
  </UiDialog>
</template>
