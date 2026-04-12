<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

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
  UiToolbarRow,
} from '@octopus/ui'

import type { AccessUserUpsertRequest } from '@octopus/schema'

import { usePagination } from '@/composables/usePagination'
import { formatList } from '@/i18n/copy'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { createStatusOptions, getScopeTypeLabel, getStatusLabel } from './helpers'
import { useAccessControlNotifications } from './useAccessControlNotifications'
import { useAccessControlSelection } from './useAccessControlSelection'

interface UserFormState {
  username: string
  displayName: string
  status: string
  password: string
  confirmPassword: string
  resetPassword: boolean
}

const { t } = useI18n()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifyError, notifySuccess, notifyWarning } = useAccessControlNotifications('access-control.users')

const selectedUserId = ref('')
const query = ref('')
const statusFilter = ref('')
const createDialogOpen = ref(false)
const deleteDialogOpen = ref(false)
const bulkDeleteDialogOpen = ref(false)
const submitError = ref('')
const savingCreate = ref(false)
const savingEdit = ref(false)
const deletingUserId = ref('')
const deletingSelectedUsers = ref(false)
const togglingUserIds = ref<string[]>([])
const userStatusOverrides = ref<Record<string, string>>({})
const hiddenUserIds = ref<string[]>([])

const createForm = reactive<UserFormState>(createEmptyForm())
const editForm = reactive<UserFormState>(createEmptyForm())

const roleMap = computed(() => new Map(accessControlStore.roles.map(role => [role.id, role.name])))
const orgUnitMap = computed(() => new Map(accessControlStore.orgUnits.map(unit => [unit.id, unit.name])))
const positionMap = computed(() => new Map(accessControlStore.positions.map(position => [position.id, position.name])))
const groupMap = computed(() => new Map(accessControlStore.userGroups.map(group => [group.id, group.name])))

const assignmentsByUserId = computed(() => {
  const grouped = new Map<string, typeof accessControlStore.userOrgAssignments>()
  for (const assignment of accessControlStore.userOrgAssignments) {
    const items = grouped.get(assignment.userId) ?? []
    items.push(assignment)
    grouped.set(assignment.userId, items)
  }
  return grouped
})

const roleNamesByUserId = computed(() => {
  const grouped = new Map<string, string[]>()
  for (const binding of accessControlStore.roleBindings) {
    if (binding.subjectType !== 'user' || binding.effect !== 'allow') {
      continue
    }
    const names = grouped.get(binding.subjectId) ?? []
    names.push(roleMap.value.get(binding.roleId) ?? binding.roleId)
    grouped.set(binding.subjectId, names)
  }
  return grouped
})

const directProjectPoliciesByUserId = computed(() => {
  const grouped = new Map<string, string[]>()
  for (const policy of accessControlStore.dataPolicies) {
    if (policy.subjectType !== 'user' || policy.resourceType !== 'project' || policy.effect !== 'allow') {
      continue
    }
    const labels = grouped.get(policy.subjectId) ?? []
    labels.push(policy.scopeType === 'selected-projects' ? (policy.projectIds.join('、') || policy.name) : getScopeTypeLabel(t, policy.scopeType))
    grouped.set(policy.subjectId, labels)
  }
  return grouped
})

const users = computed(() =>
  [...accessControlStore.users]
    .filter(user => !hiddenUserIds.value.includes(user.id))
    .sort((left, right) => left.displayName.localeCompare(right.displayName))
    .filter((user) => {
      const matchesStatus = !statusFilter.value || user.status === statusFilter.value
      if (!matchesStatus) {
        return false
      }

      const normalizedQuery = query.value.trim().toLowerCase()
      if (!normalizedQuery) {
        return true
      }

      return [
        user.displayName,
        user.username,
        user.status,
        ...(roleNamesByUserId.value.get(user.id) ?? []),
        ...userOrgLabels(user.id),
      ].join(' ').toLowerCase().includes(normalizedQuery)
    }),
)

const selectedUser = computed(() =>
  accessControlStore.users.find(user => user.id === selectedUserId.value) ?? null,
)

const statusOptions = computed(() => createStatusOptions(t))
const statusFilterOptions = computed(() => [
  { label: t('accessControl.common.filters.allStatuses'), value: '' },
  ...statusOptions.value,
])

const pagination = usePagination(users, {
  pageSize: 8,
  resetOn: [query, statusFilter],
})
const selection = useAccessControlSelection(() => accessControlStore.users, {
  getId: user => user.id,
})
const allPageSelected = computed(() => selection.isPageSelected(pagination.pagedItems.value))
const selectedUsersForDelete = computed(() =>
  selection.selectedIds.value
    .map(id => accessControlStore.users.find(user => user.id === id) ?? null)
    .filter((user): user is NonNullable<typeof user> => Boolean(user)),
)

watch(selectedUser, (user) => {
  if (!user) {
    Object.assign(editForm, createEmptyForm())
    return
  }

  Object.assign(editForm, {
    username: user.username,
    displayName: user.displayName,
    status: user.status,
    password: '',
    confirmPassword: '',
    resetPassword: false,
  } satisfies UserFormState)
}, { immediate: true })

watch(pagination.pagedItems, (records) => {
  if (selectedUserId.value && !records.some(user => user.id === selectedUserId.value)) {
    selectedUserId.value = ''
  }
}, { immediate: true })

watch(() => accessControlStore.users.map(user => [user.id, user.status] as const), (records) => {
  const nextOverrides = { ...userStatusOverrides.value }
  let changed = false

  for (const [userId, status] of Object.entries(userStatusOverrides.value)) {
    const matched = records.find(([recordId]) => recordId === userId)
    if (!matched || matched[1] === status) {
      delete nextOverrides[userId]
      changed = true
    }
  }

  if (changed) {
    userStatusOverrides.value = nextOverrides
  }
}, { immediate: true })

function createEmptyForm(): UserFormState {
  return {
    username: '',
    displayName: '',
    status: 'active',
    password: '',
    confirmPassword: '',
    resetPassword: false,
  }
}

function resetCreateForm() {
  Object.assign(createForm, createEmptyForm())
}

function selectUser(userId: string) {
  selectedUserId.value = userId
  submitError.value = ''
}

function openCreateDialog() {
  resetCreateForm()
  submitError.value = ''
  createDialogOpen.value = true
}

function closeDeleteDialog() {
  deleteDialogOpen.value = false
}

function toggleUserSelection(userId: string, value: boolean) {
  selection.toggleSelection(userId, value)
}

function togglePageSelection(value: boolean) {
  selection.selectPage(pagination.pagedItems.value, value)
}

function toRequest(form: UserFormState): AccessUserUpsertRequest {
  return {
    username: form.username.trim(),
    displayName: form.displayName.trim(),
    status: form.status,
    password: form.password || undefined,
    confirmPassword: form.confirmPassword || undefined,
    resetPassword: form.resetPassword || undefined,
  }
}

function validateForm(form: UserFormState, requirePassword: boolean) {
  if (!form.username.trim()) {
    return t('accessControl.users.validation.usernameRequired')
  }
  if (!form.displayName.trim()) {
    return t('accessControl.users.validation.displayNameRequired')
  }
  if (requirePassword && !form.password) {
    return t('accessControl.users.validation.passwordRequired')
  }
  if ((form.password || form.confirmPassword) && form.password !== form.confirmPassword) {
    return t('accessControl.users.validation.passwordMismatch')
  }
  return ''
}

function formatUserLabel(displayName: string, username: string) {
  return t('accessControl.users.feedback.toastUserLabel', { displayName, username })
}

function isUserStatusSaving(userId: string) {
  return togglingUserIds.value.includes(userId)
}

function userListStatus(userId: string, fallbackStatus: string) {
  return userStatusOverrides.value[userId] ?? fallbackStatus
}

function setUserStatusOverride(userId: string, status: string) {
  userStatusOverrides.value = {
    ...userStatusOverrides.value,
    [userId]: status,
  }
}

function clearUserStatusOverride(userId: string) {
  const { [userId]: _ignored, ...rest } = userStatusOverrides.value
  userStatusOverrides.value = rest
}

function hideUser(userId: string) {
  if (!hiddenUserIds.value.includes(userId)) {
    hiddenUserIds.value = [...hiddenUserIds.value, userId]
  }
}

async function toggleUserStatus(user: { id: string, username: string, displayName: string, status: string }, enabled: boolean) {
  const nextStatus = enabled ? 'active' : 'disabled'
  if (isUserStatusSaving(user.id) || nextStatus === user.status) {
    return
  }

  submitError.value = ''
  setUserStatusOverride(user.id, nextStatus)
  togglingUserIds.value = [...togglingUserIds.value, user.id]

  try {
    await accessControlStore.updateUser(user.id, {
      username: user.username,
      displayName: user.displayName,
      status: nextStatus,
    })
    await notifySuccess(
      t(enabled ? 'accessControl.users.feedback.toastEnabledTitle' : 'accessControl.users.feedback.toastDisabledTitle'),
      formatUserLabel(user.displayName, user.username),
    )
  } catch (error) {
    clearUserStatusOverride(user.id)
    submitError.value = error instanceof Error ? error.message : t('accessControl.users.feedback.statusUpdateFailed')
  } finally {
    togglingUserIds.value = togglingUserIds.value.filter(id => id !== user.id)
  }
}

async function handleCreate() {
  submitError.value = validateForm(createForm, true)
  if (submitError.value) {
    return
  }

  savingCreate.value = true
  try {
    const record = await accessControlStore.createUser(toRequest(createForm))
    selectedUserId.value = record.id
    createDialogOpen.value = false
    resetCreateForm()
    await notifySuccess(
      t('accessControl.users.feedback.toastSavedTitle'),
      formatUserLabel(record.displayName, record.username),
    )
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.users.feedback.saveFailed')
  } finally {
    savingCreate.value = false
  }
}

async function handleUpdate() {
  if (!selectedUser.value) {
    return
  }

  submitError.value = validateForm(editForm, false)
  if (submitError.value) {
    return
  }

  savingEdit.value = true
  try {
    const payload = toRequest(editForm)
    await accessControlStore.updateUser(selectedUser.value.id, payload)
    await notifySuccess(
      t('accessControl.users.feedback.toastSavedTitle'),
      formatUserLabel(payload.displayName, payload.username),
    )
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.users.feedback.saveFailed')
  } finally {
    savingEdit.value = false
  }
}

async function handleDelete() {
  if (!selectedUser.value) {
    return
  }

  deletingUserId.value = selectedUser.value.id
  submitError.value = ''
  try {
    const label = selectedUser.value.displayName
    await accessControlStore.deleteUser(selectedUser.value.id)
    hideUser(selectedUser.value.id)
    selectedUserId.value = ''
    deleteDialogOpen.value = false
    await notifySuccess(
      t('accessControl.users.feedback.toastDeletedTitle'),
      t('accessControl.users.feedback.toastDeletedBody', { displayName: label }),
    )
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.users.feedback.deleteFailed')
  } finally {
    deletingUserId.value = ''
  }
}

async function handleBulkDelete() {
  if (!selectedUsersForDelete.value.length) {
    bulkDeleteDialogOpen.value = false
    return
  }

  deletingSelectedUsers.value = true
  submitError.value = ''
  let successCount = 0
  let failureCount = 0

  for (const user of selectedUsersForDelete.value) {
    try {
      await accessControlStore.deleteUser(user.id)
      hideUser(user.id)
      successCount += 1
      if (selectedUserId.value === user.id) {
        selectedUserId.value = ''
      }
    } catch {
      failureCount += 1
    }
  }

  deletingSelectedUsers.value = false
  bulkDeleteDialogOpen.value = false
  await accessControlStore.reloadAll()
  selection.setSelection(
    selection.selectedIds.value.filter(id =>
      accessControlStore.users.some(user => user.id === id) && !hiddenUserIds.value.includes(id),
    ),
  )
  await nextTick()
  await new Promise(resolve => window.setTimeout(resolve, 0))

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

function labelValues(ids: string[], labelMap: Map<string, string>) {
  return ids.map(id => labelMap.get(id) ?? id)
}

function userOrgLabels(userId: string) {
  const assignments = assignmentsByUserId.value.get(userId) ?? []
  return assignments.map((assignment) => orgUnitMap.value.get(assignment.orgUnitId) ?? assignment.orgUnitId)
}

function userPositionAndGroupLabels(userId: string) {
  const assignments = assignmentsByUserId.value.get(userId) ?? []
  return assignments.flatMap((assignment) => [
    ...labelValues(assignment.positionIds, positionMap.value),
    ...labelValues(assignment.userGroupIds, groupMap.value),
  ])
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-users-shell">
    <UiStatusCallout
      v-if="submitError"
      tone="error"
      :description="submitError"
    />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedUser)"
      :detail-title="selectedUser ? selectedUser.displayName : ''"
      :detail-subtitle="t('accessControl.users.detail.subtitle')"
      :empty-detail-title="t('accessControl.users.detail.emptyTitle')"
      :empty-detail-description="t('accessControl.users.detail.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-users-toolbar">
          <template #search>
            <UiInput
              v-model="query"
              :placeholder="t('accessControl.users.toolbar.search')"
            />
          </template>
          <template #filters>
              <UiField :label="t('accessControl.users.toolbar.status')" class="w-full md:w-[180px]">
              <UiSelect v-model="statusFilter" :options="statusFilterOptions" />
            </UiField>
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
              data-testid="access-control-user-select-page-button"
              @click="togglePageSelection(!allPageSelected)"
            >
              {{ t('accessControl.common.selection.selectPage') }}
            </UiButton>
            <UiButton
              v-if="selection.hasSelection.value"
              variant="ghost"
              size="sm"
              data-testid="access-control-user-clear-selection-button"
              @click="selection.clearSelection"
            >
              {{ t('accessControl.common.selection.clear') }}
            </UiButton>
            <UiButton
              v-if="selection.hasSelection.value"
              variant="destructive"
              size="sm"
              data-testid="access-control-user-bulk-delete-button"
              @click="bulkDeleteDialogOpen = true"
            >
              {{ t('accessControl.common.bulk.delete') }}
            </UiButton>
            <UiButton data-testid="access-control-user-create-button" size="sm" @click="openCreateDialog">
              {{ t('accessControl.users.toolbar.create') }}
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.users.list.title')"
          :subtitle="t('accessControl.common.list.totalUsers', { count: pagination.totalItems.value })"
        >
          <div v-if="pagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="user in pagination.pagedItems.value"
              :key="user.id"
              layout="compact"
              interactive
              :active="selectedUserId === user.id"
              :title="user.displayName"
              :description="user.username"
              :test-id="`access-control-user-record-${user.id}`"
              @click="selectUser(user.id)"
            >
              <template #badges>
                <div class="flex items-center gap-2" @click.stop>
                  <UiCheckbox
                    :model-value="selection.isSelected(user.id)"
                    :data-testid="`access-control-user-select-${user.id}`"
                    @update:model-value="toggleUserSelection(user.id, Boolean($event))"
                  />
                  <UiSwitch
                    :model-value="userListStatus(user.id, user.status) === 'active'"
                    :disabled="isUserStatusSaving(user.id)"
                    @update:model-value="toggleUserStatus(user, $event)"
                  >
                    <span class="sr-only">
                      {{ t('accessControl.users.list.toggleStatus', { name: user.displayName }) }}
                    </span>
                  </UiSwitch>
                </div>
              </template>
              <template #meta>
                <span class="truncate text-xs text-text-secondary">
                  {{ formatList(roleNamesByUserId.get(user.id) ?? []) || t('accessControl.common.list.noRoles') }}
                </span>
              </template>
            </UiRecordCard>
          </div>
          <UiEmptyState
            v-else
            :title="t('accessControl.users.list.emptyTitle')"
            :description="t('accessControl.users.list.emptyDescription')"
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
        <div v-if="selectedUser" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedUser.displayName }}</div>
              <UiBadge :label="getStatusLabel(t, selectedUser.status)" :tone="selectedUser.status === 'active' ? 'success' : 'default'" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedUser.username }}</div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">{{ t('accessControl.users.detail.orgSummary') }}</div>
              <div class="mt-2 text-sm text-foreground">{{ formatList(userOrgLabels(selectedUser.id)) || t('accessControl.common.list.notSet') }}</div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">{{ t('accessControl.users.detail.positionGroupSummary') }}</div>
              <div class="mt-2 text-sm text-foreground">{{ formatList(userPositionAndGroupLabels(selectedUser.id)) || t('accessControl.common.list.notSet') }}</div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4 md:col-span-2">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">{{ t('accessControl.users.detail.projectSummary') }}</div>
              <div class="mt-2 text-sm text-foreground">{{ formatList(directProjectPoliciesByUserId.get(selectedUser.id) ?? []) || t('accessControl.common.list.noDirectProjects') }}</div>
            </div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.users.fields.username')">
              <UiInput v-model="editForm.username" data-testid="access-control-user-form-username" />
            </UiField>
            <UiField :label="t('accessControl.users.fields.displayName')">
              <UiInput v-model="editForm.displayName" data-testid="access-control-user-form-display-name" />
            </UiField>
            <UiField :label="t('accessControl.users.fields.status')">
              <UiSelect v-model="editForm.status" :options="statusOptions" data-testid="access-control-user-form-status" />
            </UiField>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.users.fields.newPassword')">
              <UiInput v-model="editForm.password" type="password" data-testid="access-control-user-form-password" />
            </UiField>
            <UiField :label="t('accessControl.users.fields.confirmNewPassword')">
              <UiInput v-model="editForm.confirmPassword" type="password" data-testid="access-control-user-form-confirm-password" />
            </UiField>
          </div>

          <div class="rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
            <UiCheckbox
              v-model="editForm.resetPassword"
              data-testid="access-control-user-form-reset-password"
            >
              {{ t('accessControl.users.fields.resetPassword') }}
            </UiCheckbox>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton
              variant="ghost"
              class="text-destructive"
              @click="deleteDialogOpen = true"
            >
              {{ t('accessControl.users.actions.delete') }}
            </UiButton>
            <UiButton
              :loading="savingEdit"
              data-testid="access-control-user-form-save"
              @click="handleUpdate"
            >
              {{ t('accessControl.users.actions.save') }}
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      :open="createDialogOpen"
      :title="t('accessControl.users.dialogs.createTitle')"
      :description="t('accessControl.users.dialogs.createDescription')"
      @update:open="createDialogOpen = $event"
    >
      <div class="space-y-4">
        <div class="grid gap-3 md:grid-cols-2">
          <UiField :label="t('accessControl.users.fields.username')">
            <UiInput v-model="createForm.username" data-testid="access-control-user-form-username" />
          </UiField>
          <UiField :label="t('accessControl.users.fields.displayName')">
            <UiInput v-model="createForm.displayName" data-testid="access-control-user-form-display-name" />
          </UiField>
          <UiField :label="t('accessControl.users.fields.status')">
            <UiSelect v-model="createForm.status" :options="statusOptions" data-testid="access-control-user-form-status" />
          </UiField>
        </div>

        <div class="grid gap-3 md:grid-cols-2">
          <UiField :label="t('accessControl.users.fields.password')">
            <UiInput v-model="createForm.password" type="password" data-testid="access-control-user-form-password" />
          </UiField>
          <UiField :label="t('accessControl.users.fields.confirmPassword')">
            <UiInput v-model="createForm.confirmPassword" type="password" data-testid="access-control-user-form-confirm-password" />
          </UiField>
        </div>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          :loading="savingCreate"
          data-testid="access-control-user-form-save"
          @click="handleCreate"
        >
          {{ t('accessControl.users.actions.create') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="bulkDeleteDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.users') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeleteDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedUsersForDelete.length,
          entity: t('accessControl.common.entities.users'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeleteDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingSelectedUsers"
          data-testid="access-control-user-bulk-delete-confirm"
          @click="handleBulkDelete"
        >
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="deleteDialogOpen"
      :title="t('accessControl.users.dialogs.deleteTitle')"
      :description="t('accessControl.users.dialogs.deleteDescription')"
      @update:open="deleteDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.users.dialogs.deleteConfirm', { name: selectedUser?.displayName ?? t('common.na') }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="closeDeleteDialog">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingUserId === selectedUser?.id"
          data-testid="access-control-user-delete-confirm"
          @click="handleDelete"
        >
          {{ t('common.delete') }}
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>
