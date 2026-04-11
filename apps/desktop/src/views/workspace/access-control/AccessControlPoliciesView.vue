<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'

import {
  UiBadge,
  UiButton,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPanelFrame,
  UiSelect,
  UiStatTile,
  UiStatusCallout,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'

import type {
  DataPolicyRecord,
  DataPolicyUpsertRequest,
  PermissionDefinition,
  ResourcePolicyRecord,
  ResourcePolicyUpsertRequest,
  RoleBindingRecord,
  RoleBindingUpsertRequest,
} from '@octopus/schema'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import {
  dataResourceTypeOptions,
  parseListInput,
  policyEffectOptions,
  resourceTypeOptions,
  scopeTypeOptions,
  stringifyListInput,
  subjectTypeOptions,
} from './helpers'

const accessControlStore = useWorkspaceAccessControlStore()

const submitError = ref('')
const successMessage = ref('')
const activeSection = ref('permissions')

const permissionQuery = ref('')
const bindingQuery = ref('')
const dataPolicyQuery = ref('')
const resourcePolicyQuery = ref('')
const resourcePolicyTypeFilter = ref('')

const selectedPermissionCode = ref('')
const selectedRoleBindingId = ref('')
const selectedDataPolicyId = ref('')
const selectedResourcePolicyId = ref('')

const createBindingDialogOpen = ref(false)
const createDataPolicyDialogOpen = ref(false)
const createResourcePolicyDialogOpen = ref(false)

const savingRoleBinding = ref(false)
const deletingRoleBindingId = ref('')
const savingDataPolicy = ref(false)
const deletingDataPolicyId = ref('')
const savingResourcePolicy = ref(false)
const deletingResourcePolicyId = ref('')

const sectionTabs = [
  { value: 'permissions', label: '权限目录' },
  { value: 'bindings', label: '角色绑定' },
  { value: 'data', label: '数据策略' },
  { value: 'resources', label: '资源策略' },
]

const roleBindingForm = reactive({
  roleId: '',
  subjectType: 'user',
  subjectId: '',
  effect: 'allow',
})

const dataPolicyForm = reactive({
  name: '',
  subjectType: 'user',
  subjectId: '',
  resourceType: 'project',
  scopeType: 'selected-projects',
  projectIdsText: '',
  tagsText: '',
  classificationsText: '',
  effect: 'allow',
})

const resourcePolicyForm = reactive({
  subjectType: 'user',
  subjectId: '',
  resourceType: 'agent',
  resourceId: '',
  action: 'view',
  effect: 'allow',
})

const roleMap = computed(() => new Map(accessControlStore.roles.map(role => [role.id, role.name])))
const subjectOptions = computed(() => ({
  user: accessControlStore.users.map(user => ({ label: user.displayName, value: user.id })),
  org_unit: accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
  position: accessControlStore.positions.map(position => ({ label: position.name, value: position.id })),
  user_group: accessControlStore.userGroups.map(group => ({ label: group.name, value: group.id })),
}))
const protectedResourceOptions = computed(() =>
  accessControlStore.protectedResources.map(resource => ({
    label: `${resource.name} (${resource.resourceType})`,
    value: resource.id,
    resourceType: resource.resourceType,
  })),
)
const roleOptions = computed(() => accessControlStore.roles.map(role => ({ label: role.name, value: role.id })))
const roleBindingSubjectOptions = computed(() =>
  subjectOptions.value[roleBindingForm.subjectType as keyof typeof subjectOptions.value] ?? [],
)
const dataPolicySubjectOptions = computed(() =>
  subjectOptions.value[dataPolicyForm.subjectType as keyof typeof subjectOptions.value] ?? [],
)
const resourcePolicySubjectOptions = computed(() =>
  subjectOptions.value[resourcePolicyForm.subjectType as keyof typeof subjectOptions.value] ?? [],
)
const filteredProtectedResourceOptions = computed(() =>
  protectedResourceOptions.value.filter(option => option.resourceType === resourcePolicyForm.resourceType),
)

const metrics = computed(() => ({
  permissions: accessControlStore.permissionDefinitions.length,
  bindings: accessControlStore.roleBindings.length,
  dataPolicies: accessControlStore.dataPolicies.length,
  resourcePolicies: accessControlStore.resourcePolicies.length,
}))

const filteredPermissions = computed(() => {
  const normalizedQuery = permissionQuery.value.trim().toLowerCase()
  return [...accessControlStore.permissionDefinitions]
    .sort((left, right) => left.code.localeCompare(right.code))
    .filter(permission => !normalizedQuery || [
      permission.name,
      permission.code,
      permission.description,
      permission.resourceType,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredRoleBindings = computed(() => {
  const normalizedQuery = bindingQuery.value.trim().toLowerCase()
  return [...accessControlStore.roleBindings]
    .filter(binding => !normalizedQuery || [
      roleMap.value.get(binding.roleId) ?? binding.roleId,
      binding.subjectType,
      resolveSubjectLabel(binding.subjectType, binding.subjectId),
      binding.effect,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredDataPolicies = computed(() => {
  const normalizedQuery = dataPolicyQuery.value.trim().toLowerCase()
  return [...accessControlStore.dataPolicies]
    .filter(policy => !normalizedQuery || [
      policy.name,
      policy.subjectType,
      resolveSubjectLabel(policy.subjectType, policy.subjectId),
      policy.resourceType,
      policy.scopeType,
      ...policy.classifications,
      ...policy.tags,
      ...policy.projectIds,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredResourcePolicies = computed(() => {
  const normalizedQuery = resourcePolicyQuery.value.trim().toLowerCase()
  return [...accessControlStore.resourcePolicies]
    .filter(policy => !resourcePolicyTypeFilter.value || policy.resourceType === resourcePolicyTypeFilter.value)
    .filter(policy => !normalizedQuery || [
      policy.resourceType,
      policy.action,
      resolveSubjectLabel(policy.subjectType, policy.subjectId),
      resolveResourceLabel(policy.resourceType, policy.resourceId),
      policy.effect,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const selectedPermission = computed(() =>
  accessControlStore.permissionDefinitions.find(permission => permission.code === selectedPermissionCode.value) ?? null,
)
const selectedRoleBinding = computed(() =>
  accessControlStore.roleBindings.find(binding => binding.id === selectedRoleBindingId.value) ?? null,
)
const selectedDataPolicy = computed(() =>
  accessControlStore.dataPolicies.find(policy => policy.id === selectedDataPolicyId.value) ?? null,
)
const selectedResourcePolicy = computed(() =>
  accessControlStore.resourcePolicies.find(policy => policy.id === selectedResourcePolicyId.value) ?? null,
)

watch(selectedRoleBinding, (binding) => {
  if (!binding) {
    resetRoleBindingForm()
    return
  }
  populateRoleBindingForm(binding)
}, { immediate: true })

watch(selectedDataPolicy, (policy) => {
  if (!policy) {
    resetDataPolicyForm()
    return
  }
  populateDataPolicyForm(policy)
}, { immediate: true })

watch(selectedResourcePolicy, (policy) => {
  if (!policy) {
    resetResourcePolicyForm()
    return
  }
  populateResourcePolicyForm(policy)
}, { immediate: true })

watch(filteredPermissions, (permissions) => {
  if (selectedPermissionCode.value && !permissions.some(permission => permission.code === selectedPermissionCode.value)) {
    selectedPermissionCode.value = ''
  }
})

watch(filteredRoleBindings, (bindings) => {
  if (selectedRoleBindingId.value && !bindings.some(binding => binding.id === selectedRoleBindingId.value)) {
    selectedRoleBindingId.value = ''
  }
})

watch(filteredDataPolicies, (policies) => {
  if (selectedDataPolicyId.value && !policies.some(policy => policy.id === selectedDataPolicyId.value)) {
    selectedDataPolicyId.value = ''
  }
})

watch(filteredResourcePolicies, (policies) => {
  if (selectedResourcePolicyId.value && !policies.some(policy => policy.id === selectedResourcePolicyId.value)) {
    selectedResourcePolicyId.value = ''
  }
})

function clearMessages() {
  submitError.value = ''
  successMessage.value = ''
}

function resolveSubjectLabel(subjectType: string, subjectId: string) {
  const options = subjectOptions.value[subjectType as keyof typeof subjectOptions.value] ?? []
  return options.find(option => option.value === subjectId)?.label ?? subjectId
}

function resolveResourceLabel(resourceType: string, resourceId: string) {
  return protectedResourceOptions.value.find(
    option => option.resourceType === resourceType && option.value === resourceId,
  )?.label ?? resourceId
}

function selectPermission(permission: PermissionDefinition) {
  clearMessages()
  selectedPermissionCode.value = permission.code
}

function selectRoleBinding(bindingId: string) {
  clearMessages()
  selectedRoleBindingId.value = bindingId
}

function selectDataPolicy(policyId: string) {
  clearMessages()
  selectedDataPolicyId.value = policyId
}

function selectResourcePolicy(policyId: string) {
  clearMessages()
  selectedResourcePolicyId.value = policyId
}

function resetRoleBindingForm() {
  Object.assign(roleBindingForm, {
    roleId: accessControlStore.roles[0]?.id ?? '',
    subjectType: 'user',
    subjectId: accessControlStore.users[0]?.id ?? '',
    effect: 'allow',
  })
}

function populateRoleBindingForm(binding: RoleBindingRecord) {
  Object.assign(roleBindingForm, {
    roleId: binding.roleId,
    subjectType: binding.subjectType,
    subjectId: binding.subjectId,
    effect: binding.effect,
  })
}

function resetDataPolicyForm() {
  Object.assign(dataPolicyForm, {
    name: '',
    subjectType: 'user',
    subjectId: accessControlStore.users[0]?.id ?? '',
    resourceType: 'project',
    scopeType: 'selected-projects',
    projectIdsText: '',
    tagsText: '',
    classificationsText: '',
    effect: 'allow',
  })
}

function populateDataPolicyForm(policy: DataPolicyRecord) {
  Object.assign(dataPolicyForm, {
    name: policy.name,
    subjectType: policy.subjectType,
    subjectId: policy.subjectId,
    resourceType: policy.resourceType,
    scopeType: policy.scopeType,
    projectIdsText: stringifyListInput(policy.projectIds),
    tagsText: stringifyListInput(policy.tags),
    classificationsText: stringifyListInput(policy.classifications),
    effect: policy.effect,
  })
}

function resetResourcePolicyForm() {
  const nextResourceType = protectedResourceOptions.value[0]?.resourceType ?? 'agent'
  Object.assign(resourcePolicyForm, {
    subjectType: 'user',
    subjectId: accessControlStore.users[0]?.id ?? '',
    resourceType: nextResourceType,
    resourceId: protectedResourceOptions.value.find(option => option.resourceType === nextResourceType)?.value ?? '',
    action: 'view',
    effect: 'allow',
  })
}

function populateResourcePolicyForm(policy: ResourcePolicyRecord) {
  Object.assign(resourcePolicyForm, {
    subjectType: policy.subjectType,
    subjectId: policy.subjectId,
    resourceType: policy.resourceType,
    resourceId: policy.resourceId,
    action: policy.action,
    effect: policy.effect,
  })
}

function openCreateRoleBindingDialog() {
  clearMessages()
  selectedRoleBindingId.value = ''
  resetRoleBindingForm()
  createBindingDialogOpen.value = true
}

function openCreateDataPolicyDialog() {
  clearMessages()
  selectedDataPolicyId.value = ''
  resetDataPolicyForm()
  createDataPolicyDialogOpen.value = true
}

function openCreateResourcePolicyDialog() {
  clearMessages()
  selectedResourcePolicyId.value = ''
  resetResourcePolicyForm()
  createResourcePolicyDialogOpen.value = true
}

function validateRoleBindingForm() {
  if (!roleBindingForm.roleId || !roleBindingForm.subjectId) {
    return '请选择角色和绑定主体。'
  }
  return ''
}

function validateDataPolicyForm() {
  if (!dataPolicyForm.name.trim() || !dataPolicyForm.subjectId) {
    return '请填写策略名称并选择主体。'
  }
  return ''
}

function validateResourcePolicyForm() {
  if (!resourcePolicyForm.subjectId || !resourcePolicyForm.resourceId || !resourcePolicyForm.action.trim()) {
    return '请选择主体、资源并填写动作。'
  }
  return ''
}

async function saveRoleBinding(isCreate = false) {
  submitError.value = validateRoleBindingForm()
  if (submitError.value) {
    return null
  }

  savingRoleBinding.value = true
  try {
    const payload: RoleBindingUpsertRequest = {
      roleId: roleBindingForm.roleId,
      subjectType: roleBindingForm.subjectType,
      subjectId: roleBindingForm.subjectId,
      effect: roleBindingForm.effect,
    }

    const record = isCreate || !selectedRoleBinding.value
      ? await accessControlStore.createRoleBinding(payload)
      : await accessControlStore.updateRoleBinding(selectedRoleBinding.value.id, payload)

    successMessage.value = `已保存角色绑定 ${resolveSubjectLabel(record.subjectType, record.subjectId)}`
    return record
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存角色绑定失败。'
    return null
  } finally {
    savingRoleBinding.value = false
  }
}

async function saveDataPolicy(isCreate = false) {
  submitError.value = validateDataPolicyForm()
  if (submitError.value) {
    return null
  }

  savingDataPolicy.value = true
  try {
    const payload: DataPolicyUpsertRequest = {
      name: dataPolicyForm.name.trim(),
      subjectType: dataPolicyForm.subjectType,
      subjectId: dataPolicyForm.subjectId,
      resourceType: dataPolicyForm.resourceType,
      scopeType: dataPolicyForm.scopeType,
      projectIds: parseListInput(dataPolicyForm.projectIdsText),
      tags: parseListInput(dataPolicyForm.tagsText),
      classifications: parseListInput(dataPolicyForm.classificationsText),
      effect: dataPolicyForm.effect,
    }

    const record = isCreate || !selectedDataPolicy.value
      ? await accessControlStore.createDataPolicy(payload)
      : await accessControlStore.updateDataPolicy(selectedDataPolicy.value.id, payload)

    successMessage.value = `已保存数据策略 ${record.name}`
    return record
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存数据策略失败。'
    return null
  } finally {
    savingDataPolicy.value = false
  }
}

async function saveResourcePolicy(isCreate = false) {
  submitError.value = validateResourcePolicyForm()
  if (submitError.value) {
    return null
  }

  savingResourcePolicy.value = true
  try {
    const payload: ResourcePolicyUpsertRequest = {
      subjectType: resourcePolicyForm.subjectType,
      subjectId: resourcePolicyForm.subjectId,
      resourceType: resourcePolicyForm.resourceType,
      resourceId: resourcePolicyForm.resourceId,
      action: resourcePolicyForm.action.trim(),
      effect: resourcePolicyForm.effect,
    }

    const record = isCreate || !selectedResourcePolicy.value
      ? await accessControlStore.createResourcePolicy(payload)
      : await accessControlStore.updateResourcePolicy(selectedResourcePolicy.value.id, payload)

    successMessage.value = `已保存资源策略 ${record.action}`
    return record
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存资源策略失败。'
    return null
  } finally {
    savingResourcePolicy.value = false
  }
}

async function handleCreateRoleBinding() {
  const record = await saveRoleBinding(true)
  if (!record) {
    return
  }
  createBindingDialogOpen.value = false
  selectedRoleBindingId.value = record.id
}

async function handleCreateDataPolicy() {
  const record = await saveDataPolicy(true)
  if (!record) {
    return
  }
  createDataPolicyDialogOpen.value = false
  selectedDataPolicyId.value = record.id
}

async function handleCreateResourcePolicy() {
  const record = await saveResourcePolicy(true)
  if (!record) {
    return
  }
  createResourcePolicyDialogOpen.value = false
  selectedResourcePolicyId.value = record.id
}

async function deleteRoleBinding() {
  if (!selectedRoleBinding.value) {
    return
  }

  deletingRoleBindingId.value = selectedRoleBinding.value.id
  submitError.value = ''
  try {
    const label = resolveSubjectLabel(selectedRoleBinding.value.subjectType, selectedRoleBinding.value.subjectId)
    await accessControlStore.deleteRoleBinding(selectedRoleBinding.value.id)
    selectedRoleBindingId.value = ''
    successMessage.value = `已删除角色绑定 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除角色绑定失败。'
  } finally {
    deletingRoleBindingId.value = ''
  }
}

async function deleteDataPolicy() {
  if (!selectedDataPolicy.value) {
    return
  }

  deletingDataPolicyId.value = selectedDataPolicy.value.id
  submitError.value = ''
  try {
    const label = selectedDataPolicy.value.name
    await accessControlStore.deleteDataPolicy(selectedDataPolicy.value.id)
    selectedDataPolicyId.value = ''
    successMessage.value = `已删除数据策略 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除数据策略失败。'
  } finally {
    deletingDataPolicyId.value = ''
  }
}

async function deleteResourcePolicy() {
  if (!selectedResourcePolicy.value) {
    return
  }

  deletingResourcePolicyId.value = selectedResourcePolicy.value.id
  submitError.value = ''
  try {
    const label = selectedResourcePolicy.value.action
    await accessControlStore.deleteResourcePolicy(selectedResourcePolicy.value.id)
    selectedResourcePolicyId.value = ''
    successMessage.value = `已删除资源策略 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除资源策略失败。'
  } finally {
    deletingResourcePolicyId.value = ''
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-policies-shell">
    <section class="grid gap-4 md:grid-cols-4">
      <UiStatTile label="权限目录" :value="String(metrics.permissions)" />
      <UiStatTile label="角色绑定" :value="String(metrics.bindings)" />
      <UiStatTile label="数据策略" :value="String(metrics.dataPolicies)" />
      <UiStatTile label="资源策略" :value="String(metrics.resourcePolicies)" />
    </section>

    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />
    <UiStatusCallout v-if="successMessage" tone="success" :description="successMessage" />

    <UiTabs v-model="activeSection" :tabs="sectionTabs" data-testid="access-control-policies-section-tabs" />

    <UiListDetailWorkspace
      v-if="activeSection === 'permissions'"
      :has-selection="Boolean(selectedPermission)"
      :detail-title="selectedPermission ? selectedPermission.name : ''"
      detail-subtitle="权限目录只读，提供 capability 参考和资源类型映射。"
      empty-detail-title="请选择权限"
      empty-detail-description="从左侧权限目录中选择一项后即可查看详情。"
    >
      <template #toolbar>
        <UiToolbarRow>
          <template #search>
            <UiInput v-model="permissionQuery" placeholder="搜索权限名称、code 或资源类型" />
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="权限目录" :subtitle="`共 ${filteredPermissions.length} 项 capability`">
          <div v-if="filteredPermissions.length" class="space-y-2">
            <button
              v-for="permission in filteredPermissions"
              :key="permission.code"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedPermissionCode === permission.code ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              @click="selectPermission(permission)"
            >
              <div class="space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-semibold text-foreground">{{ permission.name }}</span>
                  <UiBadge :label="permission.resourceType" subtle />
                </div>
                <div class="text-xs text-muted-foreground">{{ permission.code }}</div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无权限目录" description="当前筛选条件下没有权限定义。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedPermission" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedPermission.name }}</div>
              <UiBadge :label="selectedPermission.resourceType" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedPermission.code }}</div>
            <div class="mt-3 text-sm text-muted-foreground">{{ selectedPermission.description }}</div>
          </div>

          <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
            <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">当前主体动作矩阵</div>
            <div v-if="accessControlStore.currentResourceActionGrants.length" class="mt-3 space-y-3">
              <div
                v-for="grant in accessControlStore.currentResourceActionGrants"
                :key="grant.resourceType"
                class="rounded-[var(--radius-m)] border border-border bg-muted/30 p-3"
              >
                <div class="text-sm font-medium text-foreground">{{ grant.resourceType }}</div>
                <div class="mt-2 flex flex-wrap gap-2">
                  <UiBadge
                    v-for="action in grant.actions"
                    :key="`${grant.resourceType}:${action}`"
                    :label="action"
                    subtle
                  />
                </div>
              </div>
            </div>
            <p v-else class="mt-2 text-sm text-muted-foreground">当前主体没有生效的资源动作授权。</p>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiListDetailWorkspace
      v-else-if="activeSection === 'bindings'"
      :has-selection="Boolean(selectedRoleBinding)"
      :detail-title="selectedRoleBinding ? (roleMap.get(selectedRoleBinding.roleId) ?? selectedRoleBinding.roleId) : ''"
      detail-subtitle="主体通过角色绑定获得 capability 集合。"
      empty-detail-title="请选择角色绑定"
      empty-detail-description="从左侧绑定列表中选择一项后即可查看详情，或在右上角新建绑定。"
    >
      <template #toolbar>
        <UiToolbarRow>
          <template #search>
            <UiInput v-model="bindingQuery" placeholder="搜索角色、主体或效果" />
          </template>
          <template #actions>
            <UiButton @click="openCreateRoleBindingDialog">新建绑定</UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="角色绑定" :subtitle="`共 ${filteredRoleBindings.length} 条绑定`">
          <div v-if="filteredRoleBindings.length" class="space-y-2">
            <button
              v-for="binding in filteredRoleBindings"
              :key="binding.id"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedRoleBindingId === binding.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              @click="selectRoleBinding(binding.id)"
            >
              <div class="space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-semibold text-foreground">{{ roleMap.get(binding.roleId) ?? binding.roleId }}</span>
                  <UiBadge :label="binding.effect" subtle />
                </div>
                <div class="text-xs text-muted-foreground">{{ binding.subjectType }}</div>
                <div class="text-xs text-muted-foreground">{{ resolveSubjectLabel(binding.subjectType, binding.subjectId) }}</div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无角色绑定" description="当前筛选条件下没有角色绑定记录。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedRoleBinding" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ roleMap.get(selectedRoleBinding.roleId) ?? selectedRoleBinding.roleId }}</div>
              <UiBadge :label="selectedRoleBinding.effect" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">
              {{ selectedRoleBinding.subjectType }} / {{ resolveSubjectLabel(selectedRoleBinding.subjectType, selectedRoleBinding.subjectId) }}
            </div>
          </div>

          <div class="grid gap-3">
            <UiField label="角色">
              <UiSelect v-model="roleBindingForm.roleId" :options="roleOptions" data-testid="access-control-role-binding-role" />
            </UiField>
            <UiField label="主体类型">
              <UiSelect v-model="roleBindingForm.subjectType" :options="subjectTypeOptions" data-testid="access-control-role-binding-subject-type" />
            </UiField>
            <UiField label="主体">
              <UiSelect v-model="roleBindingForm.subjectId" :options="roleBindingSubjectOptions" data-testid="access-control-role-binding-subject-id" />
            </UiField>
            <UiField label="效果">
              <UiSelect v-model="roleBindingForm.effect" :options="policyEffectOptions" data-testid="access-control-role-binding-effect" />
            </UiField>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton
              variant="ghost"
              class="text-destructive"
              :loading="deletingRoleBindingId === selectedRoleBinding.id"
              @click="deleteRoleBinding"
            >
              删除绑定
            </UiButton>
            <UiButton :loading="savingRoleBinding" data-testid="access-control-role-binding-save" @click="saveRoleBinding()">
              保存绑定
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiListDetailWorkspace
      v-else-if="activeSection === 'data'"
      :has-selection="Boolean(selectedDataPolicy)"
      :detail-title="selectedDataPolicy ? selectedDataPolicy.name : ''"
      detail-subtitle="通过项目、标签和密级定义数据访问范围。"
      empty-detail-title="请选择数据策略"
      empty-detail-description="从左侧策略列表中选择一项后即可查看详情，或在右上角新建策略。"
    >
      <template #toolbar>
        <UiToolbarRow>
          <template #search>
            <UiInput v-model="dataPolicyQuery" placeholder="搜索策略名称、主体、资源类型或范围" />
          </template>
          <template #actions>
            <UiButton @click="openCreateDataPolicyDialog">新建策略</UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="数据策略" :subtitle="`共 ${filteredDataPolicies.length} 条策略`">
          <div v-if="filteredDataPolicies.length" class="space-y-2">
            <button
              v-for="policy in filteredDataPolicies"
              :key="policy.id"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedDataPolicyId === policy.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              @click="selectDataPolicy(policy.id)"
            >
              <div class="space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-semibold text-foreground">{{ policy.name }}</span>
                  <UiBadge :label="policy.effect" subtle />
                </div>
                <div class="text-xs text-muted-foreground">{{ resolveSubjectLabel(policy.subjectType, policy.subjectId) }}</div>
                <div class="text-xs text-muted-foreground">{{ policy.resourceType }} / {{ policy.scopeType }}</div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无数据策略" description="当前筛选条件下没有数据策略记录。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedDataPolicy" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedDataPolicy.name }}</div>
              <UiBadge :label="selectedDataPolicy.effect" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">
              {{ selectedDataPolicy.resourceType }} / {{ selectedDataPolicy.scopeType }}
            </div>
          </div>

          <div class="grid gap-3">
            <UiField label="策略名称">
              <UiInput v-model="dataPolicyForm.name" data-testid="access-control-data-policy-name" />
            </UiField>
            <UiField label="主体类型">
              <UiSelect v-model="dataPolicyForm.subjectType" :options="subjectTypeOptions" data-testid="access-control-data-policy-subject-type" />
            </UiField>
            <UiField label="主体">
              <UiSelect v-model="dataPolicyForm.subjectId" :options="dataPolicySubjectOptions" data-testid="access-control-data-policy-subject-id" />
            </UiField>
            <UiField label="资源类型">
              <UiSelect v-model="dataPolicyForm.resourceType" :options="dataResourceTypeOptions" data-testid="access-control-data-policy-resource-type" />
            </UiField>
            <UiField label="范围类型">
              <UiSelect v-model="dataPolicyForm.scopeType" :options="scopeTypeOptions" data-testid="access-control-data-policy-scope-type" />
            </UiField>
            <UiField label="项目范围" hint="多个项目 ID 用逗号分隔。">
              <UiInput v-model="dataPolicyForm.projectIdsText" data-testid="access-control-data-policy-projects" />
            </UiField>
            <UiField label="标签范围" hint="多个标签用逗号分隔。">
              <UiInput v-model="dataPolicyForm.tagsText" data-testid="access-control-data-policy-tags" />
            </UiField>
            <UiField label="密级范围" hint="多个密级用逗号分隔。">
              <UiInput v-model="dataPolicyForm.classificationsText" data-testid="access-control-data-policy-classifications" />
            </UiField>
            <UiField label="效果">
              <UiSelect v-model="dataPolicyForm.effect" :options="policyEffectOptions" data-testid="access-control-data-policy-effect" />
            </UiField>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton
              variant="ghost"
              class="text-destructive"
              :loading="deletingDataPolicyId === selectedDataPolicy.id"
              @click="deleteDataPolicy"
            >
              删除策略
            </UiButton>
            <UiButton :loading="savingDataPolicy" data-testid="access-control-data-policy-save" @click="saveDataPolicy()">
              保存策略
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiListDetailWorkspace
      v-else
      :has-selection="Boolean(selectedResourcePolicy)"
      :detail-title="selectedResourcePolicy ? selectedResourcePolicy.action : ''"
      detail-subtitle="对象级策略用于控制具体资源和工具动作。"
      empty-detail-title="请选择资源策略"
      empty-detail-description="从左侧策略列表中选择一项后即可查看详情，或在右上角新建策略。"
    >
      <template #toolbar>
        <UiToolbarRow>
          <template #search>
            <UiInput v-model="resourcePolicyQuery" placeholder="搜索动作、主体或资源对象" />
          </template>
          <template #filters>
            <UiField label="资源类型" class="w-full md:w-[220px]">
              <UiSelect v-model="resourcePolicyTypeFilter" :options="[{ label: '全部类型', value: '' }, ...resourceTypeOptions]" />
            </UiField>
          </template>
          <template #actions>
            <UiButton @click="openCreateResourcePolicyDialog">新建策略</UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="资源策略" :subtitle="`共 ${filteredResourcePolicies.length} 条策略`">
          <div v-if="filteredResourcePolicies.length" class="space-y-2">
            <button
              v-for="policy in filteredResourcePolicies"
              :key="policy.id"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedResourcePolicyId === policy.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              @click="selectResourcePolicy(policy.id)"
            >
              <div class="space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-semibold text-foreground">{{ policy.action }}</span>
                  <UiBadge :label="policy.effect" subtle />
                </div>
                <div class="text-xs text-muted-foreground">{{ resolveSubjectLabel(policy.subjectType, policy.subjectId) }}</div>
                <div class="text-xs text-muted-foreground">{{ resolveResourceLabel(policy.resourceType, policy.resourceId) }}</div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无资源策略" description="当前筛选条件下没有资源策略记录。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedResourcePolicy" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedResourcePolicy.action }}</div>
              <UiBadge :label="selectedResourcePolicy.effect" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">
              {{ resolveResourceLabel(selectedResourcePolicy.resourceType, selectedResourcePolicy.resourceId) }}
            </div>
          </div>

          <div class="grid gap-3">
            <UiField label="主体类型">
              <UiSelect v-model="resourcePolicyForm.subjectType" :options="subjectTypeOptions" data-testid="access-control-resource-policy-subject-type" />
            </UiField>
            <UiField label="主体">
              <UiSelect v-model="resourcePolicyForm.subjectId" :options="resourcePolicySubjectOptions" data-testid="access-control-resource-policy-subject-id" />
            </UiField>
            <UiField label="资源类型">
              <UiSelect v-model="resourcePolicyForm.resourceType" :options="resourceTypeOptions" data-testid="access-control-resource-policy-resource-type" />
            </UiField>
            <UiField label="资源对象">
              <UiSelect v-model="resourcePolicyForm.resourceId" :options="filteredProtectedResourceOptions" data-testid="access-control-resource-policy-resource-id" />
            </UiField>
            <UiField label="动作">
              <UiInput v-model="resourcePolicyForm.action" data-testid="access-control-resource-policy-action" />
            </UiField>
            <UiField label="效果">
              <UiSelect v-model="resourcePolicyForm.effect" :options="policyEffectOptions" data-testid="access-control-resource-policy-effect" />
            </UiField>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton
              variant="ghost"
              class="text-destructive"
              :loading="deletingResourcePolicyId === selectedResourcePolicy.id"
              @click="deleteResourcePolicy"
            >
              删除策略
            </UiButton>
            <UiButton :loading="savingResourcePolicy" data-testid="access-control-resource-policy-save" @click="saveResourcePolicy()">
              保存策略
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      :open="createBindingDialogOpen"
      title="新建角色绑定"
      description="为主体绑定角色，并设置 allow / deny 效果。"
      @update:open="createBindingDialogOpen = $event"
    >
      <div class="grid gap-3">
        <UiField label="角色">
          <UiSelect v-model="roleBindingForm.roleId" :options="roleOptions" data-testid="access-control-role-binding-role" />
        </UiField>
        <UiField label="主体类型">
          <UiSelect v-model="roleBindingForm.subjectType" :options="subjectTypeOptions" data-testid="access-control-role-binding-subject-type" />
        </UiField>
        <UiField label="主体">
          <UiSelect v-model="roleBindingForm.subjectId" :options="roleBindingSubjectOptions" data-testid="access-control-role-binding-subject-id" />
        </UiField>
        <UiField label="效果">
          <UiSelect v-model="roleBindingForm.effect" :options="policyEffectOptions" data-testid="access-control-role-binding-effect" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createBindingDialogOpen = false">取消</UiButton>
        <UiButton :loading="savingRoleBinding" data-testid="access-control-role-binding-save" @click="handleCreateRoleBinding">
          创建绑定
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createDataPolicyDialogOpen"
      title="新建数据策略"
      description="通过主体、资源类型和范围条件定义数据访问边界。"
      @update:open="createDataPolicyDialogOpen = $event"
    >
      <div class="grid gap-3">
        <UiField label="策略名称">
          <UiInput v-model="dataPolicyForm.name" data-testid="access-control-data-policy-name" />
        </UiField>
        <UiField label="主体类型">
          <UiSelect v-model="dataPolicyForm.subjectType" :options="subjectTypeOptions" data-testid="access-control-data-policy-subject-type" />
        </UiField>
        <UiField label="主体">
          <UiSelect v-model="dataPolicyForm.subjectId" :options="dataPolicySubjectOptions" data-testid="access-control-data-policy-subject-id" />
        </UiField>
        <UiField label="资源类型">
          <UiSelect v-model="dataPolicyForm.resourceType" :options="dataResourceTypeOptions" data-testid="access-control-data-policy-resource-type" />
        </UiField>
        <UiField label="范围类型">
          <UiSelect v-model="dataPolicyForm.scopeType" :options="scopeTypeOptions" data-testid="access-control-data-policy-scope-type" />
        </UiField>
        <UiField label="项目范围" hint="多个项目 ID 用逗号分隔。">
          <UiInput v-model="dataPolicyForm.projectIdsText" data-testid="access-control-data-policy-projects" />
        </UiField>
        <UiField label="标签范围" hint="多个标签用逗号分隔。">
          <UiInput v-model="dataPolicyForm.tagsText" data-testid="access-control-data-policy-tags" />
        </UiField>
        <UiField label="密级范围" hint="多个密级用逗号分隔。">
          <UiInput v-model="dataPolicyForm.classificationsText" data-testid="access-control-data-policy-classifications" />
        </UiField>
        <UiField label="效果">
          <UiSelect v-model="dataPolicyForm.effect" :options="policyEffectOptions" data-testid="access-control-data-policy-effect" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createDataPolicyDialogOpen = false">取消</UiButton>
        <UiButton :loading="savingDataPolicy" data-testid="access-control-data-policy-save" @click="handleCreateDataPolicy">
          创建策略
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createResourcePolicyDialogOpen"
      title="新建资源策略"
      description="为具体资源对象设置动作级 allow / deny 策略。"
      @update:open="createResourcePolicyDialogOpen = $event"
    >
      <div class="grid gap-3">
        <UiField label="主体类型">
          <UiSelect v-model="resourcePolicyForm.subjectType" :options="subjectTypeOptions" data-testid="access-control-resource-policy-subject-type" />
        </UiField>
        <UiField label="主体">
          <UiSelect v-model="resourcePolicyForm.subjectId" :options="resourcePolicySubjectOptions" data-testid="access-control-resource-policy-subject-id" />
        </UiField>
        <UiField label="资源类型">
          <UiSelect v-model="resourcePolicyForm.resourceType" :options="resourceTypeOptions" data-testid="access-control-resource-policy-resource-type" />
        </UiField>
        <UiField label="资源对象">
          <UiSelect v-model="resourcePolicyForm.resourceId" :options="filteredProtectedResourceOptions" data-testid="access-control-resource-policy-resource-id" />
        </UiField>
        <UiField label="动作">
          <UiInput v-model="resourcePolicyForm.action" data-testid="access-control-resource-policy-action" />
        </UiField>
        <UiField label="效果">
          <UiSelect v-model="resourcePolicyForm.effect" :options="policyEffectOptions" data-testid="access-control-resource-policy-effect" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createResourcePolicyDialogOpen = false">取消</UiButton>
        <UiButton :loading="savingResourcePolicy" data-testid="access-control-resource-policy-save" @click="handleCreateResourcePolicy">
          创建策略
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>
