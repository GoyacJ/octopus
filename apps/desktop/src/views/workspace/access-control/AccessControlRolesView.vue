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
  UiTextarea,
} from '@octopus/ui'

import type { AccessRoleRecord, RoleUpsertRequest } from '@octopus/schema'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { statusOptions } from './helpers'

const accessControlStore = useWorkspaceAccessControlStore()

const metrics = computed(() => ({
  total: accessControlStore.roles.length,
  bindings: accessControlStore.roleBindings.length,
  current: accessControlStore.currentRoleNames.length,
}))

const editingRoleId = ref('')
const saving = ref(false)
const deletingRoleId = ref('')
const submitError = ref('')

const form = reactive({
  code: '',
  name: '',
  description: '',
  status: 'active',
  permissionCodes: [] as string[],
})

function resetForm() {
  Object.assign(form, {
    code: '',
    name: '',
    description: '',
    status: 'active',
    permissionCodes: [] as string[],
  })
  editingRoleId.value = ''
  submitError.value = ''
}

function populateForm(role: AccessRoleRecord) {
  Object.assign(form, {
    code: role.code,
    name: role.name,
    description: role.description,
    status: role.status,
    permissionCodes: [...role.permissionCodes],
  })
  editingRoleId.value = role.id
  submitError.value = ''
}

async function handleSave() {
  submitError.value = ''
  if (!form.code.trim() || !form.name.trim()) {
    submitError.value = '请填写完整的角色编码和名称。'
    return
  }

  saving.value = true
  try {
    const payload: RoleUpsertRequest = {
      code: form.code.trim(),
      name: form.name.trim(),
      description: form.description.trim(),
      status: form.status,
      permissionCodes: [...form.permissionCodes],
    }
    if (editingRoleId.value) {
      await accessControlStore.updateRole(editingRoleId.value, payload)
    } else {
      await accessControlStore.createRole(payload)
    }
    resetForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存角色失败。'
  } finally {
    saving.value = false
  }
}

async function handleDelete(roleId: string) {
  deletingRoleId.value = roleId
  submitError.value = ''
  try {
    await accessControlStore.deleteRole(roleId)
    if (editingRoleId.value === roleId) {
      resetForm()
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除角色失败。'
  } finally {
    deletingRoleId.value = ''
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-roles-shell">
    <section class="grid gap-4 md:grid-cols-3">
      <UiStatTile label="角色总数" :value="String(metrics.total)" />
      <UiStatTile label="绑定数量" :value="String(metrics.bindings)" />
      <UiStatTile label="当前有效角色" :value="String(metrics.current)" tone="info" />
    </section>

    <UiStatusCallout
      v-if="submitError"
      tone="error"
      :description="submitError"
    />

    <div class="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)]">
      <UiPanelFrame variant="panel" padding="md" title="角色清单" subtitle="角色负责聚合系统预置 capability，对外授权由后端统一判定。">
        <div v-if="accessControlStore.roles.length" class="space-y-3">
          <article
            v-for="role in accessControlStore.roles"
            :key="role.id"
            class="rounded-[12px] border border-border bg-card p-4"
          >
            <div class="flex items-start justify-between gap-3">
              <div>
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="text-sm font-semibold text-foreground">{{ role.name }}</h3>
                  <UiBadge :label="role.status" subtle />
                </div>
                <p class="text-xs text-muted-foreground">{{ role.code }}</p>
              </div>
              <div class="flex gap-2">
                <UiButton size="sm" variant="ghost" data-testid="access-control-role-edit" @click="populateForm(role)">编辑</UiButton>
                <UiButton
                  size="sm"
                  variant="ghost"
                  class="text-destructive"
                  :loading="deletingRoleId === role.id"
                  data-testid="access-control-role-delete"
                  @click="handleDelete(role.id)"
                >
                  删除
                </UiButton>
              </div>
            </div>
            <p class="mt-2 text-sm text-muted-foreground">{{ role.description || '暂无描述' }}</p>
            <div class="mt-3 flex flex-wrap gap-2">
              <UiBadge
                v-for="permissionCode in role.permissionCodes"
                :key="`${role.id}:${permissionCode}`"
                :label="permissionCode"
                subtle
              />
            </div>
          </article>
        </div>
        <UiEmptyState v-else title="暂无角色" description="当前工作区没有角色记录。" />
      </UiPanelFrame>

      <UiPanelFrame
        variant="panel"
        padding="md"
        :title="editingRoleId ? '编辑角色' : '新建角色'"
        subtitle="角色页只维护角色定义和 capability 绑定；主体绑定放到权限与策略页。"
      >
        <div class="space-y-4">
          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="角色编码">
              <UiInput v-model="form.code" data-testid="access-control-role-form-code" />
            </UiField>
            <UiField label="角色名称">
              <UiInput v-model="form.name" data-testid="access-control-role-form-name" />
            </UiField>
          </div>
          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="状态">
              <UiSelect v-model="form.status" :options="statusOptions" data-testid="access-control-role-form-status" />
            </UiField>
            <UiField label="角色描述">
              <UiTextarea v-model="form.description" :rows="3" data-testid="access-control-role-form-description" />
            </UiField>
          </div>

          <UiField label="权限目录绑定">
            <div class="grid gap-2 rounded-[8px] border border-border bg-muted/35 p-3">
              <UiCheckbox
                v-for="permission in accessControlStore.permissionDefinitions"
                :key="permission.code"
                v-model="form.permissionCodes"
                :value="permission.code"
              >
                {{ permission.name }} <span class="text-xs text-muted-foreground">({{ permission.code }})</span>
              </UiCheckbox>
            </div>
          </UiField>

          <div class="flex justify-end gap-2">
            <UiButton variant="ghost" @click="resetForm">重置</UiButton>
            <UiButton :loading="saving" data-testid="access-control-role-form-save" @click="handleSave">
              {{ editingRoleId ? '保存角色' : '创建角色' }}
            </UiButton>
          </div>
        </div>
      </UiPanelFrame>
    </div>
  </div>
</template>
