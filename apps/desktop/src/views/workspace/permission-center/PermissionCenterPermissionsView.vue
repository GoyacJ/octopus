<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type {
  PermissionRecord,
  RbacPermissionTargetType,
} from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiField,
  UiInspectorPanel,
  UiInput,
  UiListDetailShell,
  UiMetricCard,
  UiPanelFrame,
  UiPagination,
  UiRecordCard,
  UiSelect,
  UiStatusCallout,
  UiTextarea,
} from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { useAgentStore } from '@/stores/agent'
import { useCatalogStore } from '@/stores/catalog'
import { useKnowledgeStore } from '@/stores/knowledge'
import { useResourceStore } from '@/stores/resource'
import { useWorkspaceAccessStore } from '@/stores/workspace-access'
import { useWorkspaceStore } from '@/stores/workspace'

const PAGE_SIZE = 6

const { t, locale } = useI18n()
const workspaceAccessStore = useWorkspaceAccessStore()
const workspaceStore = useWorkspaceStore()
const resourceStore = useResourceStore()
const knowledgeStore = useKnowledgeStore()
const agentStore = useAgentStore()
const catalogStore = useCatalogStore()

const selectedPermissionId = ref('')
const currentPage = ref(1)
const saveMessage = ref('')
const deleteDialogOpen = ref(false)
const pendingDeletePermissionId = ref('')
const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active',
  kind: 'atomic',
  targetType: 'workspace' as RbacPermissionTargetType,
  action: 'view',
  targetIds: [] as string[],
  memberPermissionIds: [] as string[],
})

const statusOptions = computed(() => {
  locale.value
  return [
    { value: 'active', label: enumLabel('recordStatus', 'active') },
    { value: 'disabled', label: enumLabel('recordStatus', 'disabled') },
  ]
})

const kindOptions = computed(() => {
  locale.value
  return [
    { value: 'atomic', label: enumLabel('rbacPermissionKind', 'atomic') },
    { value: 'bundle', label: enumLabel('rbacPermissionKind', 'bundle') },
  ]
})

const targetTypeOptions = computed(() => {
  locale.value
  return [
    { value: 'workspace', label: enumLabel('rbacPermissionTargetType', 'workspace') },
    { value: 'project', label: enumLabel('rbacPermissionTargetType', 'project') },
    { value: 'user', label: enumLabel('rbacPermissionTargetType', 'user') },
    { value: 'role', label: enumLabel('rbacPermissionTargetType', 'role') },
    { value: 'permission', label: enumLabel('rbacPermissionTargetType', 'permission') },
    { value: 'menu', label: enumLabel('rbacPermissionTargetType', 'menu') },
    { value: 'resource', label: enumLabel('rbacPermissionTargetType', 'resource') },
    { value: 'agent', label: enumLabel('rbacPermissionTargetType', 'agent') },
    { value: 'knowledge', label: enumLabel('rbacPermissionTargetType', 'knowledge') },
    { value: 'tool', label: enumLabel('rbacPermissionTargetType', 'tool') },
    { value: 'skill', label: enumLabel('rbacPermissionTargetType', 'skill') },
    { value: 'mcp', label: enumLabel('rbacPermissionTargetType', 'mcp') },
  ] satisfies Array<{ value: RbacPermissionTargetType, label: string }>
})

const metrics = computed(() => [
  { id: 'total', label: t('permissionCenter.permissions.metrics.total'), value: String(workspaceAccessStore.permissions.length) },
  {
    id: 'disabled',
    label: t('permissionCenter.permissions.metrics.disabled'),
    value: String(workspaceAccessStore.permissions.filter(permission => permission.status === 'disabled').length),
  },
])

const pageCount = computed(() => Math.max(1, Math.ceil(workspaceAccessStore.permissions.length / PAGE_SIZE)))
const pagedPermissions = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE
  return workspaceAccessStore.permissions.slice(start, start + PAGE_SIZE)
})

const roleUsageCountByPermissionId = computed(() => new Map(
  workspaceAccessStore.permissions.map(permission => [
    permission.id,
    workspaceAccessStore.roles.filter(role => role.permissionIds.includes(permission.id)).length,
  ]),
))

const targetOptions = computed(() => {
  switch (form.targetType) {
    case 'workspace':
      return workspaceStore.activeWorkspace
        ? [{ value: workspaceStore.activeWorkspace.id, label: workspaceStore.activeWorkspace.name }]
        : []
    case 'project':
      return workspaceStore.projects.map(project => ({ value: project.id, label: project.name }))
    case 'user':
      return workspaceAccessStore.users.map(user => ({ value: user.id, label: user.displayName || user.username }))
    case 'role':
      return workspaceAccessStore.roles.map(role => ({ value: role.id, label: role.name }))
    case 'permission':
      return workspaceAccessStore.permissions
        .filter(permission => permission.id !== selectedPermissionId.value)
        .map(permission => ({ value: permission.id, label: `${permission.name} · ${permission.code}` }))
    case 'menu':
      return workspaceAccessStore.menus.map(menu => ({ value: menu.id, label: menu.label }))
    case 'resource':
      return resourceStore.workspaceResources.map(resource => ({ value: resource.id, label: resource.name }))
    case 'agent':
      return agentStore.agents.map(agent => ({ value: agent.id, label: agent.name }))
    case 'knowledge':
      return knowledgeStore.workspaceKnowledge.map(knowledge => ({ value: knowledge.id, label: knowledge.title }))
    case 'tool':
      return catalogStore.tools.map(tool => ({ value: tool.id, label: tool.name }))
    default:
      return []
  }
})

const memberPermissionOptions = computed(() =>
  workspaceAccessStore.permissions
    .filter(permission => permission.id !== selectedPermissionId.value)
    .map(permission => ({
      value: permission.id,
      label: `${permission.name} · ${permission.code}`,
      description: permission.description,
    })),
)

watch(
  () => workspaceAccessStore.permissions.map(permission => permission.id).join('|'),
  () => {
    if (currentPage.value > pageCount.value) {
      currentPage.value = pageCount.value
    }
    if (!selectedPermissionId.value || !workspaceAccessStore.permissions.some(permission => permission.id === selectedPermissionId.value)) {
      applyPermission(workspaceAccessStore.permissions[0]?.id)
      return
    }
    applyPermission(selectedPermissionId.value)
  },
  { immediate: true },
)

watch(
  () => workspaceStore.currentWorkspaceId,
  (workspaceId) => {
    if (!workspaceId) {
      return
    }
    void Promise.all([
      resourceStore.loadWorkspaceResources(),
      knowledgeStore.loadWorkspaceKnowledge(),
      agentStore.load(),
      catalogStore.load(),
    ])
  },
  { immediate: true },
)

function resetFormState() {
  form.name = ''
  form.code = ''
  form.description = ''
  form.status = 'active'
  form.kind = 'atomic'
  form.targetType = 'workspace'
  form.action = 'view'
  form.targetIds = workspaceStore.currentWorkspaceId ? [workspaceStore.currentWorkspaceId] : []
  form.memberPermissionIds = []
}

function applyPermission(permissionId?: string) {
  const permission = workspaceAccessStore.permissions.find(item => item.id === permissionId)
  if (!permission) {
    selectedPermissionId.value = ''
    resetFormState()
    return
  }

  selectedPermissionId.value = permission.id
  form.name = permission.name
  form.code = permission.code
  form.description = permission.description
  form.status = permission.status
  form.kind = permission.kind
  form.targetType = permission.targetType ?? 'workspace'
  form.action = permission.action ?? 'view'
  form.targetIds = [...permission.targetIds]
  form.memberPermissionIds = [...permission.memberPermissionIds]
}

function createPermissionDraft() {
  selectedPermissionId.value = ''
  resetFormState()
  saveMessage.value = ''
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
    targetType: form.targetType,
    targetIds: form.kind === 'bundle' ? [] : [...form.targetIds],
    action: form.kind === 'bundle' ? undefined : (form.action.trim() || undefined),
    memberPermissionIds: form.kind === 'bundle' ? [...form.memberPermissionIds] : [],
  }

  if (selectedPermissionId.value) {
    const updated = await workspaceAccessStore.updatePermission(selectedPermissionId.value, record)
    applyPermission(updated.id)
    saveMessage.value = t('permissionCenter.permissions.feedback.saved')
    return
  }

  const created = await workspaceAccessStore.createPermission(record)
  selectedPermissionId.value = created.id
  applyPermission(created.id)
  saveMessage.value = t('permissionCenter.permissions.feedback.saved')
}

function promptDeletePermission(permissionId: string) {
  pendingDeletePermissionId.value = permissionId
  deleteDialogOpen.value = true
}

async function confirmDeletePermission() {
  if (!pendingDeletePermissionId.value) {
    return
  }
  await workspaceAccessStore.deletePermission(pendingDeletePermissionId.value)
  deleteDialogOpen.value = false
  pendingDeletePermissionId.value = ''
  applyPermission(workspaceAccessStore.permissions[0]?.id)
  saveMessage.value = t('permissionCenter.permissions.feedback.deleted')
}
</script>

<template>
  <div data-testid="permission-center-permissions-shell">
    <UiListDetailShell>
      <template #list>
        <section class="space-y-3">
          <div class="grid gap-3 md:grid-cols-2">
            <UiMetricCard v-for="metric in metrics" :key="metric.id" :label="metric.label" :value="metric.value" />
          </div>

          <UiPanelFrame
            variant="subtle"
            padding="md"
            :title="t('permissionCenter.permissions.listTitle')"
            :subtitle="t('permissionCenter.permissions.listSubtitle')"
          >
            <template #actions>
              <UiButton data-testid="permissions-create-button" @click="createPermissionDraft">
                {{ t('permissionCenter.permissions.actions.create') }}
              </UiButton>
            </template>
          </UiPanelFrame>

          <UiRecordCard
            v-for="permission in pagedPermissions"
            :key="permission.id"
            :title="permission.name"
            :description="permission.description"
            interactive
            :active="selectedPermissionId === permission.id"
            @click="applyPermission(permission.id)"
          >
            <template #badges>
              <UiBadge :label="enumLabel('rbacPermissionKind', permission.kind)" subtle />
              <UiBadge :label="enumLabel('recordStatus', permission.status)" subtle />
              <UiBadge
                v-if="permission.targetType"
                :label="enumLabel('rbacPermissionTargetType', permission.targetType)"
                subtle
              />
              <UiButton
                variant="destructive"
                size="sm"
                :data-testid="`permissions-delete-button-${permission.code}`"
                @click.stop="promptDeletePermission(permission.id)"
              >
                {{ t('permissionCenter.permissions.actions.delete') }}
              </UiButton>
            </template>
            <div class="mt-3 flex flex-wrap gap-2 text-xs text-text-secondary">
              <span>{{ t('permissionCenter.permissions.usedByRoles', { count: roleUsageCountByPermissionId.get(permission.id) ?? 0 }) }}</span>
              <span v-if="permission.kind === 'bundle'">{{ t('permissionCenter.permissions.bundleMembers', { count: permission.memberPermissionIds.length }) }}</span>
            </div>
          </UiRecordCard>

          <UiPagination
            v-model:page="currentPage"
            :page-count="pageCount"
            :summary-label="`${workspaceAccessStore.permissions.length}`"
            root-test-id="permissions-list-pagination"
          />
        </section>
      </template>

      <div data-testid="permission-center-permissions-inspector">
        <UiInspectorPanel
          :title="selectedPermissionId ? t('permissionCenter.permissions.listTitle') : t('permissionCenter.permissions.actions.create')"
          :subtitle="selectedPermissionId ? form.code : t('permissionCenter.permissions.listSubtitle')"
        >
          <div class="space-y-4">
            <UiStatusCallout v-if="saveMessage" tone="success" :description="saveMessage" />

            <UiField :label="t('permissionCenter.permissions.fields.name')">
              <UiInput v-model="form.name" data-testid="permissions-name-input" />
            </UiField>
            <UiField :label="t('permissionCenter.permissions.fields.code')">
              <UiInput v-model="form.code" data-testid="permissions-code-input" />
            </UiField>
            <UiField :label="t('common.status')">
              <UiSelect v-model="form.status" :options="statusOptions" data-testid="permissions-status-select" />
            </UiField>
            <UiField :label="t('permissionCenter.permissions.fields.kind')">
              <UiSelect v-model="form.kind" :options="kindOptions" data-testid="permissions-kind-select" />
            </UiField>
            <UiField :label="t('permissionCenter.permissions.fields.targetType')">
              <UiSelect v-model="form.targetType" :options="targetTypeOptions" data-testid="permissions-target-type-select" />
            </UiField>

            <UiField v-if="form.kind === 'atomic'" :label="t('permissionCenter.permissions.fields.action')">
              <UiInput v-model="form.action" data-testid="permissions-action-input" />
            </UiField>

            <UiField
              v-if="form.kind === 'atomic' && targetOptions.length"
              :label="t('permissionCenter.permissions.fields.targetIds')"
            >
              <div class="space-y-2">
                <label
                  v-for="option in targetOptions"
                  :key="option.value"
                  class="block rounded-[var(--radius-m)] border border-border bg-surface p-3"
                >
                  <UiCheckbox
                    v-model="form.targetIds"
                    :value="option.value"
                    :data-testid="`permissions-target-${option.value}`"
                  >
                    <span class="font-medium text-text-primary">{{ option.label }}</span>
                  </UiCheckbox>
                </label>
              </div>
            </UiField>

            <UiField v-else-if="form.kind === 'atomic'" :label="t('permissionCenter.permissions.fields.targetIds')">
              <UiPanelFrame variant="subtle" padding="sm">
                <div class="text-sm text-text-tertiary" data-testid="permissions-targets-empty">
                  {{ t('permissionCenter.permissions.emptyTargets') }}
                </div>
              </UiPanelFrame>
            </UiField>

            <UiField v-if="form.kind === 'bundle'" :label="t('permissionCenter.permissions.fields.memberPermissionIds')">
              <div class="space-y-2">
                <label
                  v-for="permission in memberPermissionOptions"
                  :key="permission.value"
                  class="block rounded-[var(--radius-m)] border border-border bg-surface p-3"
                >
                  <UiCheckbox
                    v-model="form.memberPermissionIds"
                    :value="permission.value"
                    :data-testid="`permissions-member-${permission.value}`"
                  >
                    <span class="font-medium text-text-primary">{{ permission.label }}</span>
                  </UiCheckbox>
                  <div class="mt-1 text-xs text-text-secondary">
                    {{ permission.description }}
                  </div>
                </label>
              </div>
            </UiField>

            <UiField :label="t('permissionCenter.permissions.fields.description')">
              <UiTextarea v-model="form.description" :rows="5" data-testid="permissions-description-input" />
            </UiField>
            <div class="flex gap-3">
              <UiButton data-testid="permissions-save-button" @click="savePermission">{{ t('permissionCenter.permissions.actions.save') }}</UiButton>
              <UiButton variant="ghost" @click="selectedPermissionId ? applyPermission(selectedPermissionId) : createPermissionDraft()">{{ t('permissionCenter.permissions.actions.reset') }}</UiButton>
            </div>
          </div>
        </UiInspectorPanel>
      </div>
    </UiListDetailShell>
  </div>

  <UiDialog
    v-model:open="deleteDialogOpen"
    :title="t('permissionCenter.permissions.deleteTitle')"
    :description="t('permissionCenter.permissions.deleteDescription')"
  >
    <template #footer>
      <UiButton variant="ghost" @click="deleteDialogOpen = false">
        {{ t('common.cancel') }}
      </UiButton>
      <UiButton data-testid="permissions-delete-confirm-button" @click="confirmDeletePermission">
        {{ t('common.confirm') }}
      </UiButton>
    </template>
  </UiDialog>
</template>
