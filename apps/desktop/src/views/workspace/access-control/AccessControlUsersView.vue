<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiField,
  UiInput,
  UiPanelFrame,
  UiSelect,
  UiStatTile,
  UiStatusCallout,
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
  [...accessControlStore.users].sort((left, right) =>
    left.displayName.localeCompare(right.displayName),
  ),
)
const metrics = computed(() => ({
  total: users.value.length,
  active: users.value.filter(user => user.status === 'active').length,
  scoped: users.value.filter(user => (directProjectPoliciesByUserId.value.get(user.id) ?? []).length > 0).length,
}))

const editingUserId = ref('')
const saving = ref(false)
const deletingUserId = ref('')
const submitError = ref('')
const successMessage = ref('')

const form = reactive<UserFormState>(createEmptyForm())

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

function resetForm() {
  Object.assign(form, createEmptyForm())
  editingUserId.value = ''
  submitError.value = ''
}

function populateForm(user: AccessUserRecord) {
  editingUserId.value = user.id
  submitError.value = ''
  successMessage.value = ''
  Object.assign(form, {
    username: user.username,
    displayName: user.displayName,
    status: user.status,
    password: '',
    confirmPassword: '',
    resetPassword: false,
  } satisfies UserFormState)
}

function toRequest(): AccessUserUpsertRequest {
  const payload: AccessUserUpsertRequest = {
    username: form.username.trim(),
    displayName: form.displayName.trim(),
    status: form.status,
    password: form.password || undefined,
    confirmPassword: form.confirmPassword || undefined,
    resetPassword: form.resetPassword || undefined,
  }
  return payload
}

function validateForm() {
  if (!form.username.trim()) {
    return '请输入账号名。'
  }
  if (!form.displayName.trim()) {
    return '请输入显示名称。'
  }
  if (!editingUserId.value && !form.password) {
    return '新建用户时必须设置密码。'
  }
  if ((form.password || form.confirmPassword) && form.password !== form.confirmPassword) {
    return '两次输入的密码不一致。'
  }
  return ''
}

async function handleSave() {
  submitError.value = validateForm()
  if (submitError.value) {
    return
  }

  saving.value = true
  try {
    const payload = toRequest()
    if (editingUserId.value) {
      await accessControlStore.updateUser(editingUserId.value, payload)
      successMessage.value = `已保存用户 ${payload.displayName}（${payload.username}）`
    } else {
      await accessControlStore.createUser(payload)
      successMessage.value = `已保存用户 ${payload.displayName}（${payload.username}）`
    }
    resetForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存用户失败。'
  } finally {
    saving.value = false
  }
}

async function handleDelete(userId: string) {
  deletingUserId.value = userId
  submitError.value = ''
  try {
    const user = accessControlStore.users.find(record => record.id === userId)
    await accessControlStore.deleteUser(userId)
    successMessage.value = user ? `已删除用户 ${user.displayName}` : '已删除用户'
    if (editingUserId.value === userId) {
      resetForm()
    }
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
</script>

<template>
  <div class="space-y-4" data-testid="access-control-users-shell">
    <section class="grid gap-4 md:grid-cols-3">
      <UiStatTile label="用户总数" :value="String(metrics.total)" />
      <UiStatTile label="启用用户" :value="String(metrics.active)" tone="success" />
      <UiStatTile label="受限范围用户" :value="String(metrics.scoped)" tone="warning" />
    </section>

    <div class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)]">
      <UiPanelFrame variant="panel" padding="md" title="用户清单" subtitle="用户、组织归属、角色绑定都以当前工作区为边界。">
        <div v-if="users.length" class="space-y-3">
          <article
            v-for="user in users"
            :key="user.id"
            class="rounded-[var(--radius-l)] border border-border bg-card p-4"
          >
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="text-sm font-semibold text-foreground">{{ user.displayName }}</h3>
                  <UiBadge :label="user.status" :tone="user.status === 'active' ? 'success' : 'default'" subtle />
                  <UiBadge :label="user.passwordState" subtle />
                </div>
                <p class="text-xs text-muted-foreground">{{ user.username }}</p>
              </div>
              <div class="flex gap-2">
                <UiButton
                  size="sm"
                  variant="ghost"
                  data-testid="access-control-user-edit"
                  @click="populateForm(user)"
                >
                  编辑
                </UiButton>
                <UiButton
                  size="sm"
                  variant="ghost"
                  class="text-destructive"
                  :loading="deletingUserId === user.id"
                  data-testid="access-control-user-delete"
                  @click="handleDelete(user.id)"
                >
                  删除
                </UiButton>
              </div>
            </div>

            <div class="mt-3 flex flex-wrap gap-2">
              <UiBadge
                v-for="role in roleNamesByUserId.get(user.id) ?? []"
                :key="`${user.id}:${role}`"
                :label="role"
                subtle
              />
            </div>

            <div class="mt-3 grid gap-3 text-xs text-muted-foreground md:grid-cols-2">
              <div>
                <span class="font-medium text-foreground">组织归属</span>
                <p class="mt-1">{{ userOrgLabels(user.id).join('、') || '未设置' }}</p>
              </div>
              <div>
                <span class="font-medium text-foreground">岗位 / 用户组</span>
                <p class="mt-1">
                  {{ userPositionAndGroupLabels(user.id).join('、') || '未设置' }}
                </p>
              </div>
              <div class="md:col-span-2">
                <span class="font-medium text-foreground">直接项目策略</span>
                <p class="mt-1">{{ (directProjectPoliciesByUserId.get(user.id) ?? []).join('；') || '未设置直接项目策略' }}</p>
              </div>
            </div>
          </article>
        </div>
        <UiEmptyState
          v-else
          title="暂无用户"
          description="当前工作区还没有用户记录。"
        />
      </UiPanelFrame>

      <UiPanelFrame
        variant="panel"
        padding="md"
        :title="editingUserId ? '编辑用户' : '新建用户'"
        subtitle="这里只维护用户身份、状态和密码生命周期。组织、角色和数据访问策略分别在组织管理与权限策略中维护。"
      >
        <div class="space-y-4">
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

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="账号名">
              <UiInput v-model="form.username" data-testid="access-control-user-form-username" />
            </UiField>
            <UiField label="显示名称">
              <UiInput v-model="form.displayName" data-testid="access-control-user-form-display-name" />
            </UiField>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="状态">
              <UiSelect v-model="form.status" :options="statusOptions" data-testid="access-control-user-form-status" />
            </UiField>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="editingUserId ? '新密码' : '密码'">
              <UiInput v-model="form.password" type="password" data-testid="access-control-user-form-password" />
            </UiField>
            <UiField :label="editingUserId ? '确认新密码' : '确认密码'">
              <UiInput v-model="form.confirmPassword" type="password" data-testid="access-control-user-form-confirm-password" />
            </UiField>
          </div>

          <div v-if="editingUserId" class="rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
            <UiCheckbox
              v-model="form.resetPassword"
              data-testid="access-control-user-form-reset-password"
            >
              下次登录重置密码
            </UiCheckbox>
          </div>

          <div class="flex flex-wrap justify-end gap-2">
            <UiButton variant="ghost" data-testid="access-control-user-form-reset" @click="resetForm">
              重置
            </UiButton>
            <UiButton
              :loading="saving"
              data-testid="access-control-user-form-save"
              @click="handleSave"
            >
              {{ editingUserId ? '保存用户' : '创建用户' }}
            </UiButton>
          </div>
        </div>
      </UiPanelFrame>
    </div>
  </div>
</template>
