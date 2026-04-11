<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPanelFrame,
  UiSelect,
  UiStatusCallout,
  UiToolbarRow,
} from '@octopus/ui'

import type { AccessUserRecord, AccessUserUpsertRequest } from '@octopus/schema'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { statusOptions } from './helpers'

interface UserFormState {
  username: string
  displayName: string
  status: string
  password: string
  confirmPassword: string
  resetPassword: boolean
}

const accessControlStore = useWorkspaceAccessControlStore()

const selectedUserId = ref('')
const query = ref('')
const statusFilter = ref('')
const createDialogOpen = ref(false)
const deleteDialogOpen = ref(false)
const submitError = ref('')
const successMessage = ref('')
const savingCreate = ref(false)
const savingEdit = ref(false)
const deletingUserId = ref('')

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
    labels.push(policy.scopeType === 'selected-projects' ? (policy.projectIds.join('、') || policy.name) : policy.scopeType)
    grouped.set(policy.subjectId, labels)
  }
  return grouped
})

const users = computed(() =>
  [...accessControlStore.users]
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

watch(users, (records) => {
  if (selectedUserId.value && !records.some(user => user.id === selectedUserId.value)) {
    selectedUserId.value = ''
  }
})

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
  successMessage.value = ''
}

function openCreateDialog() {
  resetCreateForm()
  submitError.value = ''
  successMessage.value = ''
  createDialogOpen.value = true
}

function closeDeleteDialog() {
  deleteDialogOpen.value = false
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
    return '请输入账号名。'
  }
  if (!form.displayName.trim()) {
    return '请输入显示名称。'
  }
  if (requirePassword && !form.password) {
    return '新建用户时必须设置密码。'
  }
  if ((form.password || form.confirmPassword) && form.password !== form.confirmPassword) {
    return '两次输入的密码不一致。'
  }
  return ''
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
    successMessage.value = `已保存用户 ${record.displayName}（${record.username}）`
    createDialogOpen.value = false
    resetCreateForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存用户失败。'
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
    successMessage.value = `已保存用户 ${payload.displayName}（${payload.username}）`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存用户失败。'
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
    selectedUserId.value = ''
    deleteDialogOpen.value = false
    successMessage.value = `已删除用户 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除用户失败。'
  } finally {
    deletingUserId.value = ''
  }
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

const statusFilterOptions = computed(() => [
  { label: '全部状态', value: '' },
  ...statusOptions,
])
</script>

<template>
  <div class="space-y-4" data-testid="access-control-users-shell">
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

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedUser)"
      :detail-title="selectedUser ? selectedUser.displayName : ''"
      detail-subtitle="维护当前用户的身份信息、密码生命周期与访问摘要。"
      empty-detail-title="请选择用户"
      empty-detail-description="从左侧用户列表中选择一项后即可查看详情，或点击右上角新建用户。"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-users-toolbar">
          <template #search>
            <UiInput
              v-model="query"
              placeholder="搜索姓名、账号、角色或组织"
            />
          </template>
          <template #filters>
            <UiField label="状态" class="w-full md:w-[180px]">
              <UiSelect v-model="statusFilter" :options="statusFilterOptions" />
            </UiField>
          </template>
          <template #actions>
            <UiButton data-testid="access-control-user-create-button" size="sm" @click="openCreateDialog">
              新建用户
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          title="用户列表"
          :subtitle="`共 ${users.length} 位用户`"
        >
          <div v-if="users.length" class="space-y-2">
            <button
              v-for="user in users"
              :key="user.id"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedUserId === user.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              @click="selectUser(user.id)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 space-y-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm font-semibold text-foreground">{{ user.displayName }}</span>
                    <UiBadge :label="user.status" :tone="user.status === 'active' ? 'success' : 'default'" subtle />
                  </div>
                  <p class="truncate text-xs text-muted-foreground">{{ user.username }}</p>
                  <p class="truncate text-xs text-muted-foreground">
                    {{ (roleNamesByUserId.get(user.id) ?? []).join('、') || '未绑定角色' }}
                  </p>
                </div>
                <UiBadge :label="user.passwordState" subtle />
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无用户" description="当前筛选条件下没有用户记录。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedUser" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedUser.displayName }}</div>
              <UiBadge :label="selectedUser.status" :tone="selectedUser.status === 'active' ? 'success' : 'default'" subtle />
              <UiBadge :label="selectedUser.passwordState" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedUser.username }}</div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">组织归属</div>
              <div class="mt-2 text-sm text-foreground">{{ userOrgLabels(selectedUser.id).join('、') || '未设置' }}</div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">岗位 / 用户组</div>
              <div class="mt-2 text-sm text-foreground">{{ userPositionAndGroupLabels(selectedUser.id).join('、') || '未设置' }}</div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4 md:col-span-2">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">直接项目范围</div>
              <div class="mt-2 text-sm text-foreground">{{ (directProjectPoliciesByUserId.get(selectedUser.id) ?? []).join('；') || '未设置直接项目策略' }}</div>
            </div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="账号名">
              <UiInput v-model="editForm.username" data-testid="access-control-user-form-username" />
            </UiField>
            <UiField label="显示名称">
              <UiInput v-model="editForm.displayName" data-testid="access-control-user-form-display-name" />
            </UiField>
            <UiField label="状态">
              <UiSelect v-model="editForm.status" :options="statusOptions" data-testid="access-control-user-form-status" />
            </UiField>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="新密码">
              <UiInput v-model="editForm.password" type="password" data-testid="access-control-user-form-password" />
            </UiField>
            <UiField label="确认新密码">
              <UiInput v-model="editForm.confirmPassword" type="password" data-testid="access-control-user-form-confirm-password" />
            </UiField>
          </div>

          <div class="rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
            <UiCheckbox
              v-model="editForm.resetPassword"
              data-testid="access-control-user-form-reset-password"
            >
              下次登录重置密码
            </UiCheckbox>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton
              variant="ghost"
              class="text-destructive"
              @click="deleteDialogOpen = true"
            >
              删除用户
            </UiButton>
            <UiButton
              :loading="savingEdit"
              data-testid="access-control-user-form-save"
              @click="handleUpdate"
            >
              保存用户
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      :open="createDialogOpen"
      title="新建用户"
      description="创建用户身份并设置初始密码。组织归属和角色绑定在其它页面维护。"
      @update:open="createDialogOpen = $event"
    >
      <div class="space-y-4">
        <div class="grid gap-3 md:grid-cols-2">
          <UiField label="账号名">
            <UiInput v-model="createForm.username" data-testid="access-control-user-form-username" />
          </UiField>
          <UiField label="显示名称">
            <UiInput v-model="createForm.displayName" data-testid="access-control-user-form-display-name" />
          </UiField>
          <UiField label="状态">
            <UiSelect v-model="createForm.status" :options="statusOptions" data-testid="access-control-user-form-status" />
          </UiField>
        </div>

        <div class="grid gap-3 md:grid-cols-2">
          <UiField label="密码">
            <UiInput v-model="createForm.password" type="password" data-testid="access-control-user-form-password" />
          </UiField>
          <UiField label="确认密码">
            <UiInput v-model="createForm.confirmPassword" type="password" data-testid="access-control-user-form-confirm-password" />
          </UiField>
        </div>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createDialogOpen = false">
          取消
        </UiButton>
        <UiButton
          :loading="savingCreate"
          data-testid="access-control-user-form-save"
          @click="handleCreate"
        >
          创建用户
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="deleteDialogOpen"
      title="删除用户"
      description="删除后该用户在当前工作区的身份记录会被移除，此操作不可撤销。"
      @update:open="deleteDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        确认删除
        <span class="font-semibold text-text-primary">{{ selectedUser?.displayName ?? '当前用户' }}</span>
        吗？
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="closeDeleteDialog">
          取消
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingUserId === selectedUser?.id"
          data-testid="access-control-user-delete-confirm"
          @click="handleDelete"
        >
          删除
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>
