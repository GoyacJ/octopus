<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { PermissionRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiField, UiInput, UiRecordCard, UiSelect, UiTextarea } from '@octopus/ui'

import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const userCenterStore = useUserCenterStore()
const workspaceStore = useWorkspaceStore()

const selectedPermissionId = ref('')
const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active',
  kind: 'atomic',
  targetType: 'project',
  targetIds: '',
  action: 'view',
  memberPermissionIds: '',
})

const statusOptions = [
  { value: 'active', label: 'active' },
  { value: 'disabled', label: 'disabled' },
]

const kindOptions = [
  { value: 'atomic', label: 'atomic' },
  { value: 'bundle', label: 'bundle' },
]

const targetTypeOptions = [
  { value: 'workspace', label: 'workspace' },
  { value: 'project', label: 'project' },
  { value: 'agent', label: 'agent' },
  { value: 'tool', label: 'tool' },
  { value: 'skill', label: 'skill' },
  { value: 'mcp', label: 'mcp' },
]

const metrics = computed(() => [
  { id: 'total', label: t('userCenter.permissions.metrics.total'), value: String(userCenterStore.permissions.length) },
  { id: 'bundle', label: t('userCenter.permissions.metrics.bundleHelper'), value: String(userCenterStore.permissions.filter(permission => permission.kind === 'bundle').length) },
])

watch(
  () => userCenterStore.permissions.map(permission => permission.id).join('|'),
  () => {
    if (!selectedPermissionId.value || !userCenterStore.permissions.some(permission => permission.id === selectedPermissionId.value)) {
      applyPermission(userCenterStore.permissions[0]?.id)
      return
    }
    applyPermission(selectedPermissionId.value)
  },
  { immediate: true },
)

function applyPermission(permissionId?: string) {
  const permission = userCenterStore.permissions.find(item => item.id === permissionId)
  selectedPermissionId.value = permission?.id ?? ''
  form.name = permission?.name ?? ''
  form.code = permission?.code ?? ''
  form.description = permission?.description ?? ''
  form.status = permission?.status ?? 'active'
  form.kind = permission?.kind ?? 'atomic'
  form.targetType = permission?.targetType ?? 'project'
  form.targetIds = permission?.targetIds.join(', ') ?? ''
  form.action = permission?.action ?? 'view'
  form.memberPermissionIds = permission?.memberPermissionIds.join(', ') ?? ''
}

async function savePermission() {
  if (!workspaceStore.currentWorkspaceId || !form.name.trim() || !form.code.trim()) {
    return
  }

  const record: PermissionRecord = {
    id: selectedPermissionId.value || `permission-${Date.now()}`,
    workspaceId: workspaceStore.currentWorkspaceId,
    name: form.name.trim(),
    code: form.code.trim(),
    description: form.description.trim(),
    status: form.status as PermissionRecord['status'],
    kind: form.kind as PermissionRecord['kind'],
    targetType: form.targetType as PermissionRecord['targetType'],
    targetIds: form.targetIds.split(',').map(item => item.trim()).filter(Boolean),
    action: form.action.trim() || undefined,
    memberPermissionIds: form.memberPermissionIds.split(',').map(item => item.trim()).filter(Boolean),
  }

  if (selectedPermissionId.value) {
    await userCenterStore.updatePermission(selectedPermissionId.value, record)
  } else {
    const created = await userCenterStore.createPermission(record)
    selectedPermissionId.value = created.id
  }
}
</script>

<template>
  <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
    <section class="space-y-3">
      <div class="grid gap-3 md:grid-cols-2">
        <div v-for="metric in metrics" :key="metric.id" class="rounded-xl border border-border-subtle p-4 dark:border-white/[0.05]">
          <div class="text-xs text-text-secondary">{{ metric.label }}</div>
          <div class="mt-2 text-2xl font-semibold text-text-primary">{{ metric.value }}</div>
        </div>
      </div>
      <UiRecordCard
        v-for="permission in userCenterStore.permissions"
        :key="permission.id"
        :title="permission.name"
        :description="permission.description"
        interactive
        class="cursor-pointer"
        :class="selectedPermissionId === permission.id ? 'ring-1 ring-primary' : ''"
        @click="applyPermission(permission.id)"
      >
        <template #badges>
          <UiBadge :label="permission.kind" subtle />
          <UiBadge :label="permission.status" subtle />
        </template>
      </UiRecordCard>
    </section>

    <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
      <UiField :label="t('userCenter.permissions.fields.name')">
        <UiInput v-model="form.name" />
      </UiField>
      <UiField :label="t('userCenter.permissions.fields.code')">
        <UiInput v-model="form.code" />
      </UiField>
      <UiField :label="t('common.status')">
        <UiSelect v-model="form.status" :options="statusOptions" />
      </UiField>
      <UiField :label="t('userCenter.permissions.fields.kind')">
        <UiSelect v-model="form.kind" :options="kindOptions" />
      </UiField>
      <UiField :label="t('userCenter.permissions.fields.targetType')">
        <UiSelect v-model="form.targetType" :options="targetTypeOptions" />
      </UiField>
      <UiField :label="t('userCenter.permissions.fields.action')">
        <UiInput v-model="form.action" />
      </UiField>
      <UiField :label="t('userCenter.permissions.fields.targetIds')">
        <UiTextarea v-model="form.targetIds" :rows="3" />
      </UiField>
      <UiField :label="t('userCenter.permissions.fields.memberPermissionIds')">
        <UiTextarea v-model="form.memberPermissionIds" :rows="3" />
      </UiField>
      <UiField :label="t('userCenter.permissions.fields.description')">
        <UiTextarea v-model="form.description" :rows="5" />
      </UiField>
      <UiButton @click="savePermission">{{ t('common.save') }}</UiButton>
    </section>
  </div>
</template>
