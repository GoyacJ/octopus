<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronDown, ChevronRight } from 'lucide-vue-next'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPagination,
  UiPanelFrame,
  UiRecordCard,
  UiSelect,
  UiSwitch,
  UiStatusCallout,
  UiTextarea,
  UiToolbarRow,
} from '@octopus/ui'

import type { PermissionDefinition, RoleUpsertRequest } from '@octopus/schema'

import { usePagination } from '@/composables/usePagination'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import {
  getAccessRoleDescription,
  getAccessRoleName,
  getPermissionDisplayName,
} from './display-i18n'
import { createStatusOptions, getCapabilityModuleLabel, getStatusLabel } from './helpers'
import { useAccessControlNotifications } from './useAccessControlNotifications'
import { useAccessControlSelection } from './useAccessControlSelection'

interface RoleFormState {
  code: string
  name: string
  description: string
  status: string
  permissionCodes: string[]
}

const { t } = useI18n()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifyError, notifySuccess, notifyWarning } = useAccessControlNotifications('access-control.roles')

const selectedRoleId = ref('')
const query = ref('')
const createDialogOpen = ref(false)
const deleteDialogOpen = ref(false)
const bulkDeleteDialogOpen = ref(false)
const submitError = ref('')
const savingCreate = ref(false)
const savingEdit = ref(false)
const deletingRoleId = ref('')
const deletingSelectedRoles = ref(false)
const togglingRoleIds = ref<string[]>([])
const roleStatusOverrides = ref<Record<string, string>>({})
const expandedEditPermissionModuleIds = ref<string[]>([])
const expandedCreatePermissionModuleIds = ref<string[]>([])

const createForm = reactive<RoleFormState>(createEmptyForm())
const editForm = reactive<RoleFormState>(createEmptyForm())

const filteredRoles = computed(() => {
  const normalizedQuery = query.value.trim().toLowerCase()
  return [...accessControlStore.roles]
    .sort((left, right) => getAccessRoleName(left).localeCompare(getAccessRoleName(right)))
    .filter(role => !normalizedQuery || [
      getAccessRoleName(role),
      role.code,
      getAccessRoleDescription(role),
      ...role.permissionCodes,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const selectedRole = computed(() =>
  accessControlStore.roles.find(role => role.id === selectedRoleId.value) ?? null,
)

const statusOptions = computed(() => createStatusOptions(t))
const permissionModuleItems = computed(() => {
  const grouped = new Map<string, PermissionDefinition[]>()
  for (const permission of accessControlStore.permissionDefinitions) {
    const [moduleName = permission.code] = permission.code.split('.')
    const items = grouped.get(moduleName) ?? []
    items.push(permission)
    grouped.set(moduleName, items)
  }

  return Array.from(grouped.entries())
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([moduleName, permissions]) => ({
      moduleName,
      permissions: [...permissions].sort((left, right) => left.code.localeCompare(right.code)),
    }))
})
const permissionModuleMap = computed(() =>
  new Map(permissionModuleItems.value.map(module => [module.moduleName, module])),
)

const pagination = usePagination(filteredRoles, {
  pageSize: 8,
  resetOn: [query],
})
const selection = useAccessControlSelection(() => accessControlStore.roles, {
  getId: role => role.id,
})
const allPageSelected = computed(() => selection.isPageSelected(pagination.pagedItems.value))
const selectedRolesForDelete = computed(() =>
  selection.selectedIds.value
    .map(id => accessControlStore.roles.find(role => role.id === id) ?? null)
    .filter((role): role is NonNullable<typeof role> => Boolean(role)),
)

watch(selectedRole, (role) => {
  if (!role) {
    Object.assign(editForm, createEmptyForm())
    expandedEditPermissionModuleIds.value = []
    return
  }

  Object.assign(editForm, {
    code: role.code,
    name: role.name,
    description: role.description,
    status: role.status,
    permissionCodes: [...role.permissionCodes],
  } satisfies RoleFormState)
  expandedEditPermissionModuleIds.value = buildInitialExpandedPermissionModules(role.permissionCodes)
}, { immediate: true })

watch(pagination.pagedItems, (roles) => {
  if (selectedRoleId.value && !roles.some(role => role.id === selectedRoleId.value)) {
    selectedRoleId.value = ''
  }
}, { immediate: true })

watch(permissionModuleItems, (modules) => {
  const moduleNames = new Set(modules.map(module => module.moduleName))
  expandedEditPermissionModuleIds.value = normalizeExpandedPermissionModules(
    expandedEditPermissionModuleIds.value,
    moduleNames,
    editForm.permissionCodes,
  )
  expandedCreatePermissionModuleIds.value = normalizeExpandedPermissionModules(
    expandedCreatePermissionModuleIds.value,
    moduleNames,
    createForm.permissionCodes,
  )
}, { immediate: true })

watch(createDialogOpen, (open) => {
  if (open) {
    expandedCreatePermissionModuleIds.value = buildInitialExpandedPermissionModules(createForm.permissionCodes)
    return
  }
  expandedCreatePermissionModuleIds.value = []
})

watch(() => accessControlStore.roles.map(role => [role.id, role.status] as const), (records) => {
  const nextOverrides = { ...roleStatusOverrides.value }
  let changed = false

  for (const [roleId, status] of Object.entries(roleStatusOverrides.value)) {
    const matched = records.find(([recordId]) => recordId === roleId)
    if (!matched || matched[1] === status) {
      delete nextOverrides[roleId]
      changed = true
    }
  }

  if (changed) {
    roleStatusOverrides.value = nextOverrides
  }
}, { immediate: true })

function createEmptyForm(): RoleFormState {
  return {
    code: '',
    name: '',
    description: '',
    status: 'active',
    permissionCodes: [],
  }
}

function resetCreateForm() {
  Object.assign(createForm, createEmptyForm())
  expandedCreatePermissionModuleIds.value = buildInitialExpandedPermissionModules([])
}

function selectRole(roleId: string) {
  selectedRoleId.value = roleId
  submitError.value = ''
}

function toggleRoleSelection(roleId: string, value: boolean) {
  selection.toggleSelection(roleId, value)
}

function togglePageSelection(value: boolean) {
  selection.selectPage(pagination.pagedItems.value, value)
}

function validateForm(form: RoleFormState) {
  if (!form.code.trim() || !form.name.trim()) {
    return t('accessControl.roles.validation.required')
  }
  return ''
}

function toPayload(form: RoleFormState): RoleUpsertRequest {
  return {
    code: form.code.trim(),
    name: form.name.trim(),
    description: form.description.trim(),
    status: form.status,
    permissionCodes: [...form.permissionCodes],
  }
}

function isRoleStatusSaving(roleId: string) {
  return togglingRoleIds.value.includes(roleId)
}

function roleListStatus(roleId: string, fallbackStatus: string) {
  return roleStatusOverrides.value[roleId] ?? fallbackStatus
}

function setRoleStatusOverride(roleId: string, status: string) {
  roleStatusOverrides.value = {
    ...roleStatusOverrides.value,
    [roleId]: status,
  }
}

function clearRoleStatusOverride(roleId: string) {
  const { [roleId]: _ignored, ...rest } = roleStatusOverrides.value
  roleStatusOverrides.value = rest
}

function buildInitialExpandedPermissionModules(permissionCodes: string[]) {
  const preferredModules = permissionCodes
    .map((code) => {
      const [moduleName = ''] = code.split('.')
      return moduleName
    })
    .filter(moduleName => permissionModuleMap.value.has(moduleName))
  const uniquePreferredModules = Array.from(new Set(preferredModules))

  const [preferredModule = ''] = uniquePreferredModules
  if (preferredModule) {
    return [preferredModule]
  }

  const firstModule = permissionModuleItems.value[0]?.moduleName
  return firstModule ? [firstModule] : []
}

function normalizeExpandedPermissionModules(
  expandedModuleIds: string[],
  moduleNames: Set<string>,
  permissionCodes: string[],
) {
  const normalized = expandedModuleIds.filter(moduleName => moduleNames.has(moduleName))
  if (normalized.length) {
    return normalized
  }
  return buildInitialExpandedPermissionModules(permissionCodes)
}

function togglePermissionModule(moduleIds: string[], moduleName: string) {
  if (moduleIds.includes(moduleName)) {
    return moduleIds.filter(id => id !== moduleName)
  }
  return [...moduleIds, moduleName]
}

function toggleEditPermissionModule(moduleName: string) {
  expandedEditPermissionModuleIds.value = togglePermissionModule(expandedEditPermissionModuleIds.value, moduleName)
}

function toggleCreatePermissionModule(moduleName: string) {
  expandedCreatePermissionModuleIds.value = togglePermissionModule(expandedCreatePermissionModuleIds.value, moduleName)
}

function isPermissionModuleExpanded(expandedModuleIds: string[], moduleName: string) {
  return expandedModuleIds.includes(moduleName)
}

function permissionModuleCount(itemId: string) {
  return permissionModuleMap.value.get(itemId)?.permissions.length ?? 0
}

function permissionModuleSelectedCount(permissionCodes: string[], moduleName: string) {
  const permissions = permissionModuleMap.value.get(moduleName)?.permissions ?? []
  const selected = new Set(permissionCodes)
  return permissions.filter(permission => selected.has(permission.code)).length
}

function setPermissionSelection(permissionCodes: string[], permissionCode: string, enabled: boolean) {
  const next = new Set(permissionCodes)
  if (enabled) {
    next.add(permissionCode)
  } else {
    next.delete(permissionCode)
  }
  return Array.from(next)
}

function toggleEditPermission(permissionCode: string, enabled: boolean) {
  editForm.permissionCodes = setPermissionSelection(editForm.permissionCodes, permissionCode, enabled)
}

function toggleCreatePermission(permissionCode: string, enabled: boolean) {
  createForm.permissionCodes = setPermissionSelection(createForm.permissionCodes, permissionCode, enabled)
}

async function toggleRoleStatus(role: { id: string, code: string, name: string, description: string, status: string, permissionCodes: string[] }, enabled: boolean) {
  const nextStatus = enabled ? 'active' : 'disabled'
  if (isRoleStatusSaving(role.id) || nextStatus === role.status) {
    return
  }

  submitError.value = ''
  setRoleStatusOverride(role.id, nextStatus)
  togglingRoleIds.value = [...togglingRoleIds.value, role.id]

  try {
    await accessControlStore.updateRole(role.id, {
      code: role.code,
      name: role.name,
      description: role.description,
      status: nextStatus,
      permissionCodes: [...role.permissionCodes],
    })
    await notifySuccess(
      t(enabled ? 'accessControl.roles.feedback.toastEnabledTitle' : 'accessControl.roles.feedback.toastDisabledTitle'),
      getAccessRoleName(role),
    )
  } catch (error) {
    clearRoleStatusOverride(role.id)
    submitError.value = error instanceof Error ? error.message : t('accessControl.roles.feedback.statusUpdateFailed')
  } finally {
    togglingRoleIds.value = togglingRoleIds.value.filter(id => id !== role.id)
  }
}

async function handleCreate() {
  submitError.value = validateForm(createForm)
  if (submitError.value) {
    return
  }

  savingCreate.value = true
  try {
    const record = await accessControlStore.createRole(toPayload(createForm))
    selectedRoleId.value = record.id
    createDialogOpen.value = false
    resetCreateForm()
    await notifySuccess(t('accessControl.roles.feedback.toastSavedTitle'), record.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.roles.feedback.saveFailed')
  } finally {
    savingCreate.value = false
  }
}

async function handleUpdate() {
  if (!selectedRole.value) {
    return
  }

  submitError.value = validateForm(editForm)
  if (submitError.value) {
    return
  }

  savingEdit.value = true
  try {
    const payload = toPayload(editForm)
    await accessControlStore.updateRole(selectedRole.value.id, payload)
    await notifySuccess(t('accessControl.roles.feedback.toastSavedTitle'), payload.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.roles.feedback.saveFailed')
  } finally {
    savingEdit.value = false
  }
}

async function handleDelete() {
  if (!selectedRole.value) {
    return
  }

  deletingRoleId.value = selectedRole.value.id
  submitError.value = ''
  try {
    const label = getAccessRoleName(selectedRole.value)
    await accessControlStore.deleteRole(selectedRole.value.id)
    selectedRoleId.value = ''
    deleteDialogOpen.value = false
    await notifySuccess(t('accessControl.roles.feedback.toastDeletedTitle'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.roles.feedback.deleteFailed')
  } finally {
    deletingRoleId.value = ''
  }
}

async function handleBulkDelete() {
  if (!selectedRolesForDelete.value.length) {
    bulkDeleteDialogOpen.value = false
    return
  }

  deletingSelectedRoles.value = true
  submitError.value = ''
  let successCount = 0
  let failureCount = 0

  for (const role of selectedRolesForDelete.value) {
    try {
      await accessControlStore.deleteRole(role.id)
      successCount += 1
      if (selectedRoleId.value === role.id) {
        selectedRoleId.value = ''
      }
    } catch {
      failureCount += 1
    }
  }

  deletingSelectedRoles.value = false
  bulkDeleteDialogOpen.value = false
  selection.setSelection(
    selection.selectedIds.value.filter(id =>
      accessControlStore.roles.some(role => role.id === id),
    ),
  )

  const body = t('accessControl.common.bulk.resultBody', {
    success: successCount,
    failure: failureCount,
    skipped: 0,
  })

  if (successCount > 0 && failureCount === 0) {
    selection.clearSelection()
    await notifySuccess(t('accessControl.common.bulk.resultAllSuccessTitle'), body)
    return
  }

  if (successCount > 0) {
    await notifyWarning(t('accessControl.common.bulk.resultPartialTitle'), body)
    return
  }

  await notifyError(t('accessControl.common.bulk.resultFailureTitle'), body)
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-roles-shell">
    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedRole)"
      :detail-title="selectedRole ? getAccessRoleName(selectedRole) : ''"
      :detail-subtitle="t('accessControl.roles.detail.subtitle')"
      :empty-detail-title="t('accessControl.roles.detail.emptyTitle')"
      :empty-detail-description="t('accessControl.roles.detail.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-roles-toolbar">
          <template #search>
            <UiInput v-model="query" :placeholder="t('accessControl.roles.toolbar.search')" />
          </template>
          <template #actions>
            <span
              v-if="selection.hasSelection.value"
              class="text-xs text-text-secondary"
            >
              {{ t('accessControl.common.selection.selectedCount', { count: selection.selectedCount.value }) }}
            </span>
            <UiButton
              v-if="pagination.pagedItems.value.length"
              variant="ghost"
              size="sm"
              @click="togglePageSelection(!allPageSelected)"
            >
              {{ t('accessControl.common.selection.selectPage') }}
            </UiButton>
            <UiButton
              v-if="selection.hasSelection.value"
              variant="ghost"
              size="sm"
              @click="selection.clearSelection"
            >
              {{ t('accessControl.common.selection.clear') }}
            </UiButton>
            <UiButton
              v-if="selection.hasSelection.value"
              variant="destructive"
              size="sm"
              data-testid="access-control-role-bulk-delete-button"
              @click="bulkDeleteDialogOpen = true"
            >
              {{ t('accessControl.common.bulk.delete') }}
            </UiButton>
            <UiButton data-testid="access-control-role-create-button" size="sm" @click="createDialogOpen = true">
              {{ t('accessControl.roles.toolbar.create') }}
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.roles.list.title')"
          :subtitle="t('accessControl.common.list.totalRoles', { count: pagination.totalItems.value })"
        >
          <div v-if="pagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="role in pagination.pagedItems.value"
              :key="role.id"
              layout="compact"
              interactive
              :active="selectedRoleId === role.id"
              :title="getAccessRoleName(role)"
              :description="role.code"
              :test-id="`access-control-role-record-${role.id}`"
              @click="selectRole(role.id)"
            >
              <template #badges>
                <div class="flex items-center gap-2" @click.stop>
                  <UiCheckbox
                    :model-value="selection.isSelected(role.id)"
                    :data-testid="`access-control-role-select-${role.id}`"
                    @update:model-value="toggleRoleSelection(role.id, Boolean($event))"
                  />
                  <UiSwitch
                    :model-value="roleListStatus(role.id, role.status) === 'active'"
                    :disabled="isRoleStatusSaving(role.id)"
                    @update:model-value="toggleRoleStatus(role, $event)"
                  >
                    <span class="sr-only">
                      {{ t('accessControl.roles.list.toggleStatus', { name: getAccessRoleName(role) }) }}
                    </span>
                  </UiSwitch>
                </div>
              </template>
              <template #meta>
                <span class="truncate text-xs text-text-secondary">
                  {{ t('accessControl.roles.list.permissionCount', { count: role.permissionCodes.length }) }}
                </span>
              </template>
            </UiRecordCard>
          </div>
          <UiEmptyState
            v-else
            :title="t('accessControl.roles.list.emptyTitle')"
            :description="t('accessControl.roles.list.emptyDescription')"
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
        <div v-if="selectedRole" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ getAccessRoleName(selectedRole) }}</div>
              <UiBadge :label="getStatusLabel(t, selectedRole.status)" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedRole.code }}</div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.roles.fields.code')">
              <UiInput v-model="editForm.code" />
            </UiField>
            <UiField :label="t('accessControl.roles.fields.name')">
              <UiInput v-model="editForm.name" />
            </UiField>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.roles.fields.status')">
              <UiSelect v-model="editForm.status" :options="statusOptions" />
            </UiField>
            <UiField :label="t('accessControl.roles.fields.description')">
              <UiTextarea v-model="editForm.description" :rows="3" />
            </UiField>
          </div>

          <UiField :label="t('accessControl.roles.fields.permissions')">
            <div
              data-testid="access-control-role-permissions-inspector"
              class="rounded-[var(--radius-m)] border border-border bg-muted/35 p-3"
            >
              <div class="max-h-[360px] overflow-y-auto overscroll-contain pr-1 [overflow-anchor:none] [scrollbar-gutter:stable]">
                <div class="space-y-2">
                  <section
                    v-for="module in permissionModuleItems"
                    :key="module.moduleName"
                    :data-testid="`access-control-role-permission-section-${module.moduleName}`"
                    class="rounded-[var(--radius-l)] border border-transparent bg-subtle"
                  >
                    <UiButton
                      :data-testid="`access-control-role-permission-trigger-${module.moduleName}`"
                      type="button"
                      variant="ghost"
                      class="h-auto w-full justify-start gap-2 rounded-[var(--radius-l)] px-3 py-2 text-left hover:bg-muted/45"
                      :aria-expanded="isPermissionModuleExpanded(expandedEditPermissionModuleIds, module.moduleName) ? 'true' : 'false'"
                      @click="toggleEditPermissionModule(module.moduleName)"
                    >
                      <component
                        :is="isPermissionModuleExpanded(expandedEditPermissionModuleIds, module.moduleName) ? ChevronDown : ChevronRight"
                        :size="16"
                        class="shrink-0 text-text-tertiary"
                      />
                      <div class="min-w-0 flex-1">
                        <div class="truncate text-sm font-medium text-text-primary">
                          {{ getCapabilityModuleLabel(t, module.moduleName) }}
                        </div>
                        <div class="truncate pt-0.5 text-xs text-text-secondary">
                          {{ module.moduleName }}
                        </div>
                      </div>
                      <div class="flex items-center gap-2">
                        <UiBadge
                          :label="t('accessControl.roles.list.permissionCount', { count: permissionModuleCount(module.moduleName) })"
                          subtle
                        />
                        <UiBadge
                          :label="`${permissionModuleSelectedCount(editForm.permissionCodes, module.moduleName)}`"
                          subtle
                        />
                      </div>
                    </UiButton>

                    <div
                      v-show="isPermissionModuleExpanded(expandedEditPermissionModuleIds, module.moduleName)"
                      :data-testid="`access-control-role-permission-body-${module.moduleName}`"
                      class="border-t border-border/60 px-3 py-2"
                      :aria-hidden="isPermissionModuleExpanded(expandedEditPermissionModuleIds, module.moduleName) ? 'false' : 'true'"
                    >
                      <div class="space-y-1.5">
                        <div
                          v-for="permission in module.permissions"
                          :key="permission.code"
                          :data-testid="`access-control-role-permission-row-${permission.code}`"
                          class="flex items-start gap-3 rounded-[var(--radius-m)] border border-transparent px-2 py-1.5 transition-colors hover:border-border hover:bg-muted/45"
                        >
                          <UiCheckbox
                            :model-value="editForm.permissionCodes.includes(permission.code)"
                            :data-testid="`access-control-role-permission-toggle-${permission.code}`"
                            @click.stop
                            @update:model-value="toggleEditPermission(permission.code, Boolean($event))"
                          />
                          <div class="min-w-0 flex-1">
                            <div class="truncate text-sm font-medium text-text-primary">
                              {{ getPermissionDisplayName(permission) }}
                            </div>
                            <div class="truncate pt-0.5 text-xs text-text-secondary">
                              {{ permission.code }}
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </section>
                </div>
              </div>
            </div>
          </UiField>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton variant="ghost" class="text-destructive" @click="deleteDialogOpen = true">
              {{ t('accessControl.roles.actions.delete') }}
            </UiButton>
            <UiButton :loading="savingEdit" @click="handleUpdate">
              {{ t('accessControl.roles.actions.save') }}
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      :open="createDialogOpen"
      :title="t('accessControl.roles.dialogs.createTitle')"
      :description="t('accessControl.roles.dialogs.createDescription')"
      @update:open="createDialogOpen = $event"
    >
      <div class="space-y-4">
        <div class="grid gap-3 md:grid-cols-2">
          <UiField :label="t('accessControl.roles.fields.code')">
            <UiInput v-model="createForm.code" />
          </UiField>
          <UiField :label="t('accessControl.roles.fields.name')">
            <UiInput v-model="createForm.name" />
          </UiField>
        </div>
        <div class="grid gap-3 md:grid-cols-2">
          <UiField :label="t('accessControl.roles.fields.status')">
            <UiSelect v-model="createForm.status" :options="statusOptions" />
          </UiField>
          <UiField :label="t('accessControl.roles.fields.description')">
            <UiTextarea v-model="createForm.description" :rows="3" />
          </UiField>
        </div>
        <UiField :label="t('accessControl.roles.fields.permissions')">
          <div
            data-testid="access-control-role-permissions-create"
            class="rounded-[var(--radius-m)] border border-border bg-muted/35 p-3"
          >
            <div class="max-h-[360px] overflow-y-auto overscroll-contain pr-1 [overflow-anchor:none] [scrollbar-gutter:stable]">
              <div class="space-y-2">
                <section
                  v-for="module in permissionModuleItems"
                  :key="module.moduleName"
                  :data-testid="`access-control-role-create-permission-section-${module.moduleName}`"
                  class="rounded-[var(--radius-l)] border border-transparent bg-subtle"
                >
                  <UiButton
                    :data-testid="`access-control-role-create-permission-trigger-${module.moduleName}`"
                    type="button"
                    variant="ghost"
                    class="h-auto w-full justify-start gap-2 rounded-[var(--radius-l)] px-3 py-2 text-left hover:bg-muted/45"
                    :aria-expanded="isPermissionModuleExpanded(expandedCreatePermissionModuleIds, module.moduleName) ? 'true' : 'false'"
                    @click="toggleCreatePermissionModule(module.moduleName)"
                  >
                    <component
                      :is="isPermissionModuleExpanded(expandedCreatePermissionModuleIds, module.moduleName) ? ChevronDown : ChevronRight"
                      :size="16"
                      class="shrink-0 text-text-tertiary"
                    />
                    <div class="min-w-0 flex-1">
                      <div class="truncate text-sm font-medium text-text-primary">
                        {{ getCapabilityModuleLabel(t, module.moduleName) }}
                      </div>
                      <div class="truncate pt-0.5 text-xs text-text-secondary">
                        {{ module.moduleName }}
                      </div>
                    </div>
                    <div class="flex items-center gap-2">
                      <UiBadge
                        :label="t('accessControl.roles.list.permissionCount', { count: permissionModuleCount(module.moduleName) })"
                        subtle
                      />
                      <UiBadge
                        :label="`${permissionModuleSelectedCount(createForm.permissionCodes, module.moduleName)}`"
                        subtle
                      />
                    </div>
                  </UiButton>

                  <div
                    v-show="isPermissionModuleExpanded(expandedCreatePermissionModuleIds, module.moduleName)"
                    :data-testid="`access-control-role-create-permission-body-${module.moduleName}`"
                    class="border-t border-border/60 px-3 py-2"
                    :aria-hidden="isPermissionModuleExpanded(expandedCreatePermissionModuleIds, module.moduleName) ? 'false' : 'true'"
                  >
                    <div class="space-y-1.5">
                      <div
                        v-for="permission in module.permissions"
                        :key="permission.code"
                        :data-testid="`access-control-role-create-permission-row-${permission.code}`"
                        class="flex items-start gap-3 rounded-[var(--radius-m)] border border-transparent px-2 py-1.5 transition-colors hover:border-border hover:bg-muted/45"
                      >
                        <UiCheckbox
                          :model-value="createForm.permissionCodes.includes(permission.code)"
                          :data-testid="`access-control-role-create-permission-toggle-${permission.code}`"
                          @click.stop
                          @update:model-value="toggleCreatePermission(permission.code, Boolean($event))"
                        />
                        <div class="min-w-0 flex-1">
                          <div class="truncate text-sm font-medium text-text-primary">
                            {{ getPermissionDisplayName(permission) }}
                          </div>
                          <div class="truncate pt-0.5 text-xs text-text-secondary">
                            {{ permission.code }}
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </section>
              </div>
            </div>
          </div>
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton :loading="savingCreate" @click="handleCreate">
          {{ t('accessControl.roles.actions.create') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="bulkDeleteDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.roles') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeleteDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedRolesForDelete.length,
          entity: t('accessControl.common.entities.roles'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeleteDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton variant="destructive" :loading="deletingSelectedRoles" @click="handleBulkDelete">
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="deleteDialogOpen"
      :title="t('accessControl.roles.dialogs.deleteTitle')"
      :description="t('accessControl.roles.dialogs.deleteDescription')"
      @update:open="deleteDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.roles.dialogs.deleteConfirm', { name: selectedRole ? getAccessRoleName(selectedRole) : t('common.na') }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="deleteDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton variant="destructive" :loading="deletingRoleId === selectedRole?.id" @click="handleDelete">
          {{ t('common.delete') }}
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>
