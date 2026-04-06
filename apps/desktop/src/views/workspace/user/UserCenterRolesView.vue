<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { RoleRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiField, UiInput, UiRecordCard, UiSelect, UiTextarea } from '@octopus/ui'

import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const userCenterStore = useUserCenterStore()
const workspaceStore = useWorkspaceStore()

const selectedRoleId = ref('')
const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active',
  permissionIds: '',
  menuIds: '',
})

const statusOptions = [
  { value: 'active', label: 'active' },
  { value: 'disabled', label: 'disabled' },
]

const metrics = computed(() => [
  { id: 'total', label: t('userCenter.roles.metrics.total'), value: String(userCenterStore.roles.length) },
  { id: 'disabled', label: t('userCenter.roles.metrics.disabled'), value: String(userCenterStore.roles.filter(role => role.status === 'disabled').length) },
])

watch(
  () => userCenterStore.roles.map(role => role.id).join('|'),
  () => {
    if (!selectedRoleId.value || !userCenterStore.roles.some(role => role.id === selectedRoleId.value)) {
      applyRole(userCenterStore.roles[0]?.id)
      return
    }
    applyRole(selectedRoleId.value)
  },
  { immediate: true },
)

function applyRole(roleId?: string) {
  const role = userCenterStore.roles.find(item => item.id === roleId)
  selectedRoleId.value = role?.id ?? ''
  form.name = role?.name ?? ''
  form.code = role?.code ?? ''
  form.description = role?.description ?? ''
  form.status = role?.status ?? 'active'
  form.permissionIds = role?.permissionIds.join(', ') ?? ''
  form.menuIds = role?.menuIds.join(', ') ?? ''
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
    permissionIds: form.permissionIds.split(',').map(item => item.trim()).filter(Boolean),
    menuIds: form.menuIds.split(',').map(item => item.trim()).filter(Boolean),
  }

  if (selectedRoleId.value) {
    await userCenterStore.updateRole(selectedRoleId.value, record)
  } else {
    const created = await userCenterStore.createRole(record)
    selectedRoleId.value = created.id
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
        v-for="role in userCenterStore.roles"
        :key="role.id"
        :title="role.name"
        :description="role.description"
        interactive
        class="cursor-pointer"
        :class="selectedRoleId === role.id ? 'ring-1 ring-primary' : ''"
        @click="applyRole(role.id)"
      >
        <template #badges>
          <UiBadge :label="role.status" subtle />
          <UiBadge :label="role.code" subtle />
        </template>
      </UiRecordCard>
    </section>

    <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
      <UiField :label="t('userCenter.roles.fields.name')">
        <UiInput v-model="form.name" />
      </UiField>
      <UiField :label="t('userCenter.roles.fields.code')">
        <UiInput v-model="form.code" />
      </UiField>
      <UiField :label="t('common.status')">
        <UiSelect v-model="form.status" :options="statusOptions" />
      </UiField>
      <UiField :label="t('userCenter.roles.fields.permissionIds')">
        <UiTextarea v-model="form.permissionIds" :rows="3" />
      </UiField>
      <UiField :label="t('userCenter.roles.fields.menuIds')">
        <UiTextarea v-model="form.menuIds" :rows="3" />
      </UiField>
      <UiField :label="t('userCenter.roles.fields.description')">
        <UiTextarea v-model="form.description" :rows="5" />
      </UiField>
      <div class="flex gap-3">
        <UiButton @click="saveRole">{{ t('common.save') }}</UiButton>
        <UiButton variant="ghost" @click="applyRole()">{{ t('common.reset') }}</UiButton>
      </div>
    </section>
  </div>
</template>
