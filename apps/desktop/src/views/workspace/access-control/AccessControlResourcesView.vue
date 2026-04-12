<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPagination,
  UiPanelFrame,
  UiRecordCard,
  UiSelect,
  UiStatusCallout,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'

import type { ProtectedResourceMetadataUpsertRequest, ResourcePolicyUpsertRequest } from '@octopus/schema'

import { usePagination } from '@/composables/usePagination'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import {
  createClassificationOptions,
  createPolicyEffectOptions,
  createResourceTypeOptions,
  createSubjectTypeOptions,
  getClassificationLabel,
  getPolicyEffectLabel,
  getResourceTypeLabel,
  getSubjectTypeLabel,
  parseListInput,
  stringifyListInput,
} from './helpers'
import { useAccessControlNotifications } from './useAccessControlNotifications'

const { t } = useI18n()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifySuccess } = useAccessControlNotifications('access-control.resources')

const query = ref('')
const resourceTypeFilter = ref('')
const selectedResourceKey = ref('')
const detailTab = ref('metadata')
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

const subjectTypeOptions = computed(() => createSubjectTypeOptions(t))
const policyEffectOptions = computed(() => createPolicyEffectOptions(t))
const resourceTypeOptions = computed(() => createResourceTypeOptions(t))
const classificationOptions = computed(() => createClassificationOptions(t))

const subjectOptions = computed(() => ({
  user: accessControlStore.users.map(user => ({ label: user.displayName, value: user.id })),
  org_unit: accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
  position: accessControlStore.positions.map(position => ({ label: position.name, value: position.id })),
  user_group: accessControlStore.userGroups.map(group => ({ label: group.name, value: group.id })),
}))
const ownerSubjectTypeOptions = computed(() => ([
  { label: t('accessControl.common.list.unsetOwner'), value: '' },
  ...subjectTypeOptions.value,
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

const detailTabs = computed(() => [
  { value: 'metadata', label: t('accessControl.resources.detail.tabs.metadata') },
  { value: 'policies', label: t('accessControl.resources.detail.tabs.policies') },
])

const resourceTypeFilterOptions = computed(() => [
  { label: t('accessControl.common.filters.allTypes'), value: '' },
  ...resourceTypeOptions.value,
])

const pagination = usePagination(filteredResources, {
  pageSize: 8,
  resetOn: [query, resourceTypeFilter],
})

watch(pagination.pagedItems, (resources) => {
  if (selectedResourceKey.value && !resources.some(resource => `${resource.resourceType}:${resource.id}` === selectedResourceKey.value)) {
    selectedResourceKey.value = ''
  }
}, { immediate: true })

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

function resolveSubjectLabel(subjectType: string, subjectId: string) {
  return subjectOptions.value[subjectType as keyof typeof subjectOptions.value]
    ?.find(option => option.value === subjectId)
    ?.label ?? subjectId
}

function resourceDescriptor(resource: {
  resourceType: string
  resourceSubtype?: string | null
}) {
  return [resource.resourceType, resource.resourceSubtype || t('accessControl.common.list.defaultSubtype')].join(' / ')
}

async function createPolicy() {
  submitError.value = ''
  if (!selectedResource.value) {
    submitError.value = t('accessControl.resources.feedback.selectRequired')
    return
  }
  if (!policyForm.subjectId || !policyForm.action.trim()) {
    submitError.value = t('accessControl.resources.feedback.policyRequired')
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
    await notifySuccess(t('accessControl.resources.feedback.toastPolicySaved'), payload.action)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.resources.feedback.createPolicyFailed')
  } finally {
    saving.value = false
  }
}

async function saveMetadata() {
  submitError.value = ''
  if (!selectedResource.value) {
    submitError.value = t('accessControl.resources.feedback.selectRequired')
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
    await notifySuccess(t('accessControl.resources.feedback.toastMetadataSaved'), selectedResource.value.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.resources.feedback.saveMetadataFailed')
  } finally {
    savingMetadata.value = false
  }
}

async function deletePolicy(policyId: string) {
  submitError.value = ''
  try {
    const label = selectedResourcePolicies.value.find(policy => policy.id === policyId)?.action ?? policyId
    await accessControlStore.deleteResourcePolicy(policyId)
    await notifySuccess(t('accessControl.resources.feedback.toastPolicyDeleted'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.resources.feedback.deletePolicyFailed')
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-resources-shell">
    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedResource)"
      :detail-title="selectedResource ? selectedResource.name : ''"
      :detail-subtitle="t('accessControl.resources.detail.subtitle')"
      :empty-detail-title="t('accessControl.resources.detail.emptyTitle')"
      :empty-detail-description="t('accessControl.resources.detail.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-resources-toolbar">
          <template #search>
            <UiInput v-model="query" :placeholder="t('accessControl.resources.toolbar.search')" />
          </template>
          <template #filters>
            <UiField :label="t('accessControl.resources.toolbar.resourceType')" class="w-full md:w-[220px]">
              <UiSelect v-model="resourceTypeFilter" :options="resourceTypeFilterOptions" />
            </UiField>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.resources.list.title')"
          :subtitle="t('accessControl.common.list.totalResources', { count: pagination.totalItems.value })"
        >
          <div v-if="pagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="resource in pagination.pagedItems.value"
              :key="`${resource.resourceType}:${resource.id}`"
              layout="compact"
              interactive
              :active="selectedResourceKey === `${resource.resourceType}:${resource.id}`"
              :title="resource.name"
              :description="resourceDescriptor(resource)"
              test-id="access-control-resource-select"
              @click="selectResource(resource.resourceType, resource.id)"
            >
              <template #secondary>
                <UiBadge :label="getClassificationLabel(t, resource.classification)" subtle />
                <UiBadge v-if="resource.projectId" :label="resource.projectId" subtle />
              </template>
            </UiRecordCard>
          </div>
          <UiEmptyState
            v-else
            :title="t('accessControl.resources.list.emptyTitle')"
            :description="t('accessControl.resources.list.emptyDescription')"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="pagination.currentPage.value"
              :page-count="pagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: pagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
          <div v-if="selectedResource" class="space-y-4">
            <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
              <div class="flex flex-wrap items-center gap-2">
                <div class="text-sm font-semibold text-foreground">{{ selectedResource.name }}</div>
                <UiBadge :label="getResourceTypeLabel(t, selectedResource.resourceType)" subtle />
              <UiBadge :label="getClassificationLabel(t, selectedResource.classification)" subtle />
              </div>
              <div class="mt-2 text-xs text-muted-foreground">{{ selectedResource.id }}</div>
          </div>

          <UiTabs v-model="detailTab" :tabs="detailTabs" />

          <div v-if="detailTab === 'metadata'" class="space-y-4">
            <div class="grid gap-3 md:grid-cols-2">
              <UiField :label="t('accessControl.resources.fields.resourceSubtype')">
                <UiInput v-model="metadataForm.resourceSubtype" data-testid="access-control-resource-subtype" />
              </UiField>
              <UiField :label="t('accessControl.resources.fields.projectId')">
                <UiInput v-model="metadataForm.projectId" data-testid="access-control-resource-project-id" />
              </UiField>
              <UiField :label="t('accessControl.resources.fields.tags')" :hint="t('accessControl.resources.hints.tags')">
                <UiInput v-model="metadataForm.tagsText" data-testid="access-control-resource-tags" />
              </UiField>
              <UiField :label="t('accessControl.resources.fields.classification')">
                <UiSelect v-model="metadataForm.classification" :options="classificationOptions" data-testid="access-control-resource-classification" />
              </UiField>
              <UiField :label="t('accessControl.resources.fields.ownerType')">
                <UiSelect v-model="metadataForm.ownerSubjectType" :options="ownerSubjectTypeOptions" data-testid="access-control-resource-owner-type" />
              </UiField>
              <UiField :label="t('accessControl.resources.fields.ownerId')">
                <UiSelect v-model="metadataForm.ownerSubjectId" :options="filteredOwnerSubjectOptions" data-testid="access-control-resource-owner-id" />
              </UiField>
            </div>

            <div class="flex justify-end">
              <UiButton :loading="savingMetadata" data-testid="access-control-resource-metadata-save" @click="saveMetadata">
                {{ t('accessControl.resources.actions.saveMetadata') }}
              </UiButton>
            </div>
          </div>

          <div v-else class="space-y-4">
            <div class="grid gap-3">
              <UiField :label="t('accessControl.resources.fields.subjectType')">
                <UiSelect v-model="policyForm.subjectType" :options="subjectTypeOptions" data-testid="access-control-resource-subject-type" />
              </UiField>
              <UiField :label="t('accessControl.resources.fields.subject')">
                <UiSelect v-model="policyForm.subjectId" :options="filteredSubjectOptions" data-testid="access-control-resource-subject-id" />
              </UiField>
              <UiField :label="t('accessControl.resources.fields.action')">
                <UiInput v-model="policyForm.action" data-testid="access-control-resource-action" />
              </UiField>
              <UiField :label="t('accessControl.resources.fields.effect')">
                <UiSelect v-model="policyForm.effect" :options="policyEffectOptions" data-testid="access-control-resource-effect" />
              </UiField>
            </div>

            <div class="flex justify-end">
              <UiButton :loading="saving" data-testid="access-control-resource-policy-save" @click="createPolicy">
                {{ t('accessControl.resources.actions.createPolicy') }}
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
                    <div class="text-xs text-muted-foreground">
                      {{ getSubjectTypeLabel(t, policy.subjectType) }} / {{ resolveSubjectLabel(policy.subjectType, policy.subjectId) }}
                    </div>
                  </div>
                  <div class="flex gap-2">
                    <UiBadge :label="getPolicyEffectLabel(t, policy.effect)" subtle />
                    <UiButton size="sm" variant="ghost" class="text-destructive" @click="deletePolicy(policy.id)">{{ t('accessControl.resources.actions.deletePolicy') }}</UiButton>
                  </div>
                </div>
              </article>
            </div>
            <UiEmptyState
              v-else
              :title="t('accessControl.resources.detail.policyEmptyTitle')"
              :description="t('accessControl.resources.detail.policyEmptyDescription')"
            />
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>
  </div>
</template>
