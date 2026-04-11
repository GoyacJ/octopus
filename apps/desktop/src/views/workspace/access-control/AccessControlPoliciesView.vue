<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiInput,
  UiPanelFrame,
  UiSelect,
  UiStatTile,
  UiStatusCallout,
} from '@octopus/ui'

import type {
  DataPolicyRecord,
  DataPolicyUpsertRequest,
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

const roleBindingForm = reactive({
  roleId: '',
  subjectType: 'user',
  subjectId: '',
  effect: 'allow',
})
const editingRoleBindingId = ref('')
const savingRoleBinding = ref(false)
const deletingRoleBindingId = ref('')

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
const editingDataPolicyId = ref('')
const savingDataPolicy = ref(false)
const deletingDataPolicyId = ref('')

const resourcePolicyForm = reactive({
  subjectType: 'user',
  subjectId: '',
  resourceType: 'agent',
  resourceId: '',
  action: 'view',
  effect: 'allow',
})
const editingResourcePolicyId = ref('')
const savingResourcePolicy = ref(false)
const deletingResourcePolicyId = ref('')

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

function resetRoleBindingForm() {
  Object.assign(roleBindingForm, {
    roleId: accessControlStore.roles[0]?.id ?? '',
    subjectType: 'user',
    subjectId: accessControlStore.users[0]?.id ?? '',
    effect: 'allow',
  })
  editingRoleBindingId.value = ''
}

function populateRoleBindingForm(binding: RoleBindingRecord) {
  Object.assign(roleBindingForm, {
    roleId: binding.roleId,
    subjectType: binding.subjectType,
    subjectId: binding.subjectId,
    effect: binding.effect,
  })
  editingRoleBindingId.value = binding.id
}

async function saveRoleBinding() {
  submitError.value = ''
  if (!roleBindingForm.roleId || !roleBindingForm.subjectId) {
    submitError.value = '请选择角色和绑定主体。'
    return
  }

  savingRoleBinding.value = true
  try {
    const payload: RoleBindingUpsertRequest = {
      roleId: roleBindingForm.roleId,
      subjectType: roleBindingForm.subjectType,
      subjectId: roleBindingForm.subjectId,
      effect: roleBindingForm.effect,
    }
    if (editingRoleBindingId.value) {
      await accessControlStore.updateRoleBinding(editingRoleBindingId.value, payload)
    } else {
      await accessControlStore.createRoleBinding(payload)
    }
    resetRoleBindingForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存角色绑定失败。'
  } finally {
    savingRoleBinding.value = false
  }
}

async function deleteRoleBinding(bindingId: string) {
  deletingRoleBindingId.value = bindingId
  submitError.value = ''
  try {
    await accessControlStore.deleteRoleBinding(bindingId)
    if (editingRoleBindingId.value === bindingId) {
      resetRoleBindingForm()
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除角色绑定失败。'
  } finally {
    deletingRoleBindingId.value = ''
  }
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
  editingDataPolicyId.value = ''
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
  editingDataPolicyId.value = policy.id
}

async function saveDataPolicy() {
  submitError.value = ''
  if (!dataPolicyForm.name.trim() || !dataPolicyForm.subjectId) {
    submitError.value = '请填写策略名称并选择主体。'
    return
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
    if (editingDataPolicyId.value) {
      await accessControlStore.updateDataPolicy(editingDataPolicyId.value, payload)
    } else {
      await accessControlStore.createDataPolicy(payload)
    }
    resetDataPolicyForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存数据策略失败。'
  } finally {
    savingDataPolicy.value = false
  }
}

async function deleteDataPolicy(policyId: string) {
  deletingDataPolicyId.value = policyId
  submitError.value = ''
  try {
    await accessControlStore.deleteDataPolicy(policyId)
    if (editingDataPolicyId.value === policyId) {
      resetDataPolicyForm()
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除数据策略失败。'
  } finally {
    deletingDataPolicyId.value = ''
  }
}

function resetResourcePolicyForm() {
  Object.assign(resourcePolicyForm, {
    subjectType: 'user',
    subjectId: accessControlStore.users[0]?.id ?? '',
    resourceType: protectedResourceOptions.value[0]?.resourceType ?? 'agent',
    resourceId: filteredProtectedResourceOptions.value[0]?.value ?? '',
    action: 'view',
    effect: 'allow',
  })
  editingResourcePolicyId.value = ''
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
  editingResourcePolicyId.value = policy.id
}

async function saveResourcePolicy() {
  submitError.value = ''
  if (!resourcePolicyForm.subjectId || !resourcePolicyForm.resourceId || !resourcePolicyForm.action.trim()) {
    submitError.value = '请选择主体、资源并填写动作。'
    return
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
    if (editingResourcePolicyId.value) {
      await accessControlStore.updateResourcePolicy(editingResourcePolicyId.value, payload)
    } else {
      await accessControlStore.createResourcePolicy(payload)
    }
    resetResourcePolicyForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存资源策略失败。'
  } finally {
    savingResourcePolicy.value = false
  }
}

async function deleteResourcePolicy(policyId: string) {
  deletingResourcePolicyId.value = policyId
  submitError.value = ''
  try {
    await accessControlStore.deleteResourcePolicy(policyId)
    if (editingResourcePolicyId.value === policyId) {
      resetResourcePolicyForm()
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除资源策略失败。'
  } finally {
    deletingResourcePolicyId.value = ''
  }
}

resetRoleBindingForm()
resetDataPolicyForm()
resetResourcePolicyForm()
</script>

<template>
  <div class="space-y-4" data-testid="access-control-policies-shell">
    <section class="grid gap-4 md:grid-cols-4">
      <UiStatTile label="权限目录" :value="String(accessControlStore.permissionDefinitions.length)" />
      <UiStatTile label="角色绑定" :value="String(accessControlStore.roleBindings.length)" />
      <UiStatTile label="数据策略" :value="String(accessControlStore.dataPolicies.length)" />
      <UiStatTile label="资源策略" :value="String(accessControlStore.resourcePolicies.length)" />
    </section>

    <UiStatusCallout
      v-if="submitError"
      tone="error"
      :description="submitError"
    />

    <div class="grid gap-4 xl:grid-cols-2">
      <UiPanelFrame variant="panel" padding="md" title="权限目录" subtitle="系统预置 permission code，不允许租户自造。">
        <div v-if="accessControlStore.permissionDefinitions.length" class="space-y-3">
          <article
            v-for="permission in accessControlStore.permissionDefinitions"
            :key="permission.code"
            class="rounded-[var(--radius-l)] border border-border bg-card p-4"
          >
            <div class="flex flex-wrap items-center gap-2">
              <h3 class="text-sm font-semibold text-foreground">{{ permission.name }}</h3>
              <UiBadge :label="permission.resourceType" subtle />
            </div>
            <p class="mt-2 text-xs text-muted-foreground">{{ permission.code }}</p>
            <p class="mt-2 text-sm text-muted-foreground">{{ permission.description }}</p>
          </article>
        </div>
        <UiEmptyState v-else title="暂无权限目录" description="权限目录尚未发布。" />
      </UiPanelFrame>

      <UiPanelFrame variant="panel" padding="md" title="当前主体动作矩阵" subtitle="当前快照只做界面辅助，真实鉴权以后端决策为准。">
        <div v-if="accessControlStore.currentResourceActionGrants.length" class="space-y-3">
          <article
            v-for="grant in accessControlStore.currentResourceActionGrants"
            :key="grant.resourceType"
            class="rounded-[var(--radius-l)] border border-border bg-card p-4"
          >
            <div class="text-sm font-semibold text-foreground">{{ grant.resourceType }}</div>
            <div class="mt-3 flex flex-wrap gap-2">
              <UiBadge
                v-for="action in grant.actions"
                :key="`${grant.resourceType}:${action}`"
                :label="action"
                subtle
              />
            </div>
          </article>
        </div>
        <UiEmptyState v-else title="暂无动作矩阵" description="当前主体没有生效的资源动作授权。" />
      </UiPanelFrame>
    </div>

    <div class="grid gap-4 xl:grid-cols-3">
      <UiPanelFrame variant="panel" padding="md" title="角色绑定" subtitle="主体通过角色绑定获得 capability 集合。">
        <div class="space-y-4">
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
          <div class="flex justify-end gap-2">
            <UiButton variant="ghost" @click="resetRoleBindingForm">重置</UiButton>
            <UiButton :loading="savingRoleBinding" data-testid="access-control-role-binding-save" @click="saveRoleBinding">
              {{ editingRoleBindingId ? '保存绑定' : '创建绑定' }}
            </UiButton>
          </div>
          <div v-if="accessControlStore.roleBindings.length" class="space-y-2">
            <article
              v-for="binding in accessControlStore.roleBindings"
              :key="binding.id"
              class="rounded-[var(--radius-m)] border border-border bg-card p-3"
            >
              <div class="flex items-start justify-between gap-2">
                <div>
                  <div class="text-sm font-medium text-foreground">{{ binding.roleId }}</div>
                  <div class="text-xs text-muted-foreground">{{ binding.subjectType }} / {{ binding.subjectId }}</div>
                </div>
                <div class="flex gap-2">
                  <UiBadge :label="binding.effect" subtle />
                  <UiButton size="sm" variant="ghost" @click="populateRoleBindingForm(binding)">编辑</UiButton>
                  <UiButton size="sm" variant="ghost" class="text-destructive" :loading="deletingRoleBindingId === binding.id" @click="deleteRoleBinding(binding.id)">删除</UiButton>
                </div>
              </div>
            </article>
          </div>
          <UiEmptyState v-else title="暂无角色绑定" description="尚未建立主体和角色的关系。" />
        </div>
      </UiPanelFrame>

      <UiPanelFrame variant="panel" padding="md" title="数据策略" subtitle="部门、项目、标签等范围策略在这里维护。">
        <div class="space-y-4">
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
          <div class="flex justify-end gap-2">
            <UiButton variant="ghost" @click="resetDataPolicyForm">重置</UiButton>
            <UiButton :loading="savingDataPolicy" data-testid="access-control-data-policy-save" @click="saveDataPolicy">
              {{ editingDataPolicyId ? '保存策略' : '创建策略' }}
            </UiButton>
          </div>
          <div v-if="accessControlStore.dataPolicies.length" class="space-y-2">
            <article
              v-for="policy in accessControlStore.dataPolicies"
              :key="policy.id"
              class="rounded-[var(--radius-m)] border border-border bg-card p-3"
            >
              <div class="flex items-start justify-between gap-2">
                <div>
                  <div class="text-sm font-medium text-foreground">{{ policy.name }}</div>
                  <div class="text-xs text-muted-foreground">{{ policy.subjectType }} / {{ policy.subjectId }}</div>
                  <div class="mt-1 text-xs text-muted-foreground">{{ policy.resourceType }} / {{ policy.scopeType }}</div>
                  <div v-if="policy.classifications.length" class="mt-1 text-xs text-muted-foreground">
                    密级: {{ policy.classifications.join(', ') }}
                  </div>
                </div>
                <div class="flex gap-2">
                  <UiBadge :label="policy.effect" subtle />
                  <UiButton size="sm" variant="ghost" @click="populateDataPolicyForm(policy)">编辑</UiButton>
                  <UiButton size="sm" variant="ghost" class="text-destructive" :loading="deletingDataPolicyId === policy.id" @click="deleteDataPolicy(policy.id)">删除</UiButton>
                </div>
              </div>
            </article>
          </div>
          <UiEmptyState v-else title="暂无数据策略" description="尚未定义数据范围策略。" />
        </div>
      </UiPanelFrame>

      <UiPanelFrame variant="panel" padding="md" title="资源策略" subtitle="对象级策略用于控制具体 agent、资源、知识和工具对象。">
        <div class="space-y-4">
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
          <div class="flex justify-end gap-2">
            <UiButton variant="ghost" @click="resetResourcePolicyForm">重置</UiButton>
            <UiButton :loading="savingResourcePolicy" data-testid="access-control-resource-policy-save" @click="saveResourcePolicy">
              {{ editingResourcePolicyId ? '保存策略' : '创建策略' }}
            </UiButton>
          </div>
          <div v-if="accessControlStore.resourcePolicies.length" class="space-y-2">
            <article
              v-for="policy in accessControlStore.resourcePolicies"
              :key="policy.id"
              class="rounded-[var(--radius-m)] border border-border bg-card p-3"
            >
              <div class="flex items-start justify-between gap-2">
                <div>
                  <div class="text-sm font-medium text-foreground">{{ policy.resourceType }} / {{ policy.action }}</div>
                  <div class="text-xs text-muted-foreground">{{ policy.subjectType }} / {{ policy.subjectId }}</div>
                  <div class="mt-1 text-xs text-muted-foreground">{{ policy.resourceId }}</div>
                </div>
                <div class="flex gap-2">
                  <UiBadge :label="policy.effect" subtle />
                  <UiButton size="sm" variant="ghost" @click="populateResourcePolicyForm(policy)">编辑</UiButton>
                  <UiButton size="sm" variant="ghost" class="text-destructive" :loading="deletingResourcePolicyId === policy.id" @click="deleteResourcePolicy(policy.id)">删除</UiButton>
                </div>
              </div>
            </article>
          </div>
          <UiEmptyState v-else title="暂无资源策略" description="尚未定义对象级资源策略。" />
        </div>
      </UiPanelFrame>
    </div>
  </div>
</template>
