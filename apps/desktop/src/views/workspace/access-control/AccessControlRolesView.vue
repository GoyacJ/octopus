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
  UiTextarea,
  UiToolbarRow,
} from '@octopus/ui'

import type { AccessRoleRecord, RoleUpsertRequest } from '@octopus/schema'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { statusOptions } from './helpers'

interface RoleFormState {
  code: string
  name: string
  description: string
  status: string
  permissionCodes: string[]
}

const accessControlStore = useWorkspaceAccessControlStore()

const selectedRoleId = ref('')
const query = ref('')
const createDialogOpen = ref(false)
const deleteDialogOpen = ref(false)
const submitError = ref('')
const successMessage = ref('')
const savingCreate = ref(false)
const savingEdit = ref(false)
const deletingRoleId = ref('')

const createForm = reactive<RoleFormState>(createEmptyForm())
const editForm = reactive<RoleFormState>(createEmptyForm())

const filteredRoles = computed(() => {
  const normalizedQuery = query.value.trim().toLowerCase()
  return [...accessControlStore.roles]
    .sort((left, right) => left.name.localeCompare(right.name))
    .filter(role => !normalizedQuery || [
      role.name,
      role.code,
      role.description,
      ...role.permissionCodes,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const selectedRole = computed(() =>
  accessControlStore.roles.find(role => role.id === selectedRoleId.value) ?? null,
)

watch(selectedRole, (role) => {
  if (!role) {
    Object.assign(editForm, createEmptyForm())
    return
  }

  Object.assign(editForm, {
    code: role.code,
    name: role.name,
    description: role.description,
    status: role.status,
    permissionCodes: [...role.permissionCodes],
  } satisfies RoleFormState)
}, { immediate: true })

watch(filteredRoles, (roles) => {
  if (selectedRoleId.value && !roles.some(role => role.id === selectedRoleId.value)) {
    selectedRoleId.value = ''
  }
})

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
}

function selectRole(roleId: string) {
  selectedRoleId.value = roleId
  submitError.value = ''
  successMessage.value = ''
}

function validateForm(form: RoleFormState) {
  if (!form.code.trim() || !form.name.trim()) {
    return '请填写完整的角色编码和名称。'
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
    successMessage.value = `已保存角色 ${record.name}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存角色失败。'
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
    successMessage.value = `已保存角色 ${payload.name}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存角色失败。'
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
    const label = selectedRole.value.name
    await accessControlStore.deleteRole(selectedRole.value.id)
    selectedRoleId.value = ''
    deleteDialogOpen.value = false
    successMessage.value = `已删除角色 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除角色失败。'
  } finally {
    deletingRoleId.value = ''
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-roles-shell">
    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />
    <UiStatusCallout v-if="successMessage" tone="success" :description="successMessage" />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedRole)"
      :detail-title="selectedRole ? selectedRole.name : ''"
      detail-subtitle="编辑角色定义与 capability 绑定；主体绑定在权限与策略页维护。"
      empty-detail-title="请选择角色"
      empty-detail-description="从左侧角色列表中选择一项后即可查看详情，或点击右上角创建角色。"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-roles-toolbar">
          <template #search>
            <UiInput v-model="query" placeholder="搜索角色名称、编码或权限" />
          </template>
          <template #actions>
            <UiButton data-testid="access-control-role-create-button" size="sm" @click="createDialogOpen = true">
              新建角色
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="角色列表" :subtitle="`共 ${filteredRoles.length} 个角色`">
          <div v-if="filteredRoles.length" class="space-y-2">
            <button
              v-for="role in filteredRoles"
              :key="role.id"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedRoleId === role.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              @click="selectRole(role.id)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 space-y-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm font-semibold text-foreground">{{ role.name }}</span>
                    <UiBadge :label="role.status" subtle />
                  </div>
                  <p class="text-xs text-muted-foreground">{{ role.code }}</p>
                  <p class="truncate text-xs text-muted-foreground">{{ role.permissionCodes.length }} 个 capability</p>
                </div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无角色" description="当前筛选条件下没有角色记录。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedRole" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedRole.name }}</div>
              <UiBadge :label="selectedRole.status" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedRole.code }}</div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="角色编码">
              <UiInput v-model="editForm.code" />
            </UiField>
            <UiField label="角色名称">
              <UiInput v-model="editForm.name" />
            </UiField>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="状态">
              <UiSelect v-model="editForm.status" :options="statusOptions" />
            </UiField>
            <UiField label="角色描述">
              <UiTextarea v-model="editForm.description" :rows="3" />
            </UiField>
          </div>

          <UiField label="权限目录绑定">
            <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
              <UiCheckbox
                v-for="permission in accessControlStore.permissionDefinitions"
                :key="permission.code"
                v-model="editForm.permissionCodes"
                :value="permission.code"
              >
                {{ permission.name }} <span class="text-xs text-muted-foreground">({{ permission.code }})</span>
              </UiCheckbox>
            </div>
          </UiField>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton variant="ghost" class="text-destructive" @click="deleteDialogOpen = true">
              删除角色
            </UiButton>
            <UiButton :loading="savingEdit" @click="handleUpdate">
              保存角色
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      :open="createDialogOpen"
      title="新建角色"
      description="创建角色定义并绑定 capability，主体绑定放到权限与策略页维护。"
      @update:open="createDialogOpen = $event"
    >
      <div class="space-y-4">
        <div class="grid gap-3 md:grid-cols-2">
          <UiField label="角色编码">
            <UiInput v-model="createForm.code" />
          </UiField>
          <UiField label="角色名称">
            <UiInput v-model="createForm.name" />
          </UiField>
        </div>
        <div class="grid gap-3 md:grid-cols-2">
          <UiField label="状态">
            <UiSelect v-model="createForm.status" :options="statusOptions" />
          </UiField>
          <UiField label="角色描述">
            <UiTextarea v-model="createForm.description" :rows="3" />
          </UiField>
        </div>
        <UiField label="权限目录绑定">
          <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
            <UiCheckbox
              v-for="permission in accessControlStore.permissionDefinitions"
              :key="permission.code"
              v-model="createForm.permissionCodes"
              :value="permission.code"
            >
              {{ permission.name }} <span class="text-xs text-muted-foreground">({{ permission.code }})</span>
            </UiCheckbox>
          </div>
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createDialogOpen = false">
          取消
        </UiButton>
        <UiButton :loading="savingCreate" @click="handleCreate">
          创建角色
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="deleteDialogOpen"
      title="删除角色"
      description="删除后主体与角色的关联将失去目标角色，请谨慎操作。"
      @update:open="deleteDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        确认删除
        <span class="font-semibold text-text-primary">{{ selectedRole?.name ?? '当前角色' }}</span>
        吗？
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="deleteDialogOpen = false">
          取消
        </UiButton>
        <UiButton variant="destructive" :loading="deletingRoleId === selectedRole?.id" @click="handleDelete">
          删除
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>
