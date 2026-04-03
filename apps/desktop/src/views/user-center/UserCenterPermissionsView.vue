<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Blocks, Plus, Power, ShieldCheck, Trash2 } from 'lucide-vue-next'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiField,
  UiInput,
  UiMetricCard,
  UiRadioGroup,
  UiRecordCard,
  UiSectionHeading,
  UiSelect,
  UiSurface,
  UiTabs,
  UiTextarea,
  UiToolbarRow,
} from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const viewKind = ref<'atomic' | 'bundle'>('atomic')
const selectedPermissionId = ref<string>('')

const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active' as 'active' | 'disabled',
  kind: 'atomic' as 'atomic' | 'bundle',
  targetType: 'project' as 'project' | 'agent' | 'tool' | 'skill' | 'mcp',
  targetIds: [] as string[],
  action: 'view',
  memberPermissionIds: [] as string[],
})

const permissionItems = computed(() => workbench.workspacePermissionListItems)
const visiblePermissions = computed(() =>
  permissionItems.value.filter((permission) => permission.kind === viewKind.value),
)

const targetOptions = computed(() => {
  if (form.targetType === 'project') {
    return workbench.workspaceProjects.map((project) => ({ id: project.id, label: project.name }))
  }

  if (form.targetType === 'agent') {
    return workbench.workspaceAgents.map((agent) => ({ id: agent.id, label: agent.name }))
  }

  return workbench.toolCatalogGroups
    .flatMap((group) => group.items)
    .filter((item) =>
      form.targetType === 'tool'
        ? item.kind === 'builtin'
        : item.kind === form.targetType,
    )
    .map((item) => ({ id: item.id, label: item.name }))
})

const selectedPermissionSummary = computed(() =>
  permissionItems.value.find((item) => item.id === selectedPermissionId.value),
)

const summaryMetrics = computed(() => {
  const disabledCount = permissionItems.value.filter((item) => item.status === 'disabled').length
  const bundleCount = permissionItems.value.filter((item) => item.kind === 'bundle').length
  const riskyCount = permissionItems.value.filter((item) => item.riskFlags.length).length
  return [
    {
      id: 'total',
      label: t('userCenter.permissions.metrics.total'),
      value: String(permissionItems.value.length),
      helper: t('userCenter.permissions.metrics.bundleHelper', { count: bundleCount }),
    },
    {
      id: 'disabled',
      label: t('userCenter.permissions.metrics.disabled'),
      value: String(disabledCount),
      helper: t('userCenter.permissions.metrics.disabledHelper'),
      tone: 'warning' as const,
    },
    {
      id: 'risky',
      label: t('userCenter.permissions.metrics.risky'),
      value: String(riskyCount),
      helper: t('userCenter.permissions.metrics.riskyHelper'),
      tone: 'accent' as const,
    },
  ]
})

const kindTabs = computed(() => [
  { value: 'atomic', label: t('userCenter.permissions.atomicTab') },
  { value: 'bundle', label: t('userCenter.permissions.bundleTab') },
])

const statusOptions = computed(() => [
  { value: 'active', label: t('userCenter.common.active') },
  { value: 'disabled', label: t('userCenter.common.disabled') },
])

const kindOptions = computed(() => [
  { value: 'atomic', label: t('userCenter.permissions.atomicTab') },
  { value: 'bundle', label: t('userCenter.permissions.bundleTab') },
])

const targetTypeOptions = computed(() => [
  { value: 'project', label: enumLabel('permissionTargetType', 'project') || 'project' },
  { value: 'agent', label: enumLabel('permissionTargetType', 'agent') || 'agent' },
  { value: 'tool', label: enumLabel('permissionTargetType', 'tool') || 'tool' },
  { value: 'skill', label: enumLabel('permissionTargetType', 'skill') || 'skill' },
  { value: 'mcp', label: enumLabel('permissionTargetType', 'mcp') || 'mcp' },
])

function applyPermission(permissionId?: string) {
  if (!permissionId) {
    selectedPermissionId.value = ''
    form.name = ''
    form.code = ''
    form.description = ''
    form.status = 'active'
    form.kind = viewKind.value
    form.targetType = 'project'
    form.targetIds = []
    form.action = 'view'
    form.memberPermissionIds = []
    return
  }

  const permission = workbench.workspacePermissions.find((item) => item.id === permissionId)
  if (!permission) {
    applyPermission()
    return
  }

  selectedPermissionId.value = permission.id
  form.name = permission.name
  form.code = permission.code
  form.description = permission.description
  form.status = permission.status
  form.kind = permission.kind
  form.targetType = permission.targetType ?? 'project'
  form.targetIds = [...(permission.targetIds ?? [])]
  form.action = permission.action ?? 'view'
  form.memberPermissionIds = [...(permission.memberPermissionIds ?? [])]
}

watch(
  () => [viewKind.value, workbench.currentWorkspaceId, workbench.workspacePermissions.map((permission) => permission.id).join('|')],
  () => {
    const currentVisible = visiblePermissions.value
    if (!selectedPermissionId.value || !currentVisible.some((permission) => permission.id === selectedPermissionId.value)) {
      applyPermission(currentVisible[0]?.id)
      return
    }

    applyPermission(selectedPermissionId.value)
  },
  { immediate: true },
)

function savePermission() {
  if (selectedPermissionId.value) {
    workbench.updatePermission(selectedPermissionId.value, {
      name: form.name,
      code: form.code,
      description: form.description,
      status: form.status,
      kind: form.kind,
      targetType: form.kind === 'atomic' ? form.targetType : undefined,
      targetIds: form.kind === 'atomic' ? form.targetIds : undefined,
      action: form.kind === 'atomic' ? form.action : undefined,
      memberPermissionIds: form.kind === 'bundle' ? form.memberPermissionIds : undefined,
    })
    return
  }

  const permission = workbench.createPermission({
    name: form.name,
    code: form.code,
    description: form.description,
    status: form.status,
    kind: form.kind,
    targetType: form.kind === 'atomic' ? form.targetType : undefined,
    targetIds: form.kind === 'atomic' ? form.targetIds : undefined,
    action: form.kind === 'atomic' ? form.action : undefined,
    memberPermissionIds: form.kind === 'bundle' ? form.memberPermissionIds : undefined,
  })
  applyPermission(permission.id)
}
</script>

<template>
  <section class="section-stack">
    <div class="grid gap-4 md:grid-cols-3">
      <UiMetricCard
        v-for="metric in summaryMetrics"
        :key="metric.id"
        :label="metric.label"
        :value="metric.value"
        :helper="metric.helper"
        :tone="metric.tone"
      />
    </div>

    <UiSectionHeading
      :eyebrow="t('userCenter.permissions.title')"
      :title="t('userCenter.permissions.listTitle')"
      :subtitle="t('userCenter.permissions.subtitle')"
    />

    <div class="grid gap-4 xl:grid-cols-[minmax(22rem,28rem)_minmax(0,1fr)]">
      <UiSurface :title="t('userCenter.permissions.listTitle')" :subtitle="t('userCenter.permissions.listSubtitle')">
        <UiToolbarRow test-id="user-center-permissions-toolbar" class="mb-4">
          <template #tabs>
            <UiTabs
              v-model="viewKind"
              test-id="user-center-permissions-tabs"
              variant="pill"
              :tabs="kindTabs"
            />
          </template>
          <template #actions>
            <UiButton size="sm" @click="applyPermission()">
              <Plus :size="16" />
              {{ t('userCenter.permissions.create') }}
            </UiButton>
          </template>
        </UiToolbarRow>

        <div class="space-y-3">
          <UiRecordCard
            v-for="permission in visiblePermissions"
            :key="permission.id"
            :test-id="`user-center-permission-record-${permission.id}`"
            :title="permission.name"
            :description="permission.description"
            :active="selectedPermissionId === permission.id"
            interactive
            @click="applyPermission(permission.id)"
          >
            <template #eyebrow>{{ permission.code }}</template>
            <template #badges>
              <UiBadge :label="permission.kind" subtle />
              <UiBadge :label="permission.status" :tone="permission.status === 'active' ? 'success' : 'warning'" />
            </template>
            <template #meta>
              <span v-if="permission.kind === 'atomic'" class="inline-flex items-center gap-1">
                <ShieldCheck :size="14" />
                {{ permission.targetSummary }}
              </span>
              <span v-else class="inline-flex items-center gap-1">
                <Blocks :size="14" />
                {{ t('userCenter.permissions.bundleMembers', { count: permission.bundleMemberCount }) }}
              </span>
              <span>{{ t('userCenter.permissions.usedByRoles', { count: permission.usedByRoleCount }) }}</span>
              <UiBadge v-for="flag in permission.riskFlags" :key="flag" :label="flag" subtle />
            </template>
            <template #actions>
              <UiButton variant="ghost" size="sm" @click.stop="workbench.togglePermissionStatus(permission.id)">
                <Power :size="14" />
                {{ permission.status === 'active' ? t('userCenter.permissions.disable') : t('userCenter.permissions.enable') }}
              </UiButton>
              <UiButton variant="ghost" size="sm" @click.stop="workbench.deletePermission(permission.id)">
                <Trash2 :size="14" />
                {{ t('userCenter.permissions.delete') }}
              </UiButton>
            </template>
          </UiRecordCard>
        </div>
      </UiSurface>

      <UiSurface
        data-testid="user-center-permissions-editor"
        :title="t(selectedPermissionId ? 'userCenter.permissions.editTitle' : 'userCenter.permissions.createTitle')"
        :subtitle="t('userCenter.permissions.formSubtitle')"
      >
        <div v-if="selectedPermissionSummary" class="mb-4 flex flex-wrap items-center gap-2">
          <UiBadge :label="t('userCenter.permissions.usedByRoles', { count: selectedPermissionSummary.usedByRoleCount })" subtle />
          <UiBadge v-if="selectedPermissionSummary.kind === 'bundle'" :label="t('userCenter.permissions.bundleMembers', { count: selectedPermissionSummary.bundleMemberCount })" subtle />
          <UiBadge v-else :label="selectedPermissionSummary.targetSummary" subtle />
        </div>

        <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          <UiField :label="t('userCenter.permissions.nameLabel')">
            <UiInput v-model="form.name" />
          </UiField>
          <UiField :label="t('userCenter.permissions.codeLabel')">
            <UiInput v-model="form.code" />
          </UiField>
          <UiField :label="t('userCenter.common.status')">
            <UiSelect v-model="form.status" :options="statusOptions" />
          </UiField>
          <UiField :label="t('userCenter.permissions.kindLabel')">
            <UiSelect v-model="form.kind" :options="kindOptions" />
          </UiField>
          <UiField class="md:col-span-2 xl:col-span-4" :label="t('userCenter.permissions.descriptionLabel')">
            <UiTextarea v-model="form.description" :rows="4" />
          </UiField>
        </div>

        <div
          class="mt-4 grid gap-4"
          :class="form.kind === 'atomic' ? 'xl:grid-cols-[minmax(16rem,18rem)_minmax(0,1fr)_minmax(14rem,16rem)]' : 'xl:grid-cols-1'"
        >
          <template v-if="form.kind === 'atomic'">
            <UiSurface variant="subtle" padding="sm" :title="t('userCenter.permissions.targetTypeTitle')">
              <UiRadioGroup v-model="form.targetType" direction="vertical" :options="targetTypeOptions" />
            </UiSurface>

            <UiSurface variant="subtle" padding="sm" :title="t('userCenter.permissions.targetBindingTitle')">
              <div class="mb-3 flex items-center justify-between">
                <UiBadge :label="String(form.targetIds.length)" subtle />
              </div>
              <div class="space-y-2">
                <UiCheckbox
                  v-for="item in targetOptions"
                  :key="item.id"
                  v-model="form.targetIds"
                  :value="item.id"
                  :label="item.label"
                />
              </div>
            </UiSurface>

            <UiSurface variant="subtle" padding="sm" :title="t('userCenter.permissions.actionTitle')">
              <UiInput v-model="form.action" />
            </UiSurface>
          </template>

          <UiSurface v-else variant="subtle" padding="sm" :title="t('userCenter.permissions.bundleComposeTitle')">
            <div class="mb-3 flex items-center justify-between">
              <UiBadge :label="String(form.memberPermissionIds.length)" subtle />
            </div>
            <div class="space-y-2">
              <UiCheckbox
                v-for="permission in workbench.workspacePermissions.filter((item) => item.kind === 'atomic')"
                :key="permission.id"
                v-model="form.memberPermissionIds"
                :value="permission.id"
                :label="permission.name"
              />
            </div>
          </UiSurface>
        </div>

        <div class="mt-4 flex flex-wrap justify-end gap-3">
          <UiButton variant="ghost" @click="applyPermission(visiblePermissions[0]?.id)">
            {{ t('common.cancel') }}
          </UiButton>
          <UiButton @click="savePermission">
            {{ t('common.save') }}
          </UiButton>
        </div>
      </UiSurface>
    </div>
  </section>
</template>
