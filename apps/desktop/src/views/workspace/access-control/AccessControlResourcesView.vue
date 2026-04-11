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

import type { ProtectedResourceMetadataUpsertRequest, ResourcePolicyUpsertRequest } from '@octopus/schema'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { classificationOptions, parseListInput, policyEffectOptions, stringifyListInput, subjectTypeOptions } from './helpers'

const accessControlStore = useWorkspaceAccessControlStore()

const groupedCounts = computed(() => {
  const counts = new Map<string, number>()
  accessControlStore.protectedResources.forEach((resource) => {
    counts.set(resource.resourceType, (counts.get(resource.resourceType) ?? 0) + 1)
  })
  return [...counts.entries()]
})

const selectedResourceKey = ref('')
const submitError = ref('')
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
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除资源策略失败。'
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-resources-shell">
    <section class="grid gap-4 md:grid-cols-4">
      <UiStatTile
        v-for="[resourceType, count] in groupedCounts"
        :key="resourceType"
        :label="resourceType"
        :value="String(count)"
      />
    </section>

    <UiStatusCallout
      v-if="submitError"
      tone="error"
      :description="submitError"
    />

    <div class="grid gap-4 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)]">
      <UiPanelFrame variant="panel" padding="md" title="受保护资源目录" subtitle="覆盖 agent、resource、knowledge、tool.builtin/skill/mcp。">
        <div v-if="accessControlStore.protectedResources.length" class="space-y-3">
          <article
            v-for="resource in accessControlStore.protectedResources"
            :key="`${resource.resourceType}:${resource.id}`"
            class="rounded-[12px] border border-border bg-card p-4"
          >
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h3 class="text-sm font-semibold text-foreground">{{ resource.name }}</h3>
                <p class="text-xs text-muted-foreground">{{ resource.resourceType }} / {{ resource.resourceSubtype ?? 'default' }}</p>
              </div>
              <div class="flex flex-wrap gap-2">
                <UiBadge :label="resource.classification" subtle />
                <UiBadge v-if="resource.projectId" :label="resource.projectId" subtle />
              </div>
            </div>
            <div class="mt-3 flex justify-end">
              <UiButton
                size="sm"
                variant="ghost"
                data-testid="access-control-resource-select"
                @click="selectResource(resource.resourceType, resource.id)"
              >
                配置对象策略
              </UiButton>
            </div>
          </article>
        </div>
        <UiEmptyState v-else title="暂无资源目录" description="当前工作区没有可投影的受保护资源。" />
      </UiPanelFrame>

      <UiPanelFrame variant="panel" padding="md" :title="selectedResource ? '资源元数据与对象策略' : '选择资源'" subtitle="先维护标签、密级和 owner，再配置对象级 allow / deny 策略。">
        <div v-if="selectedResource" class="space-y-4">
          <div class="rounded-[12px] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedResource.name }}</div>
              <UiBadge :label="selectedResource.resourceType" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedResource.id }}</div>
          </div>

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

          <div class="border-t border-border/80 pt-4">
            <div class="mb-3 text-sm font-semibold text-foreground">对象级资源策略</div>
          </div>

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
              class="rounded-[8px] border border-border bg-card p-3"
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
        <UiEmptyState v-else title="请选择资源" description="从左侧资源目录中选择一项后即可配置对象级策略。" />
      </UiPanelFrame>
    </div>
  </div>
</template>
