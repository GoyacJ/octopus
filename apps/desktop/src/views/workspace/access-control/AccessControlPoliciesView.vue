<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiEmptyState,
  UiField,
  UiHierarchyList,
  UiInput,
  UiListDetailWorkspace,
  UiPagination,
  UiPanelFrame,
  UiRecordCard,
  UiSelect,
  UiSurface,
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

import { usePagination } from '@/composables/usePagination'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import {
  getAccessRoleName,
  getPermissionDisplayDescription,
  getPermissionDisplayName,
  getResourceActionLabel,
} from './display-i18n'
import {
  createDataResourceTypeOptions,
  createPolicyEffectOptions,
  createResourceTypeOptions,
  createScopeTypeOptions,
  createSubjectTypeOptions,
  getCapabilityModuleLabel,
  getDataResourceTypeLabel,
  getPolicyEffectLabel,
  getResourceTypeLabel,
  getScopeTypeLabel,
  getSubjectTypeLabel,
  parseListInput,
  stringifyListInput,
} from './helpers'
import { useAccessControlNotifications } from './useAccessControlNotifications'
import { useAccessControlSelection } from './useAccessControlSelection'

interface RoleBindingFormState {
  roleId: string
  subjectType: string
  subjectId: string
  effect: string
}

interface DataPolicyFormState {
  name: string
  subjectType: string
  subjectId: string
  resourceType: string
  scopeType: string
  projectIdsText: string
  tagsText: string
  classificationsText: string
  effect: string
}

interface ResourcePolicyFormState {
  subjectType: string
  subjectId: string
  resourceType: string
  resourceId: string
  action: string
  effect: string
}

interface PermissionTreeItem {
  id: string
  label: string
  description?: string
  depth: number
  expandable?: boolean
  expanded?: boolean
  selectable?: boolean
  testId: string
  kind: 'module' | 'permission'
  permission?: PermissionDefinition
}

const { t } = useI18n()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifyError, notifySuccess, notifyWarning } = useAccessControlNotifications('access-control.policies')

const activeSection = ref('permissions')
const permissionQuery = ref('')
const bindingQuery = ref('')
const dataPolicyQuery = ref('')
const resourcePolicyQuery = ref('')
const resourcePolicyTypeFilter = ref('')
const submitError = ref('')

const selectedPermissionCode = ref('')
const selectedRoleBindingId = ref('')
const selectedDataPolicyId = ref('')
const selectedResourcePolicyId = ref('')

const createBindingDialogOpen = ref(false)
const createDataPolicyDialogOpen = ref(false)
const createResourcePolicyDialogOpen = ref(false)
const bulkDeleteBindingsDialogOpen = ref(false)
const bulkDeleteDataPoliciesDialogOpen = ref(false)
const bulkDeleteResourcePoliciesDialogOpen = ref(false)
const expandedPermissionModuleIds = ref<string[]>([])

const savingRoleBinding = ref(false)
const savingDataPolicy = ref(false)
const savingResourcePolicy = ref(false)
const deletingSelectedBindings = ref(false)
const deletingSelectedDataPolicies = ref(false)
const deletingSelectedResourcePolicies = ref(false)
const hiddenRoleBindingIds = ref<string[]>([])

const createRoleBindingForm = reactive<RoleBindingFormState>(createEmptyRoleBindingForm())
const editRoleBindingForm = reactive<RoleBindingFormState>(createEmptyRoleBindingForm())
const createDataPolicyForm = reactive<DataPolicyFormState>(createEmptyDataPolicyForm())
const editDataPolicyForm = reactive<DataPolicyFormState>(createEmptyDataPolicyForm())
const createResourcePolicyForm = reactive<ResourcePolicyFormState>(createEmptyResourcePolicyForm())
const editResourcePolicyForm = reactive<ResourcePolicyFormState>(createEmptyResourcePolicyForm())

const sectionTabs = computed(() => [
  { value: 'permissions', label: t('accessControl.policies.sections.permissions') },
  { value: 'bindings', label: t('accessControl.policies.sections.bindings') },
  { value: 'data', label: t('accessControl.policies.sections.data') },
  { value: 'resources', label: t('accessControl.policies.sections.resources') },
])

const subjectTypeOptions = computed(() => createSubjectTypeOptions(t))
const policyEffectOptions = computed(() => createPolicyEffectOptions(t))
const scopeTypeOptions = computed(() => createScopeTypeOptions(t))
const dataResourceTypeOptions = computed(() => createDataResourceTypeOptions(t))
const resourceTypeOptions = computed(() => createResourceTypeOptions(t))

const roleMap = computed(() =>
  new Map(accessControlStore.roles.map(role => [role.id, getAccessRoleName(role)])),
)
const roleOptions = computed(() =>
  accessControlStore.roles.map(role => ({ label: getAccessRoleName(role), value: role.id })),
)
const subjectOptions = computed(() => ({
  user: accessControlStore.users.map(user => ({ label: user.displayName, value: user.id })),
  org_unit: accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
  position: accessControlStore.positions.map(position => ({ label: position.name, value: position.id })),
  user_group: accessControlStore.userGroups.map(group => ({ label: group.name, value: group.id })),
}))
const resourceOptions = computed(() =>
  accessControlStore.protectedResources.map(resource => ({
    label: `${resource.name} (${resource.id})`,
    value: resource.id,
    resourceType: resource.resourceType,
  })),
)

const filteredPermissions = computed(() => {
  const normalizedQuery = permissionQuery.value.trim().toLowerCase()
  return [...accessControlStore.permissionDefinitions]
    .sort((left, right) => left.code.localeCompare(right.code))
    .filter(permission => !normalizedQuery || [
      getPermissionDisplayName(permission),
      permission.code,
      getPermissionDisplayDescription(permission),
      permission.resourceType,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})
const permissionModuleItems = computed(() => {
  const grouped = new Map<string, PermissionDefinition[]>()
  for (const permission of filteredPermissions.value) {
    const [moduleName = permission.code] = permission.code.split('.')
    const items = grouped.get(moduleName) ?? []
    items.push(permission)
    grouped.set(moduleName, items)
  }

  return Array.from(grouped.entries())
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([moduleName, permissions]) => ({
      moduleName,
      permissions: [...permissions].sort((left, right) => left.code.localeCompare(right.code)),
    }))
})

watch(permissionModuleItems, (modules) => {
  const moduleNames = modules.map(module => module.moduleName)
  const next = expandedPermissionModuleIds.value.filter(id => moduleNames.includes(id))
  if (!expandedPermissionModuleIds.value.length) {
    expandedPermissionModuleIds.value = [...moduleNames]
    return
  }
  expandedPermissionModuleIds.value = next
}, { immediate: true })

const filteredRoleBindings = computed(() => {
  const normalizedQuery = bindingQuery.value.trim().toLowerCase()
  return [...accessControlStore.roleBindings]
    .filter(binding => !hiddenRoleBindingIds.value.includes(binding.id))
    .sort((left, right) => bindingTitle(left).localeCompare(bindingTitle(right)))
    .filter(binding => !normalizedQuery || [
      bindingTitle(binding),
      bindingSubjectLabel(binding),
      binding.subjectType,
      binding.effect,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredDataPolicies = computed(() => {
  const normalizedQuery = dataPolicyQuery.value.trim().toLowerCase()
  return [...accessControlStore.dataPolicies]
    .sort((left, right) => left.name.localeCompare(right.name))
    .filter(policy => !normalizedQuery || [
      policy.name,
      resolveSubjectLabel(policy.subjectType, policy.subjectId),
      policy.resourceType,
      policy.scopeType,
      ...policy.projectIds,
      ...policy.tags,
      ...policy.classifications,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredResourcePolicies = computed(() => {
  const normalizedQuery = resourcePolicyQuery.value.trim().toLowerCase()
  return [...accessControlStore.resourcePolicies]
    .filter(policy => !resourcePolicyTypeFilter.value || policy.resourceType === resourcePolicyTypeFilter.value)
    .sort((left, right) => getResourceActionLabel(left.action).localeCompare(getResourceActionLabel(right.action)))
    .filter(policy => !normalizedQuery || [
      getResourceActionLabel(policy.action),
      policy.resourceType,
      policy.resourceId,
      resolveSubjectLabel(policy.subjectType, policy.subjectId),
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const permissionsPagination = usePagination(permissionModuleItems, {
  pageSize: 20,
  resetOn: [permissionQuery],
})
const bindingsPagination = usePagination(filteredRoleBindings, {
  pageSize: 8,
  resetOn: [bindingQuery],
})
const dataPoliciesPagination = usePagination(filteredDataPolicies, {
  pageSize: 8,
  resetOn: [dataPolicyQuery],
})
const resourcePoliciesPagination = usePagination(filteredResourcePolicies, {
  pageSize: 8,
  resetOn: [resourcePolicyQuery, resourcePolicyTypeFilter],
})
const bindingSelection = useAccessControlSelection(() => accessControlStore.roleBindings, {
  getId: binding => binding.id,
  resetOn: [activeSection],
})
const dataPolicySelection = useAccessControlSelection(() => accessControlStore.dataPolicies, {
  getId: policy => policy.id,
  resetOn: [activeSection],
})
const resourcePolicySelection = useAccessControlSelection(() => accessControlStore.resourcePolicies, {
  getId: policy => policy.id,
  resetOn: [activeSection],
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
const visiblePermissionTreeItems = computed<PermissionTreeItem[]>(() => {
  const items: PermissionTreeItem[] = []
  const queryActive = Boolean(permissionQuery.value.trim())

  for (const module of permissionsPagination.pagedItems.value) {
    const expanded = queryActive
      || expandedPermissionModuleIds.value.includes(module.moduleName)
    items.push({
      id: module.moduleName,
      label: getCapabilityModuleLabel(t, module.moduleName),
      description: module.moduleName,
      depth: 0,
      expandable: true,
      expanded,
      selectable: false,
      testId: `access-control-permission-module-${module.moduleName}`,
      kind: 'module',
    })

    if (!expanded) {
      continue
    }

    for (const permission of module.permissions) {
      items.push({
        id: permission.code,
        label: getPermissionDisplayName(permission),
        description: permission.code,
        depth: 1,
        selectable: true,
        testId: `access-control-permission-leaf-${permission.code}`,
        kind: 'permission',
        permission,
      })
    }
  }

  return items
})

const resourcePolicyFilterOptions = computed(() => [
  { label: t('accessControl.common.filters.allTypes'), value: '' },
  ...resourceTypeOptions.value,
])

const createRoleBindingSubjectOptions = computed(() => resolveSubjectOptions(createRoleBindingForm.subjectType))
const editRoleBindingSubjectOptions = computed(() => resolveSubjectOptions(editRoleBindingForm.subjectType))
const createDataPolicySubjectOptions = computed(() => resolveSubjectOptions(createDataPolicyForm.subjectType))
const editDataPolicySubjectOptions = computed(() => resolveSubjectOptions(editDataPolicyForm.subjectType))
const createResourcePolicySubjectOptions = computed(() => resolveSubjectOptions(createResourcePolicyForm.subjectType))
const editResourcePolicySubjectOptions = computed(() => resolveSubjectOptions(editResourcePolicyForm.subjectType))
const createResourceChoices = computed(() => resolveResourceOptions(createResourcePolicyForm.resourceType))
const editResourceChoices = computed(() => resolveResourceOptions(editResourcePolicyForm.resourceType))
const allVisibleBindingsSelected = computed(() => bindingSelection.isPageSelected(bindingsPagination.pagedItems.value))
const allVisibleDataPoliciesSelected = computed(() => dataPolicySelection.isPageSelected(dataPoliciesPagination.pagedItems.value))
const allVisibleResourcePoliciesSelected = computed(() =>
  resourcePolicySelection.isPageSelected(resourcePoliciesPagination.pagedItems.value),
)
const selectedBindingsForDelete = computed(() =>
  bindingSelection.selectedIds.value
    .map(id => accessControlStore.roleBindings.find(binding => binding.id === id) ?? null)
    .filter((binding): binding is NonNullable<typeof binding> => Boolean(binding)),
)
const selectedDataPoliciesForDelete = computed(() =>
  dataPolicySelection.selectedIds.value
    .map(id => accessControlStore.dataPolicies.find(policy => policy.id === id) ?? null)
    .filter((policy): policy is NonNullable<typeof policy> => Boolean(policy)),
)
const selectedResourcePoliciesForDelete = computed(() =>
  resourcePolicySelection.selectedIds.value
    .map(id => accessControlStore.resourcePolicies.find(policy => policy.id === id) ?? null)
    .filter((policy): policy is NonNullable<typeof policy> => Boolean(policy)),
)

watch(visiblePermissionTreeItems, (items) => {
  if (selectedPermissionCode.value && !items.some(item => item.kind === 'permission' && item.id === selectedPermissionCode.value)) {
    selectedPermissionCode.value = ''
  }
}, { immediate: true })

watch(bindingsPagination.pagedItems, (bindings) => {
  if (selectedRoleBindingId.value && !bindings.some(binding => binding.id === selectedRoleBindingId.value)) {
    selectedRoleBindingId.value = ''
  }
}, { immediate: true })

watch(dataPoliciesPagination.pagedItems, (policies) => {
  if (selectedDataPolicyId.value && !policies.some(policy => policy.id === selectedDataPolicyId.value)) {
    selectedDataPolicyId.value = ''
  }
}, { immediate: true })

watch(resourcePoliciesPagination.pagedItems, (policies) => {
  if (selectedResourcePolicyId.value && !policies.some(policy => policy.id === selectedResourcePolicyId.value)) {
    selectedResourcePolicyId.value = ''
  }
}, { immediate: true })

watch(selectedRoleBinding, (binding) => {
  Object.assign(editRoleBindingForm, binding ? toRoleBindingForm(binding) : createEmptyRoleBindingForm())
}, { immediate: true })

watch(selectedDataPolicy, (policy) => {
  Object.assign(editDataPolicyForm, policy ? toDataPolicyForm(policy) : createEmptyDataPolicyForm())
}, { immediate: true })

watch(selectedResourcePolicy, (policy) => {
  Object.assign(editResourcePolicyForm, policy ? toResourcePolicyForm(policy) : createEmptyResourcePolicyForm())
}, { immediate: true })

function createEmptyRoleBindingForm(): RoleBindingFormState {
  return {
    roleId: accessControlStore.roles[0]?.id ?? '',
    subjectType: 'user',
    subjectId: accessControlStore.users[0]?.id ?? '',
    effect: 'allow',
  }
}

function createEmptyDataPolicyForm(): DataPolicyFormState {
  return {
    name: '',
    subjectType: 'user',
    subjectId: accessControlStore.users[0]?.id ?? '',
    resourceType: 'project',
    scopeType: 'selected-projects',
    projectIdsText: '',
    tagsText: '',
    classificationsText: '',
    effect: 'allow',
  }
}

function createEmptyResourcePolicyForm(): ResourcePolicyFormState {
  const defaultResourceType = accessControlStore.protectedResources[0]?.resourceType ?? 'agent'
  const defaultResourceId = accessControlStore.protectedResources.find(resource => resource.resourceType === defaultResourceType)?.id ?? ''

  return {
    subjectType: 'user',
    subjectId: accessControlStore.users[0]?.id ?? '',
    resourceType: defaultResourceType,
    resourceId: defaultResourceId,
    action: 'view',
    effect: 'allow',
  }
}

function toRoleBindingForm(binding: RoleBindingRecord): RoleBindingFormState {
  return {
    roleId: binding.roleId,
    subjectType: binding.subjectType,
    subjectId: binding.subjectId,
    effect: binding.effect,
  }
}

function toDataPolicyForm(policy: DataPolicyRecord): DataPolicyFormState {
  return {
    name: policy.name,
    subjectType: policy.subjectType,
    subjectId: policy.subjectId,
    resourceType: policy.resourceType,
    scopeType: policy.scopeType,
    projectIdsText: stringifyListInput(policy.projectIds),
    tagsText: stringifyListInput(policy.tags),
    classificationsText: stringifyListInput(policy.classifications),
    effect: policy.effect,
  }
}

function toResourcePolicyForm(policy: ResourcePolicyRecord): ResourcePolicyFormState {
  return {
    subjectType: policy.subjectType,
    subjectId: policy.subjectId,
    resourceType: policy.resourceType,
    resourceId: policy.resourceId,
    action: policy.action,
    effect: policy.effect,
  }
}

function resolveSubjectOptions(subjectType: string) {
  return subjectOptions.value[subjectType as keyof typeof subjectOptions.value] ?? []
}

function resolveResourceOptions(resourceType: string) {
  return resourceOptions.value
    .filter(option => option.resourceType === resourceType)
    .map(option => ({ label: option.label, value: option.value }))
}

function resolveSubjectLabel(subjectType: string, subjectId: string) {
  return resolveSubjectOptions(subjectType).find(option => option.value === subjectId)?.label ?? subjectId
}

function resolveResourceLabel(resourceType: string, resourceId: string) {
  return accessControlStore.protectedResources.find(resource =>
    resource.resourceType === resourceType && resource.id === resourceId,
  )?.name ?? resourceId
}

function bindingTitle(binding: Pick<RoleBindingRecord, 'roleId'>) {
  return roleMap.value.get(binding.roleId) ?? binding.roleId
}

function bindingSubjectLabel(binding: Pick<RoleBindingRecord, 'subjectType' | 'subjectId'>) {
  return resolveSubjectLabel(binding.subjectType, binding.subjectId)
}

function hideRoleBinding(bindingId: string) {
  if (!hiddenRoleBindingIds.value.includes(bindingId)) {
    hiddenRoleBindingIds.value = [...hiddenRoleBindingIds.value, bindingId]
  }
}

function showRoleBinding(bindingId: string) {
  hiddenRoleBindingIds.value = hiddenRoleBindingIds.value.filter(id => id !== bindingId)
}

function selectPermission(permission: PermissionDefinition) {
  selectedPermissionCode.value = permission.code
  submitError.value = ''
  const [moduleName = permission.code] = permission.code.split('.')
  if (!expandedPermissionModuleIds.value.includes(moduleName)) {
    expandedPermissionModuleIds.value = [...expandedPermissionModuleIds.value, moduleName]
  }
}

function selectPermissionCode(permissionCode: string) {
  const permission = accessControlStore.permissionDefinitions.find(item => item.code === permissionCode)
  if (!permission) {
    return
  }
  selectPermission(permission)
}

function togglePermissionModule(moduleName: string) {
  const next = new Set(expandedPermissionModuleIds.value)
  if (next.has(moduleName)) {
    next.delete(moduleName)
  } else {
    next.add(moduleName)
  }
  expandedPermissionModuleIds.value = Array.from(next)
}

function selectRoleBinding(bindingId: string) {
  selectedRoleBindingId.value = bindingId
  submitError.value = ''
}

function selectDataPolicy(policyId: string) {
  selectedDataPolicyId.value = policyId
  submitError.value = ''
}

function selectResourcePolicy(policyId: string) {
  selectedResourcePolicyId.value = policyId
  submitError.value = ''
}

function toggleBindingSelection(bindingId: string, value: boolean) {
  bindingSelection.toggleSelection(bindingId, value)
}

function toggleVisibleBindings(value: boolean) {
  bindingSelection.selectPage(bindingsPagination.pagedItems.value, value)
}

function toggleDataPolicySelection(policyId: string, value: boolean) {
  dataPolicySelection.toggleSelection(policyId, value)
}

function toggleVisibleDataPolicies(value: boolean) {
  dataPolicySelection.selectPage(dataPoliciesPagination.pagedItems.value, value)
}

function toggleResourcePolicySelection(policyId: string, value: boolean) {
  resourcePolicySelection.toggleSelection(policyId, value)
}

function toggleVisibleResourcePolicies(value: boolean) {
  resourcePolicySelection.selectPage(resourcePoliciesPagination.pagedItems.value, value)
}

function getPermissionDefinitionByCode(permissionCode: string) {
  return accessControlStore.permissionDefinitions.find(permission => permission.code === permissionCode) ?? null
}

async function notifyBulkDeleteResult(
  successCount: number,
  failureCount: number,
  skippedCount: number,
  clearSelection: () => void,
) {
  await nextTick()

  const body = t('accessControl.common.bulk.resultBody', {
    success: successCount,
    failure: failureCount,
    skipped: skippedCount,
  })

  if (successCount > 0 && failureCount === 0 && skippedCount === 0) {
    clearSelection()
    await notifySuccess(t('accessControl.common.bulk.resultAllSuccessTitle'), body)
    return
  }

  if (successCount > 0 || skippedCount > 0) {
    await notifyWarning(t('accessControl.common.bulk.resultPartialTitle'), body)
    return
  }

  await notifyError(t('accessControl.common.bulk.resultFailureTitle'), body)
}

function openCreateRoleBindingDialog() {
  Object.assign(createRoleBindingForm, createEmptyRoleBindingForm())
  submitError.value = ''
  createBindingDialogOpen.value = true
}

function openCreateDataPolicyDialog() {
  Object.assign(createDataPolicyForm, createEmptyDataPolicyForm())
  submitError.value = ''
  createDataPolicyDialogOpen.value = true
}

function openCreateResourcePolicyDialog() {
  Object.assign(createResourcePolicyForm, createEmptyResourcePolicyForm())
  submitError.value = ''
  createResourcePolicyDialogOpen.value = true
}

function validateRoleBindingForm(form: RoleBindingFormState) {
  if (!form.roleId || !form.subjectId) {
    return t('accessControl.policies.validation.bindingRequired')
  }
  return ''
}

function validateDataPolicyForm(form: DataPolicyFormState) {
  if (!form.name.trim() || !form.subjectId) {
    return t('accessControl.policies.validation.dataRequired')
  }
  return ''
}

function validateResourcePolicyForm(form: ResourcePolicyFormState) {
  if (!form.subjectId || !form.resourceId || !form.action.trim()) {
    return t('accessControl.policies.validation.resourceRequired')
  }
  return ''
}

function toRoleBindingPayload(form: RoleBindingFormState): RoleBindingUpsertRequest {
  return {
    roleId: form.roleId,
    subjectType: form.subjectType,
    subjectId: form.subjectId,
    effect: form.effect,
  }
}

function toDataPolicyPayload(form: DataPolicyFormState): DataPolicyUpsertRequest {
  return {
    name: form.name.trim(),
    subjectType: form.subjectType,
    subjectId: form.subjectId,
    resourceType: form.resourceType,
    scopeType: form.scopeType,
    projectIds: parseListInput(form.projectIdsText),
    tags: parseListInput(form.tagsText),
    classifications: parseListInput(form.classificationsText),
    effect: form.effect,
  }
}

function toResourcePolicyPayload(form: ResourcePolicyFormState): ResourcePolicyUpsertRequest {
  return {
    subjectType: form.subjectType,
    subjectId: form.subjectId,
    resourceType: form.resourceType,
    resourceId: form.resourceId,
    action: form.action.trim(),
    effect: form.effect,
  }
}

async function handleCreateRoleBinding() {
  submitError.value = validateRoleBindingForm(createRoleBindingForm)
  if (submitError.value) {
    return
  }

  savingRoleBinding.value = true
  try {
    const record = await accessControlStore.createRoleBinding(toRoleBindingPayload(createRoleBindingForm))
    showRoleBinding(record.id)
    selectedRoleBindingId.value = record.id
    createBindingDialogOpen.value = false
    await notifySuccess(t('accessControl.policies.feedback.toastBindingSaved'), bindingSubjectLabel(record))
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.saveBindingFailed')
  } finally {
    savingRoleBinding.value = false
  }
}

async function handleSaveRoleBinding() {
  if (!selectedRoleBinding.value) {
    return
  }

  submitError.value = validateRoleBindingForm(editRoleBindingForm)
  if (submitError.value) {
    return
  }

  savingRoleBinding.value = true
  try {
    const payload = toRoleBindingPayload(editRoleBindingForm)
    await accessControlStore.updateRoleBinding(selectedRoleBinding.value.id, payload)
    showRoleBinding(selectedRoleBinding.value.id)
    await notifySuccess(t('accessControl.policies.feedback.toastBindingSaved'), resolveSubjectLabel(payload.subjectType, payload.subjectId))
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.saveBindingFailed')
  } finally {
    savingRoleBinding.value = false
  }
}

async function handleDeleteRoleBinding() {
  if (!selectedRoleBinding.value) {
    return
  }

  submitError.value = ''
  try {
    const label = bindingSubjectLabel(selectedRoleBinding.value)
    hideRoleBinding(selectedRoleBinding.value.id)
    await accessControlStore.deleteRoleBinding(selectedRoleBinding.value.id)
    selectedRoleBindingId.value = ''
    await notifySuccess(t('accessControl.policies.feedback.toastBindingDeleted'), label)
  } catch (error) {
    showRoleBinding(selectedRoleBinding.value.id)
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.deleteBindingFailed')
  }
}

async function handleBulkDeleteBindings() {
  if (!selectedBindingsForDelete.value.length) {
    bulkDeleteBindingsDialogOpen.value = false
    return
  }

  deletingSelectedBindings.value = true
  submitError.value = ''
  let successCount = 0
  let failureCount = 0

  for (const binding of selectedBindingsForDelete.value) {
    try {
      hideRoleBinding(binding.id)
      await accessControlStore.deleteRoleBinding(binding.id)
      successCount += 1
      if (selectedRoleBindingId.value === binding.id) {
        selectedRoleBindingId.value = ''
      }
    } catch {
      showRoleBinding(binding.id)
      failureCount += 1
    }
  }

  deletingSelectedBindings.value = false
  bulkDeleteBindingsDialogOpen.value = false
  bindingSelection.setSelection(
    bindingSelection.selectedIds.value.filter(id =>
      accessControlStore.roleBindings.some(binding => binding.id === id),
    ),
  )
  await notifyBulkDeleteResult(successCount, failureCount, 0, () => bindingSelection.clearSelection())
}

async function handleCreateDataPolicy() {
  submitError.value = validateDataPolicyForm(createDataPolicyForm)
  if (submitError.value) {
    return
  }

  savingDataPolicy.value = true
  try {
    const record = await accessControlStore.createDataPolicy(toDataPolicyPayload(createDataPolicyForm))
    selectedDataPolicyId.value = record.id
    createDataPolicyDialogOpen.value = false
    await notifySuccess(t('accessControl.policies.feedback.toastDataSaved'), record.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.saveDataFailed')
  } finally {
    savingDataPolicy.value = false
  }
}

async function handleSaveDataPolicy() {
  if (!selectedDataPolicy.value) {
    return
  }

  submitError.value = validateDataPolicyForm(editDataPolicyForm)
  if (submitError.value) {
    return
  }

  savingDataPolicy.value = true
  try {
    const payload = toDataPolicyPayload(editDataPolicyForm)
    await accessControlStore.updateDataPolicy(selectedDataPolicy.value.id, payload)
    await notifySuccess(t('accessControl.policies.feedback.toastDataSaved'), payload.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.saveDataFailed')
  } finally {
    savingDataPolicy.value = false
  }
}

async function handleDeleteDataPolicy() {
  if (!selectedDataPolicy.value) {
    return
  }

  submitError.value = ''
  try {
    const label = selectedDataPolicy.value.name
    await accessControlStore.deleteDataPolicy(selectedDataPolicy.value.id)
    selectedDataPolicyId.value = ''
    await notifySuccess(t('accessControl.policies.feedback.toastDataDeleted'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.deleteDataFailed')
  }
}

async function handleBulkDeleteDataPolicies() {
  if (!selectedDataPoliciesForDelete.value.length) {
    bulkDeleteDataPoliciesDialogOpen.value = false
    return
  }

  deletingSelectedDataPolicies.value = true
  submitError.value = ''
  let successCount = 0
  let failureCount = 0

  for (const policy of selectedDataPoliciesForDelete.value) {
    try {
      await accessControlStore.deleteDataPolicy(policy.id)
      successCount += 1
      if (selectedDataPolicyId.value === policy.id) {
        selectedDataPolicyId.value = ''
      }
    } catch {
      failureCount += 1
    }
  }

  deletingSelectedDataPolicies.value = false
  bulkDeleteDataPoliciesDialogOpen.value = false
  dataPolicySelection.setSelection(
    dataPolicySelection.selectedIds.value.filter(id =>
      accessControlStore.dataPolicies.some(policy => policy.id === id),
    ),
  )
  await notifyBulkDeleteResult(successCount, failureCount, 0, () => dataPolicySelection.clearSelection())
}

async function handleCreateResourcePolicy() {
  submitError.value = validateResourcePolicyForm(createResourcePolicyForm)
  if (submitError.value) {
    return
  }

  savingResourcePolicy.value = true
  try {
    const record = await accessControlStore.createResourcePolicy(toResourcePolicyPayload(createResourcePolicyForm))
    selectedResourcePolicyId.value = record.id
    createResourcePolicyDialogOpen.value = false
    await notifySuccess(t('accessControl.policies.feedback.toastResourceSaved'), getResourceActionLabel(record.action))
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.saveResourceFailed')
  } finally {
    savingResourcePolicy.value = false
  }
}

async function handleSaveResourcePolicy() {
  if (!selectedResourcePolicy.value) {
    return
  }

  submitError.value = validateResourcePolicyForm(editResourcePolicyForm)
  if (submitError.value) {
    return
  }

  savingResourcePolicy.value = true
  try {
    const payload = toResourcePolicyPayload(editResourcePolicyForm)
    await accessControlStore.updateResourcePolicy(selectedResourcePolicy.value.id, payload)
    await notifySuccess(t('accessControl.policies.feedback.toastResourceSaved'), getResourceActionLabel(payload.action))
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.saveResourceFailed')
  } finally {
    savingResourcePolicy.value = false
  }
}

async function handleDeleteResourcePolicy() {
  if (!selectedResourcePolicy.value) {
    return
  }

  submitError.value = ''
  try {
    const label = getResourceActionLabel(selectedResourcePolicy.value.action)
    await accessControlStore.deleteResourcePolicy(selectedResourcePolicy.value.id)
    selectedResourcePolicyId.value = ''
    await notifySuccess(t('accessControl.policies.feedback.toastResourceDeleted'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.policies.feedback.deleteResourceFailed')
  }
}

async function handleBulkDeleteResourcePolicies() {
  if (!selectedResourcePoliciesForDelete.value.length) {
    bulkDeleteResourcePoliciesDialogOpen.value = false
    return
  }

  deletingSelectedResourcePolicies.value = true
  submitError.value = ''
  let successCount = 0
  let failureCount = 0

  for (const policy of selectedResourcePoliciesForDelete.value) {
    try {
      await accessControlStore.deleteResourcePolicy(policy.id)
      successCount += 1
      if (selectedResourcePolicyId.value === policy.id) {
        selectedResourcePolicyId.value = ''
      }
    } catch {
      failureCount += 1
    }
  }

  deletingSelectedResourcePolicies.value = false
  bulkDeleteResourcePoliciesDialogOpen.value = false
  resourcePolicySelection.setSelection(
    resourcePolicySelection.selectedIds.value.filter(id =>
      accessControlStore.resourcePolicies.some(policy => policy.id === id),
    ),
  )
  await notifyBulkDeleteResult(successCount, failureCount, 0, () => resourcePolicySelection.clearSelection())
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-policies-shell">
    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />

    <UiTabs v-model="activeSection" :tabs="sectionTabs" data-testid="access-control-policies-section-tabs" />

    <UiListDetailWorkspace
      v-if="activeSection === 'permissions'"
      :has-selection="Boolean(selectedPermission)"
      :detail-title="selectedPermission ? getPermissionDisplayName(selectedPermission) : ''"
      :detail-subtitle="t('accessControl.policies.permissions.detailSubtitle')"
      :empty-detail-title="t('accessControl.policies.permissions.emptyTitle')"
      :empty-detail-description="t('accessControl.policies.permissions.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow>
          <template #search>
            <UiInput v-model="permissionQuery" :placeholder="t('accessControl.policies.permissions.toolbarSearch')" />
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.policies.permissions.listTitle')"
          :subtitle="t('accessControl.common.list.totalPermissions', { count: filteredPermissions.length })"
        >
          <UiHierarchyList
            v-if="visiblePermissionTreeItems.length"
            :items="visiblePermissionTreeItems"
            :selected-id="selectedPermissionCode"
            @select="selectPermissionCode"
            @toggle="togglePermissionModule"
          >
            <template #default="{ item }">
              <div class="min-w-0">
                <div class="truncate text-sm font-medium text-text-primary">
                  {{ item.label }}
                </div>
                <div class="truncate pt-0.5 text-xs text-text-secondary">
                  {{ item.description }}
                </div>
              </div>
            </template>

            <template #badges="{ item }">
              <UiBadge
                v-if="getPermissionDefinitionByCode(item.id)"
                :label="getResourceTypeLabel(t, getPermissionDefinitionByCode(item.id)?.resourceType ?? '')"
                subtle
              />
            </template>
          </UiHierarchyList>
          <UiEmptyState
            v-else
            :title="t('accessControl.policies.permissions.noListTitle')"
            :description="t('accessControl.policies.permissions.noListDescription')"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="permissionsPagination.currentPage.value"
              :page-count="permissionsPagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: permissionsPagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedPermission" class="space-y-4">
            <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
              <div class="flex flex-wrap items-center gap-2">
                <div class="text-sm font-semibold text-foreground">{{ getPermissionDisplayName(selectedPermission) }}</div>
                <UiBadge :label="getResourceTypeLabel(t, selectedPermission.resourceType)" subtle />
              </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedPermission.code }}</div>
            <div class="mt-3 text-sm text-text-secondary">{{ getPermissionDisplayDescription(selectedPermission) }}</div>
          </div>

          <UiSurface
            data-testid="access-control-permission-grant-matrix"
            variant="subtle"
            padding="md"
          >
            <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
              {{ t('accessControl.policies.permissions.grantMatrix') }}
            </div>
            <div v-if="accessControlStore.currentResourceActionGrants.length" class="mt-3 space-y-3">
              <div
                v-for="grant in accessControlStore.currentResourceActionGrants"
                :key="grant.resourceType"
                class="rounded-[var(--radius-m)] border border-border bg-muted/30 p-3"
              >
                <div class="text-sm font-medium text-foreground">{{ getResourceTypeLabel(t, grant.resourceType) }}</div>
                <div class="mt-2 flex flex-wrap gap-2">
                  <UiBadge
                    v-for="action in grant.actions"
                    :key="`${grant.resourceType}:${action}`"
                    :label="getResourceActionLabel(action)"
                    subtle
                  />
                </div>
              </div>
            </div>
            <p v-else class="mt-2 text-sm text-text-secondary">
              {{ t('accessControl.policies.permissions.grantMatrixEmpty') }}
            </p>
          </UiSurface>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiListDetailWorkspace
      v-else-if="activeSection === 'bindings'"
      :has-selection="Boolean(selectedRoleBinding)"
      :detail-title="selectedRoleBinding ? bindingTitle(selectedRoleBinding) : ''"
      :detail-subtitle="t('accessControl.policies.bindings.detailSubtitle')"
      :empty-detail-title="t('accessControl.policies.bindings.emptyTitle')"
      :empty-detail-description="t('accessControl.policies.bindings.emptyDescription')"
    >
        <template #toolbar>
          <UiToolbarRow>
            <template #search>
              <UiInput v-model="bindingQuery" :placeholder="t('accessControl.policies.bindings.toolbarSearch')" />
            </template>
            <template #actions>
              <span
                v-if="bindingSelection.hasSelection.value"
                class="text-xs text-text-secondary"
              >
                {{ t('accessControl.common.selection.selectedCount', { count: bindingSelection.selectedCount.value }) }}
              </span>
              <UiButton
                v-if="bindingsPagination.pagedItems.value.length"
                variant="ghost"
                size="sm"
                @click="toggleVisibleBindings(!allVisibleBindingsSelected)"
              >
                {{ t('accessControl.common.selection.selectPage') }}
              </UiButton>
              <UiButton
                v-if="bindingSelection.hasSelection.value"
                variant="ghost"
                size="sm"
                @click="bindingSelection.clearSelection"
              >
                {{ t('accessControl.common.selection.clear') }}
              </UiButton>
              <UiButton
                v-if="bindingSelection.hasSelection.value"
                variant="destructive"
                size="sm"
                data-testid="access-control-policies-bindings-bulk-delete-button"
                @click="bulkDeleteBindingsDialogOpen = true"
              >
                {{ t('accessControl.common.bulk.delete') }}
              </UiButton>
              <UiButton size="sm" @click="openCreateRoleBindingDialog">
                {{ t('accessControl.policies.bindings.create') }}
              </UiButton>
            </template>
          </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.policies.bindings.listTitle')"
          :subtitle="t('accessControl.common.list.totalBindings', { count: bindingsPagination.totalItems.value })"
        >
          <div v-if="bindingsPagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="binding in bindingsPagination.pagedItems.value"
              :key="binding.id"
              layout="compact"
              interactive
              :active="selectedRoleBindingId === binding.id"
              :title="bindingTitle(binding)"
              :description="bindingSubjectLabel(binding)"
              @click="selectRoleBinding(binding.id)"
            >
              <template #secondary>
                <UiBadge :label="getPolicyEffectLabel(t, binding.effect)" subtle />
              </template>
              <template #badges>
                <UiCheckbox
                  :model-value="bindingSelection.isSelected(binding.id)"
                  :data-testid="`access-control-policies-binding-select-${binding.id}`"
                  @click.stop
                  @update:model-value="toggleBindingSelection(binding.id, Boolean($event))"
                />
                <UiBadge :label="getSubjectTypeLabel(t, binding.subjectType)" subtle />
              </template>
            </UiRecordCard>
          </div>
          <UiEmptyState
            v-else
            :title="t('accessControl.policies.bindings.noListTitle')"
            :description="t('accessControl.policies.bindings.noListDescription')"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="bindingsPagination.currentPage.value"
              :page-count="bindingsPagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: bindingsPagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedRoleBinding" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ bindingTitle(selectedRoleBinding) }}</div>
              <UiBadge :label="getPolicyEffectLabel(t, selectedRoleBinding.effect)" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">
              {{ getSubjectTypeLabel(t, selectedRoleBinding.subjectType) }} / {{ bindingSubjectLabel(selectedRoleBinding) }}
            </div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.policies.bindings.fields.role')">
              <UiSelect
                v-model="editRoleBindingForm.roleId"
                :options="roleOptions"
              />
            </UiField>
            <UiField :label="t('accessControl.policies.bindings.fields.subjectType')">
              <UiSelect v-model="editRoleBindingForm.subjectType" :options="subjectTypeOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.bindings.fields.subject')">
              <UiSelect v-model="editRoleBindingForm.subjectId" :options="editRoleBindingSubjectOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.bindings.fields.effect')">
              <UiSelect v-model="editRoleBindingForm.effect" :options="policyEffectOptions" />
            </UiField>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton variant="ghost" class="text-destructive" @click="handleDeleteRoleBinding">
              {{ t('accessControl.policies.bindings.actions.delete') }}
            </UiButton>
            <UiButton :loading="savingRoleBinding" @click="handleSaveRoleBinding">
              {{ t('accessControl.policies.bindings.actions.save') }}
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiListDetailWorkspace
      v-else-if="activeSection === 'data'"
      :has-selection="Boolean(selectedDataPolicy)"
      :detail-title="selectedDataPolicy ? selectedDataPolicy.name : ''"
      :detail-subtitle="t('accessControl.policies.data.detailSubtitle')"
      :empty-detail-title="t('accessControl.policies.data.emptyTitle')"
      :empty-detail-description="t('accessControl.policies.data.emptyDescription')"
    >
        <template #toolbar>
          <UiToolbarRow>
            <template #search>
              <UiInput v-model="dataPolicyQuery" :placeholder="t('accessControl.policies.data.toolbarSearch')" />
            </template>
            <template #actions>
              <span
                v-if="dataPolicySelection.hasSelection.value"
                class="text-xs text-text-secondary"
              >
                {{ t('accessControl.common.selection.selectedCount', { count: dataPolicySelection.selectedCount.value }) }}
              </span>
              <UiButton
                v-if="dataPoliciesPagination.pagedItems.value.length"
                variant="ghost"
                size="sm"
                @click="toggleVisibleDataPolicies(!allVisibleDataPoliciesSelected)"
              >
                {{ t('accessControl.common.selection.selectPage') }}
              </UiButton>
              <UiButton
                v-if="dataPolicySelection.hasSelection.value"
                variant="ghost"
                size="sm"
                @click="dataPolicySelection.clearSelection"
              >
                {{ t('accessControl.common.selection.clear') }}
              </UiButton>
              <UiButton
                v-if="dataPolicySelection.hasSelection.value"
                variant="destructive"
                size="sm"
                data-testid="access-control-policies-data-bulk-delete-button"
                @click="bulkDeleteDataPoliciesDialogOpen = true"
              >
                {{ t('accessControl.common.bulk.delete') }}
              </UiButton>
              <UiButton size="sm" @click="openCreateDataPolicyDialog">
                {{ t('accessControl.policies.data.create') }}
              </UiButton>
            </template>
          </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.policies.data.listTitle')"
          :subtitle="t('accessControl.common.list.totalDataPolicies', { count: dataPoliciesPagination.totalItems.value })"
        >
          <div v-if="dataPoliciesPagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="policy in dataPoliciesPagination.pagedItems.value"
              :key="policy.id"
              layout="compact"
              interactive
              :active="selectedDataPolicyId === policy.id"
              :title="policy.name"
              :description="resolveSubjectLabel(policy.subjectType, policy.subjectId)"
              @click="selectDataPolicy(policy.id)"
            >
              <template #secondary>
                <UiBadge :label="getDataResourceTypeLabel(t, policy.resourceType)" subtle />
                <UiBadge :label="getScopeTypeLabel(t, policy.scopeType)" subtle />
              </template>
              <template #badges>
                <UiCheckbox
                  :model-value="dataPolicySelection.isSelected(policy.id)"
                  :data-testid="`access-control-policies-data-select-${policy.id}`"
                  @click.stop
                  @update:model-value="toggleDataPolicySelection(policy.id, Boolean($event))"
                />
              </template>
            </UiRecordCard>
          </div>
          <UiEmptyState
            v-else
            :title="t('accessControl.policies.data.noListTitle')"
            :description="t('accessControl.policies.data.noListDescription')"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="dataPoliciesPagination.currentPage.value"
              :page-count="dataPoliciesPagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: dataPoliciesPagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedDataPolicy" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedDataPolicy.name }}</div>
              <UiBadge :label="getDataResourceTypeLabel(t, selectedDataPolicy.resourceType)" subtle />
              <UiBadge :label="getPolicyEffectLabel(t, selectedDataPolicy.effect)" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">
              {{ resolveSubjectLabel(selectedDataPolicy.subjectType, selectedDataPolicy.subjectId) }}
            </div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.policies.data.fields.name')">
              <UiInput v-model="editDataPolicyForm.name" />
            </UiField>
            <UiField :label="t('accessControl.policies.data.fields.subjectType')">
              <UiSelect v-model="editDataPolicyForm.subjectType" :options="subjectTypeOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.data.fields.subject')">
              <UiSelect v-model="editDataPolicyForm.subjectId" :options="editDataPolicySubjectOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.data.fields.resourceType')">
              <UiSelect v-model="editDataPolicyForm.resourceType" :options="dataResourceTypeOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.data.fields.scopeType')">
              <UiSelect v-model="editDataPolicyForm.scopeType" :options="scopeTypeOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.data.fields.effect')">
              <UiSelect v-model="editDataPolicyForm.effect" :options="policyEffectOptions" />
            </UiField>
          </div>

          <div class="grid gap-3 md:grid-cols-3">
            <UiField :label="t('accessControl.policies.data.fields.projectIds')" :hint="t('accessControl.policies.data.hints.projectIds')">
              <UiInput v-model="editDataPolicyForm.projectIdsText" />
            </UiField>
            <UiField :label="t('accessControl.policies.data.fields.tags')" :hint="t('accessControl.policies.data.hints.tags')">
              <UiInput v-model="editDataPolicyForm.tagsText" />
            </UiField>
            <UiField :label="t('accessControl.policies.data.fields.classifications')" :hint="t('accessControl.policies.data.hints.classifications')">
              <UiInput v-model="editDataPolicyForm.classificationsText" />
            </UiField>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton variant="ghost" class="text-destructive" @click="handleDeleteDataPolicy">
              {{ t('accessControl.policies.data.actions.delete') }}
            </UiButton>
            <UiButton :loading="savingDataPolicy" @click="handleSaveDataPolicy">
              {{ t('accessControl.policies.data.actions.save') }}
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiListDetailWorkspace
      v-else
      :has-selection="Boolean(selectedResourcePolicy)"
      :detail-title="selectedResourcePolicy ? getResourceActionLabel(selectedResourcePolicy.action) : ''"
      :detail-subtitle="t('accessControl.policies.resources.detailSubtitle')"
      :empty-detail-title="t('accessControl.policies.resources.emptyTitle')"
      :empty-detail-description="t('accessControl.policies.resources.emptyDescription')"
    >
        <template #toolbar>
          <UiToolbarRow>
            <template #search>
              <UiInput v-model="resourcePolicyQuery" :placeholder="t('accessControl.policies.resources.toolbarSearch')" />
            </template>
          <template #filters>
            <UiField :label="t('accessControl.policies.resources.filter')" class="w-full md:w-[220px]">
              <UiSelect v-model="resourcePolicyTypeFilter" :options="resourcePolicyFilterOptions" />
            </UiField>
          </template>
          <template #actions>
            <span
              v-if="resourcePolicySelection.hasSelection.value"
              class="text-xs text-text-secondary"
            >
              {{ t('accessControl.common.selection.selectedCount', { count: resourcePolicySelection.selectedCount.value }) }}
            </span>
            <UiButton
              v-if="resourcePoliciesPagination.pagedItems.value.length"
              variant="ghost"
              size="sm"
              @click="toggleVisibleResourcePolicies(!allVisibleResourcePoliciesSelected)"
            >
              {{ t('accessControl.common.selection.selectPage') }}
            </UiButton>
            <UiButton
              v-if="resourcePolicySelection.hasSelection.value"
              variant="ghost"
              size="sm"
              @click="resourcePolicySelection.clearSelection"
            >
              {{ t('accessControl.common.selection.clear') }}
            </UiButton>
            <UiButton
              v-if="resourcePolicySelection.hasSelection.value"
              variant="destructive"
              size="sm"
              data-testid="access-control-policies-resources-bulk-delete-button"
              @click="bulkDeleteResourcePoliciesDialogOpen = true"
            >
              {{ t('accessControl.common.bulk.delete') }}
            </UiButton>
            <UiButton size="sm" @click="openCreateResourcePolicyDialog">
              {{ t('accessControl.policies.resources.create') }}
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.policies.resources.listTitle')"
          :subtitle="t('accessControl.common.list.totalResourcePolicies', { count: resourcePoliciesPagination.totalItems.value })"
        >
          <div v-if="resourcePoliciesPagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="policy in resourcePoliciesPagination.pagedItems.value"
              :key="policy.id"
              layout="compact"
              interactive
              :active="selectedResourcePolicyId === policy.id"
              :title="getResourceActionLabel(policy.action)"
              :description="resolveResourceLabel(policy.resourceType, policy.resourceId)"
              @click="selectResourcePolicy(policy.id)"
            >
              <template #secondary>
                <UiBadge :label="getResourceTypeLabel(t, policy.resourceType)" subtle />
                <UiBadge :label="getPolicyEffectLabel(t, policy.effect)" subtle />
              </template>
              <template #badges>
                <UiCheckbox
                  :model-value="resourcePolicySelection.isSelected(policy.id)"
                  :data-testid="`access-control-policies-resource-select-${policy.id}`"
                  @click.stop
                  @update:model-value="toggleResourcePolicySelection(policy.id, Boolean($event))"
                />
              </template>
              <template #meta>
                <span class="truncate text-xs text-text-secondary">
                  {{ resolveSubjectLabel(policy.subjectType, policy.subjectId) }}
                </span>
              </template>
            </UiRecordCard>
          </div>
          <UiEmptyState
            v-else
            :title="t('accessControl.policies.resources.noListTitle')"
            :description="t('accessControl.policies.resources.noListDescription')"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="resourcePoliciesPagination.currentPage.value"
              :page-count="resourcePoliciesPagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: resourcePoliciesPagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedResourcePolicy" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ getResourceActionLabel(selectedResourcePolicy.action) }}</div>
              <UiBadge :label="getResourceTypeLabel(t, selectedResourcePolicy.resourceType)" subtle />
              <UiBadge :label="getPolicyEffectLabel(t, selectedResourcePolicy.effect)" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">
              {{ selectedResourcePolicy.resourceId }}
            </div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.policies.resources.fields.subjectType')">
              <UiSelect v-model="editResourcePolicyForm.subjectType" :options="subjectTypeOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.resources.fields.subject')">
              <UiSelect v-model="editResourcePolicyForm.subjectId" :options="editResourcePolicySubjectOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.resources.fields.resourceType')">
              <UiSelect v-model="editResourcePolicyForm.resourceType" :options="resourceTypeOptions" />
            </UiField>
            <UiField :label="t('accessControl.policies.resources.fields.resource')">
              <UiSelect v-model="editResourcePolicyForm.resourceId" :options="editResourceChoices" />
            </UiField>
            <UiField :label="t('accessControl.policies.resources.fields.action')">
              <UiInput v-model="editResourcePolicyForm.action" />
            </UiField>
            <UiField :label="t('accessControl.policies.resources.fields.effect')">
              <UiSelect v-model="editResourcePolicyForm.effect" :options="policyEffectOptions" />
            </UiField>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton variant="ghost" class="text-destructive" @click="handleDeleteResourcePolicy">
              {{ t('accessControl.policies.resources.actions.delete') }}
            </UiButton>
            <UiButton :loading="savingResourcePolicy" @click="handleSaveResourcePolicy">
              {{ t('accessControl.policies.resources.actions.save') }}
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      :open="bulkDeleteBindingsDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.bindings') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeleteBindingsDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedBindingsForDelete.length,
          entity: t('accessControl.common.entities.bindings'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeleteBindingsDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingSelectedBindings"
          data-testid="access-control-policies-bindings-bulk-delete-confirm"
          @click="handleBulkDeleteBindings"
        >
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createBindingDialogOpen"
      :title="t('accessControl.policies.bindings.dialogTitle')"
      :description="t('accessControl.policies.bindings.dialogDescription')"
      @update:open="createBindingDialogOpen = $event"
    >
      <div class="grid gap-3 md:grid-cols-2">
        <UiField :label="t('accessControl.policies.bindings.fields.role')">
          <UiSelect
            v-model="createRoleBindingForm.roleId"
            :options="roleOptions"
          />
        </UiField>
        <UiField :label="t('accessControl.policies.bindings.fields.subjectType')">
          <UiSelect v-model="createRoleBindingForm.subjectType" :options="subjectTypeOptions" />
        </UiField>
        <UiField :label="t('accessControl.policies.bindings.fields.subject')">
          <UiSelect v-model="createRoleBindingForm.subjectId" :options="createRoleBindingSubjectOptions" />
        </UiField>
        <UiField :label="t('accessControl.policies.bindings.fields.effect')">
          <UiSelect v-model="createRoleBindingForm.effect" :options="policyEffectOptions" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createBindingDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton :loading="savingRoleBinding" @click="handleCreateRoleBinding">
          {{ t('accessControl.policies.bindings.actions.create') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="bulkDeleteDataPoliciesDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.dataPolicies') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeleteDataPoliciesDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedDataPoliciesForDelete.length,
          entity: t('accessControl.common.entities.dataPolicies'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeleteDataPoliciesDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingSelectedDataPolicies"
          data-testid="access-control-policies-data-bulk-delete-confirm"
          @click="handleBulkDeleteDataPolicies"
        >
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createDataPolicyDialogOpen"
      :title="t('accessControl.policies.data.dialogTitle')"
      :description="t('accessControl.policies.data.dialogDescription')"
      @update:open="createDataPolicyDialogOpen = $event"
    >
      <div class="space-y-4">
        <div class="grid gap-3 md:grid-cols-2">
          <UiField :label="t('accessControl.policies.data.fields.name')">
            <UiInput v-model="createDataPolicyForm.name" />
          </UiField>
          <UiField :label="t('accessControl.policies.data.fields.subjectType')">
            <UiSelect v-model="createDataPolicyForm.subjectType" :options="subjectTypeOptions" />
          </UiField>
          <UiField :label="t('accessControl.policies.data.fields.subject')">
            <UiSelect v-model="createDataPolicyForm.subjectId" :options="createDataPolicySubjectOptions" />
          </UiField>
          <UiField :label="t('accessControl.policies.data.fields.resourceType')">
            <UiSelect v-model="createDataPolicyForm.resourceType" :options="dataResourceTypeOptions" />
          </UiField>
          <UiField :label="t('accessControl.policies.data.fields.scopeType')">
            <UiSelect v-model="createDataPolicyForm.scopeType" :options="scopeTypeOptions" />
          </UiField>
          <UiField :label="t('accessControl.policies.data.fields.effect')">
            <UiSelect v-model="createDataPolicyForm.effect" :options="policyEffectOptions" />
          </UiField>
        </div>

        <div class="grid gap-3 md:grid-cols-3">
          <UiField :label="t('accessControl.policies.data.fields.projectIds')" :hint="t('accessControl.policies.data.hints.projectIds')">
            <UiInput v-model="createDataPolicyForm.projectIdsText" />
          </UiField>
          <UiField :label="t('accessControl.policies.data.fields.tags')" :hint="t('accessControl.policies.data.hints.tags')">
            <UiInput v-model="createDataPolicyForm.tagsText" />
          </UiField>
          <UiField :label="t('accessControl.policies.data.fields.classifications')" :hint="t('accessControl.policies.data.hints.classifications')">
            <UiInput v-model="createDataPolicyForm.classificationsText" />
          </UiField>
        </div>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createDataPolicyDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton :loading="savingDataPolicy" @click="handleCreateDataPolicy">
          {{ t('accessControl.policies.data.actions.create') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="bulkDeleteResourcePoliciesDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.resourcePolicies') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeleteResourcePoliciesDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedResourcePoliciesForDelete.length,
          entity: t('accessControl.common.entities.resourcePolicies'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeleteResourcePoliciesDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingSelectedResourcePolicies"
          data-testid="access-control-policies-resources-bulk-delete-confirm"
          @click="handleBulkDeleteResourcePolicies"
        >
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createResourcePolicyDialogOpen"
      :title="t('accessControl.policies.resources.dialogTitle')"
      :description="t('accessControl.policies.resources.dialogDescription')"
      @update:open="createResourcePolicyDialogOpen = $event"
    >
      <div class="grid gap-3 md:grid-cols-2">
        <UiField :label="t('accessControl.policies.resources.fields.subjectType')">
          <UiSelect v-model="createResourcePolicyForm.subjectType" :options="subjectTypeOptions" />
        </UiField>
        <UiField :label="t('accessControl.policies.resources.fields.subject')">
          <UiSelect v-model="createResourcePolicyForm.subjectId" :options="createResourcePolicySubjectOptions" />
        </UiField>
        <UiField :label="t('accessControl.policies.resources.fields.resourceType')">
          <UiSelect v-model="createResourcePolicyForm.resourceType" :options="resourceTypeOptions" />
        </UiField>
        <UiField :label="t('accessControl.policies.resources.fields.resource')">
          <UiSelect v-model="createResourcePolicyForm.resourceId" :options="createResourceChoices" />
        </UiField>
        <UiField :label="t('accessControl.policies.resources.fields.action')">
          <UiInput v-model="createResourcePolicyForm.action" />
        </UiField>
        <UiField :label="t('accessControl.policies.resources.fields.effect')">
          <UiSelect v-model="createResourcePolicyForm.effect" :options="policyEffectOptions" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createResourcePolicyDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton :loading="savingResourcePolicy" @click="handleCreateResourcePolicy">
          {{ t('accessControl.policies.resources.actions.create') }}
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>
