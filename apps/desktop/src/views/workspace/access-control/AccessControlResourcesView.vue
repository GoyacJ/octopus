<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPanelFrame,
  UiSelect,
  UiStatusCallout,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'

import type { ProtectedResourceMetadataUpsertRequest, ResourcePolicyUpsertRequest } from '@octopus/schema'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { classificationOptions, parseListInput, policyEffectOptions, resourceTypeOptions, stringifyListInput, subjectTypeOptions } from './helpers'

const accessControlStore = useWorkspaceAccessControlStore()

const query = ref('')
const resourceTypeFilter = ref('')
const selectedResourceKey = ref('')
const detailTab = ref('metadata')
const submitError = ref('')
const successMessage = ref('')
const saving = ref(false)
const savingMetadata = ref(false)

const policyForm = reactive({
  subjectType: 'user',
  subjectId: '',
  action: 'view',
  effect: 'allow',
})
const metadataForm = reactive({
  resourceSubtype: '',
  projectId: '',
  tagsText: '',
  classification: 'internal',
  ownerSubjectType: '',
  ownerSubjectId: '',
})

const subjectOptions = computed(() => ({
  user: accessControlStore.users.map(user => ({ label: user.displayName, value: user.id })),
  org_unit: accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
  position: accessControlStore.positions.map(position => ({ label: position.name, value: position.id })),
  user_group: accessControlStore.userGroups.map(group => ({ label: group.name, value: group.id })),
}))
const ownerSubjectTypeOptions = computed(() => ([
  { label: '未设置', value: '' },
  ...subjectTypeOptions,
]))

const filteredSubjectOptions = computed(() =>
  subjectOptions.value[policyForm.subjectType as keyof typeof subjectOptions.value] ?? [],
)
const filteredOwnerSubjectOptions = computed(() =>
  metadataForm.ownerSubjectType
    ? (subjectOptions.value[metadataForm.ownerSubjectType as keyof typeof subjectOptions.value] ?? [])
    : [],
)
const filteredResources = computed(() => {
  const normalizedQuery = query.value.trim().toLowerCase()
  return [...accessControlStore.protectedResources]
    .filter(resource => !resourceTypeFilter.value || resource.resourceType === resourceTypeFilter.value)
    .filter(resource => !normalizedQuery || [
      resource.name,
      resource.id,
      resource.resourceType,
      resource.resourceSubtype ?? '',
      resource.projectId ?? '',
      ...(resource.tags ?? []),
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const selectedResource = computed(() =>
  accessControlStore.protectedResources.find(resource =>
    `${resource.resourceType}:${resource.id}` === selectedResourceKey.value,
  ) ?? null,
)
const selectedResourcePolicies = computed(() =>
  accessControlStore.resourcePolicies.filter(policy =>
    policy.resourceId === selectedResource.value?.id
    && policy.resourceType === (selectedResource.value?.resourceType ?? ''),
  ),
)

const detailTabs = [
  { value: 'metadata', label: '元数据' },
  { value: 'policies', label: '对象策略' },
]

const resourceTypeFilterOptions = computed(() => [
  { label: '全部类型', value: '' },
  ...resourceTypeOptions,
  { label: '资源', value: 'resource' },
])

function syncMetadataForm() {
  Object.assign(metadataForm, {
    resourceSubtype: selectedResource.value?.resourceSubtype ?? '',
    projectId: selectedResource.value?.projectId ?? '',
    tagsText: stringifyListInput(selectedResource.value?.tags),
    classification: selectedResource.value?.classification ?? 'internal',
    ownerSubjectType: selectedResource.value?.ownerSubjectType ?? '',
    ownerSubjectId: selectedResource.value?.ownerSubjectId ?? '',
  })
}

function selectResource(resourceType: string, resourceId: string) {
  selectedResourceKey.value = `${resourceType}:${resourceId}`
  submitError.value = ''
  detailTab.value = 'metadata'
  syncMetadataForm()
  if (!policyForm.subjectId) {
    policyForm.subjectId = accessControlStore.users[0]?.id ?? ''
  }
}

async function createPolicy() {
  submitError.value = ''
  if (!selectedResource.value) {
    submitError.value = '请先选择一个受保护资源。'
    return
  }
  if (!policyForm.subjectId || !policyForm.action.trim()) {
    submitError.value = '请选择主体并填写动作。'
    return
  }

  saving.value = true
  try {
    const payload: ResourcePolicyUpsertRequest = {
      subjectType: policyForm.subjectType,
      subjectId: policyForm.subjectId,
      resourceType: selectedResource.value.resourceType,
      resourceId: selectedResource.value.id,
      action: policyForm.action.trim(),
      effect: policyForm.effect,
    }
    await accessControlStore.createResourcePolicy(payload)
    successMessage.value = '已添加对象策略'
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '创建资源策略失败。'
  } finally {
    saving.value = false
  }
}

async function saveMetadata() {
  submitError.value = ''
  if (!selectedResource.value) {
    submitError.value = '请先选择一个受保护资源。'
    return
  }

  savingMetadata.value = true
  try {
    const payload: ProtectedResourceMetadataUpsertRequest = {
      resourceSubtype: metadataForm.resourceSubtype.trim() || undefined,
      projectId: metadataForm.projectId.trim() || undefined,
      tags: parseListInput(metadataForm.tagsText),
      classification: metadataForm.classification,
      ownerSubjectType: metadataForm.ownerSubjectType || undefined,
      ownerSubjectId: metadataForm.ownerSubjectId || undefined,
    }
    await accessControlStore.upsertProtectedResource(
      selectedResource.value.resourceType,
      selectedResource.value.id,
      payload,
    )
    successMessage.value = '已保存资源元数据'
    syncMetadataForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存资源元数据失败。'
  } finally {
    savingMetadata.value = false
  }
}

async function deletePolicy(policyId: string) {
  submitError.value = ''
  try {
    await accessControlStore.deleteResourcePolicy(policyId)
    successMessage.value = '已删除对象策略'
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除资源策略失败。'
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-resources-shell">
    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />
    <UiStatusCallout v-if="successMessage" tone="success" :description="successMessage" />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedResource)"
      :detail-title="selectedResource ? selectedResource.name : ''"
      detail-subtitle="先维护元数据，再配置对象级 allow / deny 策略。"
      empty-detail-title="请选择资源"
      empty-detail-description="从左侧资源目录中选择一项后即可查看详情和对象策略。"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-resources-toolbar">
          <template #search>
            <UiInput v-model="query" placeholder="搜索资源名称、ID、标签或项目" />
          </template>
          <template #filters>
            <UiField label="资源类型" class="w-full md:w-[220px]">
              <UiSelect v-model="resourceTypeFilter" :options="resourceTypeFilterOptions" />
            </UiField>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="资源目录" :subtitle="`共 ${filteredResources.length} 项受保护资源`">
          <div v-if="filteredResources.length" class="space-y-2">
            <button
              v-for="resource in filteredResources"
              :key="`${resource.resourceType}:${resource.id}`"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedResourceKey === `${resource.resourceType}:${resource.id}` ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              data-testid="access-control-resource-select"
              @click="selectResource(resource.resourceType, resource.id)"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 space-y-1">
                  <div class="text-sm font-semibold text-foreground">{{ resource.name }}</div>
                  <div class="text-xs text-muted-foreground">{{ resource.resourceType }} / {{ resource.resourceSubtype ?? 'default' }}</div>
                </div>
                <div class="flex flex-wrap gap-2">
                  <UiBadge :label="resource.classification" subtle />
                  <UiBadge v-if="resource.projectId" :label="resource.projectId" subtle />
                </div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无资源目录" description="当前筛选条件下没有资源记录。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedResource" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedResource.name }}</div>
              <UiBadge :label="selectedResource.resourceType" subtle />
              <UiBadge :label="selectedResource.classification" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedResource.id }}</div>
          </div>

          <UiTabs v-model="detailTab" :tabs="detailTabs" />

          <div v-if="detailTab === 'metadata'" class="space-y-4">
            <div class="grid gap-3 md:grid-cols-2">
              <UiField label="资源子类型">
                <UiInput v-model="metadataForm.resourceSubtype" data-testid="access-control-resource-subtype" />
              </UiField>
              <UiField label="项目 ID">
                <UiInput v-model="metadataForm.projectId" data-testid="access-control-resource-project-id" />
              </UiField>
              <UiField label="标签" hint="多个标签用逗号分隔。">
                <UiInput v-model="metadataForm.tagsText" data-testid="access-control-resource-tags" />
              </UiField>
              <UiField label="密级">
                <UiSelect v-model="metadataForm.classification" :options="classificationOptions" data-testid="access-control-resource-classification" />
              </UiField>
              <UiField label="Owner 类型">
                <UiSelect v-model="metadataForm.ownerSubjectType" :options="ownerSubjectTypeOptions" data-testid="access-control-resource-owner-type" />
              </UiField>
              <UiField label="Owner 主体">
                <UiSelect v-model="metadataForm.ownerSubjectId" :options="filteredOwnerSubjectOptions" data-testid="access-control-resource-owner-id" />
              </UiField>
            </div>

            <div class="flex justify-end">
              <UiButton :loading="savingMetadata" data-testid="access-control-resource-metadata-save" @click="saveMetadata">
                保存资源元数据
              </UiButton>
            </div>
          </div>

          <div v-else class="space-y-4">
            <div class="grid gap-3">
              <UiField label="主体类型">
                <UiSelect v-model="policyForm.subjectType" :options="subjectTypeOptions" data-testid="access-control-resource-subject-type" />
              </UiField>
              <UiField label="主体">
                <UiSelect v-model="policyForm.subjectId" :options="filteredSubjectOptions" data-testid="access-control-resource-subject-id" />
              </UiField>
              <UiField label="动作">
                <UiInput v-model="policyForm.action" data-testid="access-control-resource-action" />
              </UiField>
              <UiField label="效果">
                <UiSelect v-model="policyForm.effect" :options="policyEffectOptions" data-testid="access-control-resource-effect" />
              </UiField>
            </div>

            <div class="flex justify-end">
              <UiButton :loading="saving" data-testid="access-control-resource-policy-save" @click="createPolicy">
                添加对象策略
              </UiButton>
            </div>

            <div v-if="selectedResourcePolicies.length" class="space-y-2">
              <article
                v-for="policy in selectedResourcePolicies"
                :key="policy.id"
                class="rounded-[var(--radius-m)] border border-border bg-card p-3"
              >
                <div class="flex items-start justify-between gap-2">
                  <div>
                    <div class="text-sm font-medium text-foreground">{{ policy.action }}</div>
                    <div class="text-xs text-muted-foreground">{{ policy.subjectType }} / {{ policy.subjectId }}</div>
                  </div>
                  <div class="flex gap-2">
                    <UiBadge :label="policy.effect" subtle />
                    <UiButton size="sm" variant="ghost" class="text-destructive" @click="deletePolicy(policy.id)">删除</UiButton>
                  </div>
                </div>
              </article>
            </div>
            <UiEmptyState v-else title="暂无对象策略" description="当前对象还没有资源策略。" />
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>
  </div>
</template>
